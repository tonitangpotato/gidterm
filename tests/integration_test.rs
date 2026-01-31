use gidterm::{Graph, Session};
use std::path::Path;

#[test]
fn test_graph_auto_load() {
    // Test that from_file works
    let graph = Graph::from_file(Path::new("test-gid-integration.yml"));
    assert!(graph.is_ok());
    
    let graph = graph.unwrap();
    assert_eq!(graph.metadata.as_ref().unwrap().project, "test-integration");
    assert_eq!(graph.tasks.len(), 5);
}

#[test]
fn test_session_creation() {
    let session = Session::new("test-project".to_string());
    assert_eq!(session.project, "test-project");
    assert!(session.tasks.is_empty());
    assert!(session.ended_at.is_none());
}

#[test]
fn test_session_task_tracking() {
    let mut session = Session::new("test".to_string());
    
    // Start a task
    session.start_task("task1".to_string());
    assert!(session.tasks.contains_key("task1"));
    assert_eq!(session.tasks["task1"].runs.len(), 1);
    
    // Add output
    session.add_output("task1", "test output".to_string());
    assert_eq!(session.tasks["task1"].runs[0].output.len(), 1);
    
    // End the task
    session.end_task("task1", gidterm::TaskStatus::Done, Some(0));
    assert_eq!(session.tasks["task1"].runs[0].status, gidterm::TaskStatus::Done);
}
