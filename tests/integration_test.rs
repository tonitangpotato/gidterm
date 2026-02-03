use gidterm::{Graph, GraphTaskStatus, Scheduler, Session};
use std::collections::HashMap;
use std::path::Path;

// === Graph Tests ===

#[test]
fn test_graph_from_file() {
    let graph = Graph::from_file(Path::new("test-gid-integration.yml"));
    assert!(graph.is_ok());

    let graph = graph.unwrap();
    assert_eq!(graph.metadata.as_ref().unwrap().project, "test-integration");
    assert_eq!(graph.tasks.len(), 5);
}

#[test]
fn test_graph_auto_load() {
    // .gid/graph.yml exists in repo root
    let graph = Graph::auto_load();
    assert!(graph.is_ok());
}

#[test]
fn test_graph_task_status_deserialization() {
    let graph = Graph::from_file(Path::new("test-gid-integration.yml")).unwrap();

    for task in graph.tasks.values() {
        assert_eq!(task.status, GraphTaskStatus::Pending);
    }
}

#[test]
fn test_graph_dependencies() {
    let graph = Graph::from_file(Path::new("test-gid-integration.yml")).unwrap();

    // "hello" has no deps — should be ready
    assert!(graph.can_start("hello"));

    // "world" depends on "hello" (pending) — should not be ready
    assert!(!graph.can_start("world"));

    // "final" depends on parallel1 + parallel2 — should not be ready
    assert!(!graph.can_start("final"));
}

#[test]
fn test_graph_get_ready_tasks() {
    let graph = Graph::from_file(Path::new("test-gid-integration.yml")).unwrap();

    let ready = graph.get_ready_tasks();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0], "hello");
}

#[test]
fn test_graph_update_status() {
    let mut graph = Graph::from_file(Path::new("test-gid-integration.yml")).unwrap();

    graph.update_task_status("hello", GraphTaskStatus::Done).unwrap();
    assert_eq!(graph.get_task("hello").unwrap().status, GraphTaskStatus::Done);

    // Now "world" should be ready
    assert!(graph.can_start("world"));
    let ready = graph.get_ready_tasks();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0], "world");
}

#[test]
fn test_graph_update_nonexistent_task() {
    let mut graph = Graph::from_file(Path::new("test-gid-integration.yml")).unwrap();
    let result = graph.update_task_status("nonexistent", GraphTaskStatus::Done);
    assert!(result.is_err());
}

// === Scheduler Tests ===

#[test]
fn test_scheduler_schedule_next() {
    let graph = Graph::from_file(Path::new("test-gid-integration.yml")).unwrap();
    let mut scheduler = Scheduler::new(graph);

    let ready = scheduler.schedule_next();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0], "hello");
}

#[test]
fn test_scheduler_marks_started() {
    let graph = Graph::from_file(Path::new("test-gid-integration.yml")).unwrap();
    let mut scheduler = Scheduler::new(graph);

    scheduler.mark_started("hello").unwrap();
    assert_eq!(scheduler.get_running(), vec!["hello"]);

    // Should not schedule "hello" again
    let ready = scheduler.schedule_next();
    assert!(ready.is_empty());
}

#[test]
fn test_scheduler_full_progression() {
    let graph = Graph::from_file(Path::new("test-gid-integration.yml")).unwrap();
    let mut scheduler = Scheduler::new(graph);

    // Start hello
    let ready = scheduler.schedule_next();
    assert_eq!(ready, vec!["hello"]);
    scheduler.mark_started("hello").unwrap();
    scheduler.mark_done("hello").unwrap();

    // Now world is ready
    let ready = scheduler.schedule_next();
    assert_eq!(ready, vec!["world"]);
    scheduler.mark_started("world").unwrap();
    scheduler.mark_done("world").unwrap();

    // Now parallel1 and parallel2 are ready
    let mut ready = scheduler.schedule_next();
    ready.sort();
    assert_eq!(ready, vec!["parallel1", "parallel2"]);

    scheduler.mark_started("parallel1").unwrap();
    scheduler.mark_started("parallel2").unwrap();
    scheduler.mark_done("parallel1").unwrap();
    scheduler.mark_done("parallel2").unwrap();

    // Now final is ready
    let ready = scheduler.schedule_next();
    assert_eq!(ready, vec!["final"]);
    scheduler.mark_started("final").unwrap();
    scheduler.mark_done("final").unwrap();

    assert!(scheduler.all_done());
}

#[test]
fn test_scheduler_failed_task_blocks_dependents() {
    let graph = Graph::from_file(Path::new("test-gid-integration.yml")).unwrap();
    let mut scheduler = Scheduler::new(graph);

    scheduler.mark_started("hello").unwrap();
    scheduler.mark_failed("hello").unwrap();

    // "world" depends on "hello" which failed — should not be ready
    let ready = scheduler.schedule_next();
    assert!(ready.is_empty());

    // But all_done should be false since pending tasks remain
    assert!(!scheduler.all_done());
}

// === Session Tests ===

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

    session.start_task("task1".to_string());
    assert!(session.tasks.contains_key("task1"));
    assert_eq!(session.tasks["task1"].runs.len(), 1);

    session.add_output("task1", "test output".to_string());
    assert_eq!(session.tasks["task1"].runs[0].output.len(), 1);

    session.end_task("task1", gidterm::TaskStatus::Done, Some(0));
    assert_eq!(
        session.tasks["task1"].runs[0].status,
        gidterm::TaskStatus::Done
    );
    assert_eq!(session.tasks["task1"].runs[0].exit_code, Some(0));
}

#[test]
fn test_session_multiple_runs() {
    let mut session = Session::new("test".to_string());

    // First run
    session.start_task("task1".to_string());
    session.end_task("task1", gidterm::TaskStatus::Failed, Some(1));

    // Second run (retry)
    session.start_task("task1".to_string());
    session.end_task("task1", gidterm::TaskStatus::Done, Some(0));

    assert_eq!(session.tasks["task1"].runs.len(), 2);
    assert_eq!(session.tasks["task1"].runs[0].status, gidterm::TaskStatus::Failed);
    assert_eq!(session.tasks["task1"].runs[1].status, gidterm::TaskStatus::Done);
}

#[test]
fn test_session_end() {
    let mut session = Session::new("test".to_string());
    assert!(session.ended_at.is_none());
    session.end();
    assert!(session.ended_at.is_some());
}

// === Semantic Commands Tests ===

#[test]
fn test_semantic_commands_from_graph() {
    use gidterm::core::Task;

    let mut tasks = HashMap::new();
    let mut sem_cmds = HashMap::new();
    sem_cmds.insert("save".to_string(), "model.save('ckpt.pth')".to_string());
    sem_cmds.insert(
        "adjust_lr".to_string(),
        "optimizer.lr = {value}".to_string(),
    );

    tasks.insert(
        "train".to_string(),
        Task {
            task_type: "ml_training".to_string(),
            description: "Train model".to_string(),
            command: Some("python train.py".to_string()),
            status: GraphTaskStatus::Pending,
            priority: None,
            depends_on: None,
            component: None,
            estimated_hours: None,
            tags: None,
            semantic_commands: Some(sem_cmds),
        },
    );

    let task = &tasks["train"];
    let cmds =
        gidterm::semantic::commands::TaskCommands::from_map(task.semantic_commands.as_ref().unwrap());

    assert_eq!(cmds.commands.len(), 2);
    assert!(cmds.get("save").is_some());
    assert!(!cmds.get("save").unwrap().needs_params());
    assert!(cmds.get("adjust_lr").unwrap().needs_params());

    let mut params = HashMap::new();
    params.insert("value".to_string(), "0.001".to_string());
    assert_eq!(
        cmds.get("adjust_lr").unwrap().render(&params),
        "optimizer.lr = 0.001"
    );
}
