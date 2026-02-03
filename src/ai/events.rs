//! Event streaming for AI integration
//!
//! JSON-serializable events that can be consumed by any control mode.

use crate::semantic::advisor::Advisory;
use crate::semantic::TaskMetrics;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::broadcast;

/// Events emitted by gidterm for AI/automation consumers
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GidEvent {
    /// Task started executing
    TaskStarted {
        task_id: String,
    },
    /// Task produced output
    TaskOutput {
        task_id: String,
        line: String,
    },
    /// Task completed successfully
    TaskCompleted {
        task_id: String,
        exit_code: i32,
    },
    /// Task failed
    TaskFailed {
        task_id: String,
        error: String,
    },
    /// Metrics updated for a task
    MetricsUpdated {
        task_id: String,
        progress: f64,
        metrics: HashMap<String, serde_json::Value>,
    },
    /// Advisory triggered
    AdvisoryTriggered {
        task_id: String,
        severity: String,
        message: String,
        suggestion: String,
    },
    /// All tasks completed
    AllDone {
        total: usize,
        succeeded: usize,
        failed: usize,
    },
}

impl GidEvent {
    /// Create a metrics updated event from TaskMetrics
    pub fn from_metrics(task_id: &str, metrics: &TaskMetrics) -> Self {
        let json_metrics: HashMap<String, serde_json::Value> = metrics
            .metrics
            .iter()
            .map(|(k, v)| {
                let jv = match v {
                    crate::semantic::MetricValue::Float(f) => serde_json::json!(f),
                    crate::semantic::MetricValue::Int(i) => serde_json::json!(i),
                    crate::semantic::MetricValue::String(s) => serde_json::json!(s),
                    crate::semantic::MetricValue::Bool(b) => serde_json::json!(b),
                };
                (k.clone(), jv)
            })
            .collect();

        GidEvent::MetricsUpdated {
            task_id: task_id.to_string(),
            progress: metrics.progress as f64,
            metrics: json_metrics,
        }
    }

    /// Create advisory events from a list
    pub fn from_advisories(task_id: &str, advisories: &[Advisory]) -> Vec<Self> {
        advisories
            .iter()
            .map(|a| GidEvent::AdvisoryTriggered {
                task_id: task_id.to_string(),
                severity: format!("{:?}", a.severity),
                message: a.message.clone(),
                suggestion: a.suggestion.clone(),
            })
            .collect()
    }

    /// Serialize to JSON line
    pub fn to_json_line(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// Broadcast-based event stream for multiple consumers
pub struct EventStream {
    tx: broadcast::Sender<GidEvent>,
}

impl EventStream {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Emit an event to all subscribers
    pub fn emit(&self, event: GidEvent) {
        let _ = self.tx.send(event);
    }

    /// Subscribe to the event stream
    pub fn subscribe(&self) -> broadcast::Receiver<GidEvent> {
        self.tx.subscribe()
    }
}

impl Default for EventStream {
    fn default() -> Self {
        Self::new(256)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let event = GidEvent::TaskStarted {
            task_id: "build".to_string(),
        };
        let json = event.to_json_line();
        assert!(json.contains("task_started"));
        assert!(json.contains("build"));
    }

    #[test]
    fn test_metrics_event() {
        let mut metrics = TaskMetrics {
            progress: 0.75,
            metrics: HashMap::new(),
            errors: Vec::new(),
            phase: None,
        };
        metrics.metrics.insert(
            "loss".to_string(),
            crate::semantic::MetricValue::Float(0.123),
        );

        let event = GidEvent::from_metrics("train", &metrics);
        let json = event.to_json_line();
        assert!(json.contains("metrics_updated"));
        assert!(json.contains("0.75"));
    }
}
