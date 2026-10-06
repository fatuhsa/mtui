use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use mtui::engine::commands::EngineCommand;
use mtui::ui::{AppTab, AppUi, NowPlayingView, UiAction};
use mtui::ui::widgets::Marquee;

#[test]
fn test_flexible_art_size_calculation() {
    // Very small screen: cannot fit cover art
    let (w, h) = NowPlayingView::calculate_art_size(8, 6);
    assert_eq!((w, h), (0, 0));

    // Standard Termux portrait mobile screen (e.g. 40x24 outer -> ~38x18 inner)
    let (w, h) = NowPlayingView::calculate_art_size(38, 18);
    assert!(w > 0 && h > 0);
    assert!(w <= 36, "Cover width must fit within inner width");
    assert!(h <= 18 - 8, "Cover height must leave room for controls");
    assert_eq!(w, h * 2, "Aspect ratio must be approximately 2:1 cells");

    // Large desktop terminal (e.g. 100x40 inner)
    let (w, h) = NowPlayingView::calculate_art_size(100, 40);
    assert_eq!(h, 14, "Cover height clamps to balanced max size 14");
    assert_eq!(w, 28, "Cover width is 2 * height");

    // Narrow terminal (width 16, height 30)
    let (w, h) = NowPlayingView::calculate_art_size(16, 30);
    assert!(w <= 14);
    assert_eq!(h, (w / 2).max(3));
}

#[test]
fn test_tab_switch_sets_clear_and_dirty_flags() {
    let mut app = AppUi::new();
    let (tx, _rx) = std::sync::mpsc::channel::<EngineCommand>();

    assert_eq!(app.current_tab, AppTab::NowPlaying);
    assert!(!app.needs_terminal_clear);

    // Switch to Files tab
    app.process_action(UiAction::SwitchTab(1), &tx);
    assert_eq!(app.current_tab, AppTab::Files);
    assert!(app.needs_terminal_clear, "Switching tab must request terminal clear");

    // Reset clear flag like main loop does
    app.needs_terminal_clear = false;

    // Switch back to NowPlaying tab
    app.process_action(UiAction::SwitchTab(0), &tx);
    assert_eq!(app.current_tab, AppTab::NowPlaying);
    assert!(app.needs_terminal_clear, "Returning to NowPlaying must request terminal clear");
    assert!(app.cover_mgr.is_dirty, "Returning to NowPlaying must mark cover dirty");
}

#[test]
fn test_marquee_render_centered() {
    let area = Rect::new(0, 0, 20, 1);
    let mut buf = Buffer::empty(area);
    let text = "Hello"; // 5 chars in 20-col box -> offset should be (20 - 5) / 2 = 7

    Marquee::render_centered(text, area, 0, &mut buf, Style::default());

    // Cells before index 7 should be empty
    for x in 0..7 {
        assert_eq!(buf.cell((x, 0)).unwrap().symbol(), " ");
    }
    // Cells 7..12 should be "Hello"
    assert_eq!(buf.cell((7, 0)).unwrap().symbol(), "H");
    assert_eq!(buf.cell((8, 0)).unwrap().symbol(), "e");
    assert_eq!(buf.cell((9, 0)).unwrap().symbol(), "l");
    assert_eq!(buf.cell((10, 0)).unwrap().symbol(), "l");
    assert_eq!(buf.cell((11, 0)).unwrap().symbol(), "o");
}

#[test]
fn test_touch_button_unicode_rendering() {
    use mtui::ui::widgets::TouchButton;
    use mtui::ui::hitmap::TouchHitMap;

    let area = Rect::new(0, 0, 10, 1);
    let mut buf = Buffer::empty(area);
    let mut hitmap = TouchHitMap::new();

    // "[  Prev ]" has 10 characters (including 3-byte glyph ).
    // In a 10-column box, the entire string must fit with closing bracket ']'!
    let btn = TouchButton::new(" Prev", UiAction::Quit);
    btn.render_and_register(area, &mut buf, &mut hitmap);

    let rendered: String = (0..10).map(|x| buf.cell((x, 0)).unwrap().symbol()).collect();
    assert_eq!(rendered, "[  Prev ]", "Button must not cut off multi-byte characters or closing brackets");

    // Minimize button in 5-column box: "[  ]"
    let min_area = Rect::new(0, 0, 5, 1);
    let mut min_buf = Buffer::empty(min_area);
    let min_btn = TouchButton::new("", UiAction::Minimize);
    min_btn.render_and_register(min_area, &mut min_buf, &mut hitmap);

    let min_rendered: String = (0..5).map(|x| min_buf.cell((x, 0)).unwrap().symbol()).collect();
    assert_eq!(min_rendered, "[  ]", "Minimize button must render [  ] fully");
}

#[test]
fn test_visualizer_beat_and_physics() {
    use mtui::ui::Visualizer;

    let mut vis = Visualizer::new();
    assert_eq!(vis.current_time(), 0.0);

    // Initial paused state
    vis.update_state(0.0, false, Some("/music/test.mp3"));
    let paused_bars = vis.render_bars(16);
    assert_eq!(paused_bars.chars().count(), 32, "16 bars with 1 space each = 32 chars");

    // Start playing at 10.0 seconds
    vis.update_state(10.0, true, Some("/music/test.mp3"));
    assert!(vis.current_time() >= 10.0);

    // Render 20 bars
    let bars1 = vis.render_bars(20);
    assert_eq!(bars1.chars().count(), 40, "20 bars with 1 space each = 40 chars");

    // Seeking to 45.0 seconds
    vis.update_state(45.0, true, Some("/music/test.mp3"));
    assert!(vis.current_time() >= 45.0 && vis.current_time() < 46.0);
}

#[test]
fn test_termux_backend_cursor_and_clear() {
    use mtui::ui::TermuxBackend;
    use ratatui::backend::Backend;
    use ratatui::layout::Position;
    use ratatui::Terminal;

    let output = Vec::<u8>::new();
    let mut backend = TermuxBackend::new(output);

    // Initial position is default (0, 0)
    let pos = backend.get_cursor_position().expect("Must get cursor position instantly");
    assert_eq!(pos, Position { x: 0, y: 0 });

    // Set cursor position updates in-memory tracked pos
    backend.set_cursor_position(Position { x: 15, y: 8 }).expect("Set cursor position");
    let pos2 = backend.get_cursor_position().expect("Must return updated position");
    assert_eq!(pos2, Position { x: 15, y: 8 });

    // Wrapping in Terminal and calling clear() must succeed without stdin CPR query
    let mut terminal = Terminal::new(backend).expect("Terminal init");
    let clear_result = terminal.clear();
    assert!(clear_result.is_ok(), "terminal.clear() must succeed instantly without crossterm CPR timeout");
}



