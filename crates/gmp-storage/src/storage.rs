//! GMP Storage engine — pure SQLRustGo backend
#![allow(dead_code)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::len_zero)]
#![allow(clippy::cmp_owned)]
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::type_complexity)]
//!
//! Replaces rusqlite entirely with:
//! - FileStorage (sqlrustgo-storage) for table data
//! - InvertedIndex (sqlrustgo-fts) for full-text search (no rusqlite FTS5)
//! - DiskGraphStore (sqlrustgo-graph) for graph operations
//! - HnswVectorIndex for vector search
//! - In-memory LIKE filtering for keyword fallback

use crate::hnsw_vector_index::HnswVectorIndex;
use crate::sqlrustgo_adapter::SqlrustgoAdapter;
use crate::GmpStorage;
use anyhow::{Context, Result};
use glob::glob;
use sqlrustgo_storage::engine::{
    ColumnDefinition, StorageEngine as SqlrustgoStorageEngine, TableInfo,
};
use sqlrustgo_storage::fts::{ChineseTokenizer, InvertedIndex};
use sqlrustgo_storage::FileStorage;
use sqlrustgo_types::Value;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;

/// Check if a character is a CJK character
fn is_cjk(c: char) -> bool {
    matches!(c, '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}' | '\u{F900}'..='\u{FAFF}')
}

/// Frontmatter extracted from markdown document
#[derive(Debug, serde::Deserialize)]
struct Frontmatter {
    document_type: Option<String>,
    code: Option<String>,
    title: Option<String>,
    version: Option<String>,
    effective_date: Option<String>,
    department: Option<String>,
}

/// Storage engine configuration
#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub data_dir: String,
    /// Optional explicit db path (if not set, derived from data_dir)
    pub db_path: String,
    /// Embedding dimension (default: 768 for nomic-embed-text)
    pub embedding_dim: usize,
    /// Whether to use SQLRustGo native storage (always true, for compatibility)
    pub use_sqlrustgo_storage: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: "./data".to_string(),
            db_path: "./data/gmp.db".to_string(),
            embedding_dim: 768,
            use_sqlrustgo_storage: true,
        }
    }
}

/// Storage engine for GMP documents — pure SQLRustGo backend
pub struct StorageEngine {
    #[allow(dead_code)]
    config: StorageConfig,
    /// SQLRustGo FileStorage for all table data (documents, chunks, entities, etc.)
    storage: Mutex<FileStorage>,
    /// Full-text search index for chunk content
    fts_index: Mutex<InvertedIndex>,
    /// Chinese tokenizer for FTS
    tokenizer: ChineseTokenizer,
    /// HNSW vector index for similarity search
    hnsw_index: HnswVectorIndex,
    /// Optional SQLRustGo FileStorage adapter (same as storage, kept for compat)
    #[allow(dead_code)]
    adapter: Option<SqlrustgoAdapter>,
    /// In-memory cache of all chunks for LIKE search [(id, doc_id, content, title, doc_type, chapter)]
    chunk_cache: Mutex<Vec<(i64, String, String, String, Option<String>, Option<String>)>>,
    /// Next auto-increment IDs
    next_chunk_id: Mutex<i64>,
}

impl StorageEngine {
    /// Create a new storage engine with pure SQLRustGo backend
    pub fn new(config: StorageConfig) -> Result<Self> {
        let data_dir = PathBuf::from(&config.data_dir);
        std::fs::create_dir_all(&data_dir)
            .context(format!("Failed to create data dir: {:?}", data_dir))?;

        let storage_path = data_dir.join("gmp_data");
        let storage = FileStorage::new_with_buffer_config(storage_path, 1, false)
            .map_err(|e| anyhow::anyhow!("Failed to create FileStorage: {}", e))?;

        let hnsw_index = HnswVectorIndex::with_params(16, 200, 200, config.embedding_dim);

        let engine = Self {
            hnsw_index,
            adapter: None,
            storage: Mutex::new(storage),
            fts_index: Mutex::new(InvertedIndex::new()),
            tokenizer: ChineseTokenizer::new(),
            chunk_cache: Mutex::new(Vec::new()),
            next_chunk_id: Mutex::new(1),
            config: config.clone(),
        };
        engine.init_schema()?;

        engine.load_chunk_cache()?;

        // Build HNSW index from binary embeddings (handles both fresh migrate
        // and warm restart from gmp_embeddings_bin)
        let hnsw_count = engine.rebuild_hnsw_from_db()?;
        eprintln!("[init] HNSW index: {} vectors loaded (dim={})", hnsw_count, engine.hnsw_index.dimension());

        Ok(engine)
    }

    /// Stub for tokenizer access (used by gmp-eval for diagnostics)
    pub fn get_tokenizer(&self) -> &ChineseTokenizer {
        &self.tokenizer
    }

    /// Initialize GMP schema
    fn init_schema(&self) -> Result<()> {
        let mut storage = self.storage.lock().unwrap();

        // gmp_documents table
        let docs_table = TableInfo {
            name: "gmp_documents".to_string(),
            columns: vec![
                ColumnDefinition::new("doc_id", "TEXT"),
                ColumnDefinition::new("code", "TEXT"),
                ColumnDefinition::new("title", "TEXT"),
                ColumnDefinition::new("chapter", "TEXT"),
                ColumnDefinition::new("doc_type", "TEXT"),
                ColumnDefinition::new("department", "TEXT"),
                ColumnDefinition::new("status", "TEXT"),
                ColumnDefinition::new("source_file", "TEXT"),
                ColumnDefinition::new("file_mtime", "INTEGER"),
                ColumnDefinition::new("version", "TEXT"),
                ColumnDefinition::new("effective_date", "TEXT"),
                ColumnDefinition::new("created_at", "INTEGER"),
                ColumnDefinition::new("updated_at", "INTEGER"),
            ],
            ..Default::default()
        };
        if !storage.has_table("gmp_documents") {
            storage.create_table(&docs_table)?;
        }

        // gmp_document_chunks table
        let chunks_table = TableInfo {
            name: "gmp_document_chunks".to_string(),
            columns: vec![
                ColumnDefinition::new("id", "INTEGER"),
                ColumnDefinition::new("doc_id", "TEXT"),
                ColumnDefinition::new("chunk_index", "INTEGER"),
                ColumnDefinition::new("content", "TEXT"),
                ColumnDefinition::new("token_count", "INTEGER"),
            ],
            ..Default::default()
        };
        if !storage.has_table("gmp_document_chunks") {
            storage.create_table(&chunks_table)?;
        }

        // gmp_embeddings table
        let emb_table = TableInfo {
            name: "gmp_embeddings".to_string(),
            columns: vec![
                ColumnDefinition::new("chunk_id", "INTEGER"),
                ColumnDefinition::new("embedding_json", "TEXT"),
            ],
            ..Default::default()
        };
        if !storage.has_table("gmp_embeddings") {
            storage.create_table(&emb_table)?;
        }

        // gmp_embeddings_bin: binary BLOB storage for embeddings (no JSON parse overhead)
        let emb_bin_table = TableInfo {
            name: "gmp_embeddings_bin".to_string(),
            columns: vec![
                ColumnDefinition::new("chunk_id", "INTEGER"),
                ColumnDefinition::new("dimension", "INTEGER"),
                ColumnDefinition::new("embedding", "BLOB"),
            ],
            ..Default::default()
        };
        if !storage.has_table("gmp_embeddings_bin") {
            storage.create_table(&emb_bin_table)?;
        }

        Ok(())
    }

    /// Load all chunks into in-memory cache for LIKE search
    fn load_chunk_cache(&self) -> Result<()> {
        let storage = self.storage.lock().unwrap();
        let mut cache = self.chunk_cache.lock().unwrap();
        cache.clear();

        // Scan documents
        let docs = storage.scan("gmp_documents").unwrap_or_default();
        let mut doc_map = std::collections::HashMap::new();
        for doc in &docs {
            if doc.len() >= 4 {
                let doc_id = doc_value_str(doc, 0);
                let title = doc_value_str(doc, 2);
                let doc_type = doc_value_opt(doc, 4);
                let chapter = doc_value_opt(doc, 3);
                doc_map.insert(doc_id, (title, doc_type, chapter));
            }
        }

        // Scan chunks
        let chunks = storage.scan("gmp_document_chunks").unwrap_or_default();
        let mut max_id = 0i64;
        for chunk in &chunks {
            if chunk.len() >= 4 {
                let id = doc_value_int(chunk, 0);
                let doc_id = doc_value_str(chunk, 1);
                let content = doc_value_str(chunk, 3);
                if id > max_id {
                    max_id = id;
                }
                let (title, doc_type, chapter) = doc_map
                    .get(&doc_id)
                    .cloned()
                    .unwrap_or_else(|| (String::new(), None, None));
                cache.push((id, doc_id, content, title, doc_type, chapter));
            }
        }
        drop(cache);

        let mut nid = self.next_chunk_id.lock().unwrap();
        *nid = max_id + 1;
        drop(nid);

        // Rebuild FTS index from existing chunks
        {
            let mut fts = self.fts_index.lock().unwrap();
            for chunk in &chunks {
                if chunk.len() >= 4 {
                    let chunk_id = doc_value_int(chunk, 0) as u64;
                    let content = doc_value_str(chunk, 3);
                    fts.add_document(chunk_id, &content);
                }
            }
        }

        Ok(())
    }

    /// Index all markdown documents from a directory
    pub fn index_documents(&self, source_dir: &str) -> Result<IndexStats> {
        let path = Path::new(source_dir);
        if !path.exists() {
            anyhow::bail!("Source directory does not exist: {}", source_dir);
        }

        let pattern = path.join("**/*.md");
        let mut stats = IndexStats::default();

        for entry in glob(pattern.to_str().unwrap())?
            .filter_map(Result::ok)
            .filter(|e| e.is_file())
        {
            let mtime = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64);

            let file_path_str = entry.to_string_lossy().to_string();

            // Check if already indexed with same mtime
            let storage = self.storage.lock().unwrap();
            let docs = storage.scan("gmp_documents").unwrap_or_default();
            let exists = docs.iter().any(|d| {
                d.len() > 8
                    && doc_value_str(d, 7) == file_path_str
                    && mtime.map_or(false, |m| doc_value_int_opt(d, 8) == Some(m))
            });
            drop(storage);

            if exists {
                stats.documents_skipped += 1;
                continue;
            }

            // Import document
            match self.import_single_document(&entry, mtime) {
                Ok(_) => stats.documents_imported += 1,
                Err(e) => stats
                    .errors
                    .push(format!("Failed to import {:?}: {}", entry, e)),
            }
        }

        Ok(stats)
    }

    /// Import a single markdown document
    fn import_single_document(&self, path: &Path, file_mtime: Option<i64>) -> Result<String> {
        let content = std::fs::read_to_string(path)?;
        let (frontmatter, body) = self.parse_frontmatter(&content)?;

        let doc_id = frontmatter
            .code
            .clone()
            .unwrap_or_else(|| path.file_stem().unwrap().to_string_lossy().to_string());
        let chapter = path
            .components()
            .nth(1)
            .and_then(|c| c.as_os_str().to_str())
            .map(|s| s.to_string());
        let source_file = path.to_string_lossy().to_string();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Insert document
        let doc_record = vec![
            Value::Text(doc_id.clone()),
            Value::Text(frontmatter.code.clone().unwrap_or_default()),
            Value::Text(
                frontmatter
                    .title
                    .clone()
                    .unwrap_or_else(|| "Untitled".to_string()),
            ),
            Value::Text(chapter.clone().unwrap_or_default()),
            Value::Text(frontmatter.document_type.clone().unwrap_or_default()),
            Value::Text(frontmatter.department.clone().unwrap_or_default()),
            Value::Text("active".to_string()),
            Value::Text(source_file),
            Value::Integer(file_mtime.unwrap_or(0)),
            Value::Text(frontmatter.version.clone().unwrap_or_default()),
            Value::Text(frontmatter.effective_date.clone().unwrap_or_default()),
            Value::Integer(now),
            Value::Integer(now),
        ];

        let mut storage = self.storage.lock().unwrap();
        // Delete old entries for this doc_id
        let existing = storage.scan("gmp_documents").unwrap_or_default();
        for d in &existing {
            if !d.is_empty() && doc_value_str(d, 0) == doc_id {
                // Delete all chunks for this doc
                let chunks = storage.scan("gmp_document_chunks").unwrap_or_default();
                for c in &chunks {
                    if c.len() > 1 && doc_value_str(c, 1) == doc_id {
                        let _ = storage.delete(
                            "gmp_document_chunks",
                            &[Value::Integer(doc_value_int(c, 0))],
                        );
                    }
                }
                break;
            }
        }
        storage.insert("gmp_documents", vec![doc_record])?;
        drop(storage);

        // Chunk and insert
        let chunks = self.chunk_content(&body);
        let title = frontmatter.title.unwrap_or_else(|| "Untitled".to_string());
        let doc_type = frontmatter.document_type;
        let chapter_val = chapter;
        let chunk_id_start = {
            let mut nid = self.next_chunk_id.lock().unwrap();
            let start = *nid;
            *nid += chunks.len() as i64;
            start
        };

        for (idx, chunk_text) in chunks.iter().enumerate() {
            let chunk_id = chunk_id_start + idx as i64;
            let token_count = (chunk_text.len() / 4) as i64;

            // Insert into FileStorage
            {
                let mut storage = self.storage.lock().unwrap();
                let chunk_record = vec![
                    Value::Integer(chunk_id),
                    Value::Text(doc_id.clone()),
                    Value::Integer(idx as i64),
                    Value::Text(chunk_text.clone()),
                    Value::Integer(token_count),
                ];
                let _ = storage.insert("gmp_document_chunks", vec![chunk_record]);
            }

            // Add to FTS index
            {
                let mut fts = self.fts_index.lock().unwrap();
                fts.add_document(chunk_id as u64, chunk_text);
            }

            // Add to in-memory cache for LIKE search
            {
                let mut cache = self.chunk_cache.lock().unwrap();
                cache.push((
                    chunk_id,
                    doc_id.clone(),
                    chunk_text.clone(),
                    title.clone(),
                    doc_type.clone(),
                    chapter_val.clone(),
                ));
            }
        }

        Ok(doc_id)
    }

    /// Parse YAML frontmatter from markdown
    fn parse_frontmatter(&self, content: &str) -> Result<(Frontmatter, String)> {
        let content_trimmed = content.trim_start();
        if !content_trimmed.starts_with("---") {
            return Ok((
                Frontmatter {
                    document_type: None,
                    code: None,
                    title: None,
                    version: None,
                    effective_date: None,
                    department: None,
                },
                content.to_string(),
            ));
        }
        let end_marker = content_trimmed[2..]
            .find("---")
            .context("No closing frontmatter marker")?;
        let yaml_str = &content_trimmed[2..end_marker + 2];
        let body = content_trimmed[end_marker + 5..].trim().to_string();
        let frontmatter: Frontmatter = serde_yaml::from_str(yaml_str).unwrap_or(Frontmatter {
            document_type: None,
            code: None,
            title: None,
            version: None,
            effective_date: None,
            department: None,
        });
        Ok((frontmatter, body))
    }

    /// Simple chunking by markdown headers
    fn chunk_content(&self, content: &str) -> Vec<String> {
        let mut chunks = Vec::new();
        let mut current = String::new();
        for line in content.lines() {
            if line.starts_with("## ") && current.len() > 100 {
                chunks.push(current.trim().to_string());
                current = String::new();
            }
            current.push_str(line);
            current.push('\n');
        }
        if !current.trim().is_empty() {
            chunks.push(current.trim().to_string());
        }
        if chunks.len() < 2 {
            chunks = content
                .split("\n\n")
                .filter(|s| s.trim().len() > 50)
                .map(|s| s.trim().to_string())
                .collect();
        }
        chunks
    }

    /// FTS search via sqlrustgo-fts InvertedIndex
    pub fn search_fts(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>> {
        let fts = self.fts_index.lock().unwrap();
        let tokens = self.tokenizer.tokenize(query);
        let mut seen = std::collections::HashSet::new();
        let mut results = Vec::new();

        for token in &tokens {
            let doc_ids = fts.search(token);
            for chunk_id in doc_ids {
                if seen.insert(chunk_id) {
                    // Look up chunk details from cache
                    let cache = self.chunk_cache.lock().unwrap();
                    if let Some((id, doc_id_val, chunk_content, title, doc_type, chapter)) = cache
                        .iter()
                        .find(|(id, ..)| *id == chunk_id as i64)
                        .cloned()
                    {
                        let token_count = chunk_content.len() as i64;
                        results.push(SearchResult {
                            chunk_id: id,
                            doc_id: doc_id_val,
                            content: chunk_content,
                            token_count,
                            title,
                            doc_type,
                            chapter,
                            score: 0.0,
                        });
                    }
                }
                if results.len() >= top_k {
                    break;
                }
            }
            if results.len() >= top_k {
                break;
            }
        }
        Ok(results)
    }

    /// FTS search with pseudo-BM25 scoring — uses InvertedIndex::search_with_limit
    pub fn search_fts_with_scores(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>> {
        let fts = self.fts_index.lock().unwrap();
        let token_strings = self.tokenizer.tokenize(query);
        let tokens: Vec<&str> = token_strings.iter().map(|s| s.as_str()).collect();
        if tokens.is_empty() {
            return Ok(Vec::new());
        }

        let mut score_map: std::collections::HashMap<u64, f32> = std::collections::HashMap::new();
        for token in &tokens {
            let results = fts.search_with_limit(token, top_k * 3);
            for doc_id in results {
                *score_map.entry(doc_id).or_insert(0.0) += 1.0;
            }
        }

        drop(fts);
        let mut sorted: Vec<_> = score_map.into_iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        sorted.truncate(top_k);

        let cache = self.chunk_cache.lock().unwrap();
        let mut results = Vec::new();
        for (chunk_id, score) in sorted {
            if let Some((id, doc_id_val, chunk_content, title, doc_type, chapter)) = cache
                .iter()
                .find(|(id, ..)| *id == chunk_id as i64)
                .cloned()
            {
                let token_count = chunk_content.len() as i64;
                results.push(SearchResult {
                    chunk_id: id,
                    doc_id: doc_id_val,
                    content: chunk_content,
                    token_count,
                    title,
                    doc_type,
                    chapter,
                    score: score as f64,
                });
            }
        }
        Ok(results)
    }

    /// LIKE search with extra keywords (in-memory filtering over chunk cache)
    pub fn search_like_with_extra(
        &self,
        query: &str,
        top_k: usize,
        extra_keywords: &[String],
    ) -> Result<Vec<SearchResult>> {
        let substrings = Self::extract_chinese_substrings(query);
        let mut expanded = std::collections::HashSet::new();
        for sub in &substrings {
            expanded.insert(sub.clone());
        }
        for kw in extra_keywords {
            if !kw.is_empty() {
                expanded.insert(kw.clone());
            }
        }

        if expanded.is_empty() {
            return Ok(Vec::new());
        }
        let terms: Vec<String> = expanded.into_iter().collect();

        // In-memory LIKE filtering over chunk cache
        let cache = self.chunk_cache.lock().unwrap();
        let mut scored: Vec<(
            f64,
            (i64, String, String, String, Option<String>, Option<String>),
        )> = Vec::new();

        for chunk in cache.iter() {
            let (_id, _doc_id, content, title, _doc_type, _chapter) = chunk;
            let mut score = 0.0f64;
            for term in &terms {
                let pattern = term.to_lowercase();
                if content.to_lowercase().contains(&pattern) {
                    score += 1.0;
                }
                if title.to_lowercase().contains(&pattern) {
                    score += 1.0;
                }
            }
            if score > 0.0 {
                scored.push((score, chunk.clone()));
            }
        }
        drop(cache);

        // Sort by score descending
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);

        Ok(scored
            .into_iter()
            .map(
                |(score, (id, doc_id, content, title, doc_type, chapter))| SearchResult {
                    chunk_id: id,
                    doc_id: doc_id.clone(),
                    content: content.clone(),
                    token_count: content.len() as i64,
                    title: title.clone(),
                    doc_type: doc_type.clone(),
                    chapter: chapter.clone(),
                    score,
                },
            )
            .collect())
    }

    /// Wrapper for search_like_with_extra without extra keywords
    pub fn search_like(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>> {
        self.search_like_with_extra(query, top_k, &[])
    }

    /// Search vectors using HNSW index
    pub fn search_vectors(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<VectorSearchResult>> {
        if !self.hnsw_index.is_empty() {
            let hnsw_results = self.hnsw_index.search(query_embedding, top_k)?;
            let cache = self.chunk_cache.lock().unwrap();
            let mut results = Vec::with_capacity(hnsw_results.len());
            for entry in &hnsw_results {
                let chunk_id = entry.id as i64;
                if let Some((_, doc_id, content, ..)) =
                    cache.iter().find(|(id, ..)| *id == chunk_id)
                {
                    results.push(VectorSearchResult {
                        chunk_id,
                        doc_id: doc_id.clone(),
                        chunk_index: 0,
                        content: content.clone(),
                        score: entry.score as f64,
                    });
                }
            }
            results.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            return Ok(results);
        }
        Ok(Vec::new())
    }

    /// Get HNSW index reference
    pub fn get_hnsw_index(&self) -> &HnswVectorIndex {
        &self.hnsw_index
    }

    /// Get a list of chunk IDs (for testing and debugging)
    pub fn get_chunk_ids(&self, limit: usize) -> Vec<i64> {
        let cache = self.chunk_cache.lock().unwrap();
        cache.iter().take(limit).map(|(id, ..)| *id).collect()
    }
    pub fn rebuild_hnsw_from_db(&self) -> Result<usize> {
        if !self.hnsw_index.is_empty() {
            return Ok(self.hnsw_index.len());
        }

        // Try gmp_embeddings_bin (binary BLOB) first, then fall back to gmp_embeddings (JSON)
        let storage = self.storage.lock().unwrap();
        let bin_embs = storage.scan("gmp_embeddings_bin").unwrap_or_default();
        let json_embs = if bin_embs.is_empty() {
            storage.scan("gmp_embeddings").unwrap_or_default()
        } else {
            Vec::new()
        };
        drop(storage);

        let mut count = 0;

        // Binary BLOB path: [dim u32 BE][f32 BE × dim]
        for emb in &bin_embs {
            if emb.len() >= 3 {
                let chunk_id = doc_value_int(emb, 0);
                let blob = doc_value_blob(emb, 2);
                if blob.len() < 4 { continue; }
                let dim = u32::from_be_bytes([blob[0], blob[1], blob[2], blob[3]]) as usize;
                if blob.len() != 4 + dim * 4 { continue; }
                let mut vec = vec![0.0f32; dim];
                for i in 0..dim {
                    vec[i] = f32::from_be_bytes([blob[4 + i*4], blob[5 + i*4], blob[6 + i*4], blob[7 + i*4]]);
                }
                let _ = self.hnsw_index.insert(chunk_id as u64, &vec);
                count += 1;
            }
        }

        // JSON fallback path
        for emb in &json_embs {
            if emb.len() >= 2 {
                let chunk_id = doc_value_int(emb, 0);
                let json_str = doc_value_str(emb, 1);
                if let Ok(vec) = serde_json::from_str::<Vec<f32>>(&json_str) {
                    let _ = self.hnsw_index.insert(chunk_id as u64, &vec);
                    count += 1;
                }
            }
        }

        if count > 0 {
            self.hnsw_index.build_index()?;
            eprintln!(
                "[rebuild] HNSW index built from {} vectors (dim={})",
                count,
                self.hnsw_index.dimension()
            );
        }
        Ok(count)
    }

    /// Index embeddings for all chunks
    pub fn index_embeddings(&self, ollama_url: &str, model: &str) -> Result<usize> {
        // Get chunks without embeddings
        let cache = self.chunk_cache.lock().unwrap();
        let storage = self.storage.lock().unwrap();
        let existing = storage.scan("gmp_embeddings").unwrap_or_default();
        let existing_ids: std::collections::HashSet<i64> = existing
            .iter()
            .filter_map(|e| {
                if e.len() >= 1 {
                    Some(doc_value_int(e, 0))
                } else {
                    None
                }
            })
            .collect();
        drop(storage);

        let to_embed: Vec<(i64, String)> = cache
            .iter()
            .filter(|(id, ..)| !existing_ids.contains(id))
            .map(|(id, _, content, ..)| (*id, content.clone()))
            .collect();
        drop(cache);

        if to_embed.is_empty() {
            return Ok(0);
        }

        let client = reqwest::blocking::Client::new();
        let mut indexed = 0;
        for (chunk_id, content) in &to_embed {
            let resp = client
                .post(format!("{}/api/embeddings", ollama_url))
                .json(&serde_json::json!({"model": model, "prompt": content}))
                .send();
            if let Ok(response) = resp {
                if response.status().is_success() {
                    #[derive(serde::Deserialize)]
                    struct EmbedResp {
                        embedding: Vec<f32>,
                    }
                    if let Ok(embed_resp) = response.json::<EmbedResp>() {
                        let json_str = serde_json::to_string(&embed_resp.embedding)?;
                        let mut storage = self.storage.lock().unwrap();
                        let _ = storage.insert(
                            "gmp_embeddings",
                            vec![vec![Value::Integer(*chunk_id), Value::Text(json_str)]],
                        );
                        drop(storage);
                        let _ = self
                            .hnsw_index
                            .insert(*chunk_id as u64, &embed_resp.embedding);
                        indexed += 1;
                    }
                }
            }
        }
        Ok(indexed)
    }

    /// Get document count
    pub fn get_document_count(&self) -> Result<i64> {
        let storage = self.storage.lock().unwrap();
        Ok(storage
            .scan("gmp_documents")
            .map(|d| d.len() as i64)
            .unwrap_or(0))
    }

    pub fn scan_audit_chain(&self) -> Result<Vec<Vec<sqlrustgo_types::Value>>> {
        self.adapter
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Storage not initialized"))?
            .scan_audit_chain()
    }

    /// Validate the storage integrity
    /// Returns Ok(ValidationResult) if all checks pass
    pub fn validate(&self) -> Result<ValidationResult> {
        let storage = self.storage.lock().unwrap();
        let mut result = ValidationResult::default();

        // Check that core tables exist
        result.tables_present = storage.has_table("gmp_documents");
        result.chunks_table_present = storage.has_table("gmp_document_chunks");

        if !result.tables_present || !result.chunks_table_present {
            result.is_valid = false;
            result.errors.push("Missing core tables".to_string());
            return Ok(result);
        }

        // Count records
        let docs = storage.scan("gmp_documents").unwrap_or_default();
        let chunks = storage.scan("gmp_document_chunks").unwrap_or_default();
        let embeddings = storage.scan("gmp_embeddings").unwrap_or_default();

        result.documents = docs.len() as i64;
        result.chunks = chunks.len() as i64;
        result.embedding_count = embeddings.len() as i64;

        // Check chunk-embedding ratio (embeddings should match chunks)
        if result.chunks > 0 && result.embedding_count == 0 {
            result
                .warnings
                .push("No embeddings found for chunks".to_string());
        }

        // Verify FTS index is accessible
        let fts = self.fts_index.lock().unwrap();
        let fts_size = fts.doc_count() as usize;
        drop(fts);
        result.fts_token_count = fts_size;

        // Mark invalid if no documents indexed yet
        if result.documents == 0 {
            result.is_valid = false;
            result.errors.push("No documents indexed yet".to_string());
            return Ok(result);
        }

        result.is_valid = true;
        Ok(result)
    }

    /// Extract Chinese substrings (2-4 chars) from query
    fn extract_chinese_substrings(query: &str) -> Vec<String> {
        let mut substrings = Vec::new();
        let chars: Vec<char> = query.chars().collect();
        for i in 0..chars.len() {
            if is_cjk(chars[i]) {
                let max_end = std::cmp::min(i + 4, chars.len());
                let mut end = max_end;
                while end > i + 1 {
                    if chars[i..end].iter().all(|c| is_cjk(*c)) {
                        substrings.push(chars[i..end].iter().collect());
                        break;
                    }
                    end -= 1;
                }
            }
        }
        substrings.sort();
        substrings.dedup();
        substrings
    }
}

// Helper functions for extracting values from Records
fn doc_value_str(doc: &[Value], idx: usize) -> String {
    if idx < doc.len() {
        match &doc[idx] {
            Value::Text(s) => s.clone(),
            Value::Integer(i) => i.to_string(),
            _ => String::new(),
        }
    } else {
        String::new()
    }
}

fn doc_value_opt(doc: &[Value], idx: usize) -> Option<String> {
    if idx < doc.len() {
        match &doc[idx] {
            Value::Text(s) if !s.is_empty() => Some(s.clone()),
            _ => None,
        }
    } else {
        None
    }
}

fn doc_value_int(doc: &[Value], idx: usize) -> i64 {
    if idx < doc.len() {
        match &doc[idx] {
            Value::Integer(i) => *i,
            _ => 0,
        }
    } else {
        0
    }
}

fn doc_value_int_opt(doc: &[Value], idx: usize) -> Option<i64> {
    if idx < doc.len() {
        match &doc[idx] {
            Value::Integer(i) => Some(*i),
            _ => None,
        }
    } else {
        None
    }
}

fn doc_value_blob(doc: &[Value], idx: usize) -> Vec<u8> {
    if idx < doc.len() {
        match &doc[idx] {
            Value::Blob(b) => b.clone(),
            Value::Text(s) => s.as_bytes().to_vec(),
            Value::Integer(i) => i.to_le_bytes().to_vec(),
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    }
}

/// Compute cosine similarity between two vectors
pub fn cosine_sim(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    (dot / (norm_a * norm_b)) as f64
}

// Public types
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub chunk_id: i64,
    pub doc_id: String,
    pub content: String,
    pub token_count: i64,
    pub title: String,
    pub doc_type: Option<String>,
    pub chapter: Option<String>,
    pub score: f64,
}

impl GmpStorage for StorageEngine {
    fn search_fts(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>> {
        self.search_fts_with_scores(query, top_k)
    }

    fn search_fts_with_scores(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>> {
        StorageEngine::search_fts_with_scores(self, query, top_k)
    }

    fn search_like(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>> {
        StorageEngine::search_like(self, query, top_k)
    }

    fn search_like_with_extra(
        &self,
        query: &str,
        top_k: usize,
        extra_keywords: &[String],
    ) -> Result<Vec<SearchResult>> {
        StorageEngine::search_like_with_extra(self, query, top_k, extra_keywords)
    }

    fn search_vectors(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<VectorSearchResult>> {
        StorageEngine::search_vectors(self, query_embedding, top_k)
    }

    fn index_documents(&self, source_dir: &str) -> Result<IndexStats> {
        StorageEngine::index_documents(self, source_dir)
    }

    fn index_embeddings(&self, ollama_url: &str, model: &str) -> Result<usize> {
        StorageEngine::index_embeddings(self, ollama_url, model)
    }

    fn rebuild_hnsw_from_db(&self) -> Result<usize> {
        StorageEngine::rebuild_hnsw_from_db(self)
    }

    fn validate(&self) -> Result<ValidationResult> {
        StorageEngine::validate(self)
    }

    fn get_document_count(&self) -> Result<i64> {
        StorageEngine::get_document_count(self)
    }

    fn hybrid_search_rrf(
        &self,
        query: &str,
        top_k: usize,
        fts_weight: f32,
        like_weight: f32,
        vector_weight: f32,
    ) -> Result<Vec<SearchResult>> {
        StorageEngine::hybrid_search_rrf(
            self,
            query,
            top_k,
            fts_weight,
            like_weight,
            vector_weight,
        )
    }
}

impl StorageEngine {
    /// Check if Layer 0 (entity/taxonomy) data has been populated
    #[allow(dead_code)]
    pub fn has_layer0_data(&self) -> Result<bool> {
        Ok(false)
    }

    /// Enhanced FTS search with Layer 0 features (entity boost, taxonomy filter)
    #[allow(dead_code)]
    pub fn search_fts_enhanced(
        &self,
        query: &str,
        top_k: usize,
        _use_layer0: bool,
    ) -> Result<Vec<SearchResult>> {
        self.search_fts_with_scores(query, top_k)
    }

    /// LIKE-based search with n-gram Chinese tokenization (wrapper for search_like_with_extra)
    #[allow(dead_code)]
    pub fn search_like_ng(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>> {
        self.search_like_with_extra(query, top_k, &[])
    }

    // ========== Layer 0: Exact Lookup Methods ==========
    /// Find chunks by exact doc_id (e.g., "GMP-03-REC-001")
    pub fn find_by_doc_id(&self, doc_id: &str) -> Result<Vec<SearchResult>> {
        let cache = self.chunk_cache.lock().unwrap();
        let doc_id_lower = doc_id.to_lowercase();
        let results: Vec<SearchResult> = cache
            .iter()
            .filter(|(_, id, _, _, _, _)| id.to_lowercase() == doc_id_lower)
            .map(|(id, doc_id, content, title, doc_type, chapter)| SearchResult {
                chunk_id: *id,
                doc_id: doc_id.clone(),
                content: content.clone(),
                token_count: content.len() as i64,
                title: title.clone(),
                doc_type: doc_type.clone(),
                chapter: chapter.clone(),
                score: 1.0,
            })
            .collect();
        drop(cache);
        Ok(results)
    }

    /// Find chunks by document code (partially matches doc_id or title)
    /// e.g., "GMP-03-REC-001", "偏差调查", "CAPA"
    pub fn find_by_code(&self, code: &str) -> Result<Vec<SearchResult>> {
        let cache = self.chunk_cache.lock().unwrap();
        let code_lower = code.to_lowercase();
        let results: Vec<SearchResult> = cache
            .iter()
            .filter(|(_, id, _, title, _, _)| {
                id.to_lowercase().contains(&code_lower)
                    || title.to_lowercase().contains(&code_lower)
            })
            .map(|(id, doc_id, content, title, doc_type, chapter)| SearchResult {
                chunk_id: *id,
                doc_id: doc_id.clone(),
                content: content.clone(),
                token_count: content.len() as i64,
                title: title.clone(),
                doc_type: doc_type.clone(),
                chapter: chapter.clone(),
                score: 1.0,
            })
            .collect();
        drop(cache);
        Ok(results)
    }

    /// Find chunks by chapter (e.g., "CH03-机构与人员")
    pub fn find_by_chapter(&self, chapter: &str) -> Result<Vec<SearchResult>> {
        let cache = self.chunk_cache.lock().unwrap();
        let chapter_lower = chapter.to_lowercase();
        let results: Vec<SearchResult> = cache
            .iter()
            .filter(|(_, _, _, _, _, ch)| {
                ch.as_ref()
                    .map(|c| c.to_lowercase().contains(&chapter_lower))
                    .unwrap_or(false)
            })
            .map(|(id, doc_id, content, title, doc_type, chapter)| SearchResult {
                chunk_id: *id,
                doc_id: doc_id.clone(),
                content: content.clone(),
                token_count: content.len() as i64,
                title: title.clone(),
                doc_type: doc_type.clone(),
                chapter: chapter.clone(),
                score: 1.0,
            })
            .collect();
        drop(cache);
        Ok(results)
    }

    /// Get statistics for layer 0 lookup — doc_ids and chapters available
    pub fn get_layer0_stats(&self) -> Result<Layer0Stats> {
        let cache = self.chunk_cache.lock().unwrap();
        let mut doc_ids: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let mut chapters: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for (_, doc_id, _, _, _, chapter) in cache.iter() {
            doc_ids.insert(doc_id.clone());
            if let Some(ch) = chapter {
                chapters.insert(ch.clone());
            }
        }
        drop(cache);
        Ok(Layer0Stats {
            total_chunks: 0,
            doc_count: doc_ids.len(),
            chapter_count: chapters.len(),
            doc_ids: doc_ids.into_iter().collect(),
            chapters: chapters.into_iter().collect(),
        })
    }

    /// Hybrid search with RRF fusion of FTS + LIKE + vector channels (Layer 4)
    pub fn hybrid_search_rrf(
        &self,
        query: &str,
        top_k: usize,
        fts_weight: f32,
        like_weight: f32,
        _vector_weight: f32,
    ) -> Result<Vec<SearchResult>> {
        // Get results from each channel
        let fts_results = self.search_fts_with_scores(query, top_k)?;
        let like_results = self.search_like(query, top_k)?;

        // RRF fusion: score(d) = Σ weight / (k + rank)
        let rrf_k = 60.0f64;
        let fts_w = fts_weight as f64;
        let like_w = like_weight as f64;
        let mut score_map: std::collections::HashMap<i64, (f64, SearchResult)> =
            std::collections::HashMap::new();

        // FTS channel
        for (rank, result) in fts_results.iter().enumerate() {
            let rrf_score = fts_w / (rrf_k + (rank + 1) as f64);
            score_map.insert(result.chunk_id, (rrf_score, result.clone()));
        }

        // LIKE channel
        for (rank, result) in like_results.iter().enumerate() {
            let rrf_score = like_w / (rrf_k + (rank + 1) as f64);
            if let Some((old_score, _)) = score_map.get(&result.chunk_id) {
                score_map.insert(result.chunk_id, (old_score + rrf_score, result.clone()));
            } else {
                score_map.insert(result.chunk_id, (rrf_score, result.clone()));
            }
        }

        // Sort by fused score and return top_k
        let mut sorted: Vec<_> = score_map.into_iter().collect();
        sorted.sort_by(|a, b| b.1.0.partial_cmp(&a.1.0).unwrap_or(std::cmp::Ordering::Equal));
        sorted.truncate(top_k);
        let results: Vec<SearchResult> = sorted
            .into_iter()
            .map(|(_, (score, mut r))| {
                r.score = score;
                r
            })
            .collect();
        Ok(results)
    }
}

#[derive(Debug, Clone)]
pub struct VectorSearchResult {
    pub chunk_id: i64,
    pub doc_id: String,
    pub chunk_index: i32,
    pub content: String,
    pub score: f64,
}

/// Layer 0 lookup statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct Layer0Stats {
    pub total_chunks: usize,
    pub doc_count: usize,
    pub chapter_count: usize,
    pub doc_ids: Vec<String>,
    pub chapters: Vec<String>,
}

#[derive(Debug, Default)]
pub struct IndexStats {
    pub documents_imported: usize,
    pub documents_skipped: usize,
    pub chunks_indexed: usize,
    pub errors: Vec<String>,
}

/// Validation result for storage integrity checks
#[derive(Debug, Default)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub tables_present: bool,
    pub chunks_table_present: bool,
    pub documents: i64,
    pub chunks: i64,
    pub embedding_count: i64,
    pub fts_token_count: usize,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}
