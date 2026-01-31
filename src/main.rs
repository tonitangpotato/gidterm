//! GidTerm CLI entry point

use anyhow::Result;
use gidterm::core::Graph;
use gidterm::ui::{DashboardView, TUI};
use std::path::PathBuf;

fn main() -> Result<()> {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("🚀 GidTerm v{}", env!("CARGO_PKG_VERSION"));

    // Parse CLI args (simple for now)
    let graph_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| ".gid/graph.yml".to_string());

    let graph_path = PathBuf::from(graph_path);

    // Load graph
    log::info!("Loading graph from: {}", graph_path.display());
    let graph = Graph::from_file(&graph_path)?;

    log::info!(
        "Loaded {} nodes, {} tasks",
        graph.nodes.len(),
        graph.tasks.len()
    );

    // Start TUI
    let mut tui = TUI::new()?;

    tui.run(|terminal| {
        terminal.draw(|f| {
            DashboardView::render(f, &graph, f.area());
        })?;

        // For now, just render once and wait for quit
        // TODO: Add proper event loop and state management
        Ok(false)
    })?;

    Ok(())
}
