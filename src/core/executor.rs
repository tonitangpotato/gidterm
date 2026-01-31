//! Task Executor - Actually runs tasks in PTY and monitors them

use super::pty::PTYHandle;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Task execution event
#[derive(Debug, Clone)]
pub enum TaskEvent {
    Started { task_id: String },
    Output { task_id: String, line: String },
    Completed { task_id: String, exit_code: i32 },
    Failed { task_id: String, error: String },
}

/// Task executor - manages running tasks
pub struct Executor {
    handles: Arc<Mutex<HashMap<String, PTYHandle>>>,
    event_tx: mpsc::UnboundedSender<TaskEvent>,
}

impl Executor {
    /// Create a new executor
    pub fn new() -> (Self, mpsc::UnboundedReceiver<TaskEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        
        (
            Self {
                handles: Arc::new(Mutex::new(HashMap::new())),
                event_tx: tx,
            },
            rx,
        )
    }

    /// Start a task
    pub async fn start_task(&self, task_id: &str, command: &str) -> Result<()> {
        log::info!("Starting task: {} with command: {}", task_id, command);

        // Create PTY
        let handle = PTYHandle::spawn(task_id, command)?;
        
        // Store handle
        {
            let mut handles = self.handles.lock().unwrap();
            handles.insert(task_id.to_string(), handle.clone());
        }

        // Send started event
        let _ = self.event_tx.send(TaskEvent::Started {
            task_id: task_id.to_string(),
        });

        // Spawn reader task
        let task_id_clone = task_id.to_string();
        let event_tx = self.event_tx.clone();
        let handles_clone = self.handles.clone();
        
        tokio::spawn(async move {
            // Read output
            loop {
                // Get handle (scope mutex to release before await)
                let mut handle_clone = {
                    let handles = handles_clone.lock().unwrap();
                    if let Some(handle) = handles.get(&task_id_clone) {
                        handle.clone()
                    } else {
                        break;
                    }
                };

                let line = handle_clone.read_line().await;

                match line {
                    Ok(Some(line)) => {
                        let _ = event_tx.send(TaskEvent::Output {
                            task_id: task_id_clone.clone(),
                            line,
                        });
                    }
                    Ok(None) => {
                        // Process ended
                        log::info!("Task {} completed", task_id_clone);
                        let _ = event_tx.send(TaskEvent::Completed {
                            task_id: task_id_clone.clone(),
                            exit_code: 0,
                        });
                        break;
                    }
                    Err(e) => {
                        log::error!("Task {} failed: {}", task_id_clone, e);
                        let _ = event_tx.send(TaskEvent::Failed {
                            task_id: task_id_clone.clone(),
                            error: e.to_string(),
                        });
                        break;
                    }
                }
            }

            // Cleanup
            let mut handles = handles_clone.lock().unwrap();
            handles.remove(&task_id_clone);
        });

        Ok(())
    }

    /// Stop a task
    pub fn stop_task(&self, task_id: &str) -> Result<()> {
        let mut handles = self.handles.lock().unwrap();
        
        if let Some(mut handle) = handles.remove(task_id) {
            handle.kill()?;
            log::info!("Stopped task: {}", task_id);
        }

        Ok(())
    }

    /// Get task output history
    pub fn get_output(&self, task_id: &str) -> Vec<String> {
        let handles = self.handles.lock().unwrap();
        
        if let Some(handle) = handles.get(task_id) {
            handle.get_output()
        } else {
            vec![]
        }
    }

    /// Check if task is running
    pub fn is_running(&self, task_id: &str) -> bool {
        let handles = self.handles.lock().unwrap();
        handles.contains_key(task_id)
    }
}
