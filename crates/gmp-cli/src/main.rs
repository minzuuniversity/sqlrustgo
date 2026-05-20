//! GMP Retrieve CLI - GMP Compliance Retrieval System

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "gmp-retrieve")]
#[command(about = "GMP Compliance Retrieval System", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Build indexes from source documents
    Index {
        /// Path to GMP documents directory
        #[arg(long, default_value = "gmp-md")]
        source: String,

        /// Validate existing index
        #[arg(long, default_value_t = false)]
        validate: bool,

        /// Database path
        #[arg(long, default_value = "./data/gmp.db")]
        db: String,
    },
    /// Search for relevant GMP documents
    Search {
        /// Search query
        #[arg(last = true)]
        query: String,

        /// Channel to use (rule, vector, fts, graph, all)
        #[arg(long, default_value = "all")]
        channel: String,

        /// Enable reranking
        #[arg(long, default_value_t = false)]
        rerank: bool,

        /// Show debug information
        #[arg(long, default_value_t = false)]
        explain: bool,

        /// Database path
        #[arg(long, default_value = "./data/gmp.db")]
        db: String,
    },
    /// Evaluate against test scenarios
    Evaluate {
        /// Path to test scenarios JSON file
        #[arg(long)]
        test_set: String,

        /// Output results to file
        #[arg(long)]
        output: Option<String>,

        /// Limit number of scenarios to test
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Interactive shell for queries
    Shell {
        /// Database path
        #[arg(long, default_value = "./data/gmp.db")]
        db: String,
    },
    /// Benchmark performance
    Bench {
        /// Number of concurrent queries
        #[arg(long, default_value = "10")]
        concurrent: usize,

        /// Number of rounds
        #[arg(long, default_value = "100")]
        rounds: usize,
    },
}

fn main() -> Result<()> {
    // Initialize logging
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Index {
            source,
            validate,
            db,
        } => {
            info!(
                "GMP Index command: source={}, validate={}, db={}",
                source, validate, db
            );
            if validate {
                let config = gmp_storage::StorageConfig {
                    data_dir: "./data".to_string(),
                    db_path: db.clone(),
                };
                let engine = gmp_storage::StorageEngine::new(config)?;
                let result = engine.validate()?;
                println!("Index Validation:");
                println!("  Documents: {}", result.documents);
                println!("  Chunks: {}", result.chunks);
                println!(
                    "  Status: {}",
                    if result.is_valid {
                        "VALID"
                    } else {
                        "NEEDS REBUILD"
                    }
                );
            } else {
                println!("Indexing from: {}...", source);
                let config = gmp_storage::StorageConfig {
                    data_dir: "./data".to_string(),
                    db_path: db,
                };
                let engine = gmp_storage::StorageEngine::new(config)?;
                let stats = engine.index_documents(&source)?;
                println!("Indexing complete:");
                println!("  Documents imported: {}", stats.documents_imported);
                println!("  Chunks indexed: {}", stats.chunks_indexed);
                if !stats.errors.is_empty() {
                    println!("  Errors: {}", stats.errors.len());
                    for e in stats.errors.iter().take(5) {
                        println!("    - {}", e);
                    }
                }
            }
        }
        Commands::Search {
            query,
            channel,
            rerank: _,
            explain,
            db,
        } => {
            info!("GMP Search: query={}, channel={}", query, channel);
            let config = gmp_storage::StorageConfig {
                data_dir: "./data".to_string(),
                db_path: db,
            };
            let engine = gmp_storage::StorageEngine::new(config)?;

            match channel.as_str() {
                "fts" | "all" => {
                    println!("Searching FTS for: '{}'\n", query);
                    let results = engine.search_fts(&query, 10)?;
                    if results.is_empty() {
                        println!("No results found.");
                    } else {
                        for (i, r) in results.iter().enumerate() {
                            println!(
                                "{}. [{}] {}",
                                i + 1,
                                r.doc_type.clone().unwrap_or_default(),
                                r.title
                            );
                            println!("   Score: {:.4}", r.score);
                            println!("   {}...", r.content.chars().take(200).collect::<String>());
                            println!();
                        }
                    }
                }
                "vector" => {
                    println!(
                        "Vector search for: '{}' (placeholder - pending Issue #10)",
                        query
                    );
                }
                "rule" => {
                    println!(
                        "Rule search for: '{}' (placeholder - pending Issue #10)",
                        query
                    );
                }
                "graph" => {
                    println!(
                        "Graph search for: '{}' (placeholder - pending Issue #10)",
                        query
                    );
                }
                _ => {
                    println!(
                        "Unknown channel: {}. Use: fts, vector, rule, graph, all",
                        channel
                    );
                }
            }

            if explain {
                println!("\nDebug: Four-layer architecture - see docs/design/gmp-sqlrustgo-four-layer-plan.md");
            }
        }
        Commands::Evaluate {
            test_set,
            output,
            limit,
        } => {
            info!("GMP Evaluate: test_set={}", test_set);
            println!(
                "Evaluating test set: {} (placeholder)\nOutput: {:?}\nLimit: {:?}",
                test_set, output, limit
            );
        }
        Commands::Shell { db } => {
            info!("Starting interactive shell");
            println!("GMP Compliance Retrieval Shell (v1.0.0-alpha)");
            println!("Type 'help' for commands, 'exit' to quit.");
            println!();
            loop {
                print!("gmp> ");
                std::io::Write::flush(&mut std::io::stdout())?;
                let mut input = String::new();
                if std::io::stdin().read_line(&mut input)? == 0 {
                    break;
                }
                let input = input.trim();
                if input.is_empty() || input == "exit" || input == "quit" {
                    break;
                }
                if input == "help" {
                    println!("Available commands:");
                    println!("  search <query>  - Search for documents");
                    println!("  index           - Build indexes");
                    println!("  exit/quit       - Exit shell");
                    continue;
                }
                if let Some(query) = input.strip_prefix("search ") {
                    let config = gmp_storage::StorageConfig {
                        data_dir: "./data".to_string(),
                        db_path: db.clone(),
                    };
                    match gmp_storage::StorageEngine::new(config) {
                        Ok(engine) => {
                            let results = engine.search_fts(query, 5).unwrap_or_default();
                            if results.is_empty() {
                                println!("No results found.");
                            } else {
                                for (i, r) in results.iter().enumerate() {
                                    println!(
                                        "{}. [{}] {}",
                                        i + 1,
                                        r.doc_type.clone().unwrap_or_default(),
                                        r.title
                                    );
                                }
                            }
                        }
                        Err(e) => println!("Error: {}", e),
                    }
                } else if input == "index" {
                    println!("Use 'gmp-retrieve index --source gmp-md/' to build indexes");
                } else {
                    println!(
                        "Unknown command: '{}'. Type 'help' for available commands.",
                        input
                    );
                }
            }
        }
        Commands::Bench { concurrent, rounds } => {
            info!("GMP Bench: concurrent={}, rounds={}", concurrent, rounds);
            println!(
                "Running benchmark: {} concurrent x {} rounds (placeholder)",
                concurrent, rounds
            );
        }
    }

    Ok(())
}
