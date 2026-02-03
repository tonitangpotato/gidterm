//! Terminal View - Full-screen terminal output for a single task

use crate::app::App;
use crate::core::GraphTaskStatus;
use crate::semantic::MetricValue;
use crate::semantic::advisor::Severity;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Sparkline, Wrap},
    Frame,
};

/// Render full-screen terminal view for the selected task
pub fn render_terminal_view(f: &mut Frame, app: &App) {
    let task_ids = app.get_task_ids();

    if task_ids.is_empty() || app.selected_task >= task_ids.len() {
        let empty = Paragraph::new("No task selected. Press Esc to return.")
            .block(Block::default().borders(Borders::ALL).title("Terminal"));
        f.render_widget(empty, f.area());
        return;
    }

    let task_id = &task_ids[app.selected_task];
    let task = app.scheduler.graph().get_task(task_id).unwrap();

    let has_metrics = app.get_task_metrics(task_id).is_some();
    let has_commands = app.get_semantic_commands(task_id).is_some();
    let has_advisories = app.get_advisories(task_id)
        .map(|a| !a.is_empty())
        .unwrap_or(false);
    let has_history = app.get_metric_history(task_id)
        .map(|h| h.snapshots.len() >= 3)
        .unwrap_or(false);

    // Layout: header + optional progress + optional sparklines + output + optional advisories + optional commands + footer
    let mut constraints = vec![Constraint::Length(3)]; // header
    if has_metrics {
        constraints.push(Constraint::Length(3)); // progress gauge
    }
    if has_history {
        constraints.push(Constraint::Length(4)); // sparklines
    }
    constraints.push(Constraint::Min(6)); // output (fills remaining)
    if has_advisories {
        constraints.push(Constraint::Length(4)); // advisories
    }
    if has_commands {
        constraints.push(Constraint::Length(3)); // semantic commands bar
    }
    constraints.push(Constraint::Length(3)); // footer

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(f.area());

    let mut chunk_idx = 0;

    // Header
    let status_icon = match task.status {
        GraphTaskStatus::Done => "✓",
        GraphTaskStatus::InProgress => "⚙",
        GraphTaskStatus::Failed => "✗",
        GraphTaskStatus::Pending => "□",
        GraphTaskStatus::Planned => "○",
    };

    let status_color = match task.status {
        GraphTaskStatus::Done => Color::Green,
        GraphTaskStatus::InProgress => Color::Yellow,
        GraphTaskStatus::Failed => Color::Red,
        _ => Color::Gray,
    };

    let header_text = Line::from(vec![
        Span::styled(
            format!(" {} {} ", status_icon, task_id),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("[{}]", task.status),
            Style::default().fg(status_color),
        ),
        Span::raw("  "),
        Span::styled(
            &task.description,
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let header = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::ALL).title("Task"));
    f.render_widget(header, chunks[chunk_idx]);
    chunk_idx += 1;

    // Progress gauge (if available)
    if has_metrics {
        let metrics = app.get_task_metrics(task_id).unwrap();
        let pct = (metrics.progress * 100.0) as u16;

        let mut label_parts = vec![format!("{}%", pct)];
        for (key, value) in &metrics.metrics {
            match value {
                MetricValue::Float(v) => label_parts.push(format!("{}: {:.4}", key, v)),
                MetricValue::Int(v) => label_parts.push(format!("{}: {}", key, v)),
                MetricValue::String(v) => label_parts.push(format!("{}: {}", key, v)),
                MetricValue::Bool(v) => label_parts.push(format!("{}: {}", key, v)),
            }
        }

        // Add ETA
        if let Some(eta) = app.get_eta(task_id) {
            label_parts.push(format!("ETA: {}", eta));
        }

        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Progress"))
            .gauge_style(Style::default().fg(Color::Green).bg(Color::DarkGray))
            .percent(pct.min(100))
            .label(label_parts.join(" | "));

        f.render_widget(gauge, chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // Sparklines panel
    if has_history {
        let history = app.get_metric_history(task_id).unwrap();
        let spark_area = chunks[chunk_idx];

        // Split horizontally for up to 3 sparklines
        let mut spark_charts: Vec<(&str, Vec<u64>, Color)> = Vec::new();

        // Loss sparkline
        let loss_vals = history.metric_values("loss", 50);
        if loss_vals.len() >= 2 {
            // Scale to u64 (multiply by 1000 for precision)
            let scaled: Vec<u64> = loss_vals.iter().map(|v| (v * 1000.0) as u64).collect();
            spark_charts.push(("Loss", scaled, Color::Red));
        }

        // Accuracy sparkline
        let acc_vals = history.metric_values("accuracy", 50);
        if acc_vals.len() >= 2 {
            let scaled: Vec<u64> = acc_vals.iter().map(|v| (v * 1000.0) as u64).collect();
            spark_charts.push(("Accuracy", scaled, Color::Green));
        }

        // Progress sparkline (if no loss/acc, show progress)
        if spark_charts.is_empty() {
            let prog_vals = history.progress_values(50);
            if prog_vals.len() >= 2 {
                let scaled: Vec<u64> = prog_vals.iter().map(|v| (v * 1000.0) as u64).collect();
                spark_charts.push(("Progress", scaled, Color::Cyan));
            }
        }

        if !spark_charts.is_empty() {
            let n = spark_charts.len();
            let spark_constraints: Vec<Constraint> = (0..n)
                .map(|_| Constraint::Ratio(1, n as u32))
                .collect();

            let spark_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(spark_constraints)
                .split(spark_area);

            for (i, (label, data, color)) in spark_charts.iter().enumerate() {
                let sparkline = Sparkline::default()
                    .block(Block::default().borders(Borders::ALL).title(*label))
                    .data(data)
                    .style(Style::default().fg(*color));
                f.render_widget(sparkline, spark_chunks[i]);
            }
        }

        chunk_idx += 1;
    }

    // Output panel (full height)
    let output_area = chunks[chunk_idx];
    let output_height = output_area.height.saturating_sub(2) as usize;
    let output_lines = app.get_task_output(task_id, output_height + app.scroll_offset);

    let visible_lines = if output_lines.len() > output_height {
        let start = output_lines.len().saturating_sub(output_height);
        output_lines[start..].to_vec()
    } else {
        output_lines
    };

    let text = if visible_lines.is_empty() {
        "(waiting for output...)".to_string()
    } else {
        visible_lines.join("\n")
    };

    let cmd_display = task
        .command
        .as_deref()
        .unwrap_or("(no command)");

    let output = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Output: {}", cmd_display)),
        )
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White));

    f.render_widget(output, output_area);
    chunk_idx += 1;

    // Advisories panel
    if has_advisories {
        let advisories = app.get_advisories(task_id).unwrap();
        let advisory_lines: Vec<Line> = advisories
            .iter()
            .take(3)
            .map(|a| {
                let (icon, color) = match a.severity {
                    Severity::Critical => ("!!", Color::Red),
                    Severity::Warning => ("!", Color::Yellow),
                    Severity::Info => ("i", Color::Cyan),
                };
                Line::from(vec![
                    Span::styled(format!(" [{}] ", icon), Style::default().fg(color).add_modifier(Modifier::BOLD)),
                    Span::raw(&a.message),
                    Span::styled(format!(" -> {}", a.suggestion), Style::default().fg(Color::DarkGray)),
                ])
            })
            .collect();

        let advisories_widget = Paragraph::new(advisory_lines)
            .block(Block::default().borders(Borders::ALL).title("Advisories"))
            .wrap(Wrap { trim: true });

        f.render_widget(advisories_widget, chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // Semantic commands bar
    if has_commands {
        let cmds = app.get_semantic_commands(task_id).unwrap();
        let cmd_labels: Vec<String> = cmds
            .labels()
            .iter()
            .enumerate()
            .map(|(i, label)| format!("[F{}] {}", i + 1, label))
            .collect();

        let commands_bar = Paragraph::new(cmd_labels.join("  "))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Semantic Commands"),
            )
            .style(Style::default().fg(Color::Cyan));

        f.render_widget(commands_bar, chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // Footer
    let footer_text = "Esc: Back | ↑↓: Switch task | k: Kill | Tab: Cycle view";
    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::DarkGray));

    f.render_widget(footer, chunks[chunk_idx]);
}
