//! Graph View - Visual DAG of task dependencies in TUI

use crate::app::App;
use crate::core::GraphTaskStatus;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use std::collections::HashMap;

/// Render a visual DAG view of task dependencies
pub fn render_graph_view(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),   // Graph
            Constraint::Length(3),  // Footer
        ])
        .split(f.area());

    render_header(f, app, chunks[0]);
    render_dag(f, app, chunks[1]);
    render_footer(f, chunks[2]);
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let graph = app.scheduler.graph();
    let total = graph.all_tasks().len();
    let done = graph.all_tasks().values()
        .filter(|t| t.status == GraphTaskStatus::Done)
        .count();
    let running = app.scheduler.get_running().len();

    let title = format!(
        "Task Graph | {}/{} done | {} running",
        done, total, running
    );

    let header = Paragraph::new(title)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Cyan));
    f.render_widget(header, area);
}

fn render_dag(f: &mut Frame, app: &App, area: Rect) {
    let graph = app.scheduler.graph();
    let tasks = graph.all_tasks();

    // Build layers: tasks grouped by dependency depth
    let layers = build_layers(tasks);
    let mut items: Vec<ListItem> = Vec::new();

    for (depth, layer_tasks) in layers.iter().enumerate() {
        // Layer header
        let indent = "  ".repeat(depth);
        let layer_header = Line::from(vec![
            Span::styled(
                format!("{}Layer {} ────", indent, depth),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        items.push(ListItem::new(layer_header));

        for task_id in layer_tasks {
            if let Some(task) = tasks.get(task_id) {
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
                    GraphTaskStatus::Pending => Color::Gray,
                    GraphTaskStatus::Planned => Color::DarkGray,
                };

                // Show dependency arrows
                let deps_str = task.depends_on.as_ref()
                    .map(|deps| {
                        if deps.is_empty() {
                            String::new()
                        } else {
                            let short_deps: Vec<&str> = deps.iter()
                                .map(|d| d.as_str())
                                .collect();
                            format!(" <── {}", short_deps.join(", "))
                        }
                    })
                    .unwrap_or_default();

                let arrow = if depth > 0 { "├─ " } else { "" };

                let line = Line::from(vec![
                    Span::raw(format!("{}  {}", indent, arrow)),
                    Span::styled(
                        format!("{} ", status_icon),
                        Style::default().fg(status_color),
                    ),
                    Span::styled(
                        task_id.to_string(),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(deps_str, Style::default().fg(Color::DarkGray)),
                ]);

                items.push(ListItem::new(line));
            }
        }

        // Spacer between layers
        items.push(ListItem::new(Line::from("")));
    }

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Dependency Graph (layered by depth)"),
    );
    f.render_widget(list, area);
}

/// Build layers: group tasks by their dependency depth
fn build_layers(tasks: &HashMap<String, crate::core::Task>) -> Vec<Vec<String>> {
    let mut depths: HashMap<String, usize> = HashMap::new();

    // Calculate depth for each task
    for task_id in tasks.keys() {
        calculate_depth(task_id, tasks, &mut depths);
    }

    // Group by depth
    let max_depth = depths.values().copied().max().unwrap_or(0);
    let mut layers: Vec<Vec<String>> = vec![Vec::new(); max_depth + 1];

    let mut sorted_tasks: Vec<(String, usize)> = depths.into_iter().collect();
    sorted_tasks.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));

    for (task_id, depth) in sorted_tasks {
        layers[depth].push(task_id);
    }

    layers
}

fn calculate_depth(
    task_id: &str,
    tasks: &HashMap<String, crate::core::Task>,
    depths: &mut HashMap<String, usize>,
) -> usize {
    if let Some(&depth) = depths.get(task_id) {
        return depth;
    }

    let depth = if let Some(task) = tasks.get(task_id) {
        if let Some(deps) = &task.depends_on {
            deps.iter()
                .map(|dep| calculate_depth(dep, tasks, depths) + 1)
                .max()
                .unwrap_or(0)
        } else {
            0
        }
    } else {
        0
    };

    depths.insert(task_id.to_string(), depth);
    depth
}

fn render_footer(f: &mut Frame, area: Rect) {
    let footer_text = "Esc: Back | Tab: Cycle view | 1: Dashboard | 2: Terminal | 3: Graph";
    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, area);
}
