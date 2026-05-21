//! GMP Storage engine - Manages document storage, FTS, Vector, and Graph indexes

use anyhow::{Context, Result};
use glob::glob;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

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
            "#,
        )
        .context("Failed to initialize schema")?;

        Ok(())
    }

    /// Index documents from a directory
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
                match self.import_document(&entry, &tx) {
                    Ok(doc_id) => {
                        stats.documents_imported += 1;
                        let _ = doc_id;
                    }
                    Err(e) => {
                        stats.errors.push(format!("{}: {}", entry.display(), e));
                    }
                }
            }

            tx.commit()?;
        }

        stats.chunks_indexed = self.get_chunk_count()?;
        Ok(stats)
    }

    /// Import a single document
    fn import_document(&self, path: &Path, tx: &rusqlite::Transaction) -> Result<String> {
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
            let token_count = chunk_text.len() / 4; // rough estimate

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
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM gmp_document_chunks",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Search using FTS
    pub fn search_fts(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"SELECT c.id, c.doc_id, c.content, c.token_count,
                      d.title, d.doc_type, d.chapter,
                      bm25(gmp_chunks_fts) as score
               FROM gmp_chunks_fts fts
               JOIN gmp_document_chunks c ON fts.chunk_id = c.id
               JOIN gmp_documents d ON c.doc_id = d.doc_id
               WHERE gmp_chunks_fts MATCH ?1
               ORDER BY score
               LIMIT ?2"#,
        )?;

        let results = stmt
            .query_map(params![query, top_k as i64], |row| {
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

        Ok(results)
    }

    /// Get document count
    pub fn get_document_count(&self) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM gmp_documents",
            [],
            |row| row.get(0),
        )?;
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
    pub documents_imported: usize,
    pub chunks_indexed: usize,
    pub errors: Vec<String>,
}

/// Validation result
#[derive(Debug)]
pub struct ValidationResult {
    pub documents: i64,
    pub chunks: usize,
    pub is_valid: bool,
}
