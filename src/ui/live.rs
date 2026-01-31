//! Live dashboard with real-time updates

use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

/// Render the live dashboard
pub fn render_live_dashboard(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),      // Header
            Constraint::Min(10),         // Task list
            Constraint::Length(10),      // Selected task output
            Constraint::Length(3),       // Footer
        ])
        .split(f.area());

    render_header(f, app, chunks[0]);
    render_task_list(f, app, chunks[1]);
    render_task_output(f, app, chunks[2]);
    render_footer(f, chunks[3]);
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let graph = app.scheduler.graph();
    
    let title = if let Some(metadata) = &graph.metadata {
        format!("📊 {} - GidTerm (Live)", metadata.project)
    } else {
        "📊 GidTerm (Live)".to_string()
    };

    // Count task statuses
    let total = graph.all_tasks().len();
    let running = app.scheduler.get_running().len();
    let done = graph.all_tasks().values()
        .filter(|t| t.status == "done")
        .count();
    let failed = graph.all_tasks().values()
        .filter(|t| t.status == "failed")
        .count();

    let status_text = format!(
        "{} | Running: {} | Done: {} | Failed: {} | Total: {}",
        title, running, done, failed, total
    );

    let header = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Cyan));

    f.render_widget(header, area);
}

fn render_task_list(f: &mut Frame, app: &App, area: Rect) {
    let task_ids = app.get_task_ids();
    
    let items: Vec<ListItem> = task_ids
        .iter()
        .enumerate()
        .map(|(idx, task_id)| {
            let task = app.scheduler.graph().get_task(task_id).unwrap();
            
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

            let priority_badge = task.priority.as_ref()
                .map(|p| match p.as_str() {
                    "critical" => "🔴",
                    "high" => "🟡",
                    "medium" => "🔵",
                    _ => "⚪",
                })
                .unwrap_or("");

            // Show output line count if any
            let output_count = app.task_outputs.get(task_id)
                .map(|lines| format!(" ({}L)", lines.len()))
                .unwrap_or_default();

            let deps_info = if let Some(deps) = &task.depends_on {
                if deps.is_empty() {
                    String::new()
                } else {
                    format!(" ← {}", deps.join(", "))
                }
            } else {
                String::new()
            };

            // Highlight selected task
            let style = if idx == app.selected_task {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            let line = Line::from(vec![
                Span::raw(format!("{} ", status_icon)),
                Span::styled(
                    task_id.clone(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                ),
                Span::raw(format!(" {}", priority_badge)),
                Span::styled(
                    format!(" [{}]", task.status),
                    Style::default().fg(status_color),
                ),
                Span::styled(output_count, Style::default().fg(Color::Cyan)),
                Span::styled(deps_info, Style::default().fg(Color::DarkGray)),
            ]);

            ListItem::new(line).style(style)
        })
        .collect();

    let task_list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Tasks (↑↓ to select)")
    );

    f.render_widget(task_list, area);
}

fn render_task_output(f: &mut Frame, app: &App, area: Rect) {
    let task_ids = app.get_task_ids();
    
    if task_ids.is_empty() || app.selected_task >= task_ids.len() {
        let empty = Paragraph::new("No task selected")
            .block(Block::default().borders(Borders::ALL).title("Output"));
        f.render_widget(empty, area);
        return;
    }

    let task_id = &task_ids[app.selected_task];
    let output_lines = app.get_task_output(task_id, 8);

    let text = if output_lines.is_empty() {
        "(no output yet)".to_string()
    } else {
        output_lines.join("\n")
    };

    let output = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Output: {}", task_id))
        )
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White));

    f.render_widget(output, area);
}

fn render_footer(f: &mut Frame, area: Rect) {
    let help_text = "q: Quit | r: Refresh | ↑↓: Select task";
    
    let footer = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::DarkGray));

    f.render_widget(footer, area);
}
