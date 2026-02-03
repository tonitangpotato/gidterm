//! UI layer - TUI and views

mod dashboard;
mod live;
pub mod views;

pub use dashboard::DashboardView;
pub use live::render_live_dashboard;
pub use views::comparison::render_comparison_view;
pub use views::graph::render_graph_view;
pub use views::project_overview::render_project_overview;
pub use views::terminal::render_terminal_view;

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
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

    /// Get mutable reference to terminal
    pub fn terminal(&mut self) -> &mut Terminal<CrosstermBackend<io::Stdout>> {
        &mut self.terminal
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
