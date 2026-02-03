//! System Notifications - macOS notification center integration
//!
//! Sends notifications when:
//! - Agent/task completes
//! - Agent/task fails/errors
//! - Agent waiting for input
//!
//! Uses osascript for macOS native notifications with sound support.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::process::Command;

/// Notification priority/urgency
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
    Critical,
}

impl Default for NotificationPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Notification event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationEvent {
    /// Task/agent completed successfully
    Complete,
    /// Task/agent failed with error
    Error,
    /// Agent waiting for user input
    WaitingInput,
    /// Task started
    Started,
    /// Warning (non-fatal issue)
    Warning,
}

impl NotificationEvent {
    /// Get emoji for event type
    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Complete => "✅",
            Self::Error => "❌",
            Self::WaitingInput => "⏳",
            Self::Started => "🚀",
            Self::Warning => "⚠️",
        }
    }

    /// Get default priority for event type
    pub fn default_priority(&self) -> NotificationPriority {
        match self {
            Self::Complete => NotificationPriority::Normal,
            Self::Error => NotificationPriority::High,
            Self::WaitingInput => NotificationPriority::High,
            Self::Started => NotificationPriority::Low,
            Self::Warning => NotificationPriority::Normal,
        }
    }

    /// Get sound name for event type
    pub fn sound(&self) -> Option<&'static str> {
        match self {
            Self::Complete => Some("Glass"),
            Self::Error => Some("Basso"),
            Self::WaitingInput => Some("Ping"),
            Self::Started => None,
            Self::Warning => Some("Pop"),
        }
    }
}

/// Notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Enable notifications
    pub enabled: bool,
    /// Notify on task complete
    pub on_complete: bool,
    /// Notify on task error
    pub on_error: bool,
    /// Notify when agent waiting for input
    pub on_waiting: bool,
    /// Notify on task start
    pub on_start: bool,
    /// Play sound with notifications
    pub sound: bool,
    /// Suppress notifications during quiet hours (23:00-08:00)
    pub quiet_hours: bool,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            on_complete: true,
            on_error: true,
            on_waiting: true,
            on_start: false,
            sound: true,
            quiet_hours: true,
        }
    }
}

impl NotificationConfig {
    /// Check if notification should be sent for event type
    pub fn should_notify(&self, event: NotificationEvent) -> bool {
        if !self.enabled {
            return false;
        }

        // Check quiet hours
        if self.quiet_hours && is_quiet_hours() {
            // Only allow high priority during quiet hours
            if event.default_priority() != NotificationPriority::High
                && event.default_priority() != NotificationPriority::Critical
            {
                return false;
            }
        }

        match event {
            NotificationEvent::Complete => self.on_complete,
            NotificationEvent::Error => self.on_error,
            NotificationEvent::WaitingInput => self.on_waiting,
            NotificationEvent::Started => self.on_start,
            NotificationEvent::Warning => self.on_complete, // Group with complete
        }
    }
}

/// Check if current time is within quiet hours (23:00-08:00)
fn is_quiet_hours() -> bool {
    use chrono::Timelike;
    let now = chrono::Local::now();
    let hour = now.hour();
    hour >= 23 || hour < 8
}

/// Notification payload
#[derive(Debug, Clone)]
pub struct Notification {
    /// Title of the notification
    pub title: String,
    /// Body/message of the notification
    pub message: String,
    /// Optional subtitle (project name)
    pub subtitle: Option<String>,
    /// Event type
    pub event: NotificationEvent,
    /// Optional sound override
    pub sound: Option<String>,
}

impl Notification {
    /// Create a new notification
    pub fn new(title: impl Into<String>, message: impl Into<String>, event: NotificationEvent) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            subtitle: None,
            event,
            sound: None,
        }
    }

    /// Set subtitle (project name)
    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Set custom sound
    pub fn with_sound(mut self, sound: impl Into<String>) -> Self {
        self.sound = Some(sound.into());
        self
    }

    /// Build notification with emoji in title
    pub fn formatted_title(&self) -> String {
        format!("{} {}", self.event.emoji(), self.title)
    }
}

/// Notification manager - sends system notifications
pub struct NotificationManager {
    config: NotificationConfig,
    /// Track recent notifications to avoid spam
    recent: Vec<(String, std::time::Instant)>,
    /// Minimum interval between duplicate notifications (seconds)
    dedup_interval: u64,
}

impl NotificationManager {
    /// Create a new notification manager
    pub fn new() -> Self {
        Self {
            config: NotificationConfig::default(),
            recent: Vec::new(),
            dedup_interval: 30,
        }
    }

    /// Create with custom config
    pub fn with_config(config: NotificationConfig) -> Self {
        Self {
            config,
            recent: Vec::new(),
            dedup_interval: 30,
        }
    }

    /// Update config
    pub fn set_config(&mut self, config: NotificationConfig) {
        self.config = config;
    }

    /// Send a notification
    pub fn send(&mut self, notification: &Notification) -> Result<()> {
        // Check if we should notify for this event
        if !self.config.should_notify(notification.event) {
            log::debug!("Notification suppressed: {:?}", notification.event);
            return Ok(());
        }

        // Deduplicate
        let key = format!("{}:{}", notification.title, notification.message);
        let now = std::time::Instant::now();
        
        // Clean old entries
        self.recent.retain(|(_, t)| now.duration_since(*t).as_secs() < self.dedup_interval);
        
        // Check for duplicate
        if self.recent.iter().any(|(k, _)| k == &key) {
            log::debug!("Notification deduplicated: {}", key);
            return Ok(());
        }
        self.recent.push((key, now));

        // Send the notification
        self.send_macos_notification(notification)
    }

    /// Send macOS notification via osascript
    fn send_macos_notification(&self, notification: &Notification) -> Result<()> {
        let title = notification.formatted_title();
        let subtitle = notification.subtitle.as_deref().unwrap_or("");
        let message = &notification.message;

        // Get sound
        let sound = if self.config.sound {
            notification.sound.as_deref()
                .or_else(|| notification.event.sound())
        } else {
            None
        };

        // Build AppleScript
        let sound_clause = if let Some(s) = sound {
            format!(" sound name \"{}\"", s)
        } else {
            String::new()
        };

        let subtitle_clause = if !subtitle.is_empty() {
            format!(" subtitle \"{}\"", escape_applescript(subtitle))
        } else {
            String::new()
        };

        let script = format!(
            r#"display notification "{}" with title "{}"{}{}"#,
            escape_applescript(message),
            escape_applescript(&title),
            subtitle_clause,
            sound_clause
        );

        let output = Command::new("osascript")
            .args(["-e", &script])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::warn!("Failed to send notification: {}", stderr);
        } else {
            log::debug!("Notification sent: {}", title);
        }

        Ok(())
    }

    /// Send task completed notification
    pub fn notify_complete(&mut self, project: &str, task: &str, duration: Option<std::time::Duration>) -> Result<()> {
        let duration_str = duration
            .map(|d| format!(" ({})", format_duration(d)))
            .unwrap_or_default();

        let notification = Notification::new(
            "Task Completed",
            format!("{}{}", task, duration_str),
            NotificationEvent::Complete,
        )
        .with_subtitle(project);

        self.send(&notification)
    }

    /// Send task error notification
    pub fn notify_error(&mut self, project: &str, task: &str, error: &str) -> Result<()> {
        let notification = Notification::new(
            "Task Failed",
            format!("{}: {}", task, truncate(error, 100)),
            NotificationEvent::Error,
        )
        .with_subtitle(project);

        self.send(&notification)
    }

    /// Send waiting for input notification
    pub fn notify_waiting(&mut self, project: &str, task: &str) -> Result<()> {
        let notification = Notification::new(
            "Waiting for Input",
            format!("{} needs your attention", task),
            NotificationEvent::WaitingInput,
        )
        .with_subtitle(project);

        self.send(&notification)
    }

    /// Send task started notification
    pub fn notify_started(&mut self, project: &str, task: &str) -> Result<()> {
        let notification = Notification::new(
            "Task Started",
            task.to_string(),
            NotificationEvent::Started,
        )
        .with_subtitle(project);

        self.send(&notification)
    }

    /// Send warning notification  
    pub fn notify_warning(&mut self, project: &str, message: &str) -> Result<()> {
        let notification = Notification::new(
            "Warning",
            message.to_string(),
            NotificationEvent::Warning,
        )
        .with_subtitle(project);

        self.send(&notification)
    }
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Escape string for AppleScript
fn escape_applescript(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// Format duration as human-readable string
fn format_duration(d: std::time::Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

/// Truncate string with ellipsis
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_applescript() {
        assert_eq!(escape_applescript("hello"), "hello");
        assert_eq!(escape_applescript("hello \"world\""), "hello \\\"world\\\"");
        assert_eq!(escape_applescript("line1\nline2"), "line1\\nline2");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(std::time::Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(std::time::Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(std::time::Duration::from_secs(3665)), "1h 1m");
    }

    #[test]
    fn test_notification_config() {
        let config = NotificationConfig::default();
        assert!(config.should_notify(NotificationEvent::Complete));
        assert!(config.should_notify(NotificationEvent::Error));
        assert!(!config.should_notify(NotificationEvent::Started));
    }

    #[test]
    fn test_notification_event_emoji() {
        assert_eq!(NotificationEvent::Complete.emoji(), "✅");
        assert_eq!(NotificationEvent::Error.emoji(), "❌");
    }
}
