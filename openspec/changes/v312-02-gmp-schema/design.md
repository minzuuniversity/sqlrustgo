# Design: V312-02 GMP Schema v3.12

## Context

The existing GMP tables (`gmp_documents`, `gmp_document_contents`, `gmp_document_keywords`, `gmp_embeddings`, `gmp_audit_log`) were designed for Stage 1 (SQLite-backed prototype). They lack:
1. **Document version history** — no way to track document evolution; re-ingestion overwrites without history.
2. **Chunk-level granularity** — all content stored as monolithic rows; vector search operates on whole-document embeddings.
3. **Relation modeling** — SOP/clause/CAPA/deviation/role/equipment are described in document content but not modeled as typed relations.
4. **Audit chain** — audit rows lack hash chaining (`previous_hash`/`event_hash`) for tamper evidence.

Source corpus at `~/gmp-platform/gmp-md` (1697 files, 9.3 MB) will be the primary ingestion source. The GMP production path currently depends on Chroma runtime; this schema serves as the foundation to replace that dependency.

## Goals / Non-Goals

**Goals:**
- Define all new tables with SQL `CREATE TABLE IF NOT EXISTS` for idempotent creation.
- `gmp_document_versions`: version history per document with source file hash.
- `gmp_chunks`: chunk-level content rows referencing document+version.
- `gmp_relations`: typed edges between documents/chunks with relation type enum.
- `gmp_audit_log` enriched: add `previous_hash` and `event_hash` columns.
- Embedding records reference `chunk_id` instead of (or in addition to) `doc_id`.
- All tables have indexes for common query patterns.

**Non-Goals:**
- No chunking algorithm implementation (deferred to V312-03).
- No embedding algorithm changes (deferred to V312-04).
- No graph projection queries (deferred to V312-06).
- No backup/restore implementation (deferred to V312-09).

## Decisions

### Decision 1: Chunk-first storage model

**Choice:** Embeddings and audit records reference `chunk_id` (not `doc_id`).

**Rationale:** Vector search works best at chunk granularity (paragraph/section level). If embeddings reference whole documents, re-ranking requires fetching all chunks and re-scoring. Chunk-level IDs enable precise citation in RAG output (V312-07).

**Alternative:** Keep `doc_id` as primary reference, add `version` column. Rejected — too coarse; forces whole-document re-indexing on any content change.

### Decision 2: SHA-256 for all hashes

**Choice:** `source_hash`, `content_hash`, `event_hash`, `previous_hash` all use SHA-256.

**Rationale:** SHA-256 is available via the existing `sha2` crate (already in workspace). No new dependency. Sufficient for tamper evidence; HMAC not required (this is audit trail, not adversary-resistant crypto).

**Alternative:** BLAKE3 for speed. Rejected — `sha2` is already a workspace dependency; avoid new deps for a single hash function.

### Decision 3: Relation types as enum stored as TEXT

**Choice:** `relation_type TEXT NOT NULL` with CHECK constraint listing valid values.

**Rationale:** Simple, portable, queryable. No enum migration needed across SQLite/SQLRustGo. JSON `properties` column handles type-specific attributes.

**Alternative:** Separate junction tables per relation type. Rejected — 7 types × 2 tables = 14 extra tables; cross-type queries require UNION.

### Decision 4: Version table is append-only

**Choice:** `gmp_document_versions` only supports INSERT; UPDATE/DELETE only via explicit `update_version` / `archive_version` API.

**Rationale:** Audit chain integrity — a document version, once created, is immutable. Any correction creates a new version entry. This matches GMP document control philosophy.

### Decision 5: `previous_hash` chain starts with `NULL` for first audit entry

**Choice:** Genesis audit row has `previous_hash = NULL`, `event_hash = SHA-256(row_content)`.

**Rationale:** Standard hash chain initialization. `NULL` is the sentinel indicating chain origin. Chain verification traverses from `NULL` predecessor to first row, then validates each subsequent `previous_hash == prior event_hash`.

## Schema

```sql
-- gmp_document_versions: version history per document
CREATE TABLE IF NOT EXISTS gmp_document_versions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    doc_id          INTEGER NOT NULL REFERENCES gmp_documents(id),
    version_number  INTEGER NOT NULL DEFAULT 1,
    source_hash     TEXT NOT NULL,          -- SHA-256 of source file content
    content_hash    TEXT NOT NULL,          -- SHA-256 of all chunk content hashes
    created_at      INTEGER NOT NULL,       -- Unix timestamp ms
    change_desc     TEXT,
    UNIQUE(doc_id, version_number)
);

-- gmp_chunks: chunk-level content
CREATE TABLE IF NOT EXISTS gmp_chunks (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    doc_id          INTEGER NOT NULL REFERENCES gmp_documents(id),
    version_number  INTEGER NOT NULL,
    chunk_index     INTEGER NOT NULL,
    section_name    TEXT,                   -- e.g. "CH01-1", "APPX-A-2"
    content_hash    TEXT NOT NULL,          -- SHA-256 of chunk text
    content_text    TEXT NOT NULL,
    created_at      INTEGER NOT NULL,
    UNIQUE(doc_id, version_number, chunk_index)
);

-- gmp_relations: typed edges between documents/chunks
CREATE TABLE IF NOT EXISTS gmp_relations (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    source_doc_id   INTEGER REFERENCES gmp_documents(id),
    source_chunk_id INTEGER REFERENCES gmp_chunks(id),
    relation_type   TEXT NOT NULL CHECK (relation_type IN (
                        'SOP','CLAUSE','CAPA','DEVIATION','ROLE','EQUIPMENT','AUDIT_FINDING'
                    )),
    target_doc_id   INTEGER REFERENCES gmp_documents(id),
    target_chunk_id INTEGER REFERENCES gmp_chunks(id),
    properties      TEXT,                    -- JSON
    created_at      INTEGER NOT NULL,
    UNIQUE(source_doc_id, source_chunk_id, relation_type, target_doc_id, target_chunk_id)
);

-- gmp_audit_log: enriched with hash chain
CREATE TABLE IF NOT EXISTS gmp_audit_log (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp       INTEGER NOT NULL,
    user_id         TEXT NOT NULL,
    action          TEXT NOT NULL,
    table_name      TEXT NOT NULL,
    record_id       TEXT,
    old_value       TEXT,
    new_value       TEXT,
    ip_address      TEXT,
    session_id      TEXT,
    previous_hash   TEXT,                   -- NULL for genesis; SHA-256 of prior row content
    event_hash      TEXT NOT NULL,          -- SHA-256 of this row's content (excl. this field)
    UNIQUE(timestamp, user_id, action, record_id)
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_versions_doc_id ON gmp_document_versions(doc_id);
CREATE INDEX IF NOT EXISTS idx_chunks_doc_version ON gmp_chunks(doc_id, version_number);
CREATE INDEX IF NOT EXISTS idx_chunks_content_hash ON gmp_chunks(content_hash);
CREATE INDEX IF NOT EXISTS idx_relations_source ON gmp_relations(source_doc_id, source_chunk_id);
CREATE INDEX IF NOT EXISTS idx_relations_target ON gmp_relations(target_doc_id, target_chunk_id);
CREATE INDEX IF NOT EXISTS idx_relations_type ON gmp_relations(relation_type);
CREATE INDEX IF NOT EXISTS idx_audit_event_hash ON gmp_audit_log(event_hash);
CREATE INDEX IF NOT EXISTS idx_audit_previous_hash ON gmp_audit_log(previous_hash);
```

## Risks / Trade-offs

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Chunking algorithm decisions (chunk size, overlap) affect retrieval quality | Medium | High | Configurable chunk size; defer optimization to V312-05 fixture |
| Schema migration from v3.11 existing data | Medium | Medium | `CREATE TABLE IF NOT EXISTS` is additive; existing rows unaffected |
| Audit chain break on manual DB edits | Low | High | Audit verification is read-only check; fails closed if chain breaks |
| Graph relation data volume grows large | Medium | Low | Index on `source_doc_id` and `relation_type`; pagination for traversal |
