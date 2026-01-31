//! UI layer - TUI and views

mod dashboard;

pub use dashboard::DashboardView;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io;

/// Main TUI controller
pub struct TUI {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TUI {
    /// Create a new TUI
    pub fn new() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self { terminal })
    }

    /// Run the TUI event loop
    pub fn run<F>(&mut self, mut render_fn: F) -> Result<()>
    where
        F: FnMut(&mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<bool>,
    {
        loop {
            // Render
            let should_quit = render_fn(&mut self.terminal)?;
            if should_quit {
                break;
            }

            // Handle events
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }
}

impl Drop for TUI {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = self.terminal.show_cursor();
    }
}
