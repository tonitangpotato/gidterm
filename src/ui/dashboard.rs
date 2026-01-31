//! Dashboard view - Unified task status display

use crate::core::Graph;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

/// Dashboard view showing all tasks
pub struct DashboardView;

impl DashboardView {
    /// Render the dashboard
    pub fn render(f: &mut Frame, graph: &Graph, area: Rect) {
        // Split into header and content
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(area);

        // Render header
        Self::render_header(f, graph, chunks[0]);

        // Render task list
        Self::render_tasks(f, graph, chunks[1]);
    }

    fn render_header(f: &mut Frame, graph: &Graph, area: Rect) {
        let title = if let Some(metadata) = &graph.metadata {
            format!("📊 {} - GidTerm", metadata.project)
        } else {
            "📊 GidTerm".to_string()
        };

        let header = Paragraph::new(title)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Cyan));

        f.render_widget(header, area);
    }

    fn render_tasks(f: &mut Frame, graph: &Graph, area: Rect) {
        let tasks: Vec<ListItem> = graph
            .all_tasks()
            .iter()
            .map(|(id, task)| {
                let status_icon = match task.status.as_str() {
                    "done" => "✓",
                    "in-progress" => "⚙",
                    "failed" => "✗",
                    _ => "□",
                };

                let status_color = match task.status.as_str() {
                    "done" => Color::Green,
                    "in-progress" => Color::Yellow,
                    "failed" => Color::Red,
                    _ => Color::Gray,
                };

                let priority_badge = task.priority.as_ref().map(|p| match p.as_str() {
                    "critical" => "🔴",
                    "high" => "🟡",
                    "medium" => "🔵",
                    _ => "⚪",
                }).unwrap_or("");

                let deps_info = if let Some(deps) = &task.depends_on {
                    if deps.is_empty() {
                        String::new()
                    } else {
                        format!(" (depends: {})", deps.join(", "))
                    }
                } else {
                    String::new()
                };

                let line = Line::from(vec![
                    Span::raw(format!("{} ", status_icon)),
                    Span::styled(id, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    Span::raw(format!(" {}", priority_badge)),
                    Span::styled(
                        format!(" [{}]", task.status),
                        Style::default().fg(status_color),
                    ),
                    Span::styled(deps_info, Style::default().fg(Color::DarkGray)),
                ]);

                ListItem::new(line)
            })
            .collect();

        let task_list = List::new(tasks)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Tasks")
            );

        f.render_widget(task_list, area);
    }
}
