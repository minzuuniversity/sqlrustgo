-- ============================================================
-- GMP-Platform SQLRustGo Schema Initialization Script
-- ============================================================
-- Version: v1.3.1
-- Date: 2026-05-24
-- Purpose: Create all Layer 0-2 tables for GMP retrieval system
--          using SQLRustGo FileStorage engine
-- Usage: sqlite3 data/gmp.db < scripts/sql/schema/gmp_schema.sql
-- ============================================================

-- ============================================================
-- Layer 1: Core Document Storage (from gmp-storage crate)
-- ============================================================

-- gmp_documents: source document metadata
CREATE TABLE IF NOT EXISTS gmp_documents (
    doc_id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    source_path TEXT,
    gmp_chapter TEXT,
    doc_type TEXT,
    regulation TEXT,
    created_at TEXT,
    updated_at TEXT,
    checksum TEXT
);

-- gmp_document_chunks: text chunks for embedding
CREATE TABLE IF NOT EXISTS gmp_document_chunks (
    chunk_id INTEGER PRIMARY KEY,
    doc_id TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    chunk_text TEXT NOT NULL,
    char_count INTEGER,
    FOREIGN KEY (doc_id) REFERENCES gmp_documents(doc_id)
);

-- gmp_chunks_fts: full-text search index
CREATE TABLE IF NOT EXISTS gmp_chunks_fts (
    chunk_id INTEGER PRIMARY KEY,
    doc_id TEXT NOT NULL,
    fts_content TEXT NOT NULL,
    FOREIGN KEY (chunk_id) REFERENCES gmp_document_chunks(chunk_id)
);

-- gmp_embeddings: vector embeddings (binary BLOB)
CREATE TABLE IF NOT EXISTS gmp_embeddings (
    chunk_id INTEGER PRIMARY KEY,
    model_name TEXT NOT NULL,
    dimension INTEGER NOT NULL,
    embedding BLOB NOT NULL,
    created_at TEXT,
    FOREIGN KEY (chunk_id) REFERENCES gmp_document_chunks(chunk_id)
);

-- gmp_hnsw_meta: HNSW index metadata
CREATE TABLE IF NOT EXISTS gmp_hnsw_meta (
    id INTEGER PRIMARY KEY,
    index_name TEXT NOT NULL,
    m INTEGER,
    ef_construction INTEGER,
    ef_search INTEGER,
    node_count INTEGER,
    created_at TEXT
);

-- ============================================================
-- Layer 2: Knowledge Graph (from gmp-kg crate)
-- ============================================================

-- gmp_terms: canonical GMP terminology (synonyms/hierarchy now in separate tables)
CREATE TABLE IF NOT EXISTS gmp_terms (
    term_id INTEGER PRIMARY KEY,
    canonical_term TEXT NOT NULL,
    term_type TEXT NOT NULL,
    synonyms TEXT,          -- kept for backward compat, NULL = use term_synonyms
    hypernyms TEXT,        -- kept for backward compat, NULL = use term_hierarchy
    hyponyms TEXT,          -- kept for backward compat, NULL = use term_hierarchy
    gmp_chapter TEXT,
    entity_type TEXT
);

-- term_synonyms: normalized term synonyms (replaces JSON in gmp_terms.synonyms)
CREATE TABLE IF NOT EXISTS term_synonyms (
    term_id INTEGER NOT NULL,
    synonym TEXT NOT NULL,
    FOREIGN KEY (term_id) REFERENCES gmp_terms(term_id)
);

-- term_hierarchy: hypernym/hyponym relationships (replaces JSON in gmp_terms)
CREATE TABLE IF NOT EXISTS term_hierarchy (
    term_id INTEGER NOT NULL,
    parent_term TEXT NOT NULL,
    hierarchy_type TEXT NOT NULL CHECK (hierarchy_type IN ('hypernym', 'hyponym')),
    FOREIGN KEY (term_id) REFERENCES gmp_terms(term_id)
);

-- doc_entities: entities extracted from documents
CREATE TABLE IF NOT EXISTS doc_entities (
    id INTEGER PRIMARY KEY,
    doc_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    surface_form TEXT NOT NULL,
    canonical_term TEXT NOT NULL,
    start_pos INTEGER,
    end_pos INTEGER,
    confidence REAL,
    FOREIGN KEY (doc_id) REFERENCES gmp_documents(doc_id)
);

-- doc_relations: entity relationships extracted from documents
CREATE TABLE IF NOT EXISTS doc_relations (
    id INTEGER PRIMARY KEY,
    doc_id TEXT NOT NULL,
    relation_type TEXT NOT NULL,
    src_entity TEXT NOT NULL,
    dst_entity TEXT NOT NULL,
    confidence REAL,
    FOREIGN KEY (doc_id) REFERENCES gmp_documents(doc_id)
);

-- doc_taxonomy: document classification
CREATE TABLE IF NOT EXISTS doc_taxonomy (
    doc_id TEXT PRIMARY KEY,
    topic TEXT,
    sub_topic TEXT,
    doc_type TEXT,
    regulation TEXT,
    risk_level TEXT,
    department TEXT,
    confidence REAL,
    FOREIGN KEY (doc_id) REFERENCES gmp_documents(doc_id)
);

-- doc_relevance: document relevance scores
CREATE TABLE IF NOT EXISTS doc_relevance (
    doc_id TEXT PRIMARY KEY,
    overall_score REAL,
    term_density REAL,
    reg_citation REAL,
    process_completeness REAL,
    entity_density REAL,
    dept_coverage REAL,
    confidence REAL,
    FOREIGN KEY (doc_id) REFERENCES gmp_documents(doc_id)
);

-- ============================================================
-- Layer 2: Extraction Rules (from run_layer0_pipeline.py)
-- ============================================================

-- extraction_rules: NER patterns from ENTITY_PATTERNS
CREATE TABLE IF NOT EXISTS extraction_rules (
    rule_id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_type TEXT NOT NULL,
    pattern TEXT NOT NULL,
    canonical_form TEXT,
    priority INTEGER DEFAULT 5,
    active INTEGER DEFAULT 1
);

-- relation_templates: RE patterns from RELATION_TEMPLATES
CREATE TABLE IF NOT EXISTS relation_templates (
    template_id INTEGER PRIMARY KEY AUTOINCREMENT,
    pattern TEXT NOT NULL,
    relation_type TEXT NOT NULL,
    src_type_hint TEXT,
    dst_type_hint TEXT,
    active INTEGER DEFAULT 1
);

-- taxonomy_rules: classification patterns from TAXONOMY_*_PATTERNS
CREATE TABLE IF NOT EXISTS taxonomy_rules (
    rule_id INTEGER PRIMARY KEY AUTOINCREMENT,
    dimension TEXT NOT NULL CHECK (dimension IN ('topic', 'doc_type', 'risk_level', 'department')),
    pattern TEXT NOT NULL,
    value TEXT NOT NULL,
    priority INTEGER DEFAULT 5,
    active INTEGER DEFAULT 1
);

-- gmp_term_patterns: keyword patterns for scoring
CREATE TABLE IF NOT EXISTS gmp_term_patterns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    term TEXT NOT NULL,
    category TEXT,
    active INTEGER DEFAULT 1
);

-- ============================================================
-- Layer 3: Retrieval (from gmp-retrieval crate)
-- ============================================================

-- retrieval_cache: embedding LRU cache (runtime, not persisted)
-- Note: managed in-memory by CachedEmbeddingStore, not stored in DB

-- ============================================================
-- Layer 3: Reranker (from gmp-reranker crate)
-- ============================================================

-- rerank_history: optional reranking audit log
CREATE TABLE IF NOT EXISTS rerank_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    query TEXT NOT NULL,
    doc_id TEXT NOT NULL,
    original_rank INTEGER,
    reranked_rank INTEGER,
    rerank_score REAL,
    timestamp TEXT DEFAULT CURRENT_TIMESTAMP
);

-- ============================================================
-- Layer 5: Query Audit (from gmp-llm crate)
-- ============================================================

-- query_audit: LLM answer audit trail
CREATE TABLE IF NOT EXISTS query_audit (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    query TEXT NOT NULL,
    answer TEXT,
    context_docs TEXT,      -- JSON array of doc_ids used
    latency_ms INTEGER,
    timestamp TEXT DEFAULT CURRENT_TIMESTAMP
);

-- ============================================================
-- Indexes for Performance
-- ============================================================

CREATE INDEX IF NOT EXISTS idx_chunks_doc_id ON gmp_document_chunks(doc_id);
CREATE INDEX IF NOT EXISTS idx_fts_doc_id ON gmp_chunks_fts(doc_id);
CREATE INDEX IF NOT EXISTS idx_entities_doc_id ON doc_entities(doc_id);
CREATE INDEX IF NOT EXISTS idx_relations_doc_id ON doc_relations(doc_id);
CREATE INDEX IF NOT EXISTS idx_entities_canonical ON doc_entities(canonical_term);
CREATE INDEX IF NOT EXISTS idx_relations_type ON doc_relations(relation_type);
CREATE INDEX IF NOT EXISTS idx_terms_canonical ON gmp_terms(canonical_term);
CREATE INDEX IF NOT EXISTS idx_synonyms_term_id ON term_synonyms(term_id);
CREATE INDEX IF NOT EXISTS idx_synonyms_text ON term_synonyms(synonym);
CREATE INDEX IF NOT EXISTS idx_hierarchy_term_id ON term_hierarchy(term_id);
CREATE INDEX IF NOT EXISTS idx_taxonomy_topic ON doc_taxonomy(topic);
CREATE INDEX IF NOT EXISTS idx_taxonomy_risk ON doc_taxonomy(risk_level);
CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON query_audit(timestamp);
