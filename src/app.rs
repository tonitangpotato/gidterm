//! Application state and main event loop

use crate::core::{Executor, Graph, Scheduler, TaskEvent};
use crate::notifications::NotificationManager;
use crate::ports::PortManager;
use crate::semantic::advisor::{Advisory, SmartAdvisor};
use crate::semantic::commands::TaskCommands;
use crate::semantic::history::{self, TaskMetricHistory};
use crate::semantic::parsers::{BuildParser, MLTrainingParser, RegexParser};
use crate::semantic::{MetricValue, ParserRegistry, TaskMetrics};
use crate::session::{Session, TaskStatus};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Max output lines stored per task in App
const MAX_APP_OUTPUT_LINES: usize = 2000;

/// Active view mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Live dashboard with task list and output
    Dashboard,
    /// Full-screen terminal for selected task
    Terminal,
    /// DAG graph visualization
    Graph,
    /// Cross-task comparison table
    Comparison,
    /// Project overview (multi-project mode)
    ProjectOverview,
}

/// Agent/task status for quick visibility
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    /// Agent is actively running
    Running,
    /// Agent is waiting for user input
    WaitingInput,
    /// Agent has completed
    Completed,
    /// Agent encountered error
    Error,
    /// Agent is paused/idle
    Idle,
}

impl AgentStatus {
    /// Get emoji for status
    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Running => "🟢",
            Self::WaitingInput => "🔵",
            Self::Completed => "✓",
            Self::Error => "🔴",
            Self::Idle => "⏸️",
        }
    }

    /// Get color for status (ratatui Color)
    pub fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            Self::Running => Color::Green,
            Self::WaitingInput => Color::Blue,
            Self::Completed => Color::Gray,
            Self::Error => Color::Red,
            Self::Idle => Color::DarkGray,
        }
    }
}

/// Project summary for unified dashboard
#[derive(Debug, Clone)]
pub struct ProjectSummary {
    pub name: String,
    pub port: Option<u16>,
    pub agent_status: AgentStatus,
    pub task_count: usize,
    pub tasks_done: usize,
    pub tasks_running: usize,
    pub tasks_failed: usize,
    pub recent_event: Option<String>,
}

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
    pub metric_history: HashMap<String, TaskMetricHistory>,
    pub advisor: SmartAdvisor,
    pub advisories: HashMap<String, Vec<Advisory>>,
    pub view_mode: ViewMode,
    pub scroll_offset: usize,
    // Phase 1: Multi-Project DX
    pub port_manager: PortManager,
    pub notification_manager: NotificationManager,
    pub selected_project: usize,
    pub search_query: String,
    pub search_mode: bool,
    pub recent_events: Vec<(Instant, String, String)>, // (time, project, message)
    pub task_start_times: HashMap<String, Instant>,
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

        let session = Session::new(project_name.clone());
        let parser_registry = Self::build_parser_registry();

        // Initialize port manager and allocate port for this project
        let mut port_manager = PortManager::default();
        if let Err(e) = port_manager.allocate(&project_name, None) {
            log::warn!("Failed to allocate port for {}: {}", project_name, e);
        }

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
            project_names: vec![project_name],
            parser_registry,
            task_metrics: HashMap::new(),
            metric_history: HashMap::new(),
            advisor: SmartAdvisor::new(),
            advisories: HashMap::new(),
            view_mode: ViewMode::Dashboard,
            scroll_offset: 0,
            // Phase 1: Multi-Project DX
            port_manager,
            notification_manager: NotificationManager::new(),
            selected_project: 0,
            search_query: String::new(),
            search_mode: false,
            recent_events: Vec::new(),
            task_start_times: HashMap::new(),
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

        // Initialize port manager and allocate ports for all projects
        let mut port_manager = PortManager::default();
        for (idx, name) in project_names.iter().enumerate() {
            let preferred_port = 3000 + idx as u16;
            if let Err(e) = port_manager.allocate(name, Some(preferred_port)) {
                log::warn!("Failed to allocate port for {}: {}", name, e);
            }
        }

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
            metric_history: HashMap::new(),
            advisor: SmartAdvisor::new(),
            advisories: HashMap::new(),
            view_mode: ViewMode::ProjectOverview, // Start with project overview in workspace mode
            scroll_offset: 0,
            // Phase 1: Multi-Project DX
            port_manager,
            notification_manager: NotificationManager::new(),
            selected_project: 0,
            search_query: String::new(),
            search_mode: false,
            recent_events: Vec::new(),
            task_start_times: HashMap::new(),
        }
    }

    /// Build the default parser registry with all built-in parsers
    fn build_parser_registry() -> ParserRegistry {
        let mut registry = ParserRegistry::new();

        // Register ML training parser
        registry.register(Box::new(MLTrainingParser::new()));

        // Register build output parser (cargo, npm, make)
        registry.register(Box::new(BuildParser::new()));

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
                    self.task_start_times.insert(task_id.clone(), Instant::now());
                    
                    // Add recent event
                    let project = self.get_project_name(&task_id).unwrap_or_else(|| self.session.project.clone());
                    let task_display = self.get_task_display_name(&task_id);
                    self.add_recent_event(&project, format!("Started: {}", task_display));
                    
                    // Send notification
                    let _ = self.notification_manager.notify_started(&project, &task_display);
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
                        self.session.add_output(&task_id, line.clone());
                        session_updated = true;

                        // Run through semantic parser
                        self.update_task_metrics(&task_id);
                        
                        // Check for waiting-for-input patterns
                        self.check_waiting_input(&task_id, &line);
                    }
                }
                TaskEvent::Completed { task_id, exit_code } => {
                    log::info!("Task completed: {} (exit: {})", task_id, exit_code);
                    if let Err(e) = self.scheduler.mark_done(&task_id) {
                        log::warn!("Failed to mark task {} done: {}", task_id, e);
                    }
                    self.session.end_task(&task_id, TaskStatus::Done, Some(exit_code));
                    session_updated = true;
                    
                    // Add recent event and send notification
                    let project = self.get_project_name(&task_id).unwrap_or_else(|| self.session.project.clone());
                    let task_display = self.get_task_display_name(&task_id);
                    let duration = self.task_start_times.get(&task_id).map(|t| t.elapsed());
                    
                    self.add_recent_event(&project, format!("Completed: {}", task_display));
                    let _ = self.notification_manager.notify_complete(&project, &task_display, duration);
                    
                    // Deactivate port if this was the main task
                    let _ = self.port_manager.deactivate(&project);
                }
                TaskEvent::Failed { task_id, error } => {
                    log::warn!("Task failed: {} - {}", task_id, error);
                    if let Err(e) = self.scheduler.mark_failed(&task_id) {
                        log::warn!("Failed to mark task {} failed: {}", task_id, e);
                    }
                    self.session.end_task(&task_id, TaskStatus::Failed, None);
                    session_updated = true;
                    
                    // Add recent event and send notification
                    let project = self.get_project_name(&task_id).unwrap_or_else(|| self.session.project.clone());
                    let task_display = self.get_task_display_name(&task_id);
                    
                    self.add_recent_event(&project, format!("Failed: {} - {}", task_display, &error));
                    let _ = self.notification_manager.notify_error(&project, &task_display, &error);
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
    
    /// Add a recent event (keeps last 50)
    fn add_recent_event(&mut self, project: &str, message: String) {
        self.recent_events.push((Instant::now(), project.to_string(), message));
        if self.recent_events.len() > 50 {
            self.recent_events.remove(0);
        }
    }
    
    /// Get task display name (strip project prefix in workspace mode)
    pub fn get_task_display_name(&self, task_id: &str) -> String {
        if self.workspace_mode {
            task_id.split(':').nth(1).unwrap_or(task_id).to_string()
        } else {
            task_id.to_string()
        }
    }
    
    /// Check if output indicates waiting for input
    fn check_waiting_input(&mut self, task_id: &str, line: &str) {
        // Common patterns that indicate waiting for input
        let waiting_patterns = [
            "press enter",
            "press any key",
            "y/n",
            "[y/n]",
            "(y/n)",
            "continue?",
            "proceed?",
            "confirm",
            "waiting for input",
            "Enter your",
            "Type your",
        ];
        
        let lower = line.to_lowercase();
        for pattern in waiting_patterns {
            if lower.contains(pattern) {
                let project = self.get_project_name(task_id).unwrap_or_else(|| self.session.project.clone());
                let task_display = self.get_task_display_name(task_id);
                
                self.add_recent_event(&project, format!("Waiting: {} - {}", task_display, line));
                let _ = self.notification_manager.notify_waiting(&project, &task_display);
                break;
            }
        }
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
                // Record to history for trend tracking
                let history = self.metric_history
                    .entry(task_id.to_string())
                    .or_insert_with(TaskMetricHistory::new);

                let float_metrics: HashMap<String, f64> = metrics.metrics.iter()
                    .filter_map(|(k, v)| match v {
                        MetricValue::Float(f) => Some((k.clone(), *f)),
                        MetricValue::Int(i) => Some((k.clone(), *i as f64)),
                        _ => None,
                    })
                    .collect();

                history.record(metrics.progress, float_metrics);

                // Run advisor
                let history_ref = self.metric_history.get(task_id);
                let new_advisories = self.advisor.evaluate(&metrics, history_ref);
                if !new_advisories.is_empty() {
                    self.advisories.insert(task_id.to_string(), new_advisories);
                }

                self.task_metrics.insert(task_id.to_string(), metrics);
            }
        }
    }

    /// Get advisories for a task
    pub fn get_advisories(&self, task_id: &str) -> Option<&Vec<Advisory>> {
        self.advisories.get(task_id)
    }

    /// Get ETA for a task as formatted string
    pub fn get_eta(&self, task_id: &str) -> Option<String> {
        let h = self.metric_history.get(task_id)?;
        let remaining = h.estimate_remaining()?;
        Some(history::format_eta(remaining))
    }

    /// Get metric history for a task
    pub fn get_metric_history(&self, task_id: &str) -> Option<&TaskMetricHistory> {
        self.metric_history.get(task_id)
    }

    /// Handle keyboard input
    pub fn handle_key(&mut self, key: KeyEvent) {
        // Handle search mode input
        if self.search_mode {
            match key.code {
                KeyCode::Esc => {
                    self.search_mode = false;
                    self.search_query.clear();
                }
                KeyCode::Enter => {
                    self.search_mode = false;
                    // Jump to first matching project/task
                    self.apply_search();
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Char('q') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) || self.view_mode == ViewMode::Dashboard || self.view_mode == ViewMode::ProjectOverview {
                    self.should_quit = true;
                } else {
                    // Return to main view from sub-views
                    self.view_mode = if self.workspace_mode { ViewMode::ProjectOverview } else { ViewMode::Dashboard };
                }
            }
            KeyCode::Esc => {
                if self.view_mode != ViewMode::Dashboard && self.view_mode != ViewMode::ProjectOverview {
                    self.view_mode = if self.workspace_mode { ViewMode::ProjectOverview } else { ViewMode::Dashboard };
                }
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
            // Quick Switch: 1-9 to switch projects
            KeyCode::Char(c) if c.is_ascii_digit() && self.workspace_mode => {
                let idx = c.to_digit(10).unwrap_or(0) as usize;
                if idx > 0 && idx <= self.project_names.len() {
                    self.selected_project = idx - 1;
                    self.jump_to_project(idx - 1);
                    self.view_mode = ViewMode::Dashboard;
                } else if idx == 0 {
                    self.view_mode = ViewMode::ProjectOverview;
                }
            }
            // View switching (non-digit keys or single project mode)
            KeyCode::Char('d') => self.view_mode = ViewMode::Dashboard,
            KeyCode::Char('t') => self.view_mode = ViewMode::Terminal,
            KeyCode::Char('g') => self.view_mode = ViewMode::Graph,
            KeyCode::Char('c') => self.view_mode = ViewMode::Comparison,
            KeyCode::Char('p') if self.workspace_mode => self.view_mode = ViewMode::ProjectOverview,
            // Search mode
            KeyCode::Char('/') => {
                self.search_mode = true;
                self.search_query.clear();
            }
            KeyCode::Enter => {
                if self.view_mode == ViewMode::ProjectOverview {
                    // Enter dashboard for selected project
                    self.view_mode = ViewMode::Dashboard;
                } else {
                    // Enter terminal view for selected task
                    self.view_mode = ViewMode::Terminal;
                    self.scroll_offset = 0;
                }
            }
            KeyCode::Tab => {
                // Cycle views
                self.view_mode = match self.view_mode {
                    ViewMode::ProjectOverview => ViewMode::Dashboard,
                    ViewMode::Dashboard => ViewMode::Terminal,
                    ViewMode::Terminal => ViewMode::Graph,
                    ViewMode::Graph => ViewMode::Comparison,
                    ViewMode::Comparison => {
                        if self.workspace_mode { ViewMode::ProjectOverview } else { ViewMode::Dashboard }
                    }
                };
            }
            KeyCode::Up => {
                if self.view_mode == ViewMode::ProjectOverview {
                    if self.selected_project > 0 {
                        self.selected_project -= 1;
                    }
                } else if self.selected_task > 0 {
                    self.selected_task -= 1;
                    self.scroll_offset = 0;
                }
            }
            KeyCode::Down => {
                if self.view_mode == ViewMode::ProjectOverview {
                    if self.selected_project + 1 < self.project_names.len() {
                        self.selected_project += 1;
                    }
                } else {
                    let task_count = self.scheduler.graph().all_tasks().len();
                    if self.selected_task + 1 < task_count {
                        self.selected_task += 1;
                        self.scroll_offset = 0;
                    }
                }
            }
            KeyCode::Left | KeyCode::Right => {
                // Navigate between projects in workspace mode
                if self.workspace_mode {
                    if key.code == KeyCode::Left && self.selected_project > 0 {
                        self.selected_project -= 1;
                        self.jump_to_project(self.selected_project);
                    } else if key.code == KeyCode::Right && self.selected_project + 1 < self.project_names.len() {
                        self.selected_project += 1;
                        self.jump_to_project(self.selected_project);
                    }
                }
            }
            _ => {}
        }
    }
    
    /// Jump to a specific project (select first task of that project)
    fn jump_to_project(&mut self, project_idx: usize) {
        if let Some(project_name) = self.project_names.get(project_idx) {
            let prefix = format!("{}:", project_name);
            let task_ids = self.get_task_ids();
            for (idx, task_id) in task_ids.iter().enumerate() {
                if task_id.starts_with(&prefix) {
                    self.selected_task = idx;
                    break;
                }
            }
        }
    }
    
    /// Apply search query to find matching project/task
    fn apply_search(&mut self) {
        if self.search_query.is_empty() {
            return;
        }
        
        let query = self.search_query.to_lowercase();
        
        // First try to match project names
        for (idx, name) in self.project_names.iter().enumerate() {
            if name.to_lowercase().contains(&query) {
                self.selected_project = idx;
                self.jump_to_project(idx);
                return;
            }
        }
        
        // Then try to match task IDs
        let task_ids = self.get_task_ids();
        for (idx, task_id) in task_ids.iter().enumerate() {
            if task_id.to_lowercase().contains(&query) {
                self.selected_task = idx;
                return;
            }
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

    /// Get semantic commands for a task (from graph YAML semantic_commands field)
    pub fn get_semantic_commands(&self, task_id: &str) -> Option<TaskCommands> {
        let task = self.scheduler.graph().get_task(task_id)?;
        let map = task.semantic_commands.as_ref()?;
        if map.is_empty() {
            return None;
        }
        Some(TaskCommands::from_map(map))
    }

    /// Execute a semantic command on a running task
    pub fn execute_semantic_command(
        &self,
        task_id: &str,
        label: &str,
        params: &HashMap<String, String>,
    ) -> anyhow::Result<()> {
        let cmds = self.get_semantic_commands(task_id)
            .ok_or_else(|| anyhow::anyhow!("No semantic commands for task {}", task_id))?;
        let cmd = cmds.get(label)
            .ok_or_else(|| anyhow::anyhow!("Command '{}' not found for task {}", label, task_id))?;
        let rendered = cmd.render(params);
        self.executor.send_input(task_id, &rendered)
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
    
    /// Get project summaries for unified dashboard
    pub fn get_project_summaries(&self) -> Vec<ProjectSummary> {
        let mut summaries = Vec::new();
        let tasks_by_project = self.get_tasks_by_project();
        let graph = self.scheduler.graph();
        
        for name in &self.project_names {
            let task_ids = tasks_by_project.get(name).cloned().unwrap_or_default();
            
            let mut task_count = 0;
            let mut tasks_done = 0;
            let mut tasks_running = 0;
            let mut tasks_failed = 0;
            
            for task_id in &task_ids {
                if let Some(task) = graph.get_task(task_id) {
                    task_count += 1;
                    match task.status {
                        crate::core::GraphTaskStatus::Done => tasks_done += 1,
                        crate::core::GraphTaskStatus::InProgress => tasks_running += 1,
                        crate::core::GraphTaskStatus::Failed => tasks_failed += 1,
                        _ => {}
                    }
                }
            }
            
            // Determine agent status
            let agent_status = if tasks_failed > 0 {
                AgentStatus::Error
            } else if tasks_running > 0 {
                AgentStatus::Running
            } else if tasks_done == task_count && task_count > 0 {
                AgentStatus::Completed
            } else {
                AgentStatus::Idle
            };
            
            // Get recent event for this project
            let recent_event = self.recent_events
                .iter()
                .rev()
                .find(|(_, p, _)| p == name)
                .map(|(_, _, msg)| msg.clone());
            
            summaries.push(ProjectSummary {
                name: name.clone(),
                port: self.port_manager.get_port(name),
                agent_status,
                task_count,
                tasks_done,
                tasks_running,
                tasks_failed,
                recent_event,
            });
        }
        
        summaries
    }
    
    /// Get recent events (last N, newest first)
    pub fn get_recent_events(&self, limit: usize) -> Vec<(String, String)> {
        self.recent_events
            .iter()
            .rev()
            .take(limit)
            .map(|(_, project, msg)| (project.clone(), msg.clone()))
            .collect()
    }
    
    /// Get port for a project
    pub fn get_project_port(&self, project: &str) -> Option<u16> {
        self.port_manager.get_port(project)
    }
    
    /// Check if in search mode
    pub fn is_search_mode(&self) -> bool {
        self.search_mode
    }
    
    /// Get current search query
    pub fn get_search_query(&self) -> &str {
        &self.search_query
    }
}
