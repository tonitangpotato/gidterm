//! Live dashboard with real-time updates and semantic metrics

use crate::app::App;
use crate::semantic::MetricValue;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
    Frame,
};

/// Render the live dashboard
pub fn render_live_dashboard(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),   // Task list
            Constraint::Length(12), // Selected task output + metrics
            Constraint::Length(3),  // Footer
        ])
        .split(f.area());

    render_header(f, app, chunks[0]);
    render_task_list(f, app, chunks[1]);
    render_task_detail(f, app, chunks[2]);
    render_footer(f, chunks[3]);
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let graph = app.scheduler.graph();

    let title = if app.workspace_mode {
        format!(
            "🌐 Workspace ({} projects) - GidTerm",
            app.project_names.len()
        )
    } else if let Some(metadata) = &graph.metadata {
        format!("📊 {} - GidTerm", metadata.project)
    } else {
        "📊 GidTerm".to_string()
    };

    // Count task statuses
    let total = graph.all_tasks().len();
    let running = app.scheduler.get_running().len();
    let done = graph
        .all_tasks()
        .values()
        .filter(|t| t.status == "done")
        .count();
    let failed = graph
        .all_tasks()
        .values()
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
    let mut items: Vec<ListItem> = Vec::new();
    let mut flat_idx = 0usize;

    if app.workspace_mode {
        let tasks_by_project = app.get_tasks_by_project();

        for project_name in &app.project_names {
            // Project header
            let project_header = Line::from(vec![Span::styled(
                format!("📁 {}", project_name),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )]);
            items.push(ListItem::new(project_header));

            if let Some(task_ids) = tasks_by_project.get(project_name) {
                for task_id in task_ids {
                    let item = render_task_item(app, task_id, flat_idx);
                    items.push(item);
                    flat_idx += 1;
                }
            }

            // Spacer
            items.push(ListItem::new(Line::from("")));
        }
    } else {
        let task_ids = app.get_task_ids();
        for (idx, task_id) in task_ids.iter().enumerate() {
            let item = render_task_item(app, task_id, idx);
            items.push(item);
        }
    }

    let task_list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Tasks (↑↓ select, k kill, q quit)"),
    );

    f.render_widget(task_list, area);
}

fn render_task_item<'a>(app: &'a App, task_id: &str, idx: usize) -> ListItem<'a> {
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

    let priority_badge = task
        .priority
        .as_ref()
        .map(|p| match p.as_str() {
            "critical" => " 🔴",
            "high" => " 🟡",
            "medium" => " 🔵",
            _ => "",
        })
        .unwrap_or("");

    // Output line count
    let output_count = app
        .task_outputs
        .get(task_id)
        .map(|lines| format!(" ({}L)", lines.len()))
        .unwrap_or_default();

    // Display name — strip project prefix in workspace mode
    let display_name = if app.workspace_mode {
        task_id.split(':').nth(1).unwrap_or(task_id)
    } else {
        task_id
    };

    // Semantic metrics summary
    let metrics_summary = if let Some(metrics) = app.get_task_metrics(task_id) {
        let mut parts = Vec::new();

        if metrics.progress > 0.0 {
            parts.push(format!("{}%", (metrics.progress * 100.0) as u32));
        }

        for (key, value) in &metrics.metrics {
            match value {
                MetricValue::Float(v) => {
                    if key == "loss" || key == "accuracy" || key == "learning_rate" {
                        parts.push(format!("{}: {:.4}", key, v));
                    }
                }
                MetricValue::Int(v) => {
                    if key == "epoch" {
                        if let Some(MetricValue::Int(total)) = metrics.metrics.get("total_epochs") {
                            parts.push(format!("ep {}/{}", v, total));
                        }
                    }
                }
                _ => {}
            }
        }

        if !metrics.errors.is_empty() {
            parts.push(format!("⚠ {} errors", metrics.errors.len()));
        }

        if parts.is_empty() {
            String::new()
        } else {
            format!(" │ {}", parts.join(" │ "))
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
        Span::raw("  "),
        Span::raw(format!("{} ", status_icon)),
        Span::styled(
            display_name.to_string(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(priority_badge.to_string()),
        Span::styled(format!(" [{}]", task.status), Style::default().fg(status_color)),
        Span::styled(output_count, Style::default().fg(Color::DarkGray)),
        Span::styled(metrics_summary, Style::default().fg(Color::Cyan)),
    ]);

    ListItem::new(line).style(style)
}

fn render_task_detail(f: &mut Frame, app: &App, area: Rect) {
    let task_ids = app.get_task_ids();

    if task_ids.is_empty() || app.selected_task >= task_ids.len() {
        let empty = Paragraph::new("No task selected")
            .block(Block::default().borders(Borders::ALL).title("Detail"));
        f.render_widget(empty, area);
        return;
    }

    let task_id = &task_ids[app.selected_task];

    // Split area: progress gauge (if available) + output
    let has_progress = app
        .get_task_metrics(task_id)
        .map(|m| m.progress > 0.0)
        .unwrap_or(false);

    if has_progress {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(4)])
            .split(area);

        // Progress gauge
        let metrics = app.get_task_metrics(task_id).unwrap();
        let pct = (metrics.progress * 100.0) as u16;

        let label = if let Some(MetricValue::Int(epoch)) = metrics.metrics.get("epoch") {
            if let Some(MetricValue::Int(total)) = metrics.metrics.get("total_epochs") {
                format!("{}% (Epoch {}/{})", pct, epoch, total)
            } else {
                format!("{}%", pct)
            }
        } else {
            format!("{}%", pct)
        };

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Progress: {}", task_id)),
            )
            .gauge_style(
                Style::default()
                    .fg(Color::Green)
                    .bg(Color::DarkGray),
            )
            .percent(pct)
            .label(label);

        f.render_widget(gauge, chunks[0]);
        render_output_panel(f, app, task_id, chunks[1]);
    } else {
        render_output_panel(f, app, task_id, area);
    }
}

fn render_output_panel(f: &mut Frame, app: &App, task_id: &str, area: Rect) {
    let height = area.height.saturating_sub(2) as usize; // minus borders
    let output_lines = app.get_task_output(task_id, height);

    let text = if output_lines.is_empty() {
        "(no output yet)".to_string()
    } else {
        output_lines.join("\n")
    };

    let output = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Output: {}", task_id)),
        )
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White));

    f.render_widget(output, area);
}

fn render_footer(f: &mut Frame, area: Rect) {
    let help_text = "q: Quit │ k: Kill task │ r: Refresh │ ↑↓: Select";

    let footer = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::DarkGray));

    f.render_widget(footer, area);
}
