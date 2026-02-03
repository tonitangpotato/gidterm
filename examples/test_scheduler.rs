#!/usr/bin/env rust-script
//! Test the scheduler logic without TUI
//! 
//! Run with: cargo run --example test_scheduler

use anyhow::Result;
use gidterm::core::{Graph, Scheduler};

fn main() -> Result<()> {
    println!("🧪 Testing GidTerm Scheduler\n");

    // Load graph
    let graph_path = std::path::PathBuf::from("test-graph.yml");
    let graph = Graph::from_file(&graph_path)?;

    println!("✅ Loaded graph:");
    println!("   - Project: {}", graph.metadata.as_ref().unwrap().project);
    println!("   - Nodes: {}", graph.nodes.len());
    println!("   - Tasks: {}", graph.tasks.len());
    println!();

    // Create scheduler
    let mut scheduler = Scheduler::new(graph);

    println!("📊 Task Dependency Analysis:\n");

    // Show all tasks with their dependencies
    for (id, task) in scheduler.graph().all_tasks() {
        let deps = task.depends_on.as_ref()
            .map(|d| d.join(", "))
            .unwrap_or_else(|| "none".to_string());
        
        let can_start = scheduler.graph().can_start(id);
        let status_icon = if can_start { "✓" } else { "⏳" };
        
        println!("{} {} [{}]", status_icon, id, task.status);
        println!("   Dependencies: {}", deps);
        println!("   Priority: {}", task.priority.as_ref().unwrap_or(&"normal".to_string()));
        if let Some(cmd) = &task.command {
            println!("   Command: {}", cmd);
        }
        println!();
    }

    // Simulate execution
    println!("🚀 Execution Simulation:\n");

    let mut round = 1;
    while !scheduler.all_done() {
        let ready = scheduler.schedule_next();
        
        if ready.is_empty() {
            println!("⏸  Round {}: No tasks ready (waiting for dependencies)", round);
            
            // Simulate completing some running tasks
            let running = scheduler.get_running();
            if !running.is_empty() {
                let task_to_complete = running[0].clone();
                println!("   ⚙  Completing: {}", task_to_complete);
                scheduler.mark_done(&task_to_complete)?;
            } else {
                println!("   ❌ Deadlock or all tasks failed!");
                break;
            }
        } else {
            println!("▶  Round {}: Starting {} tasks", round, ready.len());
            for task_id in &ready {
                println!("   ⚙  Starting: {}", task_id);
                scheduler.mark_started(task_id)?;
            }
        }
        
        println!();
        round += 1;
        
        if round > 20 {
            println!("⚠️  Stopping after 20 rounds (safety limit)");
            break;
        }
    }

    if scheduler.all_done() {
        println!("✅ All tasks completed!\n");
    }

    // Summary
    let total = scheduler.graph().all_tasks().len();
    let done = scheduler.graph().all_tasks().values()
        .filter(|t| t.status == gidterm::GraphTaskStatus::Done)
        .count();
    let failed = scheduler.graph().all_tasks().values()
        .filter(|t| t.status == gidterm::GraphTaskStatus::Failed)
        .count();
    
    println!("📈 Summary:");
    println!("   Total: {}", total);
    println!("   Done: {} ({}%)", done, (done * 100) / total);
    println!("   Failed: {}", failed);
    println!("   Pending: {}", total - done - failed);

    Ok(())
}
