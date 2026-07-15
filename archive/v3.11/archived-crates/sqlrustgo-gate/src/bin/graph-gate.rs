//! graph-gate: Evidence Graph Gate Evaluator (CLI)
//!
//! Usage:
//!   graph-gate evaluate <task_id>          # Evaluate a single task
//!   graph-gate evaluate --all              # Evaluate all tasks
//!   graph-gate status                      # Show graph stats
//!
//! Output format (legacy-compatible):
//!   { "task_id": "...", "result": "PASS|FAIL|WARN", "reason": "...", "missing": [...], "evidence_chain": [...] }

use anyhow::Result;
use clap::{Parser, Subcommand};
use evidence_graph::{GraphStore, NodeType};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "graph-gate")]
#[command(about = "Evidence Graph Gate Evaluator v4.1")]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Evaluate gate status for a task
    Evaluate(EvaluateArgs),
    /// Show graph statistics
    Status(StatusArgs),
    /// Ingest a commit (shorthand)
    IngestCommit(IngestCommitArgs),
}

#[derive(Parser, Debug)]
struct EvaluateArgs {
    /// Task ID to evaluate (e.g. "v3.7.0", "task_1")
    #[arg(default_value = "")]
    task_id: String,
    /// Evaluate all tasks
    #[arg(long, short)]
    all: bool,
    /// Path to graph database (default: ~/.evidence-graph.db)
    #[arg(long)]
    db: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct StatusArgs {
    /// Path to graph database (default: ~/.evidence-graph.db)
    #[arg(long)]
    db: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct IngestCommitArgs {
    commit_hash: String,
    #[arg(long)]
    db: Option<PathBuf>,
}

#[derive(Serialize)]
struct GateResult {
    task_id: String,
    result: String,
    reason: String,
    missing: Vec<String>,
    evidence_chain: Vec<String>,
}

fn default_db_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".evidence-graph.db")
}

fn open_store(db: Option<PathBuf>) -> anyhow::Result<GraphStore> {
    let path = db.unwrap_or_else(default_db_path);
    GraphStore::open(&path).map_err(|e| anyhow::anyhow!("GraphStore error: {}", e))
}

fn evaluate_task(store: &GraphStore, task_id: &str) -> Result<GateResult> {
    let (reachable, path) = store.check_task_completion(task_id)?;

    let mut missing = Vec::new();
    let mut evidence_chain = Vec::new();

    if path.is_empty() {
        let tasks = store.get_nodes_by_type(NodeType::Task)?;
        let task_exists = tasks.iter().any(|n| n.id == task_id);

        if !task_exists {
            missing.push("TaskNode not found".to_string());
            return Ok(GateResult {
                task_id: task_id.to_string(),
                result: "FAIL".to_string(),
                reason: "task_not_found".to_string(),
                missing,
                evidence_chain,
            });
        }

        missing.push("Task → Commit → CI → Artifact path incomplete".to_string());
        return Ok(GateResult {
            task_id: task_id.to_string(),
            result: "FAIL".to_string(),
            reason: "no_path".to_string(),
            missing,
            evidence_chain,
        });
    }

    for (i, node_id) in path.iter().enumerate() {
        evidence_chain.push(node_id.clone());
        if i < path.len() - 1 {
            let edge_type = match i {
                0 => "IMPLEMENTED_BY",
                1 => "VERIFIED_BY",
                2 => "PRODUCES",
                _ => "links_to",
            };
            evidence_chain.push(format!("{} → {}", node_id, edge_type));
        }
    }

    let result = if reachable {
        "PASS"
    } else if path.len() >= 2 {
        "WARN"
    } else {
        "FAIL"
    };
    let reason = if reachable {
        "reachability"
    } else {
        "partial_path"
    };

    Ok(GateResult {
        task_id: task_id.to_string(),
        result: result.to_string(),
        reason: reason.to_string(),
        missing,
        evidence_chain,
    })
}

fn cmd_evaluate(args: EvaluateArgs) -> Result<()> {
    let store = open_store(args.db)?;

    if args.all {
        let tasks = store.get_nodes_by_type(NodeType::Task)?;
        let mut first = true;
        println!("[");

        for task in tasks.iter() {
            if !first {
                println!(",");
            }
            first = false;
            let result = evaluate_task(&store, &task.id)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }

        println!("]");
    } else if !args.task_id.is_empty() {
        let result = evaluate_task(&store, &args.task_id)?;
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        anyhow::bail!("Either <task_id> or --all must be provided");
    }

    Ok(())
}

fn cmd_status(args: StatusArgs) -> Result<()> {
    let store = open_store(args.db)?;
    let stats = store.stats()?;

    println!("Evidence Graph Statistics");
    println!("=========================");
    println!("Total nodes: {}", stats.node_count);
    println!("Total edges: {}", stats.edge_count);
    println!("Orphan nodes: {}", stats.orphan_count);
    println!();
    println!("By type:");
    println!("  Tasks: {}", stats.task_count);
    println!("  Commits: {}", stats.commit_count);
    println!("  CI runs: {}", stats.ci_count);
    println!("  Artifacts: {}", stats.artifact_count);
    println!();

    if stats.orphan_count > 0 {
        println!(
            "⚠ WARNING: {} orphan nodes detected (INVALID)",
            stats.orphan_count
        );
    } else {
        println!("✅ No orphan nodes");
    }

    Ok(())
}

fn cmd_ingest_commit(args: IngestCommitArgs) -> Result<()> {
    let store = open_store(args.db)?;
    use evidence_graph::EvidenceIngestor;

    let ingestor = EvidenceIngestor::new(&store);
    ingestor.ingest_git_commit(&args.commit_hash, "cli", " ingested via graph-ingest")?;

    let node_id = format!("commit_{}", &args.commit_hash[..8]);
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ingested": node_id,
            "commit": args.commit_hash
        }))?
    );

    Ok(())
}

fn main() {
    let args = Args::parse();

    let result = match args.command {
        Commands::Evaluate(args) => cmd_evaluate(args),
        Commands::Status(args) => cmd_status(args),
        Commands::IngestCommit(args) => cmd_ingest_commit(args),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
