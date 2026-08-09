# GMP Corpus Ingestion

## ADDED Requirements

### Requirement: Idempotent file ingestion

Ingesting a file with the same `source_hash` as an existing version does NOT create a new version.

#### Scenario: Re-ingest unchanged file
- **WHEN** `ingest_file` is called with a file whose `source_hash` matches an existing version
- **THEN** the function returns `Err("SKIP")` and no new rows are inserted

### Requirement: New version on content change

Ingesting a file with a different `source_hash` from all existing versions creates a new `gmp_document_versions` row.

#### Scenario: File content changed
- **WHEN** a file's `source_hash` differs from any stored version
- **THEN** a new version row is inserted with incremented `version_number`

### Requirement: Ingestion report fields

`IngestionReport` contains: `documents_ingested`, `documents_skipped`, `documents_failed`, `chunks_created`, `embeddings_created`, `skipped_files`, `failed_files`, `errors`.

#### Scenario: Full corpus ingestion
- **WHEN** `ingest_corpus` is called on the GMP corpus directory
- **THEN** the report contains non-zero counts for each field that had activity

### Requirement: Source path preserved

The `source_path` relative to the corpus root is stored in the `gmp_documents` `doc_type` field.

#### Scenario: Source path storage
- **WHEN** ingesting `~/gmp-platform/gmp-md/CH01/file.md`
- **THEN** the document's `doc_type` stores `CH01/file.md`
