# Proposal: V312-02 GMP Schema v3.12

## Why

The current GMP tables (`gmp_documents`, `gmp_document_contents`, `gmp_document_keywords`, `gmp_embeddings`, `gmp_audit_log`) are functional but lack document version history, chunk-level granularity, relation modeling for SOP/clause/CAPA/deviation/role/equipment, and an idempotent schema creation contract. These gaps prevent reliable re-ingestion, evidence navigation, and compliance audit.

## What Changes

- **Idempotent schema creation**: All `CREATE TABLE IF NOT EXISTS` statements; `create_gmp_tables` becomes re-runnable without error.
- **Document version history**: `gmp_document_versions` table tracking `doc_id`, `version_number`, `source_hash`, `created_at`, `change_description`.
- **Chunk-level storage**: `gmp_chunks` table for sub-document segmentation, referencing `doc_id + version`.
- **Relation modeling**: `gmp_relations` table with `source_doc_id`, `source_chunk_id`, `relation_type` (SOP|CAPA|CLAUSE|DEVIATION|ROLE|EQUIPMENT|AUDIT_FINDING), `target_doc_id`, `target_chunk_id`, `properties` (JSON).
- **Audit row enrichment**: `previous_hash` and `event_hash` columns added to `gmp_audit_log`.
- **Schema inventory**: Document all tables, columns, indexes, and constraints in a machine-readable schema artifact.

## Capabilities

### New Capabilities

- `gmp-document-versioning`: Document version history with source hash tracking — each version references a source file hash; re-ingestion with the same source hash is idempotent.
- `gmp-chunk-management`: Sub-document chunk table — content is segmented at ingestion time with configurable chunk size; each chunk carries `doc_id`, `version`, `chunk_index`, `content_hash`.
- `gmp-relation-graph`: Relation table modeling SOP/clause/CAPA/deviation/role/equipment/audit_finding relationships between documents and chunks.
- `gmp-audit-chain`: Audit log entries include `previous_hash` (links to prior row's `event_hash`) and `event_hash` (SHA-256 of row content), enabling tamper-evident chain verification.

### Modified Capabilities

- None — this is a net-new capability layer built on top of existing tables.
