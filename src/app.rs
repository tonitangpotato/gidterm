//! Application state and main event loop

use crate::core::{Executor, Graph, Scheduler, TaskEvent};
use crate::semantic::{ParserRegistry, TaskMetrics};
use crate::semantic::parsers::{MLTrainingParser, RegexParser};
use crate::session::{Session, TaskStatus};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Max output lines stored per task in App
const MAX_APP_OUTPUT_LINES: usize = 2000;

/// Application state
pub struct App {
    pub scheduler: Scheduler,
    pub executor: Executor,
    pub event_rx: mpsc::UnboundedReceiver<TaskEvent>,
    pub task_outputs: HashMap<String, Vec<String>>,
    pub should_quit: bool,
    pub selected_task: usize,
    pub last_update: Instant,
    pub session: Session,
    pub workspace_mode: bool,
    pub project_names: Vec<String>,
    pub parser_registry: ParserRegistry,
    pub task_metrics: HashMap<String, TaskMetrics>,
}

impl App {
    /// Create a new app from graph (single project mode)
    pub fn new(graph: Graph) -> Self {
        let scheduler = Scheduler::new(graph.clone());
        let (executor, event_rx) = Executor::new();

        let project_name = graph
            .metadata
            .as_ref()
            .map(|m| m.project.clone())
            .unwrap_or_else(|| "unknown".to_string());

        let session = Session::new(project_name);
        let parser_registry = Self::build_parser_registry();

        Self {
            scheduler,
            executor,
            event_rx,
            task_outputs: HashMap::new(),
            should_quit: false,
            selected_task: 0,
            last_update: Instant::now(),
            session,
            workspace_mode: false,
            project_names: Vec::new(),
            parser_registry,
            task_metrics: HashMap::new(),
        }
    }

    /// Create app from workspace (multi-project mode)
    pub fn from_workspace(workspace: &crate::workspace::Workspace) -> Self {
        let unified_graph = workspace.to_unified_graph();
        let scheduler = Scheduler::new(unified_graph);
        let (executor, event_rx) = Executor::new();

        let session = Session::new("workspace".to_string());
        let project_names = workspace.project_names();
        let parser_registry = Self::build_parser_registry();

        Self {
            scheduler,
            executor,
            event_rx,
            task_outputs: HashMap::new(),
            should_quit: false,
            selected_task: 0,
            last_update: Instant::now(),
            session,
            workspace_mode: true,
            project_names,
            parser_registry,
            task_metrics: HashMap::new(),
        }
    }

    /// Build the default parser registry with all built-in parsers
    fn build_parser_registry() -> ParserRegistry {
        let mut registry = ParserRegistry::new();

        // Register ML training parser
        registry.register(Box::new(MLTrainingParser::new()));

        // Register generic regex parser (catches progress bars, percentages, etc.)
        registry.register(Box::new(RegexParser::default_parser()));

        registry
    }

    /// Start all ready tasks
    pub async fn start_ready_tasks(&mut self) -> Result<()> {
        let ready = self.scheduler.schedule_next();

        for task_id in ready {
            let task = self.scheduler.graph().get_task(&task_id).unwrap();

            if let Some(command) = &task.command {
                log::info!("Starting task: {} ({})", task_id, command);

                self.session.start_task(task_id.clone());
                self.executor.start_task(&task_id, command).await?;
                self.scheduler.mark_started(&task_id)?;
            } else {
                // No command, mark as done immediately
                self.scheduler.mark_done(&task_id)?;
            }
        }

        if let Err(e) = self.session.save() {
            log::warn!("Failed to save session: {}", e);
        }

        Ok(())
    }

    /// Process events from executor
    pub fn process_events(&mut self) {
        let mut session_updated = false;

        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                TaskEvent::Started { task_id } => {
                    log::info!("Task started: {}", task_id);
                }
                TaskEvent::Output { task_id, line } => {
                    if !line.is_empty() {
                        // Store output
                        let lines = self.task_outputs
                            .entry(task_id.clone())
                            .or_insert_with(Vec::new);
                        lines.push(line.clone());

                        // Cap output history
                        if lines.len() > MAX_APP_OUTPUT_LINES {
                            let drain_count = lines.len() - MAX_APP_OUTPUT_LINES;
                            lines.drain(0..drain_count);
                        }

                        // Track in session
                        self.session.add_output(&task_id, line);
                        session_updated = true;

                        // Run through semantic parser
                        self.update_task_metrics(&task_id);
                    }
                }
                TaskEvent::Completed { task_id, exit_code } => {
                    log::info!("Task completed: {} (exit: {})", task_id, exit_code);
                    if let Err(e) = self.scheduler.mark_done(&task_id) {
                        log::warn!("Failed to mark task {} done: {}", task_id, e);
                    }
                    self.session.end_task(&task_id, TaskStatus::Done, Some(exit_code));
                    session_updated = true;
                }
                TaskEvent::Failed { task_id, error } => {
                    log::warn!("Task failed: {} - {}", task_id, error);
                    if let Err(e) = self.scheduler.mark_failed(&task_id) {
                        log::warn!("Failed to mark task {} failed: {}", task_id, e);
                    }
                    self.session.end_task(&task_id, TaskStatus::Failed, None);
                    session_updated = true;
                }
            }
        }

        if session_updated {
            if let Err(e) = self.session.save() {
                log::warn!("Failed to save session: {}", e);
            }
        }

        self.last_update = Instant::now();
    }

    /// Update semantic metrics for a task based on its output
    fn update_task_metrics(&mut self, task_id: &str) {
        // Get the task type for parser selection
        let task_type = self.scheduler.graph().get_task(task_id)
            .map(|t| t.task_type.clone());

        // Get recent output (last 20 lines for parsing)
        let output = self.task_outputs.get(task_id)
            .map(|lines| {
                let start = lines.len().saturating_sub(20);
                lines[start..].join("\n")
            })
            .unwrap_or_default();

        if output.is_empty() {
            return;
        }

        // Parse through registry
        if let Ok(metrics) = self.parser_registry.parse(task_type.as_deref(), &output) {
            // Only update if we got meaningful data
            if metrics.progress > 0.0 || !metrics.metrics.is_empty() || !metrics.errors.is_empty() {
                self.task_metrics.insert(task_id.to_string(), metrics);
            }
        }
    }

    /// Handle keyboard input
    pub fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Char('r') => {
                log::info!("Manual refresh requested");
            }
            KeyCode::Char('k') => {
                // Kill selected task
                let task_ids = self.get_task_ids();
                if let Some(task_id) = task_ids.get(self.selected_task) {
                    if let Err(e) = self.executor.stop_task(task_id) {
                        log::warn!("Failed to stop task {}: {}", task_id, e);
                    }
                }
            }
            KeyCode::Up => {
                if self.selected_task > 0 {
                    self.selected_task -= 1;
                }
            }
            KeyCode::Down => {
                let task_count = self.scheduler.graph().all_tasks().len();
                if self.selected_task + 1 < task_count {
                    self.selected_task += 1;
                }
            }
            _ => {}
        }
    }

    /// Check if we should poll for input
    pub fn should_poll_input() -> Result<bool> {
        Ok(event::poll(Duration::from_millis(100))?)
    }

    /// Get keyboard event
    pub fn read_event() -> Result<Event> {
        Ok(event::read()?)
    }

    /// Get task output lines (last N)
    pub fn get_task_output(&self, task_id: &str, last_n: usize) -> Vec<String> {
        self.task_outputs
            .get(task_id)
            .map(|lines| {
                let start = lines.len().saturating_sub(last_n);
                lines[start..].to_vec()
            })
            .unwrap_or_default()
    }

    /// Get semantic metrics for a task
    pub fn get_task_metrics(&self, task_id: &str) -> Option<&TaskMetrics> {
        self.task_metrics.get(task_id)
    }

    /// Get all task IDs sorted
    pub fn get_task_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.scheduler.graph().all_tasks().keys().cloned().collect();
        ids.sort();
        ids
    }

    /// Extract project name from namespaced task ID
    pub fn get_project_name(&self, task_id: &str) -> Option<String> {
        if self.workspace_mode {
            task_id.split(':').next().map(|s| s.to_string())
        } else {
            None
        }
    }

    /// Get tasks grouped by project (for workspace mode)
    pub fn get_tasks_by_project(&self) -> HashMap<String, Vec<String>> {
        let mut grouped: HashMap<String, Vec<String>> = HashMap::new();

        if self.workspace_mode {
            for task_id in self.get_task_ids() {
                if let Some(project) = self.get_project_name(&task_id) {
                    grouped
                        .entry(project)
                        .or_insert_with(Vec::new)
                        .push(task_id);
                }
            }
        } else {
            let project = self.session.project.clone();
            grouped.insert(project, self.get_task_ids());
        }

        grouped
    }
}
