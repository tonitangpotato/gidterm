//! Metric History - track metrics over time for trend analysis, ETA, and charts

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// A single metric snapshot at a point in time
#[derive(Debug, Clone)]
pub struct MetricSnapshot {
    pub timestamp: Instant,
    pub progress: f32,
    pub metrics: HashMap<String, f64>,
}

/// History of metrics for a single task
#[derive(Debug, Clone)]
pub struct TaskMetricHistory {
    pub snapshots: Vec<MetricSnapshot>,
    pub max_snapshots: usize,
    pub started_at: Instant,
}

impl TaskMetricHistory {
    pub fn new() -> Self {
        Self {
            snapshots: Vec::new(),
            max_snapshots: 500,
            started_at: Instant::now(),
        }
    }

    /// Record a new metric snapshot
    pub fn record(&mut self, progress: f32, metrics: HashMap<String, f64>) {
        // Deduplicate: skip if progress hasn't changed and it's been < 1s
        if let Some(last) = self.snapshots.last() {
            if (last.progress - progress).abs() < 0.001
                && last.timestamp.elapsed() < Duration::from_secs(1)
            {
                return;
            }
        }

        self.snapshots.push(MetricSnapshot {
            timestamp: Instant::now(),
            progress,
            metrics,
        });

        // Cap history
        if self.snapshots.len() > self.max_snapshots {
            let drain = self.snapshots.len() - self.max_snapshots;
            self.snapshots.drain(0..drain);
        }
    }

    /// Get elapsed time since tracking started
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Estimate time remaining based on progress rate
    pub fn estimate_remaining(&self) -> Option<Duration> {
        if self.snapshots.len() < 2 {
            return None;
        }

        let latest = self.snapshots.last()?;
        let progress = latest.progress;

        if progress <= 0.0 || progress >= 1.0 {
            return None;
        }

        // Use recent progress rate (last 10 snapshots or all if fewer)
        let window = self.snapshots.len().min(10);
        let start_idx = self.snapshots.len() - window;
        let start = &self.snapshots[start_idx];
        let end = latest;

        let progress_delta = end.progress - start.progress;
        let time_delta = end.timestamp.duration_since(start.timestamp);

        if progress_delta <= 0.0 || time_delta.as_secs_f64() <= 0.0 {
            return None;
        }

        let rate = progress_delta as f64 / time_delta.as_secs_f64(); // progress per second
        let remaining_progress = (1.0 - progress) as f64;
        let remaining_secs = remaining_progress / rate;

        if remaining_secs > 0.0 && remaining_secs < 86400.0 * 7.0 {
            // Cap at 7 days
            Some(Duration::from_secs_f64(remaining_secs))
        } else {
            None
        }
    }

    /// Get progress rate (progress/second) over recent window
    pub fn progress_rate(&self) -> Option<f64> {
        if self.snapshots.len() < 2 {
            return None;
        }

        let window = self.snapshots.len().min(10);
        let start_idx = self.snapshots.len() - window;
        let start = &self.snapshots[start_idx];
        let end = self.snapshots.last()?;

        let progress_delta = end.progress - start.progress;
        let time_delta = end.timestamp.duration_since(start.timestamp);

        if time_delta.as_secs_f64() > 0.0 {
            Some(progress_delta as f64 / time_delta.as_secs_f64())
        } else {
            None
        }
    }

    /// Get the last N values of a named metric (for sparklines)
    pub fn metric_values(&self, name: &str, last_n: usize) -> Vec<f64> {
        self.snapshots
            .iter()
            .rev()
            .take(last_n)
            .filter_map(|s| s.metrics.get(name).copied())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    /// Get the last N progress values (for sparklines)
    pub fn progress_values(&self, last_n: usize) -> Vec<f64> {
        self.snapshots
            .iter()
            .rev()
            .take(last_n)
            .map(|s| s.progress as f64)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    /// Detect if a metric has plateaued (no significant change in last N snapshots)
    pub fn is_plateaued(&self, name: &str, window: usize, threshold: f64) -> bool {
        let values = self.metric_values(name, window);
        if values.len() < window {
            return false;
        }

        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        (max - min).abs() < threshold
    }

    /// Get the trend direction of a metric: positive = increasing, negative = decreasing
    pub fn trend(&self, name: &str, window: usize) -> Option<f64> {
        let values = self.metric_values(name, window);
        if values.len() < 2 {
            return None;
        }

        // Simple linear regression slope
        let n = values.len() as f64;
        let x_mean = (n - 1.0) / 2.0;
        let y_mean: f64 = values.iter().sum::<f64>() / n;

        let mut numerator = 0.0;
        let mut denominator = 0.0;

        for (i, &y) in values.iter().enumerate() {
            let x = i as f64;
            numerator += (x - x_mean) * (y - y_mean);
            denominator += (x - x_mean) * (x - x_mean);
        }

        if denominator > 0.0 {
            Some(numerator / denominator)
        } else {
            None
        }
    }

    /// Get current progress
    pub fn current_progress(&self) -> f32 {
        self.snapshots.last().map(|s| s.progress).unwrap_or(0.0)
    }

    /// Get latest value of a metric
    pub fn latest_metric(&self, name: &str) -> Option<f64> {
        self.snapshots
            .iter()
            .rev()
            .find_map(|s| s.metrics.get(name).copied())
    }
}

impl Default for TaskMetricHistory {
    fn default() -> Self {
        Self::new()
    }
}

/// Format a Duration as human-readable ETA string
pub fn format_eta(duration: Duration) -> String {
    let total_secs = duration.as_secs();

    if total_secs < 60 {
        format!("{}s", total_secs)
    } else if total_secs < 3600 {
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        format!("{}m{}s", mins, secs)
    } else {
        let hours = total_secs / 3600;
        let mins = (total_secs % 3600) / 60;
        format!("{}h{}m", hours, mins)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_record_and_retrieve() {
        let mut history = TaskMetricHistory::new();

        let mut metrics = HashMap::new();
        metrics.insert("loss".to_string(), 0.5);
        history.record(0.1, metrics.clone());

        metrics.insert("loss".to_string(), 0.3);
        history.record(0.2, metrics);

        assert_eq!(history.snapshots.len(), 2);
        assert_eq!(history.current_progress(), 0.2);
    }

    #[test]
    fn test_metric_values() {
        let mut history = TaskMetricHistory::new();

        for i in 0..5 {
            let mut m = HashMap::new();
            m.insert("loss".to_string(), 1.0 - (i as f64 * 0.2));
            history.record(i as f32 * 0.2, m);
            // Force unique timestamps
            thread::sleep(Duration::from_millis(2));
        }

        let losses = history.metric_values("loss", 5);
        assert_eq!(losses.len(), 5);
        assert!((losses[0] - 1.0).abs() < 0.01);
        assert!((losses[4] - 0.2).abs() < 0.01);
    }

    #[test]
    fn test_trend_decreasing() {
        let mut history = TaskMetricHistory::new();

        for i in 0..10 {
            let mut m = HashMap::new();
            m.insert("loss".to_string(), 1.0 - (i as f64 * 0.1));
            history.record(i as f32 * 0.1, m);
            thread::sleep(Duration::from_millis(2));
        }

        let trend = history.trend("loss", 10).unwrap();
        assert!(trend < 0.0, "Loss trend should be negative (decreasing)");
    }

    #[test]
    fn test_plateau_detection() {
        let mut history = TaskMetricHistory::new();

        for i in 0..10 {
            let mut m = HashMap::new();
            m.insert("loss".to_string(), 0.5001 + (i as f64 * 0.0001));
            history.record(i as f32 * 0.1, m);
            thread::sleep(Duration::from_millis(2));
        }

        assert!(history.is_plateaued("loss", 10, 0.01));
    }

    #[test]
    fn test_format_eta() {
        assert_eq!(format_eta(Duration::from_secs(45)), "45s");
        assert_eq!(format_eta(Duration::from_secs(125)), "2m5s");
        assert_eq!(format_eta(Duration::from_secs(3725)), "1h2m");
    }

    #[test]
    fn test_eta_estimation() {
        let mut history = TaskMetricHistory::new();

        // Simulate progress over time
        history.record(0.0, HashMap::new());
        thread::sleep(Duration::from_millis(50));
        history.record(0.5, HashMap::new());

        // 50% in ~50ms => remaining ~50ms
        let eta = history.estimate_remaining();
        assert!(eta.is_some());
        let eta = eta.unwrap();
        // Should be roughly 50ms (allow wide tolerance for CI)
        assert!(eta.as_millis() < 500, "ETA should be reasonable: {:?}", eta);
    }
}
