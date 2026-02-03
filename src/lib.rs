//! GidTerm - Graph-Driven Semantic Terminal Controller
//!
//! A semantic terminal controller that integrates project/task graphs
//! with intelligent process management.

pub mod ai;
pub mod app;
pub mod core;
pub mod notifications;
pub mod ports;
pub mod semantic;
pub mod session;
pub mod ui;
pub mod workspace;

// Re-exports
pub use app::App;
pub use core::{Executor, Graph, GraphTaskStatus, PTYHandle, Scheduler, TaskEvent};
pub use notifications::{NotificationConfig, NotificationEvent, NotificationManager};
pub use ports::{PortEntry, PortManager, PortRegistry, PortStatus};
pub use session::{Session, TaskHistory, TaskRun, TaskStatus};
pub use workspace::{Project, Workspace};

/// Result type alias
pub type Result<T> = anyhow::Result<T>;
