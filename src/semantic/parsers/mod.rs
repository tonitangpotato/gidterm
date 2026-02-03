//! Output parsers for different task types

pub mod build;
pub mod regex;
pub mod ml_training;

pub use build::BuildParser;
pub use regex::RegexParser;
pub use ml_training::MLTrainingParser;
