//! Task Executor - Runs tasks in PTY and monitors them
//!
//! Uses tokio::task::spawn_blocking for PTY reads to avoid
//! blocking the async runtime.

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

        // Spawn reader task — uses spawn_blocking for the actual I/O
        let task_id_owned = task_id.to_string();
        let event_tx = self.event_tx.clone();
        let handles_ref = self.handles.clone();
        let reader_handle = handle.clone();

        tokio::spawn(async move {
            loop {
                // Clone handle for the blocking read
                let rh = reader_handle.clone();

                // Read one line in a blocking thread
                let line_result = tokio::task::spawn_blocking(move || {
                    rh.read_line_blocking()
                })
                .await;

                match line_result {
                    Ok(Ok(Some(line))) => {
                        if !line.is_empty() {
                            let _ = event_tx.send(TaskEvent::Output {
                                task_id: task_id_owned.clone(),
                                line,
                            });
                        }
                    }
                    Ok(Ok(None)) => {
                        // EOF — process ended, get exit code
                        let exit_code = reader_handle
                            .try_wait()
                            .ok()
                            .flatten()
                            .map(|r| r.code)
                            .unwrap_or(0);

                        if exit_code == 0 {
                            log::info!("Task {} completed (exit: {})", task_id_owned, exit_code);
                            let _ = event_tx.send(TaskEvent::Completed {
                                task_id: task_id_owned.clone(),
                                exit_code,
                            });
                        } else {
                            log::warn!("Task {} failed (exit: {})", task_id_owned, exit_code);
                            let _ = event_tx.send(TaskEvent::Failed {
                                task_id: task_id_owned.clone(),
                                error: format!("Process exited with code {}", exit_code),
                            });
                        }
                        break;
                    }
                    Ok(Err(e)) => {
                        log::error!("Task {} read error: {}", task_id_owned, e);
                        let _ = event_tx.send(TaskEvent::Failed {
                            task_id: task_id_owned.clone(),
                            error: e.to_string(),
                        });
                        break;
                    }
                    Err(e) => {
                        // spawn_blocking join error
                        log::error!("Task {} spawn_blocking error: {}", task_id_owned, e);
                        let _ = event_tx.send(TaskEvent::Failed {
                            task_id: task_id_owned.clone(),
                            error: format!("Internal error: {}", e),
                        });
                        break;
                    }
                }
            }

            // Cleanup
            let mut handles = handles_ref.lock().unwrap();
            handles.remove(&task_id_owned);
        });

        Ok(())
    }

    /// Stop a task (sends kill signal)
    pub fn stop_task(&self, task_id: &str) -> Result<()> {
        let handles = self.handles.lock().unwrap();

        if let Some(handle) = handles.get(task_id) {
            handle.kill()?;
            log::info!("Stopped task: {}", task_id);
        }

        Ok(())
    }

    /// Send input to a task's PTY
    pub fn send_input(&self, task_id: &str, input: &str) -> Result<()> {
        let handles = self.handles.lock().unwrap();

        if let Some(handle) = handles.get(task_id) {
            handle.send_input(input)?;
        } else {
            anyhow::bail!("Task {} not running", task_id);
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

    /// Stop all running tasks
    pub fn stop_all(&self) {
        let handles = self.handles.lock().unwrap();
        for (task_id, handle) in handles.iter() {
            if let Err(e) = handle.kill() {
                log::warn!("Failed to kill task {}: {}", task_id, e);
            }
        }
    }
}
