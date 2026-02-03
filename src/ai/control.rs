//! Control API - unified interface for all usage modes
//!
//! Three modes:
//! - Manual: Human uses TUI directly
//! - MCP: Claude Code calls gidterm via MCP tool server
//! - Agent: Clawdbot or other automation drives programmatically

use crate::semantic::TaskMetrics;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Control mode determines how gidterm is being operated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlMode {
    /// Human interacts via TUI
    Manual,
    /// AI assistant via MCP tool calls (Claude Code)
    Mcp,
    /// Autonomous agent (clawdbot)
    Agent,
}

/// Snapshot of current gidterm state for AI consumers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub tasks: Vec<TaskSnapshot>,
    pub running_count: usize,
    pub done_count: usize,
    pub failed_count: usize,
    pub total_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSnapshot {
    pub id: String,
    pub status: String,
    pub description: String,
    pub progress: Option<f64>,
    pub metrics: Option<HashMap<String, serde_json::Value>>,
    pub last_output: Vec<String>,
}

/// Unified control interface for all modes
pub trait ControlAPI {
    /// Get current state snapshot (for AI to understand context)
    fn get_state(&self) -> Result<StateSnapshot>;

    /// Start a specific task by ID
    fn start_task(&mut self, task_id: &str) -> Result<()>;

    /// Stop/kill a running task
    fn stop_task(&mut self, task_id: &str) -> Result<()>;

    /// Get output lines for a task
    fn get_output(&self, task_id: &str, last_n: usize) -> Result<Vec<String>>;

    /// Get metrics for a task
    fn get_metrics(&self, task_id: &str) -> Result<Option<TaskMetrics>>;

    /// Send input to a running task's stdin
    fn send_input(&self, task_id: &str, input: &str) -> Result<()>;

    /// Get the active control mode
    fn mode(&self) -> ControlMode;
}

/// Command that can be sent to gidterm from any control mode
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ControlCommand {
    /// Start all ready tasks
    StartAll,
    /// Start a specific task
    StartTask { task_id: String },
    /// Stop a task
    StopTask { task_id: String },
    /// Send input to task stdin
    SendInput { task_id: String, input: String },
    /// Request state snapshot
    GetState,
    /// Request task output
    GetOutput { task_id: String, lines: usize },
    /// Quit gidterm
    Quit,
}

/// Response from gidterm to a control command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ControlResponse {
    Ok {
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<serde_json::Value>,
    },
    Error {
        message: String,
    },
}

impl ControlResponse {
    pub fn ok() -> Self {
        Self::Ok { data: None }
    }

    pub fn ok_with_data(data: serde_json::Value) -> Self {
        Self::Ok { data: Some(data) }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self::Error {
            message: msg.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_command_serialization() {
        let cmd = ControlCommand::StartTask {
            task_id: "build".to_string(),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("start_task"));
        assert!(json.contains("build"));

        let parsed: ControlCommand = serde_json::from_str(&json).unwrap();
        match parsed {
            ControlCommand::StartTask { task_id } => assert_eq!(task_id, "build"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_control_response() {
        let resp = ControlResponse::ok();
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("ok"));

        let resp = ControlResponse::error("not found");
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("not found"));
    }

    #[test]
    fn test_state_snapshot_serialization() {
        let snap = StateSnapshot {
            tasks: vec![TaskSnapshot {
                id: "build".to_string(),
                status: "done".to_string(),
                description: "Build project".to_string(),
                progress: Some(1.0),
                metrics: None,
                last_output: vec!["Compiling...".to_string()],
            }],
            running_count: 0,
            done_count: 1,
            failed_count: 0,
            total_count: 1,
        };
        let json = serde_json::to_string(&snap).unwrap();
        assert!(json.contains("build"));
        assert!(json.contains("Build project"));
    }
}
