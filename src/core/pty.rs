//! PTY (pseudo-terminal) management - spawn and monitor processes

use anyhow::Result;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex};

/// PTY handle for a single task
#[derive(Clone)]
pub struct PTYHandle {
    pub id: String,
    output_history: Arc<Mutex<Vec<String>>>,
    reader: Arc<Mutex<Option<BufReader<Box<dyn std::io::Read + Send>>>>>,
}

impl PTYHandle {
    /// Spawn a new process in a PTY
    pub fn spawn(task_id: &str, command: &str) -> Result<Self> {
        log::info!("Spawning PTY for task {}: {}", task_id, command);

        // Parse command into parts
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            anyhow::bail!("Empty command");
        }

        // Build command
        let mut cmd = CommandBuilder::new(parts[0]);
        for arg in &parts[1..] {
            cmd.arg(arg);
        }

        // Create PTY
        let pty_system = native_pty_system();
        let pty_size = PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pair = pty_system.openpty(pty_size)?;

        // Spawn command
        let _child = pair.slave.spawn_command(cmd)?;
        drop(pair.slave); // Close slave side

        // Get reader
        let reader = pair.master.try_clone_reader()?;
        let buf_reader = BufReader::new(reader);

        Ok(Self {
            id: task_id.to_string(),
            output_history: Arc::new(Mutex::new(Vec::new())),
            reader: Arc::new(Mutex::new(Some(buf_reader))),
        })
    }

    /// Read one line of output
    pub async fn read_line(&mut self) -> Result<Option<String>> {
        let mut reader_guard = self.reader.lock().unwrap();
        
        if let Some(reader) = reader_guard.as_mut() {
            let mut line = String::new();
            
            // Try to read a line (non-blocking would be better, but this works for now)
            match reader.read_line(&mut line) {
                Ok(0) => {
                    // EOF - process ended
                    *reader_guard = None;
                    Ok(None)
                }
                Ok(_) => {
                    // Got a line
                    let trimmed = line.trim_end().to_string();
                    
                    // Store in history
                    {
                        let mut history = self.output_history.lock().unwrap();
                        history.push(trimmed.clone());
                        
                        // Keep last 1000 lines
                        if history.len() > 1000 {
                            history.remove(0);
                        }
                    }
                    
                    Ok(Some(trimmed))
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No data available yet
                    Ok(Some(String::new()))
                }
                Err(e) => Err(e.into()),
            }
        } else {
            // Reader already closed
            Ok(None)
        }
    }

    /// Get output history
    pub fn get_output(&self) -> Vec<String> {
        self.output_history.lock().unwrap().clone()
    }

    /// Kill the process
    pub fn kill(&mut self) -> Result<()> {
        // Close reader (this will kill the process)
        let mut reader = self.reader.lock().unwrap();
        *reader = None;
        Ok(())
    }
}
