//! Semantic layer - Output parsing and semantic commands

pub mod advisor;
pub mod commands;
pub mod history;
pub mod parsers;
pub mod registry;

pub use registry::{OutputParser, ParsedMetrics, ParserRegistry};

use std::collections::HashMap;

/// Task metrics extracted from output
#[derive(Debug, Clone)]
pub struct TaskMetrics {
    /// Overall progress (0.0 - 1.0)
    pub progress: f32,
    
    /// Custom metrics (e.g., "loss": 0.234, "accuracy": 0.876)
    pub metrics: HashMap<String, MetricValue>,
    
    /// Current phase/stage
    pub phase: Option<String>,
    
    /// Error messages if any
    pub errors: Vec<String>,
}

/// Metric value type
#[derive(Debug, Clone)]
pub enum MetricValue {
    Float(f64),
    Int(i64),
    String(String),
    Bool(bool),
}

impl MetricValue {
    pub fn as_float(&self) -> Option<f64> {
        match self {
            MetricValue::Float(v) => Some(*v),
            MetricValue::Int(v) => Some(*v as f64),
            _ => None,
        }
    }
    
    pub fn as_int(&self) -> Option<i64> {
        match self {
            MetricValue::Int(v) => Some(*v),
            MetricValue::Float(v) => Some(*v as i64),
            _ => None,
        }
    }
    
    pub fn as_string(&self) -> Option<&str> {
        match self {
            MetricValue::String(v) => Some(v),
            _ => None,
        }
    }
}
