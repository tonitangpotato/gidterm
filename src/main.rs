//! GidTerm CLI - Live TUI with real-time task execution

use anyhow::Result;
use gidterm::app::App;
use gidterm::core::Graph;
use gidterm::ui::{render_live_dashboard, TUI};
use std::path::PathBuf;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("🚀 GidTerm v{} (Live Mode)", env!("CARGO_PKG_VERSION"));

    // Parse CLI args
    let args: Vec<String> = std::env::args().collect();
    
    // Load graph
    let graph = if args.len() > 1 {
        // Explicit file path provided
        let graph_path = PathBuf::from(&args[1]);
        log::info!("Loading graph from: {}", graph_path.display());
        Graph::from_file(&graph_path)?
    } else {
        // Auto-detect
        log::info!("Auto-detecting graph file...");
        Graph::auto_load()?
    };

    log::info!(
        "Loaded {} nodes, {} tasks",
        graph.nodes.len(),
        graph.tasks.len()
    );

    // Create app
    let mut app = App::new(graph);

    // Start initial tasks
    app.start_ready_tasks().await?;

    // Start TUI
    let mut tui = TUI::new()?;

    // Main event loop
    loop {
        // Process task events
        app.process_events();

        // Start newly ready tasks
        app.start_ready_tasks().await?;

        // Render UI
        tui.terminal().draw(|f| {
            render_live_dashboard(f, &app);
        })?;

        // Handle input
        if App::should_poll_input()? {
            let event = App::read_event()?;
            
            if let crossterm::event::Event::Key(key) = event {
                app.handle_key(key);
            }
        }

        // Check quit
        if app.should_quit {
            break;
        }

        // Small delay to avoid spinning
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    log::info!("Shutting down...");
    
    // End session and save
    app.session.end();
    if let Err(e) = app.session.save() {
        log::warn!("Failed to save final session: {}", e);
    }

    Ok(())
}
