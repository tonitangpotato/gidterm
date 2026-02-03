//! UI Views - Dashboard, Terminal, Graph, Project Overview

pub mod comparison;
pub mod graph;
pub mod project_overview;
pub mod terminal;

pub use comparison::render_comparison_view;
pub use graph::render_graph_view;
pub use project_overview::render_project_overview;
pub use terminal::render_terminal_view;
