//! Smart Advisor - rule-based advisory system for task monitoring
//!
//! Detects anomalies and provides actionable suggestions:
//! - Loss NaN (training diverged)
//! - Loss plateau (not improving)
//! - High loss after significant progress
//! - Accuracy saturation
//! - Error spikes
//! - Build failures

use super::history::TaskMetricHistory;
use super::TaskMetrics;

/// Severity of an advisory
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Info => write!(f, "INFO"),
            Severity::Warning => write!(f, "WARN"),
            Severity::Critical => write!(f, "CRIT"),
        }
    }
}

/// An advisory suggestion
#[derive(Debug, Clone)]
pub struct Advisory {
    pub severity: Severity,
    pub message: String,
    pub suggestion: String,
    /// Optional semantic command label to auto-execute
    pub auto_action: Option<String>,
}

/// Smart advisor that analyzes metrics and emits suggestions
pub struct SmartAdvisor {
    rules: Vec<Box<dyn AdvisoryRule + Send + Sync>>,
}

/// Trait for advisory rules
pub trait AdvisoryRule: Send + Sync {
    fn evaluate(
        &self,
        metrics: &TaskMetrics,
        history: Option<&TaskMetricHistory>,
    ) -> Option<Advisory>;
}

impl SmartAdvisor {
    /// Create with all built-in rules
    pub fn new() -> Self {
        let rules: Vec<Box<dyn AdvisoryRule + Send + Sync>> = vec![
            Box::new(LossNaNRule),
            Box::new(LossPlateauRule),
            Box::new(HighLossRule),
            Box::new(AccuracySaturationRule),
            Box::new(ErrorSpikeRule),
            Box::new(ConvergingWellRule),
            Box::new(BuildFailureRule),
        ];
        Self { rules }
    }

    /// Evaluate all rules and return advisories
    pub fn evaluate(
        &self,
        metrics: &TaskMetrics,
        history: Option<&TaskMetricHistory>,
    ) -> Vec<Advisory> {
        self.rules
            .iter()
            .filter_map(|rule| rule.evaluate(metrics, history))
            .collect()
    }
}

impl Default for SmartAdvisor {
    fn default() -> Self {
        Self::new()
    }
}

// === Built-in Rules ===

struct LossNaNRule;
impl AdvisoryRule for LossNaNRule {
    fn evaluate(&self, metrics: &TaskMetrics, _history: Option<&TaskMetricHistory>) -> Option<Advisory> {
        for error in &metrics.errors {
            if error.contains("NaN") || error.contains("nan") {
                return Some(Advisory {
                    severity: Severity::Critical,
                    message: "Loss is NaN - training has diverged".to_string(),
                    suggestion: "Reduce learning rate, check data for NaN values, or restore last checkpoint".to_string(),
                    auto_action: Some("early_stop".to_string()),
                });
            }
        }
        None
    }
}

struct LossPlateauRule;
impl AdvisoryRule for LossPlateauRule {
    fn evaluate(&self, _metrics: &TaskMetrics, history: Option<&TaskMetricHistory>) -> Option<Advisory> {
        let history = history?;
        if history.snapshots.len() < 20 {
            return None;
        }

        if history.is_plateaued("loss", 20, 0.005) {
            return Some(Advisory {
                severity: Severity::Warning,
                message: "Loss has plateaued - no significant improvement in recent epochs".to_string(),
                suggestion: "Try: reduce learning rate, increase model capacity, or add data augmentation".to_string(),
                auto_action: Some("adjust_lr".to_string()),
            });
        }
        None
    }
}

struct HighLossRule;
impl AdvisoryRule for HighLossRule {
    fn evaluate(&self, metrics: &TaskMetrics, _history: Option<&TaskMetricHistory>) -> Option<Advisory> {
        if metrics.progress < 0.3 {
            return None; // Too early to judge
        }

        if let Some(crate::semantic::MetricValue::Float(loss)) = metrics.metrics.get("loss") {
            if *loss > 1.0 {
                return Some(Advisory {
                    severity: Severity::Warning,
                    message: format!("Loss is still high ({:.3}) at {:.0}% progress", loss, metrics.progress * 100.0),
                    suggestion: "Check: learning rate may be too high, data may have issues, or model may be too small".to_string(),
                    auto_action: None,
                });
            }
        }
        None
    }
}

struct AccuracySaturationRule;
impl AdvisoryRule for AccuracySaturationRule {
    fn evaluate(&self, _metrics: &TaskMetrics, history: Option<&TaskMetricHistory>) -> Option<Advisory> {
        let history = history?;
        if history.snapshots.len() < 20 {
            return None;
        }

        if let Some(acc) = history.latest_metric("accuracy") {
            if acc > 0.99 && history.is_plateaued("accuracy", 10, 0.001) {
                return Some(Advisory {
                    severity: Severity::Info,
                    message: format!("Accuracy saturated at {:.1}% - model may be overfitting", acc * 100.0),
                    suggestion: "Consider early stopping, check validation accuracy, or add regularization".to_string(),
                    auto_action: Some("save_checkpoint".to_string()),
                });
            }
        }
        None
    }
}

struct ErrorSpikeRule;
impl AdvisoryRule for ErrorSpikeRule {
    fn evaluate(&self, metrics: &TaskMetrics, _history: Option<&TaskMetricHistory>) -> Option<Advisory> {
        if metrics.errors.len() > 5 {
            return Some(Advisory {
                severity: Severity::Warning,
                message: format!("{} errors detected in recent output", metrics.errors.len()),
                suggestion: "Review error log for recurring issues".to_string(),
                auto_action: None,
            });
        }
        None
    }
}

struct ConvergingWellRule;
impl AdvisoryRule for ConvergingWellRule {
    fn evaluate(&self, metrics: &TaskMetrics, history: Option<&TaskMetricHistory>) -> Option<Advisory> {
        let history = history?;
        if history.snapshots.len() < 10 {
            return None;
        }

        if let Some(trend) = history.trend("loss", 10) {
            if trend < -0.01 && metrics.progress > 0.5 {
                if let Some(crate::semantic::MetricValue::Float(loss)) = metrics.metrics.get("loss") {
                    if *loss < 0.5 {
                        return Some(Advisory {
                            severity: Severity::Info,
                            message: format!("Training converging well (loss: {:.3}, trend: {:.4})", loss, trend),
                            suggestion: "Looking good! Consider saving a checkpoint.".to_string(),
                            auto_action: Some("save_checkpoint".to_string()),
                        });
                    }
                }
            }
        }
        None
    }
}

struct BuildFailureRule;
impl AdvisoryRule for BuildFailureRule {
    fn evaluate(&self, metrics: &TaskMetrics, _history: Option<&TaskMetricHistory>) -> Option<Advisory> {
        if let Some(crate::semantic::MetricValue::Int(errors)) = metrics.metrics.get("errors") {
            if *errors > 0 {
                return Some(Advisory {
                    severity: Severity::Critical,
                    message: format!("{} compilation error(s) detected", errors),
                    suggestion: "Fix compilation errors before continuing".to_string(),
                    auto_action: None,
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::{MetricValue, TaskMetrics};
    use std::collections::HashMap;

    fn make_metrics(progress: f32, loss: f64, errors: Vec<String>) -> TaskMetrics {
        let mut metrics = HashMap::new();
        metrics.insert("loss".to_string(), MetricValue::Float(loss));
        TaskMetrics {
            progress,
            metrics,
            phase: None,
            errors,
        }
    }

    #[test]
    fn test_loss_nan_detection() {
        let advisor = SmartAdvisor::new();
        let metrics = make_metrics(0.5, 0.0, vec!["Loss is NaN - training diverged".to_string()]);

        let advisories = advisor.evaluate(&metrics, None);
        assert!(!advisories.is_empty());
        assert_eq!(advisories[0].severity, Severity::Critical);
    }

    #[test]
    fn test_high_loss_detection() {
        let advisor = SmartAdvisor::new();
        let metrics = make_metrics(0.5, 2.5, vec![]);

        let advisories = advisor.evaluate(&metrics, None);
        assert!(advisories.iter().any(|a| a.severity == Severity::Warning));
    }

    #[test]
    fn test_error_spike_detection() {
        let advisor = SmartAdvisor::new();
        let errors: Vec<String> = (0..6).map(|i| format!("error {}", i)).collect();
        let metrics = make_metrics(0.5, 0.3, errors);

        let advisories = advisor.evaluate(&metrics, None);
        assert!(advisories.iter().any(|a| a.message.contains("6 errors")));
    }

    #[test]
    fn test_build_failure_detection() {
        let advisor = SmartAdvisor::new();
        let mut metrics_map = HashMap::new();
        metrics_map.insert("errors".to_string(), MetricValue::Int(3));
        let metrics = TaskMetrics {
            progress: 1.0,
            metrics: metrics_map,
            phase: Some("Finished".to_string()),
            errors: vec![],
        };

        let advisories = advisor.evaluate(&metrics, None);
        assert!(advisories.iter().any(|a| a.severity == Severity::Critical));
    }

    #[test]
    fn test_no_false_positives_early() {
        let advisor = SmartAdvisor::new();
        let metrics = make_metrics(0.1, 2.0, vec![]);

        // At 10% progress, high loss should NOT trigger (too early)
        let advisories = advisor.evaluate(&metrics, None);
        assert!(advisories.iter().all(|a| !a.message.contains("still high")));
    }
}
