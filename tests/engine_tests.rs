use std::path::PathBuf;
use std::time::Duration;
use mtui::engine::commands::EngineCommand;
use mtui::engine::events::{LoopMode, PlaybackStatus};
use mtui::engine::playlist::Playlist;
use mtui::engine::track::Track;
use mtui::engine::{AudioEngine, MockBackend};
use mtui::ui::hitmap::{TouchHitMap, UiAction};
use ratatui::layout::Rect;

#[test]
fn test_track_parsing_and_extension_detection() {
    let path = PathBuf::from("/sdcard/Music/System Of A Down - Chop Suey!.flac");
    let track = Track::from_path(&path);

    assert_eq!(track.artist, Some("System Of A Down".to_string()));
    assert_eq!(track.title, "Chop Suey!");
    assert_eq!(track.display_name(), "System Of A Down - Chop Suey!");

    assert!(Track::is_audio_file(&path));
    assert!(Track::is_audio_file("test.mp3"));
    assert!(Track::is_audio_file("test.opus"));
    assert!(Track::is_audio_file("test.m4a"));
    assert!(!Track::is_audio_file("test.txt"));
    assert!(!Track::is_audio_file("test.jpg"));
}

#[test]
fn test_playlist_queue_and_loop_modes() {
    let mut pl = Playlist::new();
    let t1 = Track::from_path("/music/song1.mp3");
    let t2 = Track::from_path("/music/song2.mp3");
    let t3 = Track::from_path("/music/song3.mp3");

    pl.add_tracks(vec![t1.clone(), t2.clone(), t3.clone()]);
    assert_eq!(pl.tracks().len(), 3);

    // Initial selection
    pl.select_index(0);
    assert_eq!(pl.current_index(), Some(0));

    // Next without loop
    assert_eq!(pl.next(), Some(1));
    assert_eq!(pl.next(), Some(2));
    assert_eq!(pl.next(), None); // End of playlist

    // Loop All
    pl.set_loop_mode(LoopMode::All);
    assert_eq!(pl.next(), Some(0)); // Loops back to start!
    assert_eq!(pl.prev(), Some(2)); // Loops back to last!

    // Loop Track
    pl.set_loop_mode(LoopMode::Track);
    assert_eq!(pl.next(), Some(2)); // Stays on same track!
    assert_eq!(pl.prev(), Some(2));

    // Remove item
    pl.remove_index(1);
    assert_eq!(pl.tracks().len(), 2);
}

#[test]
fn test_playlist_shuffle() {
    let mut pl = Playlist::new();
    for i in 1..=10 {
        pl.add_track(Track::from_path(format!("/music/track{}.mp3", i)));
    }

    pl.select_index(0);
    pl.set_shuffle(true);

    assert!(pl.is_shuffle());
    // Should be able to advance through tracks
    let mut visited = Vec::new();
    visited.push(pl.current_index().unwrap());

    for _ in 0..9 {
        if let Some(next_idx) = pl.next() {
            visited.push(next_idx);
        }
    }

    assert_eq!(visited.len(), 10);
    // All original tracks are visited
    for i in 0..10 {
        assert!(visited.contains(&i));
    }
}

#[test]
fn test_audio_engine_commands_and_state_emission() {
    let mock = MockBackend::new();
    let (engine, state_rx) = AudioEngine::start(mock);

    // Initial state emitted on start
    let init_state = state_rx.recv_timeout(Duration::from_millis(500)).expect("Initial state");
    assert_eq!(init_state.queue.len(), 0);

    // Add track
    let track = Track::from_path("/music/test.flac");
    engine.send(EngineCommand::AddTrack(track.clone()));

    // Wait for state after AddTrack
    let state = state_rx.recv_timeout(Duration::from_millis(500)).expect("State snapshot received");
    assert_eq!(state.queue.len(), 1);

    // Play
    engine.send(EngineCommand::Play);
    let state = state_rx.recv_timeout(Duration::from_millis(500)).expect("State snapshot received");
    assert_eq!(state.status, PlaybackStatus::Playing);

    // Pause
    engine.send(EngineCommand::Pause);
    let state = state_rx.recv_timeout(Duration::from_millis(500)).expect("State snapshot received");
    assert_eq!(state.status, PlaybackStatus::Paused);

    // Volume adjustment
    engine.send(EngineCommand::SetVolume(45));
    let state = state_rx.recv_timeout(Duration::from_millis(500)).expect("State snapshot received");
    assert_eq!(state.volume, 45);

    // Stop and quit
    engine.send(EngineCommand::Quit);
}

#[test]
fn test_touch_hitmap_resolution() {
    let mut hitmap = TouchHitMap::new();

    // Register button at (x: 10, y: 5, w: 8, h: 1)
    let btn_rect = Rect::new(10, 5, 8, 1);
    hitmap.register_button(btn_rect, UiAction::SwitchTab(1));

    // Register progress bar at (x: 0, y: 10, w: 100, h: 1)
    let bar_rect = Rect::new(0, 10, 100, 1);
    hitmap.register_progress_bar(bar_rect);

    // Register volume slider at (x: 0, y: 12, w: 50, h: 1)
    let vol_rect = Rect::new(0, 12, 50, 1);
    hitmap.register_volume_bar(vol_rect);

    // Tap on button
    assert_eq!(hitmap.resolve_tap(12, 5), Some(UiAction::SwitchTab(1)));
    // Tap outside button
    assert_eq!(hitmap.resolve_tap(5, 5), None);
    assert_eq!(hitmap.resolve_tap(12, 6), None);

    // Tap on progress bar at 50%
    match hitmap.resolve_tap(50, 10) {
        Some(UiAction::Engine(EngineCommand::SeekPercent(pct))) => {
            assert!((pct - 0.5).abs() < 0.05);
        }
        other => panic!("Expected SeekPercent, got {:?}", other),
    }

    // Tap on volume bar at 80% (40 / 50 = 0.8)
    match hitmap.resolve_tap(40, 12) {
        Some(UiAction::Engine(EngineCommand::SetVolume(vol))) => {
            assert_eq!(vol, 80);
        }
        other => panic!("Expected SetVolume, got {:?}", other),
    }
}
