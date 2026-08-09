# V312-02 GMP Schema v3.12 — Implementation Tasks

## 1. Schema Definitions

- [ ] 1.1 Add `gmp_document_versions` table struct and `CREATE TABLE IF NOT EXISTS` SQL constant to `crates/gmp/src/schema.rs` (new file)
- [ ] 1.2 Add `gmp_chunks` table struct and `CREATE TABLE IF NOT EXISTS` SQL constant to `crates/gmp/src/schema.rs`
- [ ] 1.3 Add `gmp_relations` table struct and `CREATE TABLE IF NOT EXISTS` SQL constant to `crates/gmp/src/schema.rs`
- [ ] 1.4 Add `previous_hash` and `event_hash` columns to `gmp_audit_log` (update `CREATE_AUDIT_LOG_TABLE` in `audit.rs`)
- [ ] 1.5 Add all index `CREATE INDEX IF NOT EXISTS` statements to `schema.rs`
- [ ] 1.6 Export new table constants and structs from `crates/gmp/src/lib.rs`

## 2. Storage Engine Integration

- [ ] 2.1 Add `create_gmp_tables` call in `GmpExecutor::init()` to invoke all `CREATE TABLE IF NOT EXISTS` statements (include new tables)
- [ ] 2.2 Ensure `create_gmp_tables` uses idempotent creation (all tables use `IF NOT EXISTS`)

## 3. Version Management

- [ ] 3.1 Add `DocumentVersion` struct with `id`, `doc_id`, `version_number`, `source_hash`, `content_hash`, `created_at`, `change_desc`
- [ ] 3.2 Add `insert_document_version` function that creates a new version row
- [ ] 3.3 Add `get_document_versions` function to retrieve all versions for a `doc_id` ordered by `version_number`
- [ ] 3.4 Add `get_document_version` function to retrieve specific `(doc_id, version_number)`
- [ ] 3.5 Add `find_version_by_source_hash` function for idempotent re-import check
- [ ] 3.6 Add `increment_version_number` helper (compute next version_number for a doc)

## 4. Chunk Management

- [ ] 4.1 Add `Chunk` struct with `id`, `doc_id`, `version_number`, `chunk_index`, `section_name`, `content_hash`, `content_text`, `created_at`
- [ ] 4.2 Add `insert_chunk` function (upsert by doc_id + version_number + chunk_index)
- [ ] 4.3 Add `get_chunks_for_version` function ordered by `chunk_index`
- [ ] 4.4 Add `get_chunk_by_id` function
- [ ] 4.5 Add `delete_chunks_for_version` function for re-import cleanup of old chunks

## 5. Relation Graph

- [ ] 5.1 Add `RelationType` enum: `SOP`, `CLAUSE`, `CAPA`, `DEVIATION`, `ROLE`, `EQUIPMENT`, `AUDIT_FINDING`
- [ ] 5.2 Add `Relation` struct with all relation fields
- [ ] 5.3 Add `insert_relation` function (upsert by source + target + type)
- [ ] 5.4 Add `get_neighbors` function: all relations for a `doc_id` optionally filtered by `relation_type`
- [ ] 5.5 Add `get_neighbors_chunk` function: all relations for a `chunk_id`
- [ ] 5.6 Add `path_query` function: depth ≤ 3 path traversal from source to target (returns `Vec<Vec<Relation>>`)

## 6. Audit Chain

- [ ] 6.1 Update `AuditLog` struct to include `previous_hash` and `event_hash` fields
- [ ] 6.2 Implement `compute_event_hash(log: &AuditLog) -> String` using SHA-256 of concatenated fields (excluding `event_hash`)
- [ ] 6.3 Implement `verify_audit_chain(storage: &dyn StorageEngine) -> (bool, Option<i64>)` — traverse ordered by `id`, verify `previous_hash` chain; return `(true, None)` if intact, `(false, broken_at_id)` if broken
- [ ] 6.4 Update `record_audit_log` to compute and store `previous_hash` (last audit row's `event_hash`) and `event_hash` (current row's hash)
- [ ] 6.5 Add `get_last_audit_hash` helper to retrieve the most recent `event_hash`
- [ ] 6.6 Add `genesis_audit_hash` constant: SHA-256 of known genesis string for chain start verification

## 7. Tests

- [ ] 7.1 Add test for idempotent schema creation (call `create_gmp_tables` twice)
- [ ] 7.2 Add test for document version creation and idempotent re-import
- [ ] 7.3 Add test for chunk insertion and retrieval by version
- [ ] 7.4 Add test for relation insertion and neighbor query
- [ ] 7.5 Add test for audit chain integrity (`verify_audit_chain` returns intact)
- [ ] 7.6 Add test for audit chain tamper detection (`verify_audit_chain` returns broken_at)
- [ ] 7.7 Add test for graph path query depth ≤ 3

## 8. Documentation

- [ ] 8.1 Document all new table schemas in `crates/gmp/src/schema.rs` as SQL comments
- [ ] 8.2 Update `crates/gmp/src/lib.rs` module doc comment to reflect new tables
