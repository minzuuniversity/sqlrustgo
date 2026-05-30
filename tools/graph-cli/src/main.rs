//! gate: Evidence Graph Gate Evaluator CLI
//!
//! Usage:
//!   gate evaluate <task_id>     # Evaluate reachability for a task
//!   gate status                 # Show graph statistics
//!   gate --db <path> evaluate   # Use specific db path

use anyhow::Result;
use clap::{Parser, Subcommand};
use evidence_graph::{GraphStore, NodeType};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "gate")]
#[command(about = "Evidence Graph Gate Evaluator v4.1")]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Evaluate {
        task_id: String,
        #[arg(long)]
        db: Option<PathBuf>,
    },
    Status {
        #[arg(long)]
        db: Option<PathBuf>,
    },
}

#[derive(Serialize)]
struct GateResult {
    task_id: String,
    result: String,
    reason: String,
    missing: Vec<String>,
}

fn default_db() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".evidence-graph.db")
}

fn open_store(db: Option<PathBuf>) -> anyhow::Result<GraphStore> {
    let path = db.unwrap_or_else(default_db);
    GraphStore::open(&path).map_err(|e| anyhow::anyhow!("GraphStore error: {}", e))
}

fn main() {
    let args = Args::parse();

    let result = match args.command {
        Commands::Evaluate { task_id, db } => cmd_evaluate(&task_id, db),
        Commands::Status { db } => cmd_status(db),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn cmd_evaluate(task_id: &str, db: Option<PathBuf>) -> Result<()> {
    let store = open_store(db)?;

    let (reachable, path) = store.check_task_completion(task_id)?;

    if path.is_empty() {
        let tasks = store.get_nodes_by_type(NodeType::Task)?;
        let exists = tasks.iter().any(|n| n.id == task_id);

        let (result, reason, missing) = if !exists {
            (
                "FAIL",
                "task_not_found",
                vec!["TaskNode not found".to_string()],
            )
        } else {
            (
                "FAIL",
                "no_path",
                vec!["Task → Commit → CI → Artifact path incomplete".to_string()],
            )
        };

        let output = GateResult {
            task_id: task_id.to_string(),
            result: result.to_string(),
            reason: reason.to_string(),
            missing,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    let result = if reachable { "PASS" } else { "WARN" };
    let reason = if reachable { "reachability" } else { "partial_path" };

    let output = GateResult {
        task_id: task_id.to_string(),
        result: result.to_string(),
        reason: reason.to_string(),
        missing: vec![],
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn cmd_status(db: Option<PathBuf>) -> Result<()> {
    let store = open_store(db)?;
    let stats = store.stats()?;

    println!("Evidence Graph Statistics");
    println!("=========================");
    println!("Total nodes: {}", stats.node_count);
    println!("Total edges: {}", stats.edge_count);
    println!("Orphan nodes: {}", stats.orphan_count);
    println!("  Tasks: {}", stats.task_count);
    println!("  Commits: {}", stats.commit_count);
    println!("  CI runs: {}", stats.ci_count);
    println!("  Artifacts: {}", stats.artifact_count);

    if stats.orphan_count > 0 {
        println!(
            "\n⚠ WARNING: {} orphan nodes detected (INVALID)",
            stats.orphan_count
        );
    } else {
        println!("\n✅ No orphan nodes");
    }
    Ok(())
}