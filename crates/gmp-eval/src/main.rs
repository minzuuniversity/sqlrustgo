//! GMP 408 Scenario Evaluator — Detailed Per-Scenario Report
//! Outputs every query, retrieved docs, expected keywords, and pass/fail for manual review

use anyhow::{Context, Result};
use clap::Parser;
use gmp_storage::{StorageConfig, StorageEngine};
use std::collections::HashMap;

#[derive(Debug, Clone, serde::Deserialize)]
struct TestScenario {
    pub query: String,
    pub expected_doc_ids: Vec<String>, // keywords to match in content
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub top_k: Option<usize>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct ScenarioResult {
    pub index: usize,
    pub category: String,
    pub query: String,
    pub expected_keywords: Vec<String>,
    pub top_k: usize,
    pub retrieved: Vec<RetrievedDoc>,
    pub hit_keywords: Vec<String>,    // which expected kw were found
    pub missed_keywords: Vec<String>, // which expected kw were NOT found
    pub recall: f64,
    pub mrr: f64,
    pub is_pass: bool,
    pub fail_reason: String,
    pub search_mode: String, // "fts" or "vector_fallback"
}

#[derive(Debug, Clone, serde::Serialize)]
struct RetrievedDoc {
    pub rank: usize,
    pub doc_id: String,
    pub title: String,
    pub doc_type: Option<String>,
    pub chapter: Option<String>,
    pub score: f64,
    pub content_snippet: String,       // first 200 chars for review
    pub matched_keywords: Vec<String>, // which expected kw appear in this doc
}

#[derive(Debug, Clone, serde::Serialize)]
struct EvalReport {
    total: usize,
    passed: usize,
    failed: usize,
    pass_rate: f64,
    avg_recall: f64,
    avg_mrr: f64,
    fts_passed: usize,
    vector_passed: usize,
    category_stats: HashMap<String, CategoryStats>,
    scenario_results: Vec<ScenarioResult>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct CategoryStats {
    passed: usize,
    total: usize,
    pass_rate: f64,
    failed_indices: Vec<usize>,
}

#[derive(Parser, Debug)]
#[command(name = "gmp-eval")]
#[command(about = "GMP 408-Scenario Evaluation Tool — Detailed Report")]
struct Cli {
    #[arg(long, default_value = "test-data/391_scenarios.json")]
    test_set: String,
    #[arg(long, default_value = "data/gmp.db")]
    db: String,
    #[arg(long)]
    limit: Option<usize>,
    #[arg(long)]
    output: Option<String>,
    #[arg(long, default_value = "http://localhost:11434")]
    ollama_url: String,
    #[arg(long, default_value = "nomic-embed-text")]
    embed_model: String,
    #[arg(long, default_value_t = false)]
    no_hybrid: bool,
    /// Show only failed scenarios in detailed output
    #[arg(long, default_value_t = false)]
    verbose: bool,
}

fn get_query_embedding(query: &str, ollama_url: &str, model: &str) -> Option<Vec<f32>> {
    let client = reqwest::blocking::Client::new();
    let payload = serde_json::json!({"model": model, "prompt": query});
    let resp = client
        .post(format!("{}/api/embeddings", ollama_url))
        .json(&payload)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .ok()?;
    let embedding: Vec<f32> = resp.json().ok()?;
    Some(embedding)
}

fn hybrid_search(
    storage: &StorageEngine,
    query: &str,
    top_k: usize,
    ollama_url: &str,
    embed_model: &str,
) -> (Vec<gmp_storage::SearchResult>, String) {
    // Step 1: FTS with BM25 scoring
    let fts_results = storage
        .search_fts_with_scores(query, top_k)
        .unwrap_or_default();

    if !fts_results.is_empty() {
        return (fts_results, "fts".to_string());
    }

    // Step 2: Vector fallback
    let query_vec = match get_query_embedding(query, ollama_url, embed_model) {
        Some(v) => v,
        None => return (fts_results, "fts".to_string()),
    };

    let vector_results = storage
        .search_vectors(&query_vec, top_k)
        .unwrap_or_default();
    let results = vector_results
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
        .collect();

    (results, "vector_fallback".to_string())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let test_content = std::fs::read_to_string(&cli.test_set)
        .with_context(|| format!("Failed to read test set: {}", cli.test_set))?;
    let test_scenarios: Vec<TestScenario> = serde_json::from_str(&test_content)
        .with_context(|| format!("Failed to parse test set JSON from {}", cli.test_set))?;

    eprintln!("Loaded {} test scenarios", test_scenarios.len());

    let config = StorageConfig {
        data_dir: "./data".to_string(),
        db_path: cli.db,
        embedding_dim: 1024,
        use_sqlrustgo_storage: true,
    };
    let storage = StorageEngine::new(config).context("Failed to initialize storage")?;

    let limit = cli.limit.unwrap_or(408);
    let scenarios: Vec<_> = test_scenarios.iter().take(limit).collect();

    let mut total_passed = 0usize;
    let mut fts_passed = 0usize;
    let mut vector_passed = 0usize;
    let mut all_recalls: Vec<f64> = Vec::new();
    let mut all_mrrs: Vec<f64> = Vec::new();
    let mut category_stats: HashMap<String, (usize, usize, Vec<usize>)> = HashMap::new();
    let mut all_results: Vec<ScenarioResult> = Vec::new();

    for (i, scenario) in scenarios.iter().enumerate() {
        eprint!("\rEvaluating: {}/{}", i + 1, scenarios.len());

        let top_k = scenario.top_k.unwrap_or(10);
        let category = scenario
            .category
            .clone()
            .unwrap_or_else(|| "unknown".to_string());

        // Hybrid search
        let (search_results, search_mode) = if !cli.no_hybrid {
            hybrid_search(
                &storage,
                &scenario.query,
                top_k,
                &cli.ollama_url,
                &cli.embed_model,
            )
        } else {
            (
                storage
                    .search_fts_with_scores(&scenario.query, top_k)
                    .unwrap_or_default(),
                "fts_only".to_string(),
            )
        };

        let retrieved_ids: Vec<String> = search_results.iter().map(|r| r.doc_id.clone()).collect();

        // Check which expected keywords appear in retrieved docs
        let mut hit_keywords: Vec<String> = Vec::new();
        let mut missed_keywords: Vec<String> = Vec::new();

        for kw in &scenario.expected_doc_ids {
            let kw_lower = kw.to_lowercase();
            let found = retrieved_ids
                .iter()
                .any(|id| id.to_lowercase().contains(&kw_lower))
                || search_results.iter().any(|r| {
                    r.title.to_lowercase().contains(&kw_lower)
                        || r.content.to_lowercase().contains(&kw_lower)
                });
            if found {
                hit_keywords.push(kw.clone());
            } else {
                missed_keywords.push(kw.clone());
            }
        }

        let hit_count = hit_keywords.len();
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
        let fail_reason = if scenario.expected_doc_ids.is_empty() {
            "No expected keywords defined".to_string()
        } else if search_results.is_empty() {
            "No documents retrieved".to_string()
        } else if hit_count == 0 {
            format!(
                "None of the expected keywords found in top-{}: {:?}",
                top_k, scenario.expected_doc_ids
            )
        } else {
            format!(
                "Partial recall: {}/{} keywords found ({:.0}%)",
                hit_count,
                scenario.expected_doc_ids.len(),
                recall * 100.0
            )
        };

        if is_pass {
            total_passed += 1;
            if search_mode.starts_with("fts") {
                fts_passed += 1;
            } else {
                vector_passed += 1;
            }
        }

        // Build retrieved doc list with matched keywords per doc
        let retrieved: Vec<RetrievedDoc> = search_results
            .iter()
            .enumerate()
            .map(|(rank, r)| {
                let doc_kw_lower = r.doc_id.to_lowercase();
                let title_lower = r.title.to_lowercase();
                let content_lower = r.content.to_lowercase();
                let matched: Vec<String> = scenario
                    .expected_doc_ids
                    .iter()
                    .filter(|kw| {
                        let kw_l = kw.to_lowercase();
                        doc_kw_lower.contains(&kw_l)
                            || title_lower.contains(&kw_l)
                            || content_lower.contains(&kw_l)
                    })
                    .cloned()
                    .collect();

                RetrievedDoc {
                    rank: rank + 1,
                    doc_id: r.doc_id.clone(),
                    title: r.title.clone(),
                    doc_type: r.doc_type.clone(),
                    chapter: r.chapter.clone(),
                    score: r.score,
                    content_snippet: r
                        .content
                        .chars()
                        .take(200)
                        .collect::<String>()
                        .replace('\n', " ")
                        .replace('\r', ""),
                    matched_keywords: matched,
                }
            })
            .collect();

        let result = ScenarioResult {
            index: i,
            category: category.clone(),
            query: scenario.query.clone(),
            expected_keywords: scenario.expected_doc_ids.clone(),
            top_k,
            retrieved,
            hit_keywords,
            missed_keywords,
            recall,
            mrr,
            is_pass,
            fail_reason,
            search_mode: search_mode.clone(),
        };

        all_results.push(result);

        // Category tracking
        let entry = category_stats
            .entry(category.clone())
            .or_insert((0, 0, Vec::new()));
        entry.1 += 1;
        if is_pass {
            entry.0 += 1;
        } else {
            entry.2.push(i);
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
        total - total_passed
    );
    eprintln!("Pass Rate: {:.1}%", pass_rate);
    eprintln!("Avg Recall@K: {:.3}", avg_recall);
    eprintln!("Avg MRR: {:.3}", avg_mrr);
    eprintln!(
        "FTS passed: {} | Vector fallback: {}",
        fts_passed, vector_passed
    );
    eprintln!();

    eprintln!("Per-Category Results:");
    let mut cats: Vec<_> = category_stats.iter().collect();
    cats.sort_by_key(|k| k.0);
    for (cat, &(p, t, _)) in cats {
        let rate = (p as f64 / t as f64) * 100.0;
        let mark = if p == t {
            "✅"
        } else if p >= t / 2 {
            "⚠️"
        } else {
            "❌"
        };
        eprintln!("  {} {:20}: {}/{} ({:.0}%)", mark, cat, p, t, rate);
    }
    eprintln!("==================================================");

    // Build category stats for JSON
    let mut cat_report_map: HashMap<String, CategoryStats> = HashMap::new();
    for (cat, &(p, t, ref failed_indices)) in &category_stats {
        cat_report_map.insert(
            cat.clone(),
            CategoryStats {
                passed: p,
                total: t,
                pass_rate: (p as f64 / t as f64) * 100.0,
                failed_indices: failed_indices.clone(),
            },
        );
    }

    let report = EvalReport {
        total,
        passed: total_passed,
        failed: total - total_passed,
        pass_rate,
        avg_recall,
        avg_mrr,
        fts_passed,
        vector_passed,
        category_stats: cat_report_map,
        scenario_results: all_results,
    };

    if let Some(output_path) = &cli.output {
        let json = serde_json::to_string_pretty(&report).context("Serialize report")?;
        std::fs::write(output_path, json).context("Write report file")?;
        eprintln!("Detailed report saved to: {}", output_path);
    }

    Ok(())
}
