use super::handlers::update_with_services;
use super::state::{App, Env, LoopChans, Msg, Services};
use crate::Args;
use crate::app::AppCore;
use crate::ports::InterruptPort;
use crate::utils::log_utils::Logger;
use crossbeam_channel as xchan;
use crossterm::{
    event::Event,
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
};
use std::io;
use std::sync::Arc;
use std::time::Duration;

const TICK_RATE_MS: u64 = 250;

/// Launch the TUI application loop and block until the user exits.
///
/// # Errors
/// Returns an error if setting up the terminal, running the UI loop, or cleaning up the
/// terminal I/O resources fails.
pub fn run(args: &Args, logger: &Logger) -> io::Result<()> {
    let mut terminal = setup_terminal()?;
    let mut app = App::new();
    app.set_root_path(args.path.clone());
    app.auto_rebuild_all = args.tui_rebuild_all;

    let (tx, rx) = xchan::unbounded::<Msg>();
    let services = build_services(args, tx.clone())?;
    let interrupt_rx = spawn_interrupt_listener();
    let tick_rx = xchan::tick(Duration::from_millis(TICK_RATE_MS));

    let _ = tx.send(Msg::Init);
    spawn_key_forwarder(tx.clone());

    let chans = LoopChans {
        rx: &rx,
        interrupt_rx: &interrupt_rx,
        tick_rx: Some(&tick_rx),
    };
    let env = Env {
        args,
        logger,
        services: &services,
    };

    let run_result = run_loop(&mut terminal, &mut app, &chans, &env);
    let cleanup_result = cleanup_terminal(&mut terminal);

    if let Err(err) = run_result {
        logger.warn(&format!("Error in TUI: {err}"));
    }

    cleanup_result?;
    Ok(())
}

fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn build_services(args: &Args, tx: xchan::Sender<Msg>) -> io::Result<Services> {
    let discovery = Arc::new(crate::infra::discovery_adapter::FsDiscovery);
    let podman: Arc<dyn crate::ports::PodmanPort> =
        if let Some(json) = &args.tui_simulate_podman_input_json {
            crate::tui::podman_from_json(json.as_path())
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?
        } else {
            Arc::new(crate::infra::podman_adapter::PodmanCli)
        };
    let app_core = Arc::new(AppCore::new(discovery, podman));
    let working_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    Ok(Services {
        core: app_core,
        root: args.path.clone(),
        include: args.include_path_patterns.clone(),
        exclude: args.exclude_path_patterns.clone(),
        tx,
        args: args.clone(),
        working_dir,
    })
}

fn spawn_key_forwarder(tx: xchan::Sender<Msg>) {
    std::thread::spawn(move || {
        loop {
            if let Ok(Event::Key(key)) = crossterm::event::read() {
                let _ = tx.send(Msg::Key(key));
            }
        }
    });
}

fn spawn_interrupt_listener() -> xchan::Receiver<()> {
    let interrupt_std =
        Box::new(crate::infra::interrupt_adapter::CtrlcInterruptor::new()).subscribe();
    let (tx, rx) = xchan::bounded::<()>(0);
    std::thread::spawn(move || {
        let _ = interrupt_std.recv();
        let _ = tx.send(());
    });
    rx
}

/// Drive the TUI event loop until the application requests shutdown.
///
/// # Errors
/// Returns an error if drawing to the terminal fails or if terminal cleanup encounters an I/O error.
///
/// # Panics
/// Panics if the environment omits the tick channel required to drive periodic updates.
pub fn run_loop<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    chans: &LoopChans<'_>,
    env: &Env<'_>,
) -> io::Result<()> {
    env.logger.debug("TUI is running");

    while !app.should_quit {
        xchan::select! {
            recv(chans.interrupt_rx) -> _ => update_with_services(app, Msg::Interrupt, Some(env.services)),
            recv(chans.rx) -> msg => if let Ok(msg) = msg { update_with_services(app, msg, Some(env.services)); },
            recv(chans.tick_rx.expect("tick channel must be provided")) -> _ => update_with_services(app, Msg::Tick, Some(env.services)),
        }

        terminal.draw(|frame| crate::tui::ui::draw(frame, app, env.args))?;
    }

    Ok(())
}

fn cleanup_terminal<B: Backend + std::io::Write>(terminal: &mut Terminal<B>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
