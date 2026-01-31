//! Graph parser - parses .gid/graph.yml and builds task DAG

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Task graph representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Graph {
    pub metadata: Option<Metadata>,
    pub nodes: HashMap<String, Node>,
    pub tasks: HashMap<String, Task>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub project: String,
    pub version: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    #[serde(rename = "type")]
    pub node_type: String,
    pub description: String,
    pub layer: Option<String>,
    pub status: String,
    pub priority: Option<String>,
    pub depends_on: Option<Vec<String>>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    #[serde(rename = "type")]
    pub task_type: String,
    pub description: String,
    pub command: Option<String>,
    pub status: String,
    pub priority: Option<String>,
    pub depends_on: Option<Vec<String>>,
    pub component: Option<String>,
    pub estimated_hours: Option<u32>,
    pub tags: Option<Vec<String>>,
}

impl Graph {
    /// Load graph from YAML file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let graph: Graph = serde_yaml::from_str(&content)?;
        Ok(graph)
    }

    /// Get all tasks ready to run (dependencies met)
    pub fn get_ready_tasks(&self) -> Vec<String> {
        self.tasks
            .iter()
            .filter_map(|(id, task)| {
                if self.can_start(id) && task.status == "pending" {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check if a task can start (all dependencies done)
    pub fn can_start(&self, task_id: &str) -> bool {
        let Some(task) = self.tasks.get(task_id) else {
            return false;
        };

        // If no dependencies, can start
        let Some(deps) = &task.depends_on else {
            return true;
        };

        // All dependencies must be in "done" status
        deps.iter().all(|dep_id| {
            self.tasks
                .get(dep_id)
                .map(|dep_task| dep_task.status == "done")
                .unwrap_or(false)
        })
    }

    /// Update task status
    pub fn update_task_status(&mut self, task_id: &str, new_status: &str) -> Result<()> {
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.status = new_status.to_string();
            Ok(())
        } else {
            anyhow::bail!("Task {} not found", task_id)
        }
    }

    /// Get task by ID
    pub fn get_task(&self, task_id: &str) -> Option<&Task> {
        self.tasks.get(task_id)
    }

    /// Get all tasks
    pub fn all_tasks(&self) -> &HashMap<String, Task> {
        &self.tasks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_graph() {
        // TODO: Add test
    }
}
