//! ingest: Evidence Graph Event Ingestion CLI
//!
//! Authority service — only CI/Git/Gate should call this.
//! AI MUST NOT use this tool.
//!
//! Usage:
//!   ingest commit <hash> <author> <message>
//!   ingest ci <run_id> <commit_hash> <status>
//!   ingest artifact <id> <ci_run_id> <type> <sha256>
//!   ingest task <task_id> <description>
//!   ingest link <from_id> <edge_type> <to_id>

use anyhow::Result;
use clap::Parser;
use evidence_graph::{EdgeType, EvidenceIngestor, GraphNode, GraphStore, NodeType};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "ingest")]
#[command(about = "Evidence Graph Ingestion v4.1 (Authority Service)")]
struct Args {
    #[command(subcommand)]
    command: SubCommand,
}

#[derive(Parser, Debug)]
enum SubCommand {
    Commit {
        hash: String,
        author: String,
        message: String,
        #[arg(long)]
        db: Option<PathBuf>,
    },
    Ci {
        run_id: String,
        commit_hash: String,
        status: String,
        #[arg(long)]
        log_url: Option<String>,
        #[arg(long)]
        db: Option<PathBuf>,
    },
    Artifact {
        id: String,
        ci_run_id: String,
        artifact_type: String,
        sha256: String,
        #[arg(long)]
        db: Option<PathBuf>,
    },
    Task {
        task_id: String,
        description: String,
        #[arg(long)]
        db: Option<PathBuf>,
    },
    Link {
        from_id: String,
        edge_type: String,
        to_id: String,
        #[arg(long)]
        db: Option<PathBuf>,
    },
    Status {
        #[arg(long)]
        db: Option<PathBuf>,
    },
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

fn parse_edge(s: &str) -> Option<EdgeType> {
    match s.to_uppercase().as_str() {
        "IMPLEMENTED_BY" => Some(EdgeType::ImplementedBy),
        "VERIFIED_BY" => Some(EdgeType::VerifiedBy),
        "PRODUCES" => Some(EdgeType::Produces),
        "VALIDATES" => Some(EdgeType::Validates),
        "REQUIRES" => Some(EdgeType::Requires),
        "CAUSES" => Some(EdgeType::Causes),
        _ => None,
    }
}

fn run_command(cmd: SubCommand) -> Result<()> {
    match cmd {
        SubCommand::Commit { hash, author, message, db } => {
            let store = open_store(db)?;
            let ingestor = EvidenceIngestor::new(&store);
            ingestor.ingest_git_commit(&hash, &author, &message)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "ingested": format!("commit_{}", &hash[..8]),
                    "type": "commit"
                }))?
            );
        }
        SubCommand::Ci { run_id, commit_hash, status, log_url, db } => {
            let store = open_store(db)?;
            let ingestor = EvidenceIngestor::new(&store);
            ingestor.ingest_ci_run(&run_id, &commit_hash, &status, log_url.as_deref())?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "ingested": format!("ci_{}", run_id),
                    "type": "ci_run"
                }))?
            );
        }
        SubCommand::Artifact { id, ci_run_id, artifact_type, sha256, db } => {
            let store = open_store(db)?;
            let ingestor = EvidenceIngestor::new(&store);
            ingestor.ingest_artifact(&id, &ci_run_id, &artifact_type, &sha256)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "ingested": format!("artifact_{}", id),
                    "type": "artifact"
                }))?
            );
        }
        SubCommand::Task { task_id, description, db } => {
            let store = open_store(db)?;
            let node = GraphNode::new(
                task_id.clone(),
                NodeType::Task,
                description.clone(),
                "plan_ingestion",
            );
            store.add_node(&node)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "ingested": task_id,
                    "type": "task"
                }))?
            );
        }
        SubCommand::Link { from_id, edge_type, to_id, db } => {
            let store = open_store(db)?;
            let et = parse_edge(&edge_type)
                .ok_or_else(|| anyhow::anyhow!("Invalid edge type: {}", edge_type))?;
            use evidence_graph::GraphEdge;
            let edge = GraphEdge::new(from_id.clone(), to_id.clone(), et);
            store.add_edge(&edge)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "linked": format!("{} -> {} -> {}", from_id, edge_type, to_id)
                }))?
            );
        }
        SubCommand::Status { db } => {
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
        }
    }
    Ok(())
}

fn main() {
    let args = Args::parse();
    if let Err(e) = run_command(args.command) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}