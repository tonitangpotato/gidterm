//! GidTerm CLI - Graph-Driven Semantic Terminal Controller

use anyhow::Result;
use clap::{Parser, Subcommand};
use gidterm::app::{App, ViewMode};
use gidterm::core::Graph;
use gidterm::ports::PortRegistry;
use gidterm::ui::{
    render_comparison_view, render_graph_view, render_live_dashboard, render_project_overview,
    render_terminal_view, TUI,
};
use gidterm::workspace::Workspace;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "gidterm", version, about = "Graph-Driven Semantic Terminal Controller")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run tasks from a graph file (default behavior)
    Run {
        /// Path to graph YAML file (auto-detects if not specified)
        #[arg(short, long)]
        graph: Option<PathBuf>,

        /// Workspace mode: discover and run all projects
        #[arg(short, long)]
        workspace: bool,
    },

    /// Show status of tasks in a graph
    Status {
        /// Path to graph YAML file
        #[arg(short, long)]
        graph: Option<PathBuf>,
    },

    /// Initialize a new task graph in the current directory
    Init {
        /// Output file path
        #[arg(short, long, default_value = "graph.yml")]
        output: PathBuf,
    },

    /// Show session history
    History {
        /// Number of recent sessions to show
        #[arg(short, long, default_value = "5")]
        count: usize,
    },

    /// Start a single task by ID
    Start {
        /// Task ID to start
        task_id: String,

        /// Path to graph YAML file
        #[arg(short, long)]
        graph: Option<PathBuf>,
    },

    /// Show port allocations
    Ports {
        /// Clean up stale port allocations
        #[arg(long)]
        cleanup: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    match cli.command {
        None | Some(Commands::Run { .. }) => {
            let (graph_path, workspace) = match &cli.command {
                Some(Commands::Run { graph, workspace }) => (graph.clone(), *workspace),
                _ => (None, false),
            };
            run_tui(graph_path, workspace).await
        }
        Some(Commands::Status { graph }) => cmd_status(graph),
        Some(Commands::Init { output }) => cmd_init(&output),
        Some(Commands::History { count }) => cmd_history(count),
        Some(Commands::Start { task_id, graph }) => cmd_start(&task_id, graph).await,
        Some(Commands::Ports { cleanup }) => cmd_ports(cleanup),
    }
}

async fn run_tui(graph_path: Option<PathBuf>, workspace: bool) -> Result<()> {
    log::info!("🚀 GidTerm v{} (Live Mode)", env!("CARGO_PKG_VERSION"));

    let mut app = if workspace {
        let root = std::env::current_dir()?;
        log::info!("🌐 Workspace mode: discovering projects in {}", root.display());
        let workspace = Workspace::discover(&root)?;
        log::info!(
            "Found {} projects with {} total tasks",
            workspace.project_count(),
            workspace.total_task_count()
        );
        App::from_workspace(&workspace)
    } else {
        let graph = if let Some(path) = graph_path {
            log::info!("Loading graph from: {}", path.display());
            Graph::from_file(&path)?
        } else {
            log::info!("Auto-detecting graph file...");
            Graph::auto_load()?
        };
        log::info!("Loaded {} nodes, {} tasks", graph.nodes.len(), graph.tasks.len());
        App::new(graph)
    };

    app.start_ready_tasks().await?;

    let mut tui = TUI::new()?;

    loop {
        app.process_events();
        app.start_ready_tasks().await?;

        tui.terminal().draw(|f| {
            match app.view_mode {
                ViewMode::Dashboard => render_live_dashboard(f, &app),
                ViewMode::Terminal => render_terminal_view(f, &app),
                ViewMode::Graph => render_graph_view(f, &app),
                ViewMode::Comparison => render_comparison_view(f, &app),
                ViewMode::ProjectOverview => render_project_overview(f, &app),
            }
        })?;

        if App::should_poll_input()? {
            let event = App::read_event()?;
            if let crossterm::event::Event::Key(key) = event {
                app.handle_key(key);
            }
        }

        if app.should_quit {
            break;
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    log::info!("Shutting down...");
    app.executor.stop_all();
    app.session.end();
    if let Err(e) = app.session.save() {
        log::warn!("Failed to save final session: {}", e);
    }

    Ok(())
}

fn cmd_status(graph_path: Option<PathBuf>) -> Result<()> {
    let graph = if let Some(path) = graph_path {
        Graph::from_file(&path)?
    } else {
        Graph::auto_load()?
    };

    if let Some(meta) = &graph.metadata {
        println!("Project: {}", meta.project);
    }

    println!("\nTasks ({}):", graph.tasks.len());
    let mut tasks: Vec<_> = graph.tasks.iter().collect();
    tasks.sort_by_key(|(id, _)| (*id).clone());

    for (id, task) in &tasks {
        let deps = match &task.depends_on {
            Some(d) if !d.is_empty() => format!(" (depends: {})", d.join(", ")),
            _ => String::new(),
        };
        println!("  {} [{}]{} - {}", task.status, id, deps, task.description);
    }

    let done = tasks.iter().filter(|(_, t)| t.status == gidterm::core::GraphTaskStatus::Done).count();
    let total = tasks.len();
    println!("\nProgress: {}/{} ({:.0}%)", done, total, if total > 0 { done as f64 / total as f64 * 100.0 } else { 0.0 });

    Ok(())
}

fn cmd_init(output: &PathBuf) -> Result<()> {
    if output.exists() {
        anyhow::bail!("File already exists: {}. Use --output to specify a different path.", output.display());
    }

    let template = r#"metadata:
  project: my-project
  version: "1.0"

tasks:
  build:
    description: Build the project
    command: cargo build
    status: pending
    depends_on: []

  test:
    description: Run tests
    command: cargo test
    status: pending
    depends_on: [build]
"#;

    std::fs::write(output, template)?;
    println!("Created task graph: {}", output.display());
    println!("Run `gidterm run` to start executing tasks.");
    Ok(())
}

fn cmd_history(count: usize) -> Result<()> {
    let session_dir = PathBuf::from(".gidterm/sessions");
    if !session_dir.exists() {
        println!("No session history found.");
        return Ok(());
    }

    let mut entries: Vec<_> = std::fs::read_dir(&session_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "json").unwrap_or(false))
        .collect();

    entries.sort_by_key(|e| std::cmp::Reverse(e.file_name()));
    entries.truncate(count);

    if entries.is_empty() {
        println!("No session history found.");
        return Ok(());
    }

    println!("Recent sessions:");
    for entry in &entries {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        // Parse timestamp from filename (2026-01-31-17-34-21.json)
        let display = name_str.trim_end_matches(".json");
        println!("  {}", display);
    }

    Ok(())
}

async fn cmd_start(task_id: &str, graph_path: Option<PathBuf>) -> Result<()> {
    let graph = if let Some(path) = graph_path {
        Graph::from_file(&path)?
    } else {
        Graph::auto_load()?
    };

    let task = graph.get_task(task_id)
        .ok_or_else(|| anyhow::anyhow!("Task '{}' not found", task_id))?;

    let command = task.command.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Task '{}' has no command", task_id))?;

    println!("Starting task: {} ({})", task_id, command);
    println!("  {}", task.description);

    let status = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .status()
        .await?;

    if status.success() {
        println!("\nTask '{}' completed successfully.", task_id);
    } else {
        println!("\nTask '{}' failed with exit code: {:?}", task_id, status.code());
    }

    Ok(())
}

fn cmd_ports(cleanup: bool) -> Result<()> {
    let mut registry = PortRegistry::load()?;

    if cleanup {
        let cleaned = registry.cleanup_stale()?;
        println!("Cleaned up {} stale port allocation(s).", cleaned);
        return Ok(());
    }

    let allocations = registry.list_allocations();

    if allocations.is_empty() {
        println!("No port allocations.");
        return Ok(());
    }

    println!("{:<6} {:<20} {:<8} {:<10}", "PORT", "PROJECT", "PID", "STATUS");
    println!("{}", "-".repeat(50));

    for entry in allocations {
        let pid_str = entry.pid.map(|p| p.to_string()).unwrap_or_else(|| "-".to_string());
        let status = if entry.active {
            "🟢 active"
        } else if gidterm::ports::is_port_available(entry.port) {
            "⏸️  reserved"
        } else {
            "🔴 external"
        };

        println!("{:<6} {:<20} {:<8} {:<10}", entry.port, entry.project, pid_str, status);
    }

    Ok(())
}
