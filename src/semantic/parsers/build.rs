//! Build output parser - cargo, npm, make, etc.

use crate::semantic::{MetricValue, OutputParser, ParsedMetrics, TaskMetrics};
use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;

/// Parser for build tool output
pub struct BuildParser {
    // Cargo patterns
    compiling_re: Regex,
    warning_re: Regex,
    error_re: Regex,
    finished_re: Regex,
    test_result_re: Regex,
    // npm patterns
    npm_warn_re: Regex,
    npm_err_re: Regex,
    // Generic step patterns
    step_re: Regex,
}

impl BuildParser {
    pub fn new() -> Self {
        Self {
            compiling_re: Regex::new(r"Compiling\s+(\S+)\s+v").unwrap(),
            warning_re: Regex::new(r"warning(?:\[[\w]+\])?:").unwrap(),
            error_re: Regex::new(r"(?i)^error(?:\[[\w]+\])?:").unwrap(),
            finished_re: Regex::new(r"Finished\s+`?(\w+)`?\s+.*in\s+([\d.]+)s").unwrap(),
            test_result_re: Regex::new(r"test result:.*?(\d+) passed.*?(\d+) failed").unwrap(),
            npm_warn_re: Regex::new(r"npm warn").unwrap(),
            npm_err_re: Regex::new(r"npm ERR!").unwrap(),
            step_re: Regex::new(r"\[(\d+)/(\d+)\]").unwrap(),
        }
    }

    fn count_pattern(&self, output: &str, re: &Regex) -> i64 {
        output.lines().filter(|l| re.is_match(l)).count() as i64
    }
}

impl Default for BuildParser {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputParser for BuildParser {
    fn name(&self) -> &str {
        "build"
    }

    fn parse(&self, output: &str) -> Result<ParsedMetrics> {
        let mut metrics = HashMap::new();
        let mut errors = Vec::new();

        // Count warnings and errors
        let warning_count = self.count_pattern(output, &self.warning_re)
            + self.count_pattern(output, &self.npm_warn_re);
        let error_count = self.count_pattern(output, &self.error_re)
            + self.count_pattern(output, &self.npm_err_re);
        let crate_count = self.count_pattern(output, &self.compiling_re);

        if warning_count > 0 {
            metrics.insert("warnings".to_string(), MetricValue::Int(warning_count));
        }
        if error_count > 0 {
            metrics.insert("errors".to_string(), MetricValue::Int(error_count));
        }
        if crate_count > 0 {
            metrics.insert("crates_compiled".to_string(), MetricValue::Int(crate_count));
        }

        // Extract build time from "Finished" line
        for line in output.lines().rev() {
            if let Some(caps) = self.finished_re.captures(line) {
                if let Some(time_str) = caps.get(2) {
                    if let Ok(secs) = time_str.as_str().parse::<f64>() {
                        metrics.insert("build_time_secs".to_string(), MetricValue::Float(secs));
                    }
                }
                if let Some(profile) = caps.get(1) {
                    metrics.insert(
                        "profile".to_string(),
                        MetricValue::String(profile.as_str().to_string()),
                    );
                }
                break;
            }
        }

        // Extract test results
        for line in output.lines().rev() {
            if let Some(caps) = self.test_result_re.captures(line) {
                let passed: i64 = caps.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
                let failed: i64 = caps.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
                metrics.insert("tests_passed".to_string(), MetricValue::Int(passed));
                metrics.insert("tests_failed".to_string(), MetricValue::Int(failed));
                break;
            }
        }

        // Extract step-based progress [3/10]
        let mut progress = 0.0;
        for line in output.lines().rev() {
            if let Some(caps) = self.step_re.captures(line) {
                let current: f32 = caps.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0.0);
                let total: f32 = caps.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(1.0);
                if total > 0.0 {
                    progress = current / total;
                }
                break;
            }
        }

        // If finished line found, progress is 100%
        if metrics.contains_key("build_time_secs") {
            progress = 1.0;
        }

        // Collect error lines
        for line in output.lines() {
            if self.error_re.is_match(line) || self.npm_err_re.is_match(line) {
                errors.push(line.to_string());
            }
        }

        // Detect phase
        let phase = if output.contains("Compiling") && !metrics.contains_key("build_time_secs") {
            Some("Compiling".to_string())
        } else if output.contains("Linking") {
            Some("Linking".to_string())
        } else if output.contains("test result:") {
            Some("Testing".to_string())
        } else if metrics.contains_key("build_time_secs") {
            Some("Finished".to_string())
        } else {
            None
        };

        Ok(TaskMetrics {
            progress,
            metrics,
            phase,
            errors,
        })
    }

    fn can_parse(&self, output: &str) -> bool {
        self.compiling_re.is_match(output)
            || self.finished_re.is_match(output)
            || self.npm_err_re.is_match(output)
            || self.step_re.is_match(output)
    }

    fn supported_types(&self) -> Vec<&str> {
        vec!["build", "compile", "Build", "BugFix", "Refactor"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cargo_build_output() {
        let parser = BuildParser::new();

        let output = r#"   Compiling serde v1.0.204
   Compiling tokio v1.40.0
   Compiling gidterm v0.2.0
warning[unused_import]: unused import
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.32s"#;

        let metrics = parser.parse(output).unwrap();

        assert_eq!(metrics.progress, 1.0);
        assert_eq!(metrics.metrics["crates_compiled"].as_int(), Some(3));
        assert_eq!(metrics.metrics["warnings"].as_int(), Some(1));
        assert_eq!(metrics.metrics["build_time_secs"].as_float(), Some(4.32));
        assert_eq!(metrics.phase, Some("Finished".to_string()));
    }

    #[test]
    fn test_cargo_test_output() {
        let parser = BuildParser::new();

        let output = "test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out";

        let metrics = parser.parse(output).unwrap();
        assert_eq!(metrics.metrics["tests_passed"].as_int(), Some(16));
        assert_eq!(metrics.metrics["tests_failed"].as_int(), Some(0));
    }

    #[test]
    fn test_step_progress() {
        let parser = BuildParser::new();

        let output = "[3/10] Building module foo\n[4/10] Building module bar";

        let metrics = parser.parse(output).unwrap();
        assert_eq!(metrics.progress, 0.4);
    }

    #[test]
    fn test_error_extraction() {
        let parser = BuildParser::new();

        let output = "error[E0308]: mismatched types\n  --> src/main.rs:10:5";

        let metrics = parser.parse(output).unwrap();
        assert!(!metrics.errors.is_empty());
        assert_eq!(metrics.metrics["errors"].as_int(), Some(1));
    }
}
