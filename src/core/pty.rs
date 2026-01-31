//! PTY Manager - Create and manage pseudo-terminals

use anyhow::Result;
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

/// Handle to a managed PTY
pub struct PTYHandle {
    pub id: String,
    pair: Arc<Mutex<portable_pty::PtyPair>>,
    reader: Option<Box<dyn Read + Send>>,
    writer: Option<Box<dyn Write + Send>>,
}

/// PTY Manager for creating and controlling terminals
pub struct PTYManager {
    pty_system: NativePtySystem,
    ptys: HashMap<String, PTYHandle>,
    next_id: usize,
}

impl PTYManager {
    /// Create a new PTY manager
    pub fn new() -> Self {
        Self {
            pty_system: NativePtySystem::default(),
            ptys: HashMap::new(),
            next_id: 0,
        }
    }

    /// Spawn a new PTY with command
    pub fn spawn(&mut self, command: &str) -> Result<String> {
        // Parse command
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            anyhow::bail!("Empty command");
        }

        let mut cmd_builder = CommandBuilder::new(parts[0]);
        if parts.len() > 1 {
            for arg in &parts[1..] {
                cmd_builder.arg(arg);
            }
        }

        // Create PTY pair
        let pty_size = PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pair = self.pty_system.openpty(pty_size)?;

        // Spawn the command
        let _child = pair.slave.spawn_command(cmd_builder)?;

        // Get reader and writer
        let reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;

        // Generate ID
        let id = format!("pty-{}", self.next_id);
        self.next_id += 1;

        // Store handle
        let handle = PTYHandle {
            id: id.clone(),
            pair: Arc::new(Mutex::new(pair)),
            reader: Some(reader),
            writer: Some(writer),
        };

        self.ptys.insert(id.clone(), handle);

        Ok(id)
    }

    /// Read output from PTY
    pub fn read_output(&mut self, pty_id: &str, buf: &mut [u8]) -> Result<usize> {
        let handle = self
            .ptys
            .get_mut(pty_id)
            .ok_or_else(|| anyhow::anyhow!("PTY {} not found", pty_id))?;

        if let Some(reader) = &mut handle.reader {
            Ok(reader.read(buf)?)
        } else {
            anyhow::bail!("No reader available")
        }
    }

    /// Write input to PTY
    pub fn write_input(&mut self, pty_id: &str, data: &[u8]) -> Result<()> {
        let handle = self
            .ptys
            .get_mut(pty_id)
            .ok_or_else(|| anyhow::anyhow!("PTY {} not found", pty_id))?;

        if let Some(writer) = &mut handle.writer {
            writer.write_all(data)?;
            writer.flush()?;
            Ok(())
        } else {
            anyhow::bail!("No writer available")
        }
    }

    /// Get all PTY IDs
    pub fn list_ptys(&self) -> Vec<String> {
        self.ptys.keys().cloned().collect()
    }
}
