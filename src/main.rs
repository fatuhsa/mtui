use std::io::{self, stdout, Write};
use std::time::{Duration, Instant};
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use mtui::engine::{AudioEngine, EngineCommand, EngineStateSnapshot, MockBackend, MpvBackend};
use mtui::ui::{AppUi, UiAction};

fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn restore_terminal(mut terminal: Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Attempt to restore terminal on panic
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));
}

fn main() -> anyhow::Result<()> {
    install_panic_hook();

    let mut terminal = setup_terminal()?;

    // Initialize audio engine
    // Prefer MpvBackend, fallback to MockBackend if mpv fails to spawn
    let (engine, state_rx) = match MpvBackend::new() {
        Ok(mpv) => AudioEngine::start(mpv),
        Err(e) => {
            eprintln!("Warning: Failed to initialize MPV ({}), falling back to mock backend", e);
            AudioEngine::start(MockBackend::new())
        }
    };

    let mut app_ui = AppUi::new();
    let mut current_state = EngineStateSnapshot::default();
    let engine_tx = engine.command_sender();

    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(150);
    let mut should_quit = false;

    while !should_quit {
        // Check for updated engine state
        while let Ok(new_state) = state_rx.try_recv() {
            let track_changed = current_state.current_track.as_ref().map(|t| &t.path)
                != new_state.current_track.as_ref().map(|t| &t.path);
            if track_changed {
                app_ui.needs_terminal_clear = true;
                app_ui.cover_mgr.mark_dirty();
            }
            current_state = new_state;
        }

        // If a terminal clear was requested (e.g. tab change, resize, new track),
        // wipe the terminal to eradicate old Sixel/iTerm2 graphics planes
        if app_ui.needs_terminal_clear {
            terminal.clear()?;
            app_ui.needs_terminal_clear = false;
        }

        // Render current UI
        terminal.draw(|f| {
            app_ui.draw(f, &current_state);
        })?;

        // If a terminal graphic (Sixel, iTerm2, Blocks) was emitted, output directly at target cell
        if let Some((x, y, ref seq)) = app_ui.pending_graphic.take() {
            print!("\x1b[{};{}H{}", y + 1, x + 1, seq);
            let _ = stdout().flush();
        }

        // Poll for inputs (touch mouse events, keyboard, or resize)
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_millis(20));

        if event::poll(timeout)? {
            match event::read()? {
                Event::Mouse(mouse_event) => {
                    if let Some(action) = app_ui.handle_mouse_event(mouse_event) {
                        if action == UiAction::Minimize {
                            // Minimize: suspend to shell while audio continues playing in background
                            disable_raw_mode()?;
                            execute!(
                                terminal.backend_mut(),
                                LeaveAlternateScreen,
                                DisableMouseCapture
                            )?;
                            terminal.show_cursor()?;

                            println!("\n\x1b[1;36m[mtui minimized]\x1b[0m Music is playing in background.");
                            println!("Type \x1b[1;32mfg\x1b[0m and press Enter to restore mtui.\n");
                            let _ = stdout().flush();

                            #[cfg(unix)]
                            unsafe {
                                libc::raise(libc::SIGTSTP);
                            }

                            // When resumed via `fg` (SIGCONT):
                            enable_raw_mode()?;
                            execute!(
                                terminal.backend_mut(),
                                EnterAlternateScreen,
                                EnableMouseCapture
                            )?;
                            terminal.hide_cursor()?;
                            terminal.clear()?;
                            app_ui.cover_mgr.mark_dirty();
                        } else if app_ui.process_action(action, &engine_tx) {
                            should_quit = true;
                        }
                    }
                }
                Event::Key(key_event) => {
                    // Always allow Ctrl+C to exit
                    if key_event.modifiers.contains(KeyModifiers::CONTROL)
                        && key_event.code == KeyCode::Char('c')
                    {
                        should_quit = true;
                    } else if let Some(action) = app_ui.handle_key_event(key_event) {
                        if action == UiAction::Minimize {
                            disable_raw_mode()?;
                            execute!(
                                terminal.backend_mut(),
                                LeaveAlternateScreen,
                                DisableMouseCapture
                            )?;
                            terminal.show_cursor()?;

                            println!("\n\x1b[1;36m[mtui minimized]\x1b[0m Music is playing in background.");
                            println!("Type \x1b[1;32mfg\x1b[0m and press Enter to restore mtui.\n");
                            let _ = stdout().flush();

                            #[cfg(unix)]
                            unsafe {
                                libc::raise(libc::SIGTSTP);
                            }

                            enable_raw_mode()?;
                            execute!(
                                terminal.backend_mut(),
                                EnterAlternateScreen,
                                EnableMouseCapture
                            )?;
                            terminal.hide_cursor()?;
                            terminal.clear()?;
                            app_ui.cover_mgr.mark_dirty();
                        } else if app_ui.process_action(action, &engine_tx) {
                            should_quit = true;
                        }
                    }
                }
                Event::Resize(_, _) => {
                    // Terminal resized: redrawn automatically on next iteration
                    app_ui.needs_terminal_clear = true;
                    app_ui.cover_mgr.mark_dirty();
                }
                _ => {}
            }
        }

        // Advance animation/marquee tick
        if last_tick.elapsed() >= tick_rate {
            app_ui.on_tick();
            last_tick = Instant::now();
        }
    }

    // Clean shutdown
    let _ = engine_tx.send(EngineCommand::Quit);
    restore_terminal(terminal)?;

    Ok(())
}
