# V312-03 GMP Corpus Ingestion — Implementation Tasks

## 1. Implementation

- [ ] 1.1 `ingestion.rs` — `ingest_file` function with idempotency check and version creation
- [ ] 1.2 `ingestion.rs` — `ingest_corpus` function walking directory tree
- [ ] 1.3 `ingestion.rs` — `IngestionReport` struct with all required fields
- [ ] 1.4 `ingestion.rs` — `ensure_document` helper
- [ ] 1.5 Add `ingestion` module to `lib.rs`

## 2. Tests

- [ ] 2.1 Test idempotent re-import (same file ingested twice)
- [ ] 2.2 Test new version on content change
- [ ] 2.3 Test ingestion report counts
- [ ] 2.4 Test source path preservation
