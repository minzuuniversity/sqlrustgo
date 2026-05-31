//! graph-ingest: Evidence Graph Event Ingestion CLI
//!
//! Usage:
//!   graph-ingest commit <hash> <author> <message>
//!   graph-ingest ci <run_id> <commit_hash> <status> [--log-url <url>]
//!   graph-ingest artifact <id> <ci_run_id> <name>
//!   graph-ingest task <task_id> <description>
//!   graph-ingest link <from_id> <edge_type> <to_id>
//!
//! This is an AUTHORITY SERVICE - only CI/Git/Gate should call this.
//! AI systems are EXPLICITLY PROHIBITED from using this tool.

use anyhow::Result;
use clap::{Parser, Subcommand};
use evidence_graph::{EdgeType, EvidenceIngestor, GraphNode, GraphStore, NodeType};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "graph-ingest")]
#[command(about = "Evidence Graph Event Ingestion v4.1 (Authority Service)")]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Ingest a git commit event
    Commit {
        /// Full commit hash (40 chars)
        hash: String,
        /// Author email
        author: String,
        /// Commit message
        message: String,
        /// Path to graph database (default: ~/.evidence-graph.db)
        #[arg(long)]
        db: Option<PathBuf>,
    },
    /// Ingest a CI run event
    Ci {
        /// CI run ID
        run_id: String,
        /// Associated commit hash
        commit_hash: String,
        /// CI status (PASS/FAIL/PENDING)
        status: String,
        /// CI log URL (optional)
        #[arg(long)]
        log_url: Option<String>,
        /// Path to graph database (default: ~/.evidence-graph.db)
        #[arg(long)]
        db: Option<PathBuf>,
    },
    /// Ingest a test artifact
    Artifact {
        /// Artifact ID
        id: String,
        /// Associated CI run ID
        ci_run_id: String,
        /// Artifact name/type
        name: String,
        /// SHA256 of artifact
        sha256: String,
        /// Path to graph database (default: ~/.evidence-graph.db)
        #[arg(long)]
        db: Option<PathBuf>,
    },
    /// Ingest a task node
    Task {
        /// Task ID
        task_id: String,
        /// Task description
        description: String,
        /// Path to graph database (default: ~/.evidence-graph.db)
        #[arg(long)]
        db: Option<PathBuf>,
    },
    /// Link two nodes
    Link {
        /// Source node ID
        from_id: String,
        /// Edge type
        edge_type: String,
        /// Target node ID
        to_id: String,
        /// Path to graph database (default: ~/.evidence-graph.db)
        #[arg(long)]
        db: Option<PathBuf>,
    },
    /// Status of graph
    Status {
        /// Path to graph database (default: ~/.evidence-graph.db)
        #[arg(long)]
        db: Option<PathBuf>,
    },
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

fn parse_edge_type(s: &str) -> Option<EdgeType> {
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

fn cmd_commit(hash: String, author: String, message: String, db: Option<PathBuf>) -> Result<()> {
    let store = open_store(db)?;
    let ingestor = EvidenceIngestor::new(&store);
    ingestor.ingest_git_commit(&hash, &author, &message)?;
    let node_id = format!("commit_{}", &hash[..8]);
    println!("{{\"ingested\": \"{}\", \"type\": \"commit\"}}", node_id);
    Ok(())
}

fn cmd_ci(
    run_id: String,
    commit_hash: String,
    status: String,
    log_url: Option<String>,
    db: Option<PathBuf>,
) -> Result<()> {
    let store = open_store(db)?;
    let ingestor = EvidenceIngestor::new(&store);
    ingestor.ingest_ci_run(&run_id, &commit_hash, &status, log_url.as_deref())?;
    println!("{{\"ingested\": \"ci_{}\", \"type\": \"ci_run\"}}", run_id);
    Ok(())
}

fn cmd_artifact(
    id: String,
    ci_run_id: String,
    name: String,
    sha256: String,
    db: Option<PathBuf>,
) -> Result<()> {
    let store = open_store(db)?;
    let ingestor = EvidenceIngestor::new(&store);
    ingestor.ingest_artifact(&id, &ci_run_id, &name, &sha256)?;
    println!(
        "{{\"ingested\": \"artifact_{}\", \"type\": \"artifact\"}}",
        id
    );
    Ok(())
}

fn cmd_task(task_id: String, description: String, db: Option<PathBuf>) -> Result<()> {
    let store = open_store(db)?;
    let node = GraphNode::new(
        task_id.clone(),
        NodeType::Task,
        description.clone(),
        "plan_ingestion",
    );
    store.add_node(&node)?;
    println!("{{\"ingested\": \"{}\", \"type\": \"task\"}}", task_id);
    Ok(())
}

fn cmd_link(
    from_id: String,
    to_id: String,
    edge_type_str: String,
    db: Option<PathBuf>,
) -> Result<()> {
    let store = open_store(db)?;
    let edge_type = parse_edge_type(&edge_type_str)
        .ok_or_else(|| anyhow::anyhow!("Invalid edge type: {}", edge_type_str))?;
    use evidence_graph::GraphEdge;
    let edge = GraphEdge::new(from_id.clone(), to_id.clone(), edge_type);
    store.add_edge(&edge)?;

    println!(
        "{{\"linked\": \"{} → {} → {}\"}}",
        args.from_id, args.edge_type, args.to_id
    );
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
    println!();
    println!("  Tasks: {}", stats.task_count);
    println!("  Commits: {}", stats.commit_count);
    println!("  CI runs: {}", stats.ci_count);
    println!("  Artifacts: {}", stats.artifact_count);
    Ok(())
}

fn main() {
    let args = Args::parse();

    let result = match args.command {
        Commands::Commit {
            hash,
            author,
            message,
            db,
        } => cmd_commit(hash, author, message, db),
        Commands::Ci {
            run_id,
            commit_hash,
            status,
            log_url,
            db,
        } => cmd_ci(run_id, commit_hash, status, log_url, db),
        Commands::Artifact {
            id,
            ci_run_id,
            name,
            sha256,
            db,
        } => cmd_artifact(id, ci_run_id, name, sha256, db),
        Commands::Task {
            task_id,
            description,
            db,
        } => cmd_task(task_id, description, db),
        Commands::Link {
            from_id,
            to_id,
            edge_type,
            db,
        } => cmd_link(from_id, to_id, edge_type, db),
        Commands::Status { db } => cmd_status(db),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
