use std::time::{Duration, Instant};
use std::{path::PathBuf, sync::mpsc};

use podman_compose_mgr::Args;
use podman_compose_mgr::tui::app::{self, App, Services, Msg};
use crossbeam_channel as xchan;
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

    // Message channel (unused except for init we won't send)
    let (tx, rx) = xchan::unbounded::<Msg>();
    // Interrupt channel: send after a short delay
    let (int_tx, int_rx_std) = mpsc::channel::<()>();
    let (int_c_tx, int_c_rx) = xchan::bounded::<()>(0);
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        let _ = int_tx.send(());
    });
    std::thread::spawn(move || { let _ = int_rx_std.recv(); let _ = int_c_tx.send(()); });

    let start = Instant::now();
    // Minimal services; effects won't run in this test
    let discovery = std::sync::Arc::new(podman_compose_mgr::infra::discovery_adapter::FsDiscovery);
    let podman = std::sync::Arc::new(podman_compose_mgr::infra::podman_adapter::PodmanCli);
    let core = std::sync::Arc::new(podman_compose_mgr::app::AppCore::new(discovery, podman));
    let services = Services { core, root: args.path.clone(), include: vec![], exclude: vec![], tx };

    let chans = app::LoopChans { rx: &rx, interrupt_rx: &int_c_rx, tick_rx: Some(&xchan::tick(Duration::from_millis(16))) };
    let env = app::Env { args: &args, logger: &logger, services: &services };
    let res = app::run_loop(&mut terminal, &mut app, &chans, &env);
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
