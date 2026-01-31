//! Session persistence - track task history across runs

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const SESSIONS_DIR: &str = ".gidterm/sessions";

/// A session represents one gidterm run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub project: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub tasks: HashMap<String, TaskHistory>,
}

/// History of a single task across multiple runs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskHistory {
    pub task_id: String,
    pub runs: Vec<TaskRun>,
}

/// A single run of a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRun {
    pub started: DateTime<Utc>,
    pub ended: Option<DateTime<Utc>>,
    pub status: TaskStatus,
    pub output: Vec<String>,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Done,
    Failed,
}

impl Session {
    /// Create a new session
    pub fn new(project: String) -> Self {
        let id = Utc::now().format("%Y-%m-%d-%H-%M-%S").to_string();
        Self {
            id,
            project,
            started_at: Utc::now(),
            ended_at: None,
            tasks: HashMap::new(),
        }
    }

    /// Get session file path
    fn session_path(&self) -> PathBuf {
        Path::new(SESSIONS_DIR).join(format!("{}.json", self.id))
    }

    /// Save session to disk
    pub fn save(&self) -> Result<()> {
        // Ensure sessions directory exists
        fs::create_dir_all(SESSIONS_DIR)?;

        let path = self.session_path();
        let json = serde_json::to_string_pretty(self)?;
        fs::write(&path, json)?;

        // Update latest symlink
        let latest_path = Path::new(SESSIONS_DIR).join("latest.json");
        #[cfg(unix)]
        {
            use std::os::unix::fs as unix_fs;
            let _ = fs::remove_file(&latest_path); // Ignore if doesn't exist
            unix_fs::symlink(&path, &latest_path)?;
        }

        Ok(())
    }

    /// Load session from disk
    pub fn load(id: &str) -> Result<Self> {
        let path = Path::new(SESSIONS_DIR).join(format!("{}.json", id));
        let content = fs::read_to_string(&path)?;
        let session: Session = serde_json::from_str(&content)?;
        Ok(session)
    }

    /// Load the latest session
    pub fn load_latest() -> Result<Self> {
        let latest_path = Path::new(SESSIONS_DIR).join("latest.json");
        if !latest_path.exists() {
            anyhow::bail!("No latest session found");
        }

        let content = fs::read_to_string(&latest_path)?;
        let session: Session = serde_json::from_str(&content)?;
        Ok(session)
    }

    /// List all sessions
    pub fn list_all() -> Result<Vec<String>> {
        if !Path::new(SESSIONS_DIR).exists() {
            return Ok(Vec::new());
        }

        let mut sessions = Vec::new();
        for entry in fs::read_dir(SESSIONS_DIR)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if stem != "latest" {
                        sessions.push(stem.to_string());
                    }
                }
            }
        }

        sessions.sort();
        sessions.reverse(); // Most recent first
        Ok(sessions)
    }

    /// Start tracking a task
    pub fn start_task(&mut self, task_id: String) {
        let task_history = self.tasks.entry(task_id.clone()).or_insert(TaskHistory {
            task_id: task_id.clone(),
            runs: Vec::new(),
        });

        task_history.runs.push(TaskRun {
            started: Utc::now(),
            ended: None,
            status: TaskStatus::Running,
            output: Vec::new(),
            exit_code: None,
        });
    }

    /// End task with status
    pub fn end_task(&mut self, task_id: &str, status: TaskStatus, exit_code: Option<i32>) {
        if let Some(task_history) = self.tasks.get_mut(task_id) {
            if let Some(last_run) = task_history.runs.last_mut() {
                last_run.ended = Some(Utc::now());
                last_run.status = status;
                last_run.exit_code = exit_code;
            }
        }
    }

    /// Add output line to current task run
    pub fn add_output(&mut self, task_id: &str, line: String) {
        if let Some(task_history) = self.tasks.get_mut(task_id) {
            if let Some(last_run) = task_history.runs.last_mut() {
                last_run.output.push(line);
            }
        }
    }

    /// End the session
    pub fn end(&mut self) {
        self.ended_at = Some(Utc::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new("test-project".to_string());
        assert_eq!(session.project, "test-project");
        assert!(session.ended_at.is_none());
        assert!(session.tasks.is_empty());
    }

    #[test]
    fn test_task_tracking() {
        let mut session = Session::new("test".to_string());
        session.start_task("task1".to_string());

        assert!(session.tasks.contains_key("task1"));
        let task = &session.tasks["task1"];
        assert_eq!(task.runs.len(), 1);
        assert_eq!(task.runs[0].status, TaskStatus::Running);

        session.end_task("task1", TaskStatus::Done, Some(0));
        let task = &session.tasks["task1"];
        assert_eq!(task.runs[0].status, TaskStatus::Done);
        assert_eq!(task.runs[0].exit_code, Some(0));
    }
}
