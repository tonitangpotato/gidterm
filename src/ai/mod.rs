//! AI Integration Layer
//!
//! Provides a unified ControlAPI trait for three usage modes:
//! 1. Manual TUI - human operates gidterm directly
//! 2. Claude Code via MCP - AI assistant controls gidterm through tool calls
//! 3. Clawdbot automation - autonomous agent drives gidterm programmatically
//!
//! All modes share the same event stream and control interface.

pub mod control;
pub mod events;

pub use control::{ControlAPI, ControlMode};
pub use events::{GidEvent, EventStream};
