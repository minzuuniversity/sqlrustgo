//! GMP Retrieve CLI - GMP Compliance Retrieval System

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::time::Instant;
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

        /// Database path
        #[arg(long, default_value = "./data/gmp.db")]
        db: String,
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

        /// Database path
        #[arg(long, default_value = "./data/gmp.db")]
        db: String,
    },
    /// Chat with GMP knowledge base using LLM + RAG
    Chat {
        /// Query text
        #[arg(last = true)]
        query: String,

        /// Ollama base URL
        #[arg(long, default_value = "http://localhost:11434")]
        ollama_url: String,

        /// LLM model to use
        #[arg(long, default_value = "llama3.2")]
        model: String,

        /// Maximum context tokens
        #[arg(long, default_value_t = 4096)]
        max_context: usize,

        /// Enable intent-based channel routing
        #[arg(long, default_value_t = true)]
        route: bool,

        /// Show intent classification result
        #[arg(long, default_value_t = false)]
        show_intent: bool,

        /// Database path
        #[arg(long, default_value = "./data/gmp.db")]
        db: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
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
            info!("GMP Index command: source={}, validate={}, db={}", source, validate, db);
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
                println!("Status: {}", if result.is_valid { "VALID" } else { "NEEDS REBUILD" });
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
                    println!("Vector search for: '{}' (placeholder)", query);
                }
                "rule" => {
                    println!("Rule search for: '{}' (placeholder)", query);
                }
                "graph" => {
                    println!("Graph search for: '{}' (placeholder)", query);
                }
                _ => {
                    println!("Unknown channel: {}. Use: fts, vector, rule, graph, all", channel);
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
            db,
        } => {
            info!("GMP Evaluate: test_set={}", test_set);
            let limit = limit.unwrap_or(10);

            let test_content = std::fs::read_to_string(&test_set)
                .with_context(|| format!("Failed to read test set: {}", test_set))?;
            let test_scenarios: Vec<TestScenario> = serde_json::from_str(&test_content)
                .with_context(|| format!("Failed to parse test set JSON from {}", test_set))?;

            println!("Loaded {} test scenarios", test_scenarios.len());

            let config = gmp_storage::StorageConfig {
                data_dir: "./data".to_string(),
                db_path: db,
            };
            let storage = gmp_storage::StorageEngine::new(config)
                .context("Failed to initialize storage")?;

            let mut results = Vec::new();
            let mut passed = 0u32;
            let total = test_scenarios.len().min(limit);

            for (i, scenario) in test_scenarios.iter().take(limit).enumerate() {
                print!("\rEvaluating: {}/{}", i + 1, total);
                std::io::Write::flush(&mut std::io::stdout()).unwrap_or(());

                let search_results = storage
                    .search_fts(&scenario.query, scenario.top_k.unwrap_or(5))
                    .unwrap_or_default();

                let retrieved_ids: Vec<String> =
                    search_results.iter().map(|r| r.doc_id.clone()).collect();

                let score = calculate_metrics(&scenario.expected_doc_ids, &retrieved_ids);
                let is_pass = score.recall > 0.0;

                if is_pass {
                    passed += 1;
                }

                results.push(EvalResult {
                    query: scenario.query.clone(),
                    expected: scenario.expected_doc_ids.clone(),
                    retrieved: retrieved_ids,
                    recall: score.recall,
                    mrr: score.mrr,
                    passed: is_pass,
                });
            }
            println!();

            let total_recall: f64 = results.iter().map(|r| r.recall).sum();
            let total_mrr: f64 = results.iter().map(|r| r.mrr).sum();
            let avg_recall = total_recall / total as f64;
            let avg_mrr = total_mrr / total as f64;
            let pass_rate = (passed as f64 / total as f64) * 100.0;

            println!("\n========== EVALUATION REPORT ==========");
            println!(
                "Total: {} | Passed: {} | Failed: {}",
                total,
                passed,
                total - passed as usize
            );
            println!("Pass Rate: {:.1}%", pass_rate);
            println!("Avg Recall@K: {:.3}", avg_recall);
            println!("Avg MRR: {:.3}", avg_mrr);
            println!("======================================");

            if let Some(output_path) = output {
                let report = EvalReport {
                    total,
                    passed,
                    failed: total - passed as usize,
                    pass_rate,
                    avg_recall,
                    avg_mrr,
                    results,
                };
                let json = serde_json::to_string_pretty(&report).context("Failed to serialize report")?;
                std::fs::write(&output_path, json)
                    .with_context(|| format!("Failed to write report to {}", output_path))?;
                println!("Report saved to: {}", output_path);
            }
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
        Commands::Bench {
            concurrent,
            rounds,
            db,
        } => {
            use std::time::Instant;
            use tokio::task;

            info!("GMP Bench: concurrent={}, rounds={}", concurrent, rounds);
            println!(
                "Running benchmark: {} concurrent x {} rounds",
                concurrent, rounds
            );

            let config = gmp_storage::StorageConfig {
                data_dir: "./data".to_string(),
                db_path: db.clone(),
            };
            let storage = match gmp_storage::StorageEngine::new(config) {
                Ok(s) => s,
                Err(e) => {
                    println!("Error: Failed to initialize storage: {}", e);
                    return Ok(());
                }
            };

            let queries = [
                "GMP原料药管理要求",
                "洁净区环境监测",
                "偏差调查流程",
                "变更控制程序",
                "供应商审计",
                "批记录审核",
                "验证主计划",
                "召回程序",
            ];

            println!("\nWarming up...");
            let _ = storage.search_fts("warmup", 5);

            println!("Running benchmark...\n");
            let overall_start = Instant::now();
            let mut latencies: Vec<u64> = Vec::new();

            for round in 0..rounds {
                let round_start = Instant::now();
                let query_idx = round % queries.len();
                let query = queries[query_idx].to_string();
                let db_path = db.clone();
                let data_dir = "./data".to_string();

                let handles: Vec<_> = (0..concurrent)
                    .map(|_| {
                        let q = query.clone();
                        let dp = db_path.clone();
                        let dd = data_dir.clone();
                        task::spawn_blocking(move || {
                            let cfg = gmp_storage::StorageConfig {
                                data_dir: dd,
                                db_path: dp,
                            };
                            gmp_storage::StorageEngine::new(cfg)
                                .and_then(|s| s.search_fts(&q, 5))
                        })
                    })
                    .collect();

                let mut round_latencies: Vec<u64> = Vec::new();
                for h in handles {
                    match h.await {
                        Ok(Ok(_)) => {
                            round_latencies.push(round_start.elapsed().as_millis() as u64);
                        }
                        _ => {}
                    }
                }
                latencies.extend(round_latencies);

                print!(
                    "\rRound {}/{} - {} ops done",
                    round + 1,
                    rounds,
                    latencies.len()
                );
                std::io::Write::flush(&mut std::io::stdout()).unwrap_or(());
            }
            println!();

            let total_ms = overall_start.elapsed().as_millis() as f64;
            let ops_count = latencies.len() as f64;
            let qps = if total_ms > 0.0 {
                (ops_count / total_ms) * 1000.0
            } else {
                0.0
            };

            latencies.sort();
            let p50 = latencies
                .get(latencies.len() * 50 / 100)
                .copied()
                .unwrap_or(0);
            let p95 = latencies
                .get(latencies.len() * 95 / 100)
                .copied()
                .unwrap_or(0);
            let p99 = latencies
                .get(latencies.len() * 99 / 100)
                .copied()
                .unwrap_or(0);
            let min = *latencies.first().unwrap_or(&0);
            let max = *latencies.last().unwrap_or(&0);
            let avg = if ops_count > 0.0 {
                latencies.iter().sum::<u64>() as f64 / ops_count
            } else {
                0.0
            };

            println!("\n========== BENCHMARK REPORT ==========");
            println!("Total operations: {}", ops_count);
            println!("Total time: {:.0} ms", total_ms);
            println!("QPS: {:.1}", qps);
            println!("Latency (ms):");
            println!("  Min:    {}", min);
            println!("  Avg:    {:.1}", avg);
            println!("  P50:    {}", p50);
            println!("  P95:    {}", p95);
            println!("  P99:    {}", p99);
            println!("  Max:    {}", max);
            println!("=====================================");
        }
        Commands::Chat {
            query,
            ollama_url,
            model,
            max_context,
            route,
            show_intent,
            db,
        } => {
            use gmp_llm::LlmClient;
            use std::time::Instant;

            println!("GMP Chat (LLM + RAG)\n");

            // Build context from storage (sync)
            let config = gmp_storage::StorageConfig {
                data_dir: "./data".to_string(),
                db_path: db,
            };
            let storage = match gmp_storage::StorageEngine::new(config) {
                Ok(s) => s,
                Err(e) => {
                    println!("Error: Failed to initialize storage: {}", e);
                    return Ok(());
                }
            };

            let chunks: Vec<String> = storage
                .search_fts(&query, 5)
                .unwrap_or_default()
                .into_iter()
                .map(|r| {
                    format!(
                        "[{}] {}\n\n{}",
                        r.title,
                        r.doc_type.clone().unwrap_or_default(),
                        r.content
                    )
                })
                .collect();

            if chunks.is_empty() {
                println!("No relevant documents found. Please run 'gmp-retrieve index' first.");
                return Ok(());
            }

            println!("[Retrieval] {} chunks found\n---", chunks.len());

            // Run async LLM operations
            let llm = LlmClient::new(ollama_url.clone(), model.clone());

            // Route if enabled
            if route {
                let start = Instant::now();
                match llm.classify_intent(&query).await {
                    Ok(intent) => {
                        println!(
                            "[Intent] {:?} ({:.0}ms)",
                            intent,
                            start.elapsed().as_secs_f64() * 1000.0
                        );
                        if show_intent {
                            let route_result = llm.route_query(&query).await?;
                            println!("[Routing] Channels: {:?}", route_result.channels);
                        }
                    }
                    Err(e) => println!("[Intent] Classification unavailable: {}", e),
                }
                println!();
            }

            // Generate answer with RAG
            let start = Instant::now();
            match llm.generate_answer(&query, &chunks, max_context).await {
                Ok(answer) => {
                    println!(
                        "\n[LLM Response] ({:.0}ms)\n{}",
                        start.elapsed().as_secs_f64() * 1000.0,
                        answer
                    );
                }
                Err(e) => {
                    println!("\n[LLM] Error: {}", e);
                    println!("Make sure Ollama is running: ollama serve");
                }
            }
        }
    }

    Ok(())
}

// ========== Evaluation Types ==========

#[derive(Debug, Clone, serde::Deserialize)]
pub struct TestScenario {
    pub query: String,
    pub expected_doc_ids: Vec<String>,
    #[serde(default)]
    pub top_k: Option<usize>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EvalResult {
    pub query: String,
    pub expected: Vec<String>,
    pub retrieved: Vec<String>,
    pub recall: f64,
    pub mrr: f64,
    pub passed: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EvalReport {
    pub total: usize,
    pub passed: u32,
    pub failed: usize,
    pub pass_rate: f64,
    pub avg_recall: f64,
    pub avg_mrr: f64,
    pub results: Vec<EvalResult>,
}

#[derive(Debug, Clone)]
struct Metrics {
    recall: f64,
    mrr: f64,
}

/// Calculate Recall@K and MRR given expected and retrieved doc IDs
fn calculate_metrics(expected: &[String], retrieved: &[String]) -> Metrics {
    if expected.is_empty() {
        return Metrics {
            recall: 0.0,
            mrr: 0.0,
        };
    }
    if retrieved.is_empty() {
        return Metrics {
            recall: 0.0,
            mrr: 0.0,
        };
    }

    let expected_set: std::collections::HashSet<_> = expected.iter().collect();
    let retrieved_set: std::collections::HashSet<_> = retrieved.iter().collect();

    let intersection: Vec<_> = expected_set.intersection(&retrieved_set).collect();
    let recall = intersection.len() as f64 / expected.len() as f64;

    let mrr = if let Some((rank, _)) = retrieved
        .iter()
        .enumerate()
        .find(|(_, id)| expected_set.contains(id))
    {
        1.0 / (rank + 1) as f64
    } else {
        0.0
    };

    Metrics { recall, mrr }
}
