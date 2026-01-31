//! GidTerm - Graph-Driven Semantic Terminal Controller
//!
//! A semantic terminal controller that integrates project/task graphs
//! with intelligent process management.

pub mod app;
pub mod core;
pub mod semantic;
pub mod session;
pub mod ui;

// Re-exports
pub use app::App;
pub use core::{Executor, Graph, PTYHandle, Scheduler, TaskEvent};
pub use session::{Session, TaskHistory, TaskRun, TaskStatus};

/// Result type alias
pub type Result<T> = anyhow::Result<T>;

/// GidTerm engine - main entry point for library usage
pub struct GidTermEngine {
    graph: core::Graph,
    // TODO: Add other fields
}

impl GidTermEngine {
    /// Create a new engine from a graph file
    pub fn from_graph(path: &std::path::Path) -> Result<Self> {
        let graph = core::Graph::from_file(path)?;
        Ok(Self { graph })
    }

    /// Start a task
    pub fn start_task(&mut self, _task_id: &str) -> Result<()> {
        // TODO: Implement
        todo!("start_task")
    }
}
