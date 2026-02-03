//! Task Scheduler - DAG-based task dependency scheduling

use super::{Graph, GraphTaskStatus};
use anyhow::Result;
use std::collections::HashSet;

/// Task scheduler with dependency resolution
pub struct Scheduler {
    graph: Graph,
    running: HashSet<String>,
}

impl Scheduler {
    /// Create a new scheduler from graph
    pub fn new(graph: Graph) -> Self {
        Self {
            graph,
            running: HashSet::new(),
        }
    }

    /// Schedule next tasks to run
    pub fn schedule_next(&mut self) -> Vec<String> {
        let ready = self.graph.get_ready_tasks();
        
        // Filter out tasks that are already running
        ready
            .into_iter()
            .filter(|id| !self.running.contains(id))
            .collect()
    }

    /// Mark task as started
    pub fn mark_started(&mut self, task_id: &str) -> Result<()> {
        self.graph.update_task_status(task_id, GraphTaskStatus::InProgress)?;
        self.running.insert(task_id.to_string());
        Ok(())
    }

    /// Mark task as completed
    pub fn mark_done(&mut self, task_id: &str) -> Result<()> {
        self.graph.update_task_status(task_id, GraphTaskStatus::Done)?;
        self.running.remove(task_id);
        Ok(())
    }

    /// Mark task as failed
    pub fn mark_failed(&mut self, task_id: &str) -> Result<()> {
        self.graph.update_task_status(task_id, GraphTaskStatus::Failed)?;
        self.running.remove(task_id);
        Ok(())
    }

    /// Get currently running tasks
    pub fn get_running(&self) -> Vec<String> {
        self.running.iter().cloned().collect()
    }

    /// Get graph reference
    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    /// Check if all tasks are done
    pub fn all_done(&self) -> bool {
        self.running.is_empty()
            && self
                .graph
                .all_tasks()
                .values()
                .all(|task| task.status == GraphTaskStatus::Done || task.status == GraphTaskStatus::Failed)
    }
}
