//! Cross-task comparison view - compare metrics across multiple tasks

use crate::app::App;
use crate::core::GraphTaskStatus;
use crate::semantic::MetricValue;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
    Frame,
};

/// Render cross-task comparison table
pub fn render_comparison_view(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),   // Comparison table
            Constraint::Length(5), // Summary/recommendation
            Constraint::Length(3),  // Footer
        ])
        .split(f.area());

    render_header(f, app, chunks[0]);
    render_comparison_table(f, app, chunks[1]);
    render_summary(f, app, chunks[2]);
    render_footer(f, chunks[3]);
}

fn render_header(f: &mut Frame, _app: &App, area: ratatui::layout::Rect) {
    let header = Paragraph::new("Cross-Task Comparison")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Cyan));
    f.render_widget(header, area);
}

fn render_comparison_table(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let task_ids = app.get_task_ids();

    // Collect all metric keys across all tasks
    let mut all_metrics: Vec<String> = Vec::new();
    for task_id in &task_ids {
        if let Some(metrics) = app.get_task_metrics(task_id) {
            for key in metrics.metrics.keys() {
                if !all_metrics.contains(key) {
                    all_metrics.push(key.clone());
                }
            }
        }
    }
    all_metrics.sort();

    // Build header: Task | Status | Progress | ETA | <metric1> | <metric2> | ...
    let mut header_cells = vec![
        Cell::from("Task").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Status").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Progress").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("ETA").style(Style::default().add_modifier(Modifier::BOLD)),
    ];
    for metric_name in &all_metrics {
        header_cells.push(
            Cell::from(metric_name.as_str()).style(Style::default().add_modifier(Modifier::BOLD)),
        );
    }
    let header_row = Row::new(header_cells).height(1);

    // Find best values for highlighting
    let mut best_values: std::collections::HashMap<String, (f64, bool)> = std::collections::HashMap::new();
    for task_id in &task_ids {
        if let Some(metrics) = app.get_task_metrics(task_id) {
            for (key, value) in &metrics.metrics {
                if let Some(v) = value.as_float() {
                    let lower_is_better = key == "loss" || key == "errors" || key == "warnings";
                    let entry = best_values.entry(key.clone()).or_insert((v, lower_is_better));
                    if lower_is_better {
                        if v < entry.0 {
                            entry.0 = v;
                        }
                    } else if v > entry.0 {
                        entry.0 = v;
                    }
                }
            }
        }
    }

    // Build rows
    let rows: Vec<Row> = task_ids
        .iter()
        .filter(|id| {
            // Only show tasks with some metrics or that are running
            app.get_task_metrics(id).is_some()
                || app.scheduler.graph().get_task(id)
                    .map(|t| t.status == GraphTaskStatus::InProgress)
                    .unwrap_or(false)
        })
        .map(|task_id| {
            let task = app.scheduler.graph().get_task(task_id).unwrap();
            let metrics = app.get_task_metrics(task_id);

            let status_str = task.status.to_string();
            let progress_str = metrics
                .map(|m| format!("{:.0}%", m.progress * 100.0))
                .unwrap_or_else(|| "-".to_string());
            let eta_str = app.get_eta(task_id).unwrap_or_else(|| "-".to_string());

            let display_name = if app.workspace_mode {
                task_id.split(':').nth(1).unwrap_or(task_id)
            } else {
                task_id
            };

            let mut cells = vec![
                Cell::from(display_name.to_string()),
                Cell::from(status_str).style(Style::default().fg(match task.status {
                    GraphTaskStatus::Done => Color::Green,
                    GraphTaskStatus::InProgress => Color::Yellow,
                    GraphTaskStatus::Failed => Color::Red,
                    _ => Color::Gray,
                })),
                Cell::from(progress_str),
                Cell::from(eta_str),
            ];

            for metric_name in &all_metrics {
                let cell = if let Some(m) = metrics {
                    if let Some(value) = m.metrics.get(metric_name) {
                        let v_float = value.as_float();
                        let is_best = v_float.map(|v| {
                            best_values.get(metric_name)
                                .map(|(best, _)| (v - best).abs() < 0.0001)
                                .unwrap_or(false)
                        }).unwrap_or(false);

                        let text = match value {
                            MetricValue::Float(v) => format!("{:.4}", v),
                            MetricValue::Int(v) => format!("{}", v),
                            MetricValue::String(v) => v.clone(),
                            MetricValue::Bool(v) => format!("{}", v),
                        };

                        if is_best {
                            Cell::from(format!("{} *", text))
                                .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                        } else {
                            Cell::from(text)
                        }
                    } else {
                        Cell::from("-")
                    }
                } else {
                    Cell::from("-")
                };
                cells.push(cell);
            }

            Row::new(cells)
        })
        .collect();

    // Column widths
    let mut widths = vec![
        Constraint::Min(15),     // Task
        Constraint::Length(12),  // Status
        Constraint::Length(10),  // Progress
        Constraint::Length(10),  // ETA
    ];
    for _ in &all_metrics {
        widths.push(Constraint::Length(12));
    }

    let table = Table::new(rows, widths)
        .header(header_row)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Metrics (* = best)"),
        );

    f.render_widget(table, area);
}

fn render_summary(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let task_ids = app.get_task_ids();

    // Find task with best loss
    let mut best_loss: Option<(&str, f64)> = None;
    let mut best_acc: Option<(&str, f64)> = None;

    for task_id in &task_ids {
        if let Some(metrics) = app.get_task_metrics(task_id) {
            if let Some(MetricValue::Float(loss)) = metrics.metrics.get("loss") {
                if best_loss.is_none() || *loss < best_loss.unwrap().1 {
                    best_loss = Some((task_id, *loss));
                }
            }
            if let Some(MetricValue::Float(acc)) = metrics.metrics.get("accuracy") {
                if best_acc.is_none() || *acc > best_acc.unwrap().1 {
                    best_acc = Some((task_id, *acc));
                }
            }
        }
    }

    let mut summary_lines = Vec::new();
    if let Some((task, loss)) = best_loss {
        summary_lines.push(Line::from(vec![
            Span::raw("  Best Loss: "),
            Span::styled(
                format!("{:.4} ({})", loss, task),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
        ]));
    }
    if let Some((task, acc)) = best_acc {
        summary_lines.push(Line::from(vec![
            Span::raw("  Best Accuracy: "),
            Span::styled(
                format!("{:.2}% ({})", acc * 100.0, task),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
        ]));
    }
    if summary_lines.is_empty() {
        summary_lines.push(Line::from("  No comparable metrics yet"));
    }

    let summary = Paragraph::new(summary_lines)
        .block(Block::default().borders(Borders::ALL).title("Summary"));
    f.render_widget(summary, area);
}

fn render_footer(f: &mut Frame, area: ratatui::layout::Rect) {
    let footer_text = "Esc: Back | Tab: Cycle view | 1: Dashboard | 2: Terminal | 3: Graph | 4: Compare";
    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, area);
}
