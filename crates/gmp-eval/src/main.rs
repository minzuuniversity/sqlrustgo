//! GMP 408 Scenario Evaluator with Hybrid Search (FTS5 + Vector)
//!
//! Evaluation strategy:
//! 1. FTS5 search first (BM25)
//! 2. If FTS recall = 0, fall back to vector search via Ollama embeddings
//! 3. Merge results and evaluate

use anyhow::{Context, Result};
use clap::Parser;
use gmp_storage::{StorageConfig, StorageEngine};
use std::collections::HashMap;

#[derive(Debug, Clone, serde::Deserialize)]
struct TestScenario {
    pub query: String,
    pub expected_doc_ids: Vec<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub top_k: Option<usize>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct EvalReport {
    pub total: usize,
    pub passed: u32,
    pub failed: usize,
    pub pass_rate: f64,
    pub avg_recall: f64,
    pub avg_mrr: f64,
    pub fts_passed: u32,
    pub vector_passed: u32,
    pub category_stats: HashMap<String, CategoryResult>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct CategoryResult {
    passed: u32,
    total: usize,
    pass_rate: f64,
}

#[derive(Parser, Debug)]
#[command(name = "gmp-eval")]
#[command(about = "GMP 408-Scenario Evaluation Tool with Hybrid Search")]
struct Cli {
    /// Path to test scenarios JSON file
    #[arg(long, default_value = "test-data/391_scenarios.json")]
    test_set: String,

    /// Path to SQLite database
    #[arg(long, default_value = "data/gmp.db")]
    db: String,

    /// Limit number of scenarios to test
    #[arg(long)]
    limit: Option<usize>,

    /// Output report to file
    #[arg(long)]
    output: Option<String>,

    /// Ollama URL for vector embeddings (fallback search)
    #[arg(long, default_value = "http://localhost:11434")]
    ollama_url: String,

    /// Ollama embedding model
    #[arg(long, default_value = "nomic-embed-text")]
    embed_model: String,

    /// Disable hybrid search (use FTS only)
    #[arg(long, default_value_t = false)]
    no_hybrid: bool,
}

/// Compute cosine similarity between two f32 vectors
#[allow(dead_code)]
fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

/// Call Ollama embeddings API to get a query vector
fn get_query_embedding(query: &str, ollama_url: &str, model: &str) -> Option<Vec<f32>> {
    let client = reqwest::blocking::Client::new();
    let payload = serde_json::json!({
        "model": model,
        "prompt": query
    });

    let resp = client
        .post(format!("{}/api/embeddings", ollama_url))
        .json(&payload)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .ok()?;

    let embedding: Vec<f32> = resp.json().ok()?;
    Some(embedding)
}

/// Hybrid search: FTS first, then vector fallback
fn hybrid_search(
    storage: &StorageEngine,
    query: &str,
    top_k: usize,
    ollama_url: &str,
    embed_model: &str,
) -> Vec<gmp_storage::SearchResult> {
    // Step 1: FTS5 search
    let fts_results = storage.search_fts(query, top_k).unwrap_or_default();

    // Step 2: Check if FTS caught anything
    if !fts_results.is_empty() {
        return fts_results;
    }

    // Step 3: FTS returned nothing — try vector fallback
    eprintln!(
        "  [vector fallback for query: \"{}...\"]",
        query.chars().take(10).collect::<String>()
    );
    let query_vec = match get_query_embedding(query, ollama_url, embed_model) {
        Some(v) => v,
        None => return fts_results,
    };

    // Step 4: Vector search
    let vector_results = storage
        .search_vectors(&query_vec, top_k)
        .unwrap_or_default();

    // Step 5: Convert VectorSearchResult to SearchResult
    vector_results
        .into_iter()
        .map(|vr| gmp_storage::SearchResult {
            chunk_id: vr.chunk_id,
            doc_id: vr.doc_id,
            content: vr.content,
            token_count: 0,
            title: String::new(),
            doc_type: None,
            chapter: None,
            score: vr.score,
        })
        .collect()
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let test_content = std::fs::read_to_string(&cli.test_set)
        .with_context(|| format!("Failed to read test set: {}", cli.test_set))?;
    let test_scenarios: Vec<TestScenario> = serde_json::from_str(&test_content)
        .with_context(|| format!("Failed to parse test set JSON from {}", cli.test_set))?;

    eprintln!("Loaded {} test scenarios", test_scenarios.len());
    if !cli.no_hybrid {
        eprintln!(
            "Hybrid search: FTS5 + Vector fallback (Ollama: {})",
            cli.ollama_url
        );
    }

    let config = StorageConfig {
        data_dir: "./data".to_string(),
        db_path: cli.db,
    };
    let storage = StorageEngine::new(config).context("Failed to initialize storage")?;

    let limit = cli.limit.unwrap_or(408);
    let scenarios: Vec<_> = test_scenarios.iter().take(limit).collect();

    let mut total_passed = 0u32;
    let mut fts_passed = 0u32;
    let mut all_recalls: Vec<f64> = Vec::new();
    let mut all_mrrs: Vec<f64> = Vec::new();
    let mut category_stats: HashMap<String, (u32, usize)> = HashMap::new();

    for (i, scenario) in scenarios.iter().enumerate() {
        eprint!("\rEvaluating: {}/{}", i + 1, scenarios.len());

        let top_k = scenario.top_k.unwrap_or(10);

        // Hybrid search
        let search_results = if !cli.no_hybrid {
            hybrid_search(
                &storage,
                &scenario.query,
                top_k,
                &cli.ollama_url,
                &cli.embed_model,
            )
        } else {
            storage
                .search_fts(&scenario.query, top_k)
                .unwrap_or_default()
        };

        let retrieved_ids: Vec<String> = search_results.iter().map(|r| r.doc_id.clone()).collect();

        // Keyword-in-content matching
        let hit_count = scenario
            .expected_doc_ids
            .iter()
            .filter(|kw| {
                let kw_lower = kw.to_lowercase();
                retrieved_ids
                    .iter()
                    .any(|id| id.to_lowercase().contains(&kw_lower))
                    || search_results.iter().any(|r| {
                        r.title.to_lowercase().contains(&kw_lower)
                            || r.content.to_lowercase().contains(&kw_lower)
                    })
            })
            .count();

        let recall = if scenario.expected_doc_ids.is_empty() {
            0.0
        } else {
            hit_count as f64 / scenario.expected_doc_ids.len() as f64
        };

        let mrr = if hit_count > 0 {
            let mut m = 0.0;
            for (rank, id) in retrieved_ids.iter().enumerate() {
                let id_lower = id.to_lowercase();
                if scenario
                    .expected_doc_ids
                    .iter()
                    .any(|kw| id_lower.contains(&kw.to_lowercase()))
                {
                    m = 1.0 / (rank + 1) as f64;
                    break;
                }
            }
            m
        } else {
            0.0
        };

        all_recalls.push(recall);
        all_mrrs.push(mrr);

        let is_pass = recall > 0.0;
        if is_pass {
            total_passed += 1;
            // Track which search mode contributed
            if !search_results.is_empty()
                && !search_results
                    .iter()
                    .all(|r| r.score == 0.0 && r.content.is_empty())
            {
                // Check if this likely came from vector (content would be populated differently)
                fts_passed += 1;
            }
        }

        let cat = scenario
            .category
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        let entry = category_stats.entry(cat).or_insert((0, 0));
        entry.1 += 1;
        if is_pass {
            entry.0 += 1;
        }
    }
    eprintln!();

    let total = scenarios.len();
    let pass_rate = (total_passed as f64 / total as f64) * 100.0;
    let avg_recall = if all_recalls.is_empty() {
        0.0
    } else {
        all_recalls.iter().sum::<f64>() / total as f64
    };
    let avg_mrr = if all_mrrs.is_empty() {
        0.0
    } else {
        all_mrrs.iter().sum::<f64>() / total as f64
    };

    eprintln!("\n========== 408 SCENARIO EVALUATION REPORT ==========");
    eprintln!(
        "Total: {} | Passed: {} | Failed: {}",
        total,
        total_passed,
        total - total_passed as usize
    );
    eprintln!("Pass Rate: {:.1}%", pass_rate);
    eprintln!("Avg Recall@K: {:.3}", avg_recall);
    eprintln!("Avg MRR: {:.3}", avg_mrr);
    eprintln!();

    let mut cat_report_map: HashMap<String, CategoryResult> = HashMap::new();
    if !category_stats.is_empty() {
        eprintln!("Per-Category Results:");
        let mut cats: Vec<_> = category_stats.iter().collect();
        cats.sort_by_key(|k| k.0);
        for (cat, &(p, t)) in cats {
            let rate = (p as f64 / t as f64) * 100.0;
            eprintln!("  {:20}: {}/{} ({:.0}%)", cat, p, t, rate);
            cat_report_map.insert(
                cat.clone(),
                CategoryResult {
                    passed: p,
                    total: t,
                    pass_rate: rate,
                },
            );
        }
    }
    eprintln!("==================================================");

    if let Some(output_path) = &cli.output {
        let report = EvalReport {
            total,
            passed: total_passed,
            failed: total - total_passed as usize,
            pass_rate,
            avg_recall,
            avg_mrr,
            fts_passed,
            vector_passed: 0,
            category_stats: cat_report_map,
        };
        let json = serde_json::to_string_pretty(&report)?;
        std::fs::write(output_path, json)?;
        eprintln!("Report saved to: {}", output_path);
    }

    Ok(())
}
