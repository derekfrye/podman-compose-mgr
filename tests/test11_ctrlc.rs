use std::time::{Duration, Instant};
use std::{path::PathBuf, sync::mpsc};

use podman_compose_mgr::Args;
use podman_compose_mgr::tui::app::{self, App};
use podman_compose_mgr::utils::log_utils::Logger;
use ratatui::{Terminal, backend::TestBackend};

// In-process test: TUI exits when receiving an interrupt via channel.
#[test]
fn tui_interrupt_exits_quickly() {
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).expect("terminal");

    let mut app = App::new();
    app.root_path = PathBuf::from("tests/test1");
    let args = Args {
        path: PathBuf::from("tests/test1"),
        ..Default::default()
    };
    let logger = Logger::new(0);

    // Scan results channel (unused in this test)
    let (_scan_tx, scan_rx) = mpsc::channel::<Vec<podman_compose_mgr::domain::DiscoveredImage>>();
    // Interrupt channel: send after a short delay
    let (int_tx, int_rx) = mpsc::channel::<()>();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        let _ = int_tx.send(());
    });

    let start = Instant::now();
    let res = app::run_loop(
        &mut terminal,
        &mut app,
        Duration::from_millis(16),
        &args,
        &logger,
        &scan_rx,
        &int_rx,
    );
    assert!(res.is_ok());
    assert!(app.should_quit);
    assert!(start.elapsed() < Duration::from_secs(2));
}

// In-process test: CLI traversal stops immediately on interrupt.
#[test]
fn cli_interrupt_stops_traversal() {
    use podman_compose_mgr::interfaces::{MockCommandHelper, MockReadInteractiveInputHelper};
    use podman_compose_mgr::walk_dirs::walk_dirs_with_helpers_and_interrupt;

    let args = Args {
        path: PathBuf::from("tests/test1"),
        ..Default::default()
    };
    let logger = Logger::new(0);

    // Send interrupt before traversal starts
    let (tx, rx) = mpsc::channel::<()>();
    tx.send(()).unwrap();

    // Mocks won't be exercised because interrupt comes first
    let cmd_helper = MockCommandHelper::new();
    let read_val_helper = MockReadInteractiveInputHelper::new();

    let res = walk_dirs_with_helpers_and_interrupt(
        &args,
        &cmd_helper,
        &read_val_helper,
        &logger,
        Some(&rx),
    );
    assert!(res.is_ok());
}
