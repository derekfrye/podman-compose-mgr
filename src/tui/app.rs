use crate::Args;
use crate::utils::log_utils::Logger;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::{
    io,
    time::{Duration, Instant},
};

use super::ui;

pub struct App {
    pub title: String,
    pub should_quit: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            title: "Podman Compose Manager".to_string(),
            should_quit: false,
        }
    }
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_key(&mut self, key: KeyCode) {
        if let KeyCode::Char('q') = key {
            self.should_quit = true;
        }
    }
}

pub fn run(args: &Args, logger: &Logger) -> io::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let app = App::new();
    let tick_rate = Duration::from_millis(250);
    let res = run_app(&mut terminal, app, tick_rate, args, logger);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        logger.warn(&format!("Error in TUI: {}", err));
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
    args: &Args,
    logger: &Logger,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    
    logger.debug("TUI is running");

    while !app.should_quit {
        terminal.draw(|f| ui::draw(f, &app, args))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                app.on_key(key.code);
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    Ok(())
}