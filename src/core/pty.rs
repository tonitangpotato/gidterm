//! PTY (pseudo-terminal) management - spawn and monitor processes

use anyhow::Result;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::io::{BufRead, BufReader, Read};
use std::sync::{Arc, Mutex};

/// Output line limit per task
const MAX_OUTPUT_LINES: usize = 1000;

/// PTY handle for a single task
#[derive(Clone)]
pub struct PTYHandle {
    pub id: String,
    output_history: Arc<Mutex<Vec<String>>>,
    reader: Arc<Mutex<Option<BufReader<Box<dyn Read + Send>>>>>,
    child: Arc<Mutex<Option<Box<dyn Child + Send + Sync>>>>,
    master: Arc<Mutex<Option<Box<dyn MasterPty + Send>>>>,
}

impl PTYHandle {
    /// Spawn a new process in a PTY
    ///
    /// Commands are wrapped in `sh -c "..."` to support:
    /// - Pipes: `cat file | grep foo`
    /// - Chaining: `cd dir && npm run dev`
    /// - Quoted args: `echo "hello world"`
    /// - Environment variables: `FOO=bar cmd`
    pub fn spawn(task_id: &str, command: &str) -> Result<Self> {
        log::info!("Spawning PTY for task {}: {}", task_id, command);

        if command.trim().is_empty() {
            anyhow::bail!("Empty command");
        }

        // Wrap in sh -c for proper shell interpretation
        let mut cmd = CommandBuilder::new("sh");
        cmd.arg("-c");
        cmd.arg(command);

        // Create PTY
        let pty_system = native_pty_system();
        let pty_size = PtySize {
            rows: 24,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pair = pty_system.openpty(pty_size)?;

        // Spawn command
        let child = pair.slave.spawn_command(cmd)?;
        drop(pair.slave); // Close slave side

        // Get reader from master
        let reader = pair.master.try_clone_reader()?;
        let buf_reader = BufReader::new(reader);

        Ok(Self {
            id: task_id.to_string(),
            output_history: Arc::new(Mutex::new(Vec::new())),
            reader: Arc::new(Mutex::new(Some(buf_reader))),
            child: Arc::new(Mutex::new(Some(child))),
            master: Arc::new(Mutex::new(Some(pair.master))),
        })
    }

    /// Read one line of output (blocking — call from spawn_blocking!)
    pub fn read_line_blocking(&self) -> Result<Option<String>> {
        let mut reader_guard = self.reader.lock().unwrap();

        if let Some(reader) = reader_guard.as_mut() {
            let mut line = String::new();

            match reader.read_line(&mut line) {
                Ok(0) => {
                    // EOF - process ended
                    *reader_guard = None;
                    Ok(None)
                }
                Ok(_) => {
                    let trimmed = line.trim_end().to_string();

                    // Store in history
                    {
                        let mut history = self.output_history.lock().unwrap();
                        history.push(trimmed.clone());

                        // Cap history
                        if history.len() > MAX_OUTPUT_LINES {
                            let drain_count = history.len() - MAX_OUTPUT_LINES;
                            history.drain(0..drain_count);
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
            Ok(None)
        }
    }

    /// Get output history
    pub fn get_output(&self) -> Vec<String> {
        self.output_history.lock().unwrap().clone()
    }

    /// Send input to the PTY (for semantic commands)
    pub fn send_input(&self, input: &str) -> Result<()> {
        let master_guard = self.master.lock().unwrap();
        if let Some(master) = master_guard.as_ref() {
            use std::io::Write;
            let mut writer = master.take_writer()?;
            writer.write_all(input.as_bytes())?;
            writer.write_all(b"\n")?;
            writer.flush()?;
            Ok(())
        } else {
            anyhow::bail!("PTY master already closed for task {}", self.id)
        }
    }

    /// Try to get exit status (non-blocking)
    pub fn try_wait(&self) -> Result<Option<ExitResult>> {
        let mut child_guard = self.child.lock().unwrap();
        if let Some(child) = child_guard.as_mut() {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let code = status
                        .exit_code()
                        .try_into()
                        .unwrap_or(1);
                    Ok(Some(ExitResult { code }))
                }
                Ok(None) => Ok(None), // Still running
                Err(e) => Err(e.into()),
            }
        } else {
            Ok(Some(ExitResult { code: -1 })) // Child already gone
        }
    }

    /// Kill the process (SIGKILL equivalent)
    pub fn kill(&self) -> Result<()> {
        // Kill child process
        {
            let mut child_guard = self.child.lock().unwrap();
            if let Some(mut child) = child_guard.take() {
                child.kill()?;
                log::info!("Killed process for task {}", self.id);
            }
        }

        // Close reader
        {
            let mut reader = self.reader.lock().unwrap();
            *reader = None;
        }

        // Close master
        {
            let mut master = self.master.lock().unwrap();
            *master = None;
        }

        Ok(())
    }

    /// Check if process is still alive
    pub fn is_alive(&self) -> bool {
        let child_guard = self.child.lock().unwrap();
        child_guard.is_some()
    }
}

/// Result from process exit
#[derive(Debug, Clone)]
pub struct ExitResult {
    pub code: i32,
}

impl std::fmt::Debug for PTYHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PTYHandle")
            .field("id", &self.id)
            .field("alive", &self.is_alive())
            .finish()
    }
}
