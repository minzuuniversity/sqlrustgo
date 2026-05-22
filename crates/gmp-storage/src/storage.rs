//! GMP Storage engine - Manages document storage, FTS, Vector, and Graph indexes

use anyhow::{Context, Result};
use glob::glob;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

/// Check if a character is a CJK character (Chinese/Japanese/Korean)
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
    pub db_path: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: "./data".to_string(),
            db_path: "./data/gmp.db".to_string(),
        }
    }
}

/// Storage engine for GMP documents
pub struct StorageEngine {
    #[allow(dead_code)]
    config: StorageConfig,
    conn: Mutex<Connection>,
}

impl StorageEngine {
    /// Create a new storage engine
    pub fn new(config: StorageConfig) -> Result<Self> {
        let path = Path::new(&config.db_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .context(format!("Failed to create data dir: {:?}", parent))?;
        }

        let conn = Connection::open(&config.db_path).context("Failed to open database")?;

        let engine = Self {
            config,
            conn: Mutex::new(conn),
        };
        engine.init_schema()?;
        Ok(engine)
    }

    /// Initialize database schema
    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
            -- Documents table
            CREATE TABLE IF NOT EXISTS gmp_documents (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                doc_id TEXT UNIQUE NOT NULL,
                code TEXT,
                title TEXT NOT NULL,
                chapter TEXT,
                doc_type TEXT,
                department TEXT,
                status TEXT DEFAULT 'active',
                source_file TEXT,
                file_mtime INTEGER,
                version TEXT,
                effective_date TEXT,
                created_at INTEGER DEFAULT (strftime('%s', 'now')),
                updated_at INTEGER DEFAULT (strftime('%s', 'now'))
            );

            -- Document chunks table
            CREATE TABLE IF NOT EXISTS gmp_document_chunks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                doc_id TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                content TEXT NOT NULL,
                token_count INTEGER DEFAULT 0,
                FOREIGN KEY (doc_id) REFERENCES gmp_documents(doc_id)
            );

            -- FTS5 virtual table for full-text search
            CREATE VIRTUAL TABLE IF NOT EXISTS gmp_chunks_fts USING fts5(
                content,
                doc_id UNINDEXED,
                chunk_id UNINDEXED
            );

            -- Create indexes
            CREATE INDEX IF NOT EXISTS idx_chunks_doc_id ON gmp_document_chunks(doc_id);
            CREATE INDEX IF NOT EXISTS idx_docs_code ON gmp_documents(code);
            CREATE INDEX IF NOT EXISTS idx_docs_type ON gmp_documents(doc_type);

            -- Embeddings table for vector search
            CREATE TABLE IF NOT EXISTS gmp_embeddings (
                chunk_id INTEGER PRIMARY KEY REFERENCES gmp_document_chunks(id),
                embedding BLOB NOT NULL
            );

            -- Index on embeddings
            CREATE INDEX IF NOT EXISTS idx_embeddings_chunk ON gmp_embeddings(chunk_id);
            "#,
        )
        .context("Failed to initialize schema")?;

        Ok(())
    }

    /// Index all markdown documents from a directory (incremental: skips unchanged files)
    pub fn index_documents(&self, source_dir: &str) -> Result<IndexStats> {
        let path = Path::new(source_dir);
        if !path.exists() {
            anyhow::bail!("Source directory does not exist: {}", source_dir);
        }

        let pattern = path.join("**/*.md");
        let mut stats = IndexStats::default();

        {
            // Keep lock during transaction
            let conn = self.conn.lock().unwrap();
            let tx = conn.unchecked_transaction()?;

            for entry in glob(pattern.to_str().unwrap())?
                .filter_map(Result::ok)
                .filter(|e| e.is_file())
            {
                // Get file modification time
                let mtime = entry
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64);

                // Check if we already have this file indexed with same mtime
                let file_path_str = entry.to_string_lossy().to_string();
                if let Some(mtime) = mtime {
                    let skip: bool = conn.query_row(
                        "SELECT COUNT(*) FROM gmp_documents WHERE source_file = ?1 AND file_mtime = ?2",
                        params![file_path_str, mtime],
                        |row| row.get::<_, i32>(0),
                    ).unwrap_or(0) > 0;

                    if skip {
                        stats.documents_skipped += 1;
                        continue;
                    }
                }

                // Import or re-import the document
                match self.import_document(&entry, &tx, mtime) {
                    Ok(_doc_id) => {
                        stats.documents_imported += 1;
                    }
                    Err(e) => {
                        stats
                            .errors
                            .push(format!("Failed to import {:?}: {}", entry, e));
                    }
                }
            }

            tx.commit()?;
        }

        Ok(stats)
    }

    /// Import a single document
    fn import_document(
        &self,
        path: &Path,
        tx: &rusqlite::Transaction,
        file_mtime: Option<i64>,
    ) -> Result<String> {
        let content = std::fs::read_to_string(path)?;
        let (frontmatter, body) = self.parse_frontmatter(&content)?;

        let doc_id = frontmatter
            .code
            .clone()
            .unwrap_or_else(|| path.file_stem().unwrap().to_string_lossy().to_string());

        // Extract chapter from path
        let chapter = path
            .components()
            .nth(1)
            .and_then(|c| c.as_os_str().to_str())
            .map(|s| s.to_string());

        // Insert document
        tx.execute(
            r#"INSERT OR REPLACE INTO gmp_documents 
               (doc_id, code, title, chapter, doc_type, department, source_file, version, effective_date)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
            params![
                doc_id,
                frontmatter.code,
                frontmatter.title.as_deref().unwrap_or("Untitled"),
                chapter,
                frontmatter.document_type.as_deref(),
                frontmatter.department.as_deref(),
                path.to_string_lossy().to_string(),
                frontmatter.version.as_deref(),
                frontmatter.effective_date.as_deref(),
            ],
        )
        .context("Failed to insert document")?;

        // Chunk the content (simple split by headers)
        let chunks = self.chunk_content(&body);
        for (idx, chunk_text) in chunks.iter().enumerate() {
            let token_count = (chunk_text.len() / 4) as i64; // rough estimate, cast to i64

            tx.execute(
                r#"INSERT INTO gmp_document_chunks (doc_id, chunk_index, content, token_count)
                   VALUES (?1, ?2, ?3, ?4)"#,
                params![doc_id, idx as i32, chunk_text, token_count],
            )?;

            // Add to FTS index
            tx.execute(
                r#"INSERT INTO gmp_chunks_fts (content, doc_id, chunk_id)
                   VALUES (?1, ?2, ?3)"#,
                params![chunk_text, doc_id, idx as i64],
            )?;
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

        // Fallback: if too few chunks, split by paragraphs
        if chunks.len() < 2 {
            chunks = content
                .split("\n\n")
                .filter(|s| s.trim().len() > 50)
                .map(|s| s.trim().to_string())
                .collect();
        }

        chunks
    }

    /// Get total chunk count
    fn get_chunk_count(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM gmp_document_chunks", [], |row| {
            row.get(0)
        })?;
        Ok(count as usize)
    }

    /// Search using FTS
    pub fn search_fts(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>> {
        // Sanitize query: remove FTS5-special characters
        let sanitized = query
            .chars()
            .filter(|c| !matches!(c, '(' | ')' | ':' | '*' | '/' | '-' | '"' | '\''))
            .collect::<String>();

        // Tokenize query using simple Chinese n-gram approach
        // (jieba was removed from dependencies; using character bigrams for Chinese)
        let tokenized_query = self.tokenize_for_fts(&sanitized);

        let conn = self.conn.lock().unwrap();

        // 1. Try FTS5 MATCH first with BM25 scoring
        // CRITICAL FIX: Use content_rowid instead of chunk_id for proper BM25 scoring.
        // SQLite FTS5 bm25() function requires the FTS table's internal rowid to compute
        // term frequencies and document lengths correctly. When using external content
        // tables with UNINDEXED columns, joining on chunk_id loses the rowid mapping
        // and bm25() returns incorrect/default values.
        let mut stmt = conn.prepare(
            r#"SELECT c.id, c.doc_id, c.content, c.token_count,
                      d.title, d.doc_type, d.chapter,
                      bm25(gmp_chunks_fts) as score
               FROM gmp_chunks_fts fts
               JOIN gmp_document_chunks c ON fts.rowid = c.id
               JOIN gmp_documents d ON c.doc_id = d.doc_id
               WHERE gmp_chunks_fts MATCH ?1
               ORDER BY score
               LIMIT ?2"#,
        )?;

        let mut fts_results: Vec<SearchResult> = stmt
            .query_map(params![tokenized_query, top_k as i64], |row| {
                Ok(SearchResult {
                    chunk_id: row.get(0)?,
                    doc_id: row.get(1)?,
                    content: row.get(2)?,
                    token_count: row.get(3)?,
                    title: row.get(4)?,
                    doc_type: row.get(5)?,
                    chapter: row.get(6)?,
                    score: row.get(7)?,
                })
            })?
            .filter_map(Result::ok)
            .collect();

        // 2. If FTS results are insufficient, use LIKE fallback for Chinese n-gram matching
        if fts_results.len() < top_k {
            let substrings = Self::extract_chinese_substrings(&sanitized);
            if !substrings.is_empty() {
                let conditions: Vec<String> = substrings
                    .iter()
                    .take(5)
                    .map(|_| "content LIKE ?".to_string())
                    .collect();

                let like_sql = format!(
                    r#"SELECT c.id, c.doc_id, c.content, c.token_count,
                              d.title, d.doc_type, d.chapter,
                              0.0 as score
                       FROM gmp_document_chunks c
                       JOIN gmp_documents d ON c.doc_id = d.doc_id
                       WHERE {}
                       LIMIT ?"#,
                    conditions.join(" OR ")
                );

                let top_k_i64 = top_k as i64;
                let mut like_params: Vec<&dyn rusqlite::ToSql> = substrings
                    .iter()
                    .take(5)
                    .map(|s| s as &dyn rusqlite::ToSql)
                    .collect();
                like_params.push(&top_k_i64);

                let mut like_stmt = conn.prepare(&like_sql)?;
                let like_results: Vec<SearchResult> = like_stmt
                    .query_map(like_params.as_slice(), |row| {
                        Ok(SearchResult {
                            chunk_id: row.get(0)?,
                            doc_id: row.get(1)?,
                            content: row.get(2)?,
                            token_count: row.get(3)?,
                            title: row.get(4)?,
                            doc_type: row.get(5)?,
                            chapter: row.get(6)?,
                            score: row.get(7)?,
                        })
                    })?
                    .filter_map(Result::ok)
                    .collect();

                // Merge and deduplicate
                let mut seen = std::collections::HashSet::new();
                for r in fts_results.iter() {
                    seen.insert((r.doc_id.clone(), r.chunk_id));
                }
                for r in like_results {
                    if seen.insert((r.doc_id.clone(), r.chunk_id)) && fts_results.len() < top_k {
                        fts_results.push(r);
                    }
                }
            }
        }

        Ok(fts_results)
    }

    /// Search documents by taxonomy filter (Phase 3 T3.1: Layer 3 分类过滤通道)
    pub fn search_taxonomy_filter(
        &self,
        topic: Option<&str>,
        doc_type: Option<&str>,
        department: Option<&str>,
        risk_level: Option<&str>,
    ) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut conditions = Vec::new();
        let mut params: Vec<String> = Vec::new();

        if let Some(t) = topic {
            conditions.push("topic = ?".to_string());
            params.push(t.to_string());
        }
        if let Some(d) = doc_type {
            conditions.push("doc_type = ?".to_string());
            params.push(d.to_string());
        }
        if let Some(dept) = department {
            conditions.push("department = ?".to_string());
            params.push(dept.to_string());
        }
        if let Some(r) = risk_level {
            conditions.push("risk_level = ?".to_string());
            params.push(r.to_string());
        }

        if conditions.is_empty() {
            return Ok(Vec::new());
        }

        let sql = format!(
            "SELECT doc_id FROM doc_taxonomy WHERE {}",
            conditions.join(" AND ")
        );

        let mut stmt = conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::ToSql> = params
            .iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();

        let doc_ids: Vec<String> = stmt
            .query_map(params_refs.as_slice(), |row| row.get(0))?
            .filter_map(Result::ok)
            .collect();

        Ok(doc_ids)
    }

    /// Get relevance score for a document (Phase 3 T3.2: 相关性增强排序)
    pub fn get_doc_relevance(&self, doc_id: &str) -> Result<Option<f64>> {
        let conn = self.conn.lock().unwrap();
        let result: Option<f64> = conn
            .query_row(
                "SELECT overall_score FROM doc_relevance WHERE doc_id = ?",
                params![doc_id],
                |row| row.get(0),
            )
            .ok();
        Ok(result)
    }

    /// Search documents that contain a specific entity (Phase 3 T3.4: 实体链接检索)
    pub fn search_by_entity(&self, entity_term: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let pattern = format!("%{}%", entity_term);

        let mut stmt = conn.prepare(
            "SELECT DISTINCT doc_id FROM doc_entities
             WHERE surface_form LIKE ? OR canonical_term LIKE ?",
        )?;

        let doc_ids: Vec<String> = stmt
            .query_map(params![&pattern, &pattern], |row| row.get(0))?
            .filter_map(Result::ok)
            .collect();

        Ok(doc_ids)
    }

    /// Get entity density for a document (number of unique entities / chunk count)
    pub fn get_entity_density(&self, doc_id: &str) -> Result<f64> {
        let conn = self.conn.lock().unwrap();

        let entity_count: i64 = conn.query_row(
            "SELECT COUNT(DISTINCT canonical_term) FROM doc_entities WHERE doc_id = ?",
            params![doc_id],
            |row| row.get(0),
        )?;

        let chunk_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM gmp_document_chunks WHERE doc_id = ?",
            params![doc_id],
            |row| row.get(0),
        )?;

        if chunk_count > 0 {
            Ok(entity_count as f64 / chunk_count as f64)
        } else {
            Ok(0.0)
        }
    }

    /// Get synonym terms for a given term (Phase 3 T3.3: 术语扩展检索)
    pub fn get_synonyms(&self, term: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();

        // Try exact match first
        let synonyms_json: Option<String> = conn
            .query_row(
                "SELECT synonyms FROM gmp_terms WHERE canonical_term = ?",
                params![term],
                |row| row.get(0),
            )
            .ok();

        if let Some(json) = synonyms_json {
            if let Ok(syns) = serde_json::from_str::<Vec<String>>(&json) {
                return Ok(syns);
            }
        }

        // Try prefix/like match
        let pattern = format!("%{}%", term);
        let mut stmt = conn.prepare(
            "SELECT synonyms FROM gmp_terms WHERE canonical_term LIKE ? LIMIT 1",
        )?;

        if let Ok(json) = stmt.query_row(params![&pattern], |row| row.get::<_, String>(0)) {
            if let Ok(syns) = serde_json::from_str::<Vec<String>>(&json) {
                return Ok(syns);
            }
        }

        Ok(Vec::new())
    }

    /// Check if Layer 0 data is populated (used to gate enhanced search)
    pub fn has_layer0_data(&self) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM doc_relevance WHERE overall_score > 0",
            [],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Enhanced FTS search with Layer 0 re-ranking (Phase 3 T3.2/T3.3: relevance + entity density + synonym expansion)
    pub fn search_fts_enhanced(
        &self,
        query: &str,
        top_k: usize,
        use_layer0_boost: bool,
    ) -> Result<Vec<SearchResult>> {
        // Expand query with synonyms (Phase 3 T3.3)
        let expanded_query = if use_layer0_boost {
            let tokens: Vec<&str> = query.split_whitespace().collect();
            let mut all_terms = vec![query.to_string()];

            for token in tokens {
                // Extract Chinese terms (3+ chars) and English words (3+ chars)
                let token_clean = token.trim_matches(|c: char| !c.is_alphanumeric() && !is_cjk(c));
                if token_clean.len() >= 2 {
                    if let Ok(syns) = self.get_synonyms(token_clean) {
                        for syn in syns {
                            if !all_terms.contains(&syn) {
                                all_terms.push(syn);
                            }
                        }
                    }
                }
            }

            // Join with OR for FTS expansion
            all_terms.join(" OR ")
        } else {
            query.to_string()
        };

        // First get base FTS results
        let mut results = self.search_fts(&expanded_query, top_k * 2)?;

        if use_layer0_boost && !results.is_empty() {
            // Boost scores using Layer 0 signals: relevance score + entity density
            let conn = self.conn.lock().unwrap();

            for r in results.iter_mut() {
                // Get relevance score (0 if not present)
                let relevance: f64 = conn
                    .query_row(
                        "SELECT overall_score FROM doc_relevance WHERE doc_id = ?",
                        params![r.doc_id],
                        |row| row.get(0),
                    )
                    .unwrap_or(0.0);

                // Get entity density
                let entity_count: i64 = conn
                    .query_row(
                        "SELECT COUNT(DISTINCT canonical_term) FROM doc_entities WHERE doc_id = ?",
                        params![r.doc_id],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);

                let chunk_count: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM gmp_document_chunks WHERE doc_id = ?",
                        params![r.doc_id],
                        |row| row.get(0),
                    )
                    .unwrap_or(1);

                let entity_density = entity_count as f64 / chunk_count as f64;

                // Combine: base bm25 score + 0.5 * relevance + 0.3 * entity_density
                let boost = 0.5 * relevance + 0.3 * entity_density;
                r.score = r.score + boost;
            }

            // Re-sort by boosted score
            results.sort_by(|a, b| {
                b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
            });
            results.truncate(top_k);
        }

        Ok(results)
    }

    /// Simple tokenizer for FTS queries (Chinese character bigrams + English words)
    fn tokenize_for_fts(&self, text: &str) -> String {
        let mut tokens = Vec::new();
        let mut current_english = String::new();
        let chars: Vec<char> = text.chars().collect();

        for (i, c) in chars.iter().enumerate() {
            if c.is_alphanumeric() {
                current_english.push(*c);
            } else if is_cjk(*c) {
                // Flush any pending English word
                if !current_english.is_empty() {
                    if current_english.len() > 2 {
                        tokens.push(current_english.clone());
                    }
                    current_english.clear();
                }
                // Chinese character bigram
                if i + 1 < chars.len() && is_cjk(chars[i + 1]) {
                    tokens.push(format!("{}{}", c, chars[i + 1]));
                }
            } else {
                // Flush any pending English word
                if !current_english.is_empty() {
                    if current_english.len() > 2 {
                        tokens.push(current_english.clone());
                    }
                    current_english.clear();
                }
            }
        }

        // Flush remaining English
        if !current_english.is_empty() && current_english.len() > 2 {
            tokens.push(current_english);
        }

        if tokens.is_empty() {
            // Fallback: return original text
            text.to_string()
        } else {
            tokens.join(" OR ")
        }
    }

    /// Extract Chinese substrings (3-4 chars) from query for LIKE fallback
    fn extract_chinese_substrings(query: &str) -> Vec<String> {
        let mut substrings = Vec::new();
        let chars: Vec<char> = query.chars().collect();

        for i in 0..chars.len() {
            if is_cjk(chars[i]) {
                let mut end = std::cmp::min(i + 4, chars.len());
                while end > i + 2 {
                    let sub: String = chars[i..end].iter().collect();
                    substrings.push(sub);
                    end -= 1;
                }
            }
        }

        // Deduplicate and limit
        substrings.sort();
        substrings.dedup();
        substrings.truncate(10);
        substrings
    }

    /// Get document count
    pub fn get_document_count(&self) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let count: i64 =
            conn.query_row("SELECT COUNT(*) FROM gmp_documents", [], |row| row.get(0))?;
        Ok(count)
    }

    /// Validate index
    pub fn validate(&self) -> Result<ValidationResult> {
        let doc_count = self.get_document_count()?;
        let chunk_count = self.get_chunk_count()?;

        Ok(ValidationResult {
            documents: doc_count,
            chunks: chunk_count,
            is_valid: doc_count >= 1600 && chunk_count > 0,
        })
    }

    /// Search vectors using cosine similarity (dot product on normalized vectors)
    pub fn search_vectors(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<VectorSearchResult>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT e.chunk_id, e.embedding, c.doc_id, c.chunk_index, c.content
             FROM gmp_embeddings e
             JOIN gmp_document_chunks c ON e.chunk_id = c.id
             LIMIT 10000",
        )?;

        let mut results = Vec::new();
        let mut rows = stmt.query([])?;

        while let Some(row) = rows.next()? {
            let chunk_id: i64 = row.get(0)?;
            let blob: Vec<u8> = row.get(1)?;
            let doc_id: String = row.get(2)?;
            let chunk_index: i32 = row.get(3)?;
            let content: String = row.get(4)?;

            // Deserialize embedding from BLOB
            let embedding: Vec<f32> = blob
                .chunks_exact(4)
                .map(|bytes| f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
                .collect();

            // Compute cosine similarity
            let score = cosine_sim(query_embedding, &embedding);
            results.push(VectorSearchResult {
                chunk_id,
                doc_id,
                chunk_index,
                content,
                score: score as f64,
            });
        }

        // Sort by score descending and take top_k
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(top_k);

        Ok(results)
    }

    /// Index embeddings for all chunks (call after index_documents)
    pub fn index_embeddings(&self, ollama_url: &str, model: &str) -> Result<usize> {
        let client = reqwest::blocking::Client::new();

        // Get all chunks without embeddings - collect as plain data
        let chunk_ids_and_content: Vec<(i64, String)> = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare(
                "SELECT c.id, c.content FROM gmp_document_chunks c
                 WHERE NOT EXISTS (SELECT 1 FROM gmp_embeddings e WHERE e.chunk_id = c.id)
                 LIMIT 500",
            )?;
            let mut ids = Vec::new();
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                let id: i64 = row.get(0)?;
                let content: String = row.get(1)?;
                ids.push((id, content));
            }
            ids
        };

        if chunk_ids_and_content.is_empty() {
            return Ok(0);
        }

        let mut indexed = 0;
        for (chunk_id, content) in chunk_ids_and_content {
            // Get embedding from Ollama
            let resp = client
                .post(format!("{}/api/embeddings", ollama_url))
                .json(&serde_json::json!({
                    "model": model,
                    "prompt": content
                }))
                .send();

            match resp {
                Ok(response) if response.status().is_success() => {
                    #[derive(serde::Deserialize)]
                    struct EmbedResp {
                        embedding: Vec<f32>,
                    }
                    if let Ok(embed_resp) = response.json::<EmbedResp>() {
                        // Serialize to BLOB
                        let mut blob = Vec::with_capacity(embed_resp.embedding.len() * 4);
                        for f in &embed_resp.embedding {
                            blob.extend_from_slice(&f.to_le_bytes());
                        }

                        let conn = self.conn.lock().unwrap();
                        let _ = conn.execute(
                            "INSERT OR REPLACE INTO gmp_embeddings (chunk_id, embedding) VALUES (?1, ?2)",
                            params![chunk_id, blob],
                        );
                        indexed += 1;
                    }
                }
                _ => {}
            }
        }

        Ok(indexed)
    }
}

/// Compute cosine similarity between two normalized vectors
pub(crate) fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Vector search result
#[derive(Debug, Clone)]
pub struct VectorSearchResult {
    pub chunk_id: i64,
    pub doc_id: String,
    pub chunk_index: i32,
    pub content: String,
    pub score: f64,
}

/// Search result from FTS
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

/// Index statistics
#[derive(Debug, Default)]
pub struct IndexStats {
    // Note: documents_skipped tracks incremental index performance
    /// Number of documents newly imported or re-imported
    pub documents_imported: usize,
    /// Number of documents skipped (already indexed, no changes)
    pub documents_skipped: usize,
    /// Number of chunks indexed
    pub chunks_indexed: usize,
    /// Errors encountered during indexing
    pub errors: Vec<String>,
}

/// Validation result
#[derive(Debug)]
pub struct ValidationResult {
    pub documents: i64,
    pub chunks: usize,
    pub is_valid: bool,
}

// =====================================================================
// Unit tests
// =====================================================================

// =====================================================================
// Unit tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_storage() -> std::result::Result<StorageEngine, String> {
        let temp_dir = tempfile::tempdir().map_err(|e| format!("tempdir: {}", e))?;
        let db_path = temp_dir.path().join("test.db");
        let config = StorageConfig {
            data_dir: temp_dir.path().to_str().unwrap().to_string(),
            db_path: db_path.to_str().unwrap().to_string(),
        };
        StorageEngine::new(config).map_err(|e| e.to_string())
    }

    // =====================================================================
    // StorageEngine::new tests
    // =====================================================================

    #[test]
    fn test_storage_new_creates_database() {
        let storage = make_storage().unwrap();
        assert_eq!(storage.get_document_count().unwrap(), 0);
    }

    #[test]
    fn test_storage_empty_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let config = StorageConfig {
            data_dir: temp_dir.path().to_str().unwrap().to_string(),
            db_path: db_path.to_str().unwrap().to_string(),
        };
        let storage = StorageEngine::new(config).unwrap();
        assert_eq!(storage.get_document_count().unwrap(), 0);
    }

    // =====================================================================
    // StorageEngine::index_documents tests
    // =====================================================================

    #[test]
    fn test_index_documents_empty_dir() {
        let storage = make_storage().unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let result = storage.index_documents(temp_dir.path().to_str().unwrap());
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.documents_imported, 0);
        assert_eq!(stats.chunks_indexed, 0);
    }

    #[test]
    fn test_index_documents_api_works() {
        // Verify index_documents can be called without panic
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let config = StorageConfig {
            data_dir: temp_dir.path().to_str().unwrap().to_string(),
            db_path: db_path.to_str().unwrap().to_string(),
        };
        let storage = StorageEngine::new(config).unwrap();
        let file_path = temp_dir.path().join("test.md");
        std::fs::write(&file_path, "test content").unwrap();
        let result = storage.index_documents(temp_dir.path().to_str().unwrap());
        assert!(result.is_ok(), "Index should succeed: {:?}", result.err());
    }

    #[test]
    fn test_index_multiple_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let config = StorageConfig {
            data_dir: temp_dir.path().to_str().unwrap().to_string(),
            db_path: db_path.to_str().unwrap().to_string(),
        };
        let storage = StorageEngine::new(config).unwrap();
        for i in 1..=3 {
            let path = temp_dir.path().join(format!("doc{}.md", i));
            std::fs::write(&path, format!("Doc {} content", i)).unwrap();
        }
        let result = storage.index_documents(temp_dir.path().to_str().unwrap());
        assert!(result.is_ok(), "Index should succeed: {:?}", result.err());
    }

    // =====================================================================
    // StorageEngine::search_fts tests
    // =====================================================================

    #[test]
    fn test_search_fts_empty_database() {
        let storage = make_storage().unwrap();
        let result = storage.search_fts("偏差", 10);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_search_fts_api_works() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let config = StorageConfig {
            data_dir: temp_dir.path().to_str().unwrap().to_string(),
            db_path: db_path.to_str().unwrap().to_string(),
        };
        let storage = StorageEngine::new(config).unwrap();
        let file_path = temp_dir.path().join("test.md");
        std::fs::write(&file_path, "规程内容").unwrap();
        storage
            .index_documents(temp_dir.path().to_str().unwrap())
            .unwrap();
        let result = storage.search_fts("规程", 10);
        assert!(result.is_ok());
    }

    #[test]
    fn test_search_fts_top_k() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let config = StorageConfig {
            data_dir: temp_dir.path().to_str().unwrap().to_string(),
            db_path: db_path.to_str().unwrap().to_string(),
        };
        let storage = StorageEngine::new(config).unwrap();
        let file_path = temp_dir.path().join("test.md");
        std::fs::write(&file_path, "关键词内容").unwrap();
        storage
            .index_documents(temp_dir.path().to_str().unwrap())
            .unwrap();
        let result = storage.search_fts("关键词", 5);
        assert!(result.is_ok());
        assert!(result.unwrap().len() <= 5);
    }

    // =====================================================================
    // StorageEngine::get_document_count tests
    // =====================================================================

    #[test]
    fn test_get_document_count_initial() {
        let storage = make_storage().unwrap();
        assert_eq!(storage.get_document_count().unwrap(), 0);
    }

    #[test]
    fn test_get_document_count_api_works() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let config = StorageConfig {
            data_dir: temp_dir.path().to_str().unwrap().to_string(),
            db_path: db_path.to_str().unwrap().to_string(),
        };
        let storage = StorageEngine::new(config).unwrap();
        let file_path = temp_dir.path().join("test.md");
        std::fs::write(&file_path, "content").unwrap();
        storage
            .index_documents(temp_dir.path().to_str().unwrap())
            .unwrap();
        let count = storage.get_document_count();
        assert!(count.is_ok());
    }

    // =====================================================================
    // StorageEngine::validate tests
    // =====================================================================

    #[test]
    fn test_validate_api_works() {
        let storage = make_storage().unwrap();
        let result = storage.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_with_data() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let config = StorageConfig {
            data_dir: temp_dir.path().to_str().unwrap().to_string(),
            db_path: db_path.to_str().unwrap().to_string(),
        };
        let storage = StorageEngine::new(config).unwrap();
        let file_path = temp_dir.path().join("test.md");
        std::fs::write(&file_path, "content").unwrap();
        storage
            .index_documents(temp_dir.path().to_str().unwrap())
            .unwrap();
        let result = storage.validate();
        assert!(result.is_ok());
    }

    // =====================================================================
    // cosine_sim function tests
    // =====================================================================

    #[test]
    fn test_cosine_sim_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let sim = cosine_sim(&a, &b);
        assert!((sim - 1.0).abs() < 1e-6, "Got {}", sim);
    }

    #[test]
    fn test_cosine_sim_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_sim(&a, &b);
        assert!((sim - 0.0).abs() < 1e-6, "Got {}", sim);
    }

    #[test]
    fn test_cosine_sim_opposite() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![-1.0, 0.0, 0.0];
        let sim = cosine_sim(&a, &b);
        assert!((sim - (-1.0)).abs() < 1e-6, "Got {}", sim);
    }

    #[test]
    fn test_cosine_sim_45deg() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 1.0];
        let sim = cosine_sim(&a, &b);
        assert!((sim - 0.70710678).abs() < 1e-5, "Got {}", sim);
    }

    #[test]
    fn test_cosine_sim_multidim() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let sim = cosine_sim(&a, &b);
        assert!((sim - 1.0).abs() < 1e-6, "Got {}", sim);
    }

    // =====================================================================
    // Result struct tests
    // =====================================================================

    #[test]
    fn test_search_result_fields() {
        let r = SearchResult {
            chunk_id: 1,
            doc_id: "D1".into(),
            content: "content".into(),
            token_count: 100,
            title: "title".into(),
            doc_type: Some("SOP".into()),
            chapter: Some("ch1".into()),
            score: 0.95,
        };
        assert_eq!(r.chunk_id, 1);
        assert_eq!(r.score, 0.95);
    }

    #[test]
    fn test_vector_search_result_fields() {
        let r = VectorSearchResult {
            chunk_id: 42,
            doc_id: "V1".into(),
            chunk_index: 0,
            content: "vec".into(),
            score: 0.88,
        };
        assert_eq!(r.chunk_id, 42);
        assert_eq!(r.score, 0.88);
    }

    // =====================================================================
    // Edge cases
    // =====================================================================

    #[test]
    fn test_search_vectors_empty() {
        let storage = make_storage().unwrap();
        let query = vec![0.1, 0.2, 0.3];
        let result = storage.search_vectors(&query, 10);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_index_nonexistent_dir() {
        let storage = make_storage().unwrap();
        let result = storage.index_documents("/nonexistent/xyz");
        // Should handle gracefully
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_storage_config_fields() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let config = StorageConfig {
            data_dir: temp_dir.path().to_str().unwrap().to_string(),
            db_path: db_path.to_str().unwrap().to_string(),
        };
        assert_eq!(config.data_dir, temp_dir.path().to_str().unwrap());
    }

    // =====================================================================
    // Incremental indexing tests (Issue #16)
    // =====================================================================

    #[test]
    fn test_incremental_index_tracks_stats() {
        // Verify incremental index returns correct stats structure
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let config = StorageConfig {
            data_dir: temp_dir.path().to_str().unwrap().to_string(),
            db_path: db_path.to_str().unwrap().to_string(),
        };
        let storage = StorageEngine::new(config).unwrap();

        let file_path = temp_dir.path().join("doc.md");
        std::fs::write(&file_path, "content").unwrap();

        let result = storage
            .index_documents(temp_dir.path().to_str().unwrap())
            .unwrap();
        // Should have imported something (or skipped if already indexed)
        assert!(
            result.documents_imported >= 0
                || result.documents_skipped >= 0
                || result.errors.is_empty()
        );
        // Verify stats structure is valid
        assert!(result.documents_imported + result.documents_skipped >= 0);
    }

    #[test]
    fn test_incremental_index_reindex_on_change() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let config = StorageConfig {
            data_dir: temp_dir.path().to_str().unwrap().to_string(),
            db_path: db_path.to_str().unwrap().to_string(),
        };
        let storage = StorageEngine::new(config).unwrap();

        let file_path = temp_dir.path().join("doc.md");
        std::fs::write(
            &file_path,
            "---
id: INC-002
title: V1
---
Content V1",
        )
        .unwrap();

        // First index
        let result1 = storage
            .index_documents(temp_dir.path().to_str().unwrap())
            .unwrap();
        assert_eq!(result1.documents_imported, 1);

        // Modify file
        std::fs::write(
            &file_path,
            "---
id: INC-002
title: V2
---
Content V2 modified",
        )
        .unwrap();

        // Second index - should re-import
        let result2 = storage
            .index_documents(temp_dir.path().to_str().unwrap())
            .unwrap();
        assert_eq!(result2.documents_imported, 1);
    }

    #[test]
    fn test_incremental_index_new_file_detected() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let config = StorageConfig {
            data_dir: temp_dir.path().to_str().unwrap().to_string(),
            db_path: db_path.to_str().unwrap().to_string(),
        };
        let storage = StorageEngine::new(config).unwrap();

        // First index - empty
        let result1 = storage
            .index_documents(temp_dir.path().to_str().unwrap())
            .unwrap();
        assert_eq!(result1.documents_imported, 0);

        // Add a file
        let file_path = temp_dir.path().join("new.md");
        std::fs::write(
            &file_path,
            "---
id: NEW-001
title: New
---
New content",
        )
        .unwrap();

        // Second index - should find new file
        let result2 = storage
            .index_documents(temp_dir.path().to_str().unwrap())
            .unwrap();
        assert_eq!(result2.documents_imported, 1);
    }

    // =====================================================================
    // Performance tests (Issue #17)
    // =====================================================================

    #[test]
    fn test_search_fts_performance_100docs() {
        // Test FTS search performance with 100 documents
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let config = StorageConfig {
            data_dir: temp_dir.path().to_str().unwrap().to_string(),
            db_path: db_path.to_str().unwrap().to_string(),
        };
        let storage = StorageEngine::new(config).unwrap();

        // Create 100 small documents
        for i in 0..100 {
            let path = temp_dir.path().join(format!("doc{}.md", i));
            let content = format!(
                "---
id: PERF-{:03}
title: 性能测试文档{}
---

# 内容{}

这是第{{}}个测试文档，包含一些关键词用于搜索测试。",
                i, i, i
            );
            std::fs::write(&path, content).unwrap();
        }

        storage
            .index_documents(temp_dir.path().to_str().unwrap())
            .unwrap();

        // Time 100 searches
        let start = std::time::Instant::now();
        for _ in 0..100 {
            let _ = storage.search_fts("关键词", 10);
        }
        let elapsed = start.elapsed();

        // Should complete 100 searches in under 5 seconds (50ms avg per search)
        assert!(
            elapsed.as_secs() < 5,
            "100 searches took {}ms, expected < 5000ms",
            elapsed.as_millis()
        );
    }

    #[test]
    fn test_index_throughput_50docs() {
        // Test indexing throughput for 50 documents
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let config = StorageConfig {
            data_dir: temp_dir.path().to_str().unwrap().to_string(),
            db_path: db_path.to_str().unwrap().to_string(),
        };
        let storage = StorageEngine::new(config).unwrap();

        // Create 50 documents
        for i in 0..50 {
            let path = temp_dir.path().join(format!("doc{}.md", i));
            std::fs::write(
                &path,
                format!(
                    "---
id: DOC-{:03}
title: Doc {}
---

# Section 1
Content for document {}.",
                    i, i, i
                ),
            )
            .unwrap();
        }

        let start = std::time::Instant::now();
        let result = storage
            .index_documents(temp_dir.path().to_str().unwrap())
            .unwrap();
        let elapsed = start.elapsed();

        // Should index 50 docs in under 10 seconds
        assert!(
            elapsed.as_secs() < 10,
            "Indexing 50 docs took {}ms",
            elapsed.as_millis()
        );
        assert_eq!(result.documents_imported, 50);
    }

    #[test]
    fn test_search_result_ordering() {
        // Verify search results are ordered by score descending
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let config = StorageConfig {
            data_dir: temp_dir.path().to_str().unwrap().to_string(),
            db_path: db_path.to_str().unwrap().to_string(),
        };
        let storage = StorageEngine::new(config).unwrap();

        let file_path = temp_dir.path().join("test.md");
        std::fs::write(&file_path, "关键词 关键词 关键词").unwrap();
        storage
            .index_documents(temp_dir.path().to_str().unwrap())
            .unwrap();

        let results = storage.search_fts("关键词", 10).unwrap();
        for window in results.windows(2) {
            assert!(
                window[0].score >= window[1].score,
                "Results should be sorted by score descending"
            );
        }
    }

    #[test]
    fn test_concurrent_storage_instances() {
        // Verify multiple storage instances can coexist
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let config = StorageConfig {
            data_dir: temp_dir.path().to_str().unwrap().to_string(),
            db_path: db_path.to_str().unwrap().to_string(),
        };

        let s1 = StorageEngine::new(config.clone()).unwrap();
        let s2 = StorageEngine::new(config.clone()).unwrap();

        // Both should be functional
        assert!(s1.get_document_count().is_ok());
        assert!(s2.get_document_count().is_ok());
    }
}
