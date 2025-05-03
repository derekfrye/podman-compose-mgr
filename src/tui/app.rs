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
    
    // Run the app and handle cleanup on exit or error
    let res = run_app(&mut terminal, app, tick_rate, args, logger);
    
    // Always restore terminal state, even on error
    let cleanup_result = cleanup_terminal(&mut terminal);
    
    // Handle any errors
    if let Err(err) = res {
        logger.warn(&format!("Error in TUI: {}", err));
    }
    
    // If cleanup failed but the app was ok, return that error
    cleanup_result?;

    Ok(())
}

// Separate function for terminal cleanup to ensure it always happens
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

fn run_app<B: Backend + std::io::Write>(
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