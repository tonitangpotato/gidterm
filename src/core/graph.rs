//! Graph parser - parses .gid/graph.yml and builds task DAG

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::Path;

/// Task status enum — replaces raw status strings
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GraphTaskStatus {
    Pending,
    #[serde(alias = "in-progress")]
    InProgress,
    Done,
    Failed,
    Planned,
}

impl Default for GraphTaskStatus {
    fn default() -> Self {
        Self::Pending
    }
}

impl fmt::Display for GraphTaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::InProgress => write!(f, "in-progress"),
            Self::Done => write!(f, "done"),
            Self::Failed => write!(f, "failed"),
            Self::Planned => write!(f, "planned"),
        }
    }
}

/// Task graph representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Graph {
    pub metadata: Option<Metadata>,
    #[serde(default)]
    pub nodes: HashMap<String, Node>,
    #[serde(default)]
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
    #[serde(rename = "type", default)]
    pub task_type: String,
    pub description: String,
    pub command: Option<String>,
    #[serde(default)]
    pub status: GraphTaskStatus,
    pub priority: Option<String>,
    pub depends_on: Option<Vec<String>>,
    pub component: Option<String>,
    pub estimated_hours: Option<u32>,
    pub tags: Option<Vec<String>>,
    pub semantic_commands: Option<HashMap<String, String>>,
}

impl Graph {
    /// Load graph from YAML file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let graph: Graph = serde_yaml::from_str(&content)?;
        Ok(graph)
    }

    /// Load from gid project directory
    pub fn from_gid_project(project_dir: &Path) -> Result<Self> {
        let gid_path = project_dir.join(".gid/graph.yml");
        if !gid_path.exists() {
            anyhow::bail!("No .gid/graph.yml found in {:?}", project_dir);
        }
        Self::from_file(&gid_path)
    }

    /// Auto-detect and load graph from current directory
    /// Priority:
    /// 1. .gid/graph.yml (gid project)
    /// 2. gidterm.yml (standalone config)
    /// 3. Return error if none found
    pub fn auto_load() -> Result<Self> {
        // Try .gid/graph.yml first
        let gid_path = Path::new(".gid/graph.yml");
        if gid_path.exists() {
            return Self::from_file(gid_path);
        }

        // Fall back to gidterm.yml
        let standalone_path = Path::new("gidterm.yml");
        if standalone_path.exists() {
            return Self::from_file(standalone_path);
        }

        anyhow::bail!(
            "No graph file found. Expected .gid/graph.yml or gidterm.yml in current directory."
        )
    }

    /// Get all tasks ready to run (dependencies met)
    pub fn get_ready_tasks(&self) -> Vec<String> {
        self.tasks
            .iter()
            .filter_map(|(id, task)| {
                if self.can_start(id) && task.status == GraphTaskStatus::Pending {
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
                .map(|dep_task| dep_task.status == GraphTaskStatus::Done)
                .unwrap_or(false)
        })
    }

    /// Update task status
    pub fn update_task_status(&mut self, task_id: &str, new_status: GraphTaskStatus) -> Result<()> {
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.status = new_status;
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
    

    #[test]
    fn test_parse_graph() {
        // TODO: Add test
    }
}
