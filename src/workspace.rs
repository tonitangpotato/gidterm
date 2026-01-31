//! Multi-project workspace management

use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::Graph;

/// A workspace containing multiple projects
#[derive(Debug, Clone)]
pub struct Workspace {
    pub root: PathBuf,
    pub projects: HashMap<String, Project>,
}

/// A single project within a workspace
#[derive(Debug, Clone)]
pub struct Project {
    pub name: String,
    pub path: PathBuf,
    pub graph: Graph,
}

impl Workspace {
    /// Discover and load all projects in a directory
    pub fn discover(root: &Path) -> Result<Self> {
        let mut projects = HashMap::new();

        // Walk subdirectories looking for .gid/graph.yml
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let graph_path = path.join(".gid/graph.yml");
                if graph_path.exists() {
                    let name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    match Graph::from_file(&graph_path) {
                        Ok(graph) => {
                            projects.insert(
                                name.clone(),
                                Project {
                                    name,
                                    path,
                                    graph,
                                },
                            );
                        }
                        Err(e) => {
                            log::warn!("Failed to load graph from {:?}: {}", graph_path, e);
                        }
                    }
                }
            }
        }

        if projects.is_empty() {
            anyhow::bail!("No projects found with .gid/graph.yml in {}", root.display());
        }

        Ok(Self {
            root: root.to_path_buf(),
            projects,
        })
    }

    /// Load specific projects by name
    pub fn load_projects(root: &Path, project_names: &[String]) -> Result<Self> {
        let mut projects = HashMap::new();

        for name in project_names {
            let project_path = root.join(name);
            let graph_path = project_path.join(".gid/graph.yml");

            if !graph_path.exists() {
                anyhow::bail!("No .gid/graph.yml found in project: {}", name);
            }

            let graph = Graph::from_file(&graph_path)?;
            projects.insert(
                name.clone(),
                Project {
                    name: name.clone(),
                    path: project_path,
                    graph,
                },
            );
        }

        Ok(Self {
            root: root.to_path_buf(),
            projects,
        })
    }

    /// Create a unified graph with namespaced task IDs
    /// Task IDs become: "project:task_id"
    pub fn to_unified_graph(&self) -> Graph {
        let mut unified_tasks = HashMap::new();
        let mut unified_nodes = HashMap::new();

        for (project_name, project) in &self.projects {
            // Namespace tasks with project name
            for (task_id, task) in &project.graph.tasks {
                let namespaced_id = format!("{}:{}", project_name, task_id);
                
                // Clone and update dependencies to be namespaced too
                let mut namespaced_task = task.clone();
                if let Some(deps) = &task.depends_on {
                    namespaced_task.depends_on = Some(
                        deps.iter()
                            .map(|dep| format!("{}:{}", project_name, dep))
                            .collect(),
                    );
                }

                unified_tasks.insert(namespaced_id, namespaced_task);
            }

            // Namespace nodes too
            for (node_id, node) in &project.graph.nodes {
                let namespaced_id = format!("{}:{}", project_name, node_id);
                let mut namespaced_node = node.clone();
                
                if let Some(deps) = &node.depends_on {
                    namespaced_node.depends_on = Some(
                        deps.iter()
                            .map(|dep| format!("{}:{}", project_name, dep))
                            .collect(),
                    );
                }

                unified_nodes.insert(namespaced_id, namespaced_node);
            }
        }

        Graph {
            metadata: Some(crate::core::Metadata {
                project: "workspace".to_string(),
                version: Some("1.0.0".to_string()),
                description: Some(format!("{} projects", self.projects.len())),
            }),
            nodes: unified_nodes,
            tasks: unified_tasks,
        }
    }

    /// Get project count
    pub fn project_count(&self) -> usize {
        self.projects.len()
    }

    /// Get total task count across all projects
    pub fn total_task_count(&self) -> usize {
        self.projects
            .values()
            .map(|p| p.graph.tasks.len())
            .sum()
    }

    /// Get project by name
    pub fn get_project(&self, name: &str) -> Option<&Project> {
        self.projects.get(name)
    }

    /// List all project names
    pub fn project_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.projects.keys().cloned().collect();
        names.sort();
        names
    }
}

#[cfg(test)]
mod tests {
    

    #[test]
    fn test_workspace_creation() {
        // Test that workspace can be created
        // (Actual discovery would need real filesystem)
    }
}
