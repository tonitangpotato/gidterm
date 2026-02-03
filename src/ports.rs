//! Port Management - Auto-assign and track development server ports
//!
//! Maintains a global registry at ~/.gidterm/ports.json to avoid conflicts.
//! Supports automatic port allocation, $PORT env var injection, and status tracking.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::TcpListener;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Default port range for auto-allocation
const PORT_RANGE_START: u16 = 3000;
const PORT_RANGE_END: u16 = 3999;

/// Port allocation entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortEntry {
    /// Allocated port number
    pub port: u16,
    /// Project name
    pub project: String,
    /// Process ID (if running)
    pub pid: Option<u32>,
    /// Whether the port is currently active
    pub active: bool,
    /// Timestamp when allocated
    pub allocated_at: u64,
    /// Last seen active timestamp
    pub last_active: Option<u64>,
    /// Optional description/task name
    pub description: Option<String>,
}

/// Port allocation status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortStatus {
    /// Port is available
    Available,
    /// Port is reserved but not in use
    Reserved,
    /// Port is actively in use
    Active,
    /// Port is in use by external process
    ExternallyUsed,
}

/// Port registry - maintains global port assignments
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PortRegistry {
    /// Port allocations keyed by project name
    pub allocations: HashMap<String, PortEntry>,
    /// Port to project mapping for quick lookup
    #[serde(skip)]
    port_map: HashMap<u16, String>,
}

impl PortRegistry {
    /// Get the default registry path
    pub fn default_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".gidterm")
            .join("ports.json")
    }

    /// Load registry from default location
    pub fn load() -> Result<Self> {
        let path = Self::default_path();
        Self::load_from(&path)
    }

    /// Load registry from specific path
    pub fn load_from(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path)?;
        let mut registry: Self = serde_json::from_str(&content)?;
        registry.rebuild_port_map();
        Ok(registry)
    }

    /// Save registry to default location
    pub fn save(&self) -> Result<()> {
        let path = Self::default_path();
        self.save_to(&path)
    }

    /// Save registry to specific path
    pub fn save_to(&self, path: &PathBuf) -> Result<()> {
        // Ensure directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Rebuild internal port map
    fn rebuild_port_map(&mut self) {
        self.port_map.clear();
        for (project, entry) in &self.allocations {
            self.port_map.insert(entry.port, project.clone());
        }
    }

    /// Get port for a project (allocate if needed)
    pub fn get_or_allocate(&mut self, project: &str, preferred: Option<u16>) -> Result<u16> {
        // Check if already allocated
        if let Some(entry) = self.allocations.get(project) {
            // Verify it's still available (not taken by external process)
            if is_port_available(entry.port) || entry.active {
                return Ok(entry.port);
            }
            // Port was taken externally, need to reallocate
            log::warn!(
                "Port {} for project {} was taken externally, reallocating",
                entry.port,
                project
            );
        }

        // Try preferred port first
        if let Some(pref) = preferred {
            if is_port_available(pref) && !self.port_map.contains_key(&pref) {
                self.allocate(project, pref)?;
                return Ok(pref);
            }
        }

        // Find next available port
        let port = self.find_available_port()?;
        self.allocate(project, port)?;
        Ok(port)
    }

    /// Find an available port in the range
    fn find_available_port(&self) -> Result<u16> {
        for port in PORT_RANGE_START..=PORT_RANGE_END {
            if !self.port_map.contains_key(&port) && is_port_available(port) {
                return Ok(port);
            }
        }
        anyhow::bail!("No available ports in range {}-{}", PORT_RANGE_START, PORT_RANGE_END)
    }

    /// Allocate a specific port to a project
    fn allocate(&mut self, project: &str, port: u16) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let entry = PortEntry {
            port,
            project: project.to_string(),
            pid: None,
            active: false,
            allocated_at: now,
            last_active: None,
            description: None,
        };

        // Remove old allocation if exists
        if let Some(old) = self.allocations.remove(project) {
            self.port_map.remove(&old.port);
        }

        self.port_map.insert(port, project.to_string());
        self.allocations.insert(project.to_string(), entry);
        self.save()?;
        Ok(())
    }

    /// Mark a port as active with optional PID
    pub fn mark_active(&mut self, project: &str, pid: Option<u32>) -> Result<()> {
        if let Some(entry) = self.allocations.get_mut(project) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            entry.active = true;
            entry.pid = pid;
            entry.last_active = Some(now);
            self.save()?;
        }
        Ok(())
    }

    /// Mark a port as inactive
    pub fn mark_inactive(&mut self, project: &str) -> Result<()> {
        if let Some(entry) = self.allocations.get_mut(project) {
            entry.active = false;
            entry.pid = None;
            self.save()?;
        }
        Ok(())
    }

    /// Release a port allocation
    pub fn release(&mut self, project: &str) -> Result<()> {
        if let Some(entry) = self.allocations.remove(project) {
            self.port_map.remove(&entry.port);
            self.save()?;
        }
        Ok(())
    }

    /// Get port status
    pub fn get_status(&self, project: &str) -> PortStatus {
        if let Some(entry) = self.allocations.get(project) {
            if entry.active {
                PortStatus::Active
            } else if is_port_available(entry.port) {
                PortStatus::Reserved
            } else {
                PortStatus::ExternallyUsed
            }
        } else {
            PortStatus::Available
        }
    }

    /// Get all allocations as a sorted vec
    pub fn list_allocations(&self) -> Vec<&PortEntry> {
        let mut entries: Vec<&PortEntry> = self.allocations.values().collect();
        entries.sort_by_key(|e| e.port);
        entries
    }

    /// Clean up stale allocations (ports that are no longer active)
    pub fn cleanup_stale(&mut self) -> Result<usize> {
        let stale: Vec<String> = self
            .allocations
            .iter()
            .filter(|(_, entry)| {
                !entry.active && !is_port_available(entry.port)
            })
            .map(|(k, _)| k.clone())
            .collect();

        let count = stale.len();
        for project in stale {
            self.allocations.remove(&project);
        }
        
        self.rebuild_port_map();
        if count > 0 {
            self.save()?;
        }
        Ok(count)
    }

    /// Refresh status of all allocations
    pub fn refresh_status(&mut self) -> Result<()> {
        for entry in self.allocations.values_mut() {
            // Check if PID is still running
            if let Some(pid) = entry.pid {
                if !is_process_running(pid) {
                    entry.active = false;
                    entry.pid = None;
                }
            }
        }
        self.save()
    }
}

/// Check if a port is available for binding
pub fn is_port_available(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

/// Check if a process is running
#[cfg(unix)]
fn is_process_running(pid: u32) -> bool {
    use std::process::Command;
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_process_running(_pid: u32) -> bool {
    // On non-Unix, assume running (conservative)
    true
}

/// Port manager for a single project/workspace session
pub struct PortManager {
    registry: PortRegistry,
    project_ports: HashMap<String, u16>,
}

impl PortManager {
    /// Create a new port manager
    pub fn new() -> Result<Self> {
        let registry = PortRegistry::load()?;
        Ok(Self {
            registry,
            project_ports: HashMap::new(),
        })
    }

    /// Allocate port for a project and return it
    pub fn allocate(&mut self, project: &str, preferred: Option<u16>) -> Result<u16> {
        let port = self.registry.get_or_allocate(project, preferred)?;
        self.project_ports.insert(project.to_string(), port);
        Ok(port)
    }

    /// Get environment variables for a project (includes $PORT)
    pub fn get_env(&self, project: &str) -> HashMap<String, String> {
        let mut env = HashMap::new();
        if let Some(port) = self.project_ports.get(project) {
            env.insert("PORT".to_string(), port.to_string());
            env.insert("GIDTERM_PORT".to_string(), port.to_string());
        }
        env
    }

    /// Mark project port as active
    pub fn activate(&mut self, project: &str, pid: Option<u32>) -> Result<()> {
        self.registry.mark_active(project, pid)
    }

    /// Mark project port as inactive
    pub fn deactivate(&mut self, project: &str) -> Result<()> {
        self.registry.mark_inactive(project)
    }

    /// Get port for a project (if allocated)
    pub fn get_port(&self, project: &str) -> Option<u16> {
        self.project_ports.get(project).copied()
    }

    /// Get all allocations
    pub fn list(&self) -> Vec<&PortEntry> {
        self.registry.list_allocations()
    }

    /// Cleanup stale allocations
    pub fn cleanup(&mut self) -> Result<usize> {
        self.registry.cleanup_stale()
    }
}

impl Default for PortManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            registry: PortRegistry::default(),
            project_ports: HashMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_port_availability() {
        // Port 65535 is typically available
        // This is a basic smoke test
        let _ = is_port_available(65534);
    }

    #[test]
    fn test_registry_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("ports.json");

        let mut registry = PortRegistry::default();
        registry.allocate("test-project", 3000).unwrap();
        registry.save_to(&path).unwrap();

        let loaded = PortRegistry::load_from(&path).unwrap();
        assert!(loaded.allocations.contains_key("test-project"));
        assert_eq!(loaded.allocations["test-project"].port, 3000);
    }

    #[test]
    fn test_port_allocation() {
        let mut registry = PortRegistry::default();
        
        // First allocation should get preferred port
        let port1 = registry.get_or_allocate("project1", Some(3000)).unwrap();
        assert_eq!(port1, 3000);

        // Same project should get same port
        let port1_again = registry.get_or_allocate("project1", Some(4000)).unwrap();
        assert_eq!(port1_again, 3000);

        // Different project should get different port
        let port2 = registry.get_or_allocate("project2", Some(3000)).unwrap();
        assert_ne!(port2, 3000); // 3000 is taken
    }
}
