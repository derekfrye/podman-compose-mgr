use super::handlers::update_with_services;
use super::state::{App, Env, LoopChans, Msg, Services};
use crate::Args;
use crate::app::AppCore;
use crate::ports::InterruptPort;
use crate::utils::log_utils::Logger;
use crossbeam_channel as xchan;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event},
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

pub fn run(args: &Args, logger: &Logger) -> io::Result<()> {
    let mut terminal = setup_terminal()?;
    let mut app = App::new();
    app.set_root_path(args.path.clone());

    let (tx, rx) = xchan::unbounded::<Msg>();
    let services = build_services(args, tx.clone());
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
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

fn build_services(args: &Args, tx: xchan::Sender<Msg>) -> Services {
    let discovery = Arc::new(crate::infra::discovery_adapter::FsDiscovery);
    let podman = Arc::new(crate::infra::podman_adapter::PodmanCli);
    let app_core = Arc::new(AppCore::new(discovery, podman));
    Services {
        core: app_core,
        root: args.path.clone(),
        include: args.include_path_patterns.clone(),
        exclude: args.exclude_path_patterns.clone(),
        tx,
    }
}

fn spawn_key_forwarder(tx: xchan::Sender<Msg>) {
    std::thread::spawn(move || {
        loop {
            if let Ok(event) = crossterm::event::read() {
                if let Event::Key(key) = event {
                    let _ = tx.send(Msg::Key(key));
                }
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
            default => {}
        }

        terminal.draw(|frame| crate::tui::ui::draw(frame, app, env.args))?;
    }

    Ok(())
}

fn cleanup_terminal<B: Backend + std::io::Write>(terminal: &mut Terminal<B>) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
