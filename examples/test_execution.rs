#!/usr/bin/env rust-script
//! Test actual task execution
//! 
//! Run with: cargo run --example test_execution

use anyhow::Result;
use gidterm::core::{Executor, Graph, Scheduler, TaskEvent};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("🚀 Testing GidTerm Task Execution\n");

    // Load graph from command line arg or default
    let graph_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "simple-test.yml".to_string());
    let graph_path = PathBuf::from(graph_path);
    
    println!("Loading graph from: {}\n", graph_path.display());
    let graph = Graph::from_file(&graph_path)?;

    println!("✅ Loaded {} tasks\n", graph.tasks.len());

    // Create executor and scheduler
    let (executor, mut event_rx) = Executor::new();
    let mut scheduler = Scheduler::new(graph);

    println!("▶  Starting task execution...\n");

    // Track completed tasks
    let completed = Arc::new(Mutex::new(Vec::new()));
    let completed_clone = completed.clone();

    // Spawn event handler
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            match event {
                TaskEvent::Started { task_id } => {
                    println!("  ⚙  {} started", task_id);
                }
                TaskEvent::Output { task_id, line } => {
                    if !line.is_empty() {
                        println!("  │  {}: {}", task_id, line);
                    }
                }
                TaskEvent::Completed { task_id, exit_code } => {
                    println!("  ✓  {} completed (exit code: {})", task_id, exit_code);
                    completed_clone.lock().unwrap().push(task_id);
                }
                TaskEvent::Failed { task_id, error } => {
                    println!("  ✗  {} failed: {}", task_id, error);
                    completed_clone.lock().unwrap().push(task_id);
                }
            }
        }
    });

    // Main loop
    let mut round = 1;
    loop {
        // Check for completed tasks and mark them
        {
            let mut done_tasks = completed.lock().unwrap();
            for task_id in done_tasks.drain(..) {
                if scheduler.graph().get_task(&task_id).is_some() {
                    let _ = scheduler.mark_done(&task_id);
                }
            }
        }

        // Schedule next batch
        let ready = scheduler.schedule_next();

        if !ready.is_empty() {
            println!("Round {}: Starting {} tasks", round, ready.len());
            
            for task_id in &ready {
                let task = scheduler.graph().get_task(task_id).unwrap();
                
                if let Some(command) = &task.command {
                    // Actually start the task
                    executor.start_task(task_id, command).await?;
                    scheduler.mark_started(task_id)?;
                } else {
                    // No command, just mark as done
                    scheduler.mark_done(task_id)?;
                }
            }
            
            println!();
        }

        // Check if all done
        if scheduler.all_done() {
            println!("\n✅ All tasks completed!");
            break;
        }

        // Wait a bit
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        round += 1;
        
        // Safety limit
        if round > 50 {
            println!("\n⚠️  Stopping after 50 rounds (some tasks may still be running)");
            break;
        }
    }

    // Give some time for final outputs
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    Ok(())
}
