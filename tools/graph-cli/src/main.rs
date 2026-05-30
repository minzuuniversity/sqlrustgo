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

    // check_task_completion returns:
    // - (false, []) if task not found in graph
    // - (false, [task_id]) if task found but no complete chain
    // - (true, [task_id, commit, ci, artifact]) if complete chain exists
    let task_exists = store.get_nodes_by_type(NodeType::Task)?
        .iter().any(|n| n.id == task_id);

    if !task_exists {
        // Rule G-03: No evidence = UNVERIFIED, not PASS
        // Task not found in graph means no authoritative evidence exists
        let output = GateResult {
            task_id: task_id.to_string(),
            result: "UNVERIFIED".to_string(),
            reason: "task_not_in_graph".to_string(),
            missing: vec!["Task node not found — no authoritative evidence".to_string()],
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    if !reachable {
        // Rule G-01: Task exists but no complete evidence chain
        // Rule G-03: Incomplete evidence chain = UNVERIFIED, block release
        let missing = if path.len() == 1 {
            vec!["Task→Commit edge missing (IMPLEMENTED_BY)".to_string()]
        } else if path.len() == 2 {
            vec!["Commit→CI edge missing (VERIFIED_BY)".to_string()]
        } else if path.len() == 3 {
            vec!["CI→Artifact edge missing (PRODUCES)".to_string()]
        } else {
            vec!["Evidence chain incomplete".to_string()]
        };

        let output = GateResult {
            task_id: task_id.to_string(),
            result: "UNVERIFIED".to_string(),
            reason: "incomplete_evidence_chain".to_string(),
            missing,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Rule G-01: PASS only if complete evidence chain exists
    // Path: Task → Commit → CI_PASS → Artifact
    let output = GateResult {
        task_id: task_id.to_string(),
        result: "PASS".to_string(),
        reason: "reachability".to_string(),
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