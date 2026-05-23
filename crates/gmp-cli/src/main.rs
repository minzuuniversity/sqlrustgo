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
    /// Populate Layer 0 tables (NER, RE, Taxonomy, Relevance) using Ollama + rules
    Populate {
        /// Ollama base URL
        #[arg(long, default_value = "http://192.168.0.252:11434")]
        ollama_url: String,

        /// Ollama model for NER/classification
        #[arg(long, default_value = "qwen2.5:14b-instruct-q4_K_M")]
        model: String,

        /// Batch size for parallel processing
        #[arg(long, default_value_t = 10)]
        batch_size: usize,

        /// Database path
        #[arg(long, default_value = "./data/gmp.db")]
        db: String,

        /// Stage to run (all|ner|re|taxonomy|relevance)
        #[arg(long, default_value = "all")]
        stage: String,
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
            info!(
                "GMP Index command: source={}, validate={}, db={}",
                source, validate, db
            );
            if validate {
                let config = gmp_storage::StorageConfig {
                    embedding_dim: 768,
                    data_dir: "./data".to_string(),
                    use_sqlrustgo_storage: true,
                    db_path: db.clone(),
                };
                let engine = gmp_storage::StorageEngine::new(config)?;
                let result = engine.validate()?;
                println!("Index Validation:");
                println!("  Documents: {}", result.documents);
                println!("  Chunks: {}", result.chunks);
                println!(
                    "Status: {}",
                    if result.is_valid {
                        "VALID"
                    } else {
                        "NEEDS REBUILD"
                    }
                );
            } else {
                println!("Indexing from: {}...", source);
                let config = gmp_storage::StorageConfig {
                    embedding_dim: 768,
                    data_dir: "./data".to_string(),
                    use_sqlrustgo_storage: true,
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
            rerank,
            explain,
            db,
        } => {
            use gmp_core::ScoredChunk;
            use gmp_retrieval::channel::FourChannelEngine;
            use gmp_retrieval::RrfFusion;

            info!(
                "GMP Search: query={}, channel={}, rerank={}",
                query, channel, rerank
            );

            let config = gmp_storage::StorageConfig {
                embedding_dim: 768,
                data_dir: "./data".to_string(),
                use_sqlrustgo_storage: true,
                db_path: db,
            };
            let storage = match gmp_storage::StorageEngine::new(config) {
                Ok(s) => s,
                Err(e) => {
                    println!("Error: Failed to initialize storage: {}", e);
                    return Ok(());
                }
            };

            // Build four-channel engine
            let four_channel = FourChannelEngine::new(storage, None);
            let fusion = RrfFusion::new(60.0);

            // Run four-channel search via tokio runtime
            let results: Vec<ScoredChunk> = std::thread::scope(|s| {
                s.spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();
                    rt.block_on(async { four_channel.search(&query, 20).await })
                })
                .join()
                .unwrap()
            });

            let fused: Vec<ScoredChunk> = if channel == "all" {
                // Full fusion with all channels using equal weights
                fusion.fuse(vec![(results.clone(), 1.0f32)])
            } else {
                results.clone()
            };

            // Apply reranking if requested
            let final_results: Vec<ScoredChunk> = if rerank && !fused.is_empty() {
                let reranker = gmp_reranker::create_reranker(
                    "http://localhost:11434",
                    "bge-reranker-v2-m3:latest",
                );
                match std::thread::scope(|s| {
                    s.spawn(|| {
                        let rt = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .unwrap();
                        rt.block_on(reranker.rerank(&query, fused.clone(), 20))
                    })
                    .join()
                    .unwrap()
                }) {
                    Ok(reranked) => reranked,
                    Err(e) => {
                        println!("Reranking failed: {}, using fusion results", e);
                        fused
                    }
                }
            } else {
                fused
            };

            if final_results.is_empty() {
                println!("No results found.");
            } else {
                println!("Found {} results:\n", final_results.len());
                for (i, r) in final_results.iter().enumerate() {
                    println!(
                        "{}. [{}][{}] {}",
                        i + 1,
                        r.source,
                        if rerank { "RERANKED" } else { "FUSED" },
                        r.doc_id
                    );
                    println!("   Score: {:.4}", r.score);
                    if explain {
                        println!(
                            "   Content: {}...",
                            r.content.chars().take(150).collect::<String>()
                        );
                    }
                    println!();
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
                embedding_dim: 768,
                data_dir: "./data".to_string(),
                use_sqlrustgo_storage: true,
                db_path: db,
            };
            let storage =
                gmp_storage::StorageEngine::new(config).context("Failed to initialize storage")?;

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
                let json =
                    serde_json::to_string_pretty(&report).context("Failed to serialize report")?;
                std::fs::write(&output_path, json)
                    .with_context(|| format!("Failed to write report to {}", output_path))?;
                println!("Report saved to: {}", output_path);
            }
        }
        Commands::Shell { db } => {
            info!("Starting interactive shell");
            println!("GMP Compliance Retrieval Shell (v1.0.0)");
            println!(
                "Commands: search <q>, route <q>, rag <q>, stats, channel <c> <q>, help, exit"
            );
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
                    println!("Commands:");
                    println!("  search <q>    - FTS search");
                    println!("  route <q>     - Intent classification + channel routing");
                    println!("  rag <q>       - LLM RAG Q&A");
                    println!("  stats         - Index statistics");
                    println!("  channel <c> <q> - Search in channel (fts/vector/rule/graph)");
                    println!("  help          - Show this help");
                    println!("  exit/quit     - Exit shell");
                    continue;
                }
                if input == "stats" {
                    let config = gmp_storage::StorageConfig {
                        embedding_dim: 768,
                        data_dir: "./data".to_string(),
                        use_sqlrustgo_storage: true,
                        db_path: db.clone(),
                };
                    match gmp_storage::StorageEngine::new(config) {
                        Ok(engine) => match engine.validate() {
                            Ok(result) => {
                                println!("Index Statistics:");
                                println!("  Documents: {}", result.documents);
                                println!("  Chunks:   {}", result.chunks);
                                println!(
                                    "  Status:   {}",
                                    if result.is_valid {
                                        "VALID"
                                    } else {
                                        "NEEDS REBUILD"
                                    }
                                );
                            }
                            Err(e) => println!("Error: {}", e),
                        },
                        Err(e) => println!("Error: {}", e),
                    }
                    continue;
                }
                if let Some(query) = input.strip_prefix("search ") {
                    let config = gmp_storage::StorageConfig {
                        embedding_dim: 768,
                        data_dir: "./data".to_string(),
                        use_sqlrustgo_storage: true,
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
                } else if let Some(query) = input.strip_prefix("route ") {
                    let rt = match tokio::runtime::Runtime::new() {
                        Ok(rt) => rt,
                        Err(e) => {
                            println!("Error: cannot create runtime: {}", e);
                            continue;
                        }
                    };
                    let q = query.to_string();
                    rt.block_on(async {
                        use gmp_llm::LlmClient;
                        let llm = LlmClient::new(
                            "http://localhost:11434".to_string(),
                            "llama3.2".to_string(),
                        );
                        match llm.classify_intent(&q).await {
                            Ok(intent) => println!("[Intent] {:?}", intent),
                            Err(e) => println!("[Intent] Error: {} (is Ollama running?)", e),
                        }
                    });
                } else if let Some(query) = input.strip_prefix("rag ") {
                    let config = gmp_storage::StorageConfig {
                        embedding_dim: 768,
                        data_dir: "./data".to_string(),
                        use_sqlrustgo_storage: true,
                        db_path: db.clone(),
                };
                    let chunks: Vec<String> = match gmp_storage::StorageEngine::new(config) {
                        Ok(engine) => engine
                            .search_fts(query, 3)
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
                            .collect(),
                        Err(e) => {
                            println!("Error: {}", e);
                            continue;
                        }
                    };
                    if chunks.is_empty() {
                        println!("No documents found. Run 'index' first.");
                        continue;
                    }
                    let rt = match tokio::runtime::Runtime::new() {
                        Ok(rt) => rt,
                        Err(e) => {
                            println!("Error: cannot create runtime: {}", e);
                            continue;
                        }
                    };
                    let q = query.to_string();
                    let c = chunks.clone();
                    rt.block_on(async {
                        use gmp_llm::LlmClient;
                        let llm = LlmClient::new(
                            "http://localhost:11434".to_string(),
                            "llama3.2".to_string(),
                        );
                        match llm.generate_answer(&q, &c, 4096).await {
                            Ok(ans) => println!("{}", ans),
                            Err(e) => println!("[LLM] Error: {} (is Ollama running?)", e),
                        }
                    });
                } else if let Some(rest) = input.strip_prefix("channel ") {
                    let parts: Vec<&str> = rest.splitn(2, ' ').collect();
                    if parts.len() < 2 {
                        println!("Usage: channel <fts|vector|rule|graph> <query>");
                        continue;
                    }
                    let channel = parts[0];
                    let query = parts[1];
                    if channel != "fts" {
                        println!(
                            "Channel '{}' not yet implemented (only 'fts' available)",
                            channel
                        );
                        continue;
                    }
                    let config = gmp_storage::StorageConfig {
                        embedding_dim: 768,
                        data_dir: "./data".to_string(),
                        use_sqlrustgo_storage: true,
                        db_path: db.clone(),
                };
                    match gmp_storage::StorageEngine::new(config) {
                        Ok(engine) => {
                            let results = engine.search_fts(query, 5).unwrap_or_default();
                            if results.is_empty() {
                                println!("No results found.");
                            } else {
                                for (i, r) in results.iter().enumerate() {
                                    println!("{}. {}", i + 1, r.title);
                                }
                            }
                        }
                        Err(e) => println!("Error: {}", e),
                    }
                } else {
                    println!("Unknown command. Type 'help' for available commands.");
                }
            }
        }
        Commands::Bench {
            concurrent,
            rounds,
            db,
        } => {
            use tokio::task;

            info!("GMP Bench: concurrent={}, rounds={}", concurrent, rounds);
            println!(
                "Running benchmark: {} concurrent x {} rounds",
                concurrent, rounds
            );

            let config = gmp_storage::StorageConfig {
                embedding_dim: 768,
                data_dir: "./data".to_string(),
                use_sqlrustgo_storage: true,
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
                                embedding_dim: 768,
                                use_sqlrustgo_storage: true,
                            };
                            gmp_storage::StorageEngine::new(cfg).and_then(|s| s.search_fts(&q, 5))
                        })
                    })
                    .collect();

                let mut round_latencies: Vec<u64> = Vec::new();
                for h in handles {
                    if let Ok(Ok(_)) = h.await {
                        round_latencies.push(round_start.elapsed().as_millis() as u64);
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

            println!("GMP Chat (LLM + RAG)\n");

            // Build context from storage (sync)
            let config = gmp_storage::StorageConfig {
                embedding_dim: 768,
                data_dir: "./data".to_string(),
                use_sqlrustgo_storage: true,
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
        Commands::Populate {
            ollama_url,
            model,
            batch_size: _batch_size,
            db,
            stage,
        } => {
            use gmp_llm::LlmClient;
            use std::time::Instant;

            println!("GMP Layer 0 Population\n");
            println!("  Ollama: {} ({})", ollama_url, model);
            println!("  Stage:  {}", stage);
            println!("  DB:     {}\n", db);

            // Initialize storage engine
            let config = gmp_storage::StorageConfig {
                data_dir: std::path::Path::new(&db)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "./data".to_string()),
                db_path: db.clone(),
            };
            let storage = match gmp_storage::StorageEngine::new(config) {
                Ok(s) => s,
                Err(e) => {
                    println!("Error: Failed to initialize storage: {}", e);
                    return Ok(());
                }
            };

            // Get connection for Layer 0 population
            let conn = storage.get_conn().lock().unwrap();

            // Initialize LLM client
            let llm = LlmClient::new(ollama_url.clone(), model.clone());

            // ---- Stage 1: NER - Extract entities from documents ----
            if stage == "all" || stage == "ner" {
                println!("[Stage 1/4] NER - Extracting GMP entities...");
                let start = Instant::now();

                // Get all documents
                let doc_count: i64 = conn
                    .query_row("SELECT COUNT(*) FROM gmp_documents", [], |r| r.get(0))
                    .unwrap_or(0);
                println!("  Processing {} documents...", doc_count);

                // Load GMP terms from gmp_terms for rule-based NER
                let terms: Vec<(String, String, String)> = {
                    let mut stmt = match conn.prepare(
                        "SELECT canonical_term, term_type, COALESCE(entity_type, '') FROM gmp_terms LIMIT 500",
                    ) {
                        Ok(s) => s,
                        Err(_) => {
                            println!("  Warning: gmp_terms table empty, run 'sqlite3 gmp.db < gmp_terms.sql' first");
                            return Ok(());
                        }
                    };
                    stmt.query_map([], |r| {
                        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
                    })
                    .unwrap()
                    .filter_map(Result::ok)
                    .collect()
                };
                println!("  Loaded {} GMP terms for entity matching", terms.len());

                // Get all documents for NER
                let docs: Vec<(String, String)> = {
                    let mut stmt = conn
                        .prepare("SELECT doc_id, COALESCE(title, '') || ' ' || COALESCE(code, '') FROM gmp_documents")
                        .unwrap();
                    stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
                        .unwrap()
                        .filter_map(Result::ok)
                        .collect()
                };

                let mut entity_id = conn
                    .query_row("SELECT COALESCE(MAX(id), 0) + 1 FROM doc_entities", [], |r| r.get::<_, i64>(0))
                    .unwrap_or(1);

                let mut inserted = 0;
                for (doc_id, doc_text) in docs.iter().take(100) {
                    // Rule-based entity extraction: match GMP terms in document
                    let text_lower = doc_text.to_lowercase();
                    let mut seen_terms: std::collections::HashSet<String> = std::collections::HashSet::new();

                    for (term, term_type, entity_type) in &terms {
                        let term_lower = term.to_lowercase();
                        // Skip short terms and whitespace terms
                        if term_lower.trim().len() < 2 {
                            continue;
                        }
                        // Check if term appears in document (simple substring match)
                        if text_lower.contains(&term_lower) && !seen_terms.contains(term_lower.as_str()) {
                            seen_terms.insert(term_lower.clone());

                            // Extract surface form (case-insensitive first occurrence)
                            if let Some(pos) = text_lower.find(&term_lower) {
                                let start_pos = pos as i64;
                                let end_pos = (pos + term.len()) as i64;
                                let surface = &doc_text[pos..pos + term.len()];

                                conn.execute(
                                    "INSERT OR IGNORE INTO doc_entities (id, doc_id, entity_type, surface_form, canonical_term, start_pos, end_pos, confidence) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                                    params![entity_id, doc_id, entity_type, surface, term, start_pos, end_pos, 1.0],
                                ).ok();
                                entity_id += 1;
                                inserted += 1;
                            }
                        }
                    }
                }

                // Also call Ollama for enhanced NER on a sample
                println!("  [Ollama] Calling NER on top documents...");
                let sample_docs: Vec<String> = docs.iter().take(5).map(|(id, _)| id.clone()).collect();
                for doc_id in sample_docs {
                    let doc_content: String = conn
                        .query_row(
                            "SELECT COALESCE(title, '') || char(10) || COALESCE(code, '') FROM gmp_documents WHERE doc_id = ?",
                            params![doc_id],
                            |r| r.get::<_, String>(0),
                        )
                        .unwrap_or_default();

                    let prompt = format!(
                        "从以下GMP文档中提取所有实体（人名、机构名、法规条款、文件名、程序名等）。\n以JSON数组格式返回，每个实体包含：surface_form（原文）、canonical_term（规范名）、entity_type（实体类型）。\n\n文档：{}\n\n返回JSON：",
                        doc_content.chars().take(500).collect::<String>()
                    );

                    if let Ok(resp) = tokio::runtime::Handle::current().block_on(async {
                        llm.chat(vec![gmp_llm::ChatMessage {
                            role: "user".to_string(),
                            content: prompt,
                        }])
                        .await
                    }) {
                        let content = resp.message.content.trim();
                        // Parse JSON entities from response
                        if let Ok(entities) = serde_json::from_str::<Vec<serde_json::Value>>(content) {
                            for entity in entities {
                                let surface = entity.get("surface_form").and_then(|v| v.as_str()).unwrap_or("");
                                let canonical = entity.get("canonical_term").and_then(|v| v.as_str()).unwrap_or(surface);
                                let e_type = entity.get("entity_type").and_then(|v| v.as_str()).unwrap_or("entity");

                                if !surface.is_empty() {
                                    conn.execute(
                                        "INSERT OR IGNORE INTO doc_entities (id, doc_id, entity_type, surface_form, canonical_term, start_pos, end_pos, confidence) VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL, ?6)",
                                        params![entity_id, doc_id, e_type, surface, canonical, 0.8],
                                    ).ok();
                                    entity_id += 1;
                                    inserted += 1;
                                }
                            }
                        }
                    }
                }

                println!("  NER complete: {} entities inserted in {:.1}s", inserted, start.elapsed().as_secs_f64());
            }

            // ---- Stage 2: RE - Extract relations ----
            if stage == "all" || stage == "re" {
                println!("\n[Stage 2/4] RE - Extracting relations...");
                let start = Instant::now();


                // Rule-based relation extraction from entities in same document
                let mut rel_id = conn
                    .query_row("SELECT COALESCE(MAX(id), 0) + 1 FROM doc_relations", [], |r| r.get::<_, i64>(0))
                    .unwrap_or(1);

                let docs_with_entities: Vec<String> = {
                    let mut stmt = conn
                        .prepare("SELECT DISTINCT doc_id FROM doc_entities LIMIT 200")
                        .unwrap();
                    stmt.query_map([], |r| r.get::<_, String>(0))
                        .unwrap()
                        .filter_map(Result::ok)
                        .collect()
                };

                let mut inserted = 0;
                for doc_id in docs_with_entities {
                    let entities: Vec<(String, String)> = {
                        let mut stmt = conn
                            .prepare("SELECT surface_form, entity_type FROM doc_entities WHERE doc_id = ?")
                            .unwrap();
                        stmt.query_map(params![doc_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
                            .unwrap()
                            .filter_map(Result::ok)
                            .collect()
                    };

                    // Create relations between entities of different types
                    for (i, (src, src_type)) in entities.iter().enumerate() {
                        for (j, (dst, dst_type)) in entities.iter().enumerate() {
                            if i != j && src_type != dst_type {
                                // Cross-type relation: likely meaningful
                                let rel_type = format!("{}_to_{}", src_type, dst_type);
                                conn.execute(
                                    "INSERT OR IGNORE INTO doc_relations (id, doc_id, relation_type, src_entity, dst_entity, confidence) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                                    params![rel_id, doc_id, rel_type, src, dst, 0.7],
                                ).ok();
                                rel_id += 1;
                                inserted += 1;
                            }
                        }
                    }
                }

                println!("  RE complete: {} relations inserted in {:.1}s", inserted, start.elapsed().as_secs_f64());
            }

            // ---- Stage 3: Taxonomy - Classify documents ----
            if stage == "all" || stage == "taxonomy" {
                println!("\n[Stage 3/4] Taxonomy - Classifying documents...");
                let start = Instant::now();


                // Rule-based taxonomy from document metadata + content keywords
                let keywords_map = std::collections::HashMap::from([
                    ("验证", ("质量控制", "验证与确认")),
                    ("清洁", ("生产", "清洁与卫生")),
                    ("设备", ("工程", "设备管理")),
                    ("原料", ("仓储", "物料管理")),
                    ("偏差", ("质量", "偏差与纠正")),
                    ("变更", ("质量", "变更控制")),
                    ("供应商", ("采购", "供应商管理")),
                    ("文件", ("质量", "文件管理")),
                    ("培训", ("人力资源", "培训管理")),
                    ("洁净区", ("生产", "环境监控")),
                ]);

                let docs: Vec<(String, String, String)> = {
                    let mut stmt = conn
                        .prepare("SELECT doc_id, COALESCE(title, ''), COALESCE(code, '') FROM gmp_documents")
                        .unwrap();
                    stmt.query_map([], |r| {
                        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
                    })
                    .unwrap()
                    .filter_map(Result::ok)
                    .collect()
                };

                let mut inserted = 0;
                for (doc_id, title, code) in docs {
                    let text_lower = format!("{} {}", title, code).to_lowercase();
                    let mut best_topic = "质量管理";
                    let mut best_sub = "GMP基础";
                    let mut best_risk = "中";
                    let mut confidence = 0.5;

                    for (kw, (dept, sub)) in &keywords_map {
                        if text_lower.contains(kw) {
                            best_topic = dept;
                            best_sub = sub;
                            best_risk = if kw == &"偏差" || kw == &"变更" { "高" } else { "中" };
                            confidence = 0.7;
                            break;
                        }
                    }

                    // Also use Ollama for classification on a sample
                    if confidence < 0.7 {
                        let prompt = format!(
                            "对以下GMP文档进行分类，返回JSON：{{\"topic\":\"主题\",\"sub_topic\":\"子主题\",\"doc_type\":\"类型\",\"risk_level\":\"风险等级(高/中/低)\",\"department\":\"部门\"}}\n\n文档：{}\n\nJSON：",
                            title.chars().take(200).collect::<String>()
                        );

                        if let Ok(resp) = tokio::runtime::Handle::current().block_on(async {
                            llm.chat(vec![gmp_llm::ChatMessage {
                                role: "user".to_string(),
                                content: prompt,
                            }])
                            .await
                        }) {
                            if let Ok(cats) = serde_json::from_str::<serde_json::Value>(resp.message.content.trim()) {
                                best_topic = cats.get("topic").and_then(|v| v.as_str()).unwrap_or(best_topic);
                                best_sub = cats.get("sub_topic").and_then(|v| v.as_str()).unwrap_or(best_sub);
                                best_risk = cats.get("risk_level").and_then(|v| v.as_str()).unwrap_or(best_risk);
                                confidence = 0.8;
                            }
                        }
                    }

                    conn.execute(
                        "INSERT OR REPLACE INTO doc_taxonomy (doc_id, topic, sub_topic, doc_type, regulation, risk_level, department, confidence) VALUES (?1, ?2, ?3, NULL, NULL, ?4, ?5, ?6)",
                        params![doc_id, best_topic, best_sub, best_risk, best_topic, confidence],
                    ).ok();
                    inserted += 1;
                }

                println!("  Taxonomy complete: {} docs classified in {:.1}s", inserted, start.elapsed().as_secs_f64());
            }

            // ---- Stage 4: Relevance - Score documents ----
            if stage == "all" || stage == "relevance" {
                println!("\n[Stage 4/4] Relevance - Scoring documents...");
                let start = Instant::now();


                let docs: Vec<String> = {
                    let mut stmt = conn.prepare("SELECT doc_id FROM gmp_documents").unwrap();
                    stmt.query_map([], |r| r.get::<_, String>(0))
                        .unwrap()
                        .filter_map(Result::ok)
                        .collect()
                };

                let mut inserted = 0;
                for doc_id in docs {
                    // Calculate term density from entities
                    let entity_count: i64 = conn
                        .query_row(
                            "SELECT COUNT(*) FROM doc_entities WHERE doc_id = ?",
                            params![doc_id],
                            |r| r.get(0),
                        )
                        .unwrap_or(0);

                    let chunk_count: i64 = conn
                        .query_row(
                            "SELECT COUNT(*) FROM gmp_document_chunks WHERE doc_id = ?",
                            params![doc_id],
                            |r| r.get(0),
                        )
                        .unwrap_or(1);

                    let entity_density: f64 = entity_count as f64 / chunk_count as f64;
                    let term_density: f64 = entity_density.min(1.0);
                    let reg_citation: f64 = 0.3;
                    let process_completeness: f64 = 0.5_f64.min(entity_density * 2.0);
                    let dept_coverage: f64 = 0.4;
                    let overall: f64 = (term_density * 0.4 + reg_citation * 0.2 + process_completeness * 0.2 + entity_density * 0.2).min(1.0);

                    conn.execute(
                        "INSERT OR REPLACE INTO doc_relevance (doc_id, overall_score, term_density, reg_citation, process_completeness, entity_density, dept_coverage, confidence) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                        params![doc_id, overall, term_density, reg_citation, process_completeness, entity_density, dept_coverage, 0.7],
                    ).ok();
                    inserted += 1;
                }

                println!("  Relevance complete: {} docs scored in {:.1}s", inserted, start.elapsed().as_secs_f64());
            }

            // Final stats
            println!("\n========== Layer 0 Population Complete ==========");
            println!("  Entities:   {}", conn.query_row("SELECT COUNT(*) FROM doc_entities", [], |r| r.get::<_, i64>(0)).unwrap_or(0));
            println!("  Relations:  {}", conn.query_row("SELECT COUNT(*) FROM doc_relations", [], |r| r.get::<_, i64>(0)).unwrap_or(0));
            println!("  Taxonomy:   {}", conn.query_row("SELECT COUNT(*) FROM doc_taxonomy", [], |r| r.get::<_, i64>(0)).unwrap_or(0));
            println!("  Relevance:  {}", conn.query_row("SELECT COUNT(*) FROM doc_relevance", [], |r| r.get::<_, i64>(0)).unwrap_or(0));
            println!("================================================");
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
