# GMP Document Versioning

## ADDED Requirements

### Requirement: Document version creation on first import

A document imported for the first time creates `gmp_document_versions` row with `version_number = 1`, `source_hash = SHA-256(source_content)`, `content_hash = SHA-256(all_chunk_hashes)`, `created_at = now_ms()`.

#### Scenario: First import of a new document
- **WHEN** a document with `source_path = "/CH01/file1.md"` and `source_hash = H1` is imported and no existing version with `source_hash = H1` exists
- **THEN** `gmp_document_versions` contains exactly one row for that document with `version_number = 1`, `source_hash = H1`

### Requirement: Idempotent re-import with same source hash

Re-importing a document with the same `source_path` and `source_hash` does NOT create a new version row; the existing version is returned.

#### Scenario: Re-import unchanged document
- **WHEN** a document with `source_path = "/CH01/file1.md"` and `source_hash = H1` is re-imported and a version with `source_hash = H1` already exists
- **THEN** no new `gmp_document_versions` row is created; the existing version row is returned

### Requirement: New version on content change

Re-importing a document with the same `source_path` but different `source_hash` creates a new version row with incremented `version_number`.

#### Scenario: Re-import with changed content
- **WHEN** a document with `source_path = "/CH01/file1.md"` and `source_hash = H2` (different from stored `H1`) is re-imported
- **THEN** `gmp_document_versions` contains a new row with `version_number = 2`, `source_hash = H2`, `doc_id` unchanged

### Requirement: Document version queries

The system can retrieve all versions of a document ordered by `version_number` ascending, and retrieve a specific version by `doc_id + version_number`.

#### Scenario: Query all versions
- **WHEN** querying all versions for a document with 3 versions
- **THEN** results are returned in `version_number` ascending order

#### Scenario: Query specific version
- **WHEN** querying version 2 of a document
- **THEN** exactly one row is returned with `version_number = 2`

### Requirement: Version immutable after creation

A `gmp_document_versions` row cannot be UPDATEd or DELETEd via public API. Corrections create a new version row.

#### Scenario: Version immutability enforcement
- **WHEN** attempting to update a `gmp_document_versions` row via the public API
- **THEN** the operation returns an error or is rejected

### Requirement: Schema creation idempotent

`CREATE TABLE IF NOT EXISTS` is used for all GMP tables; running schema creation multiple times does not error.

#### Scenario: Double schema creation
- **WHEN** `create_gmp_tables` is called twice on a fresh storage engine
- **THEN** the second call succeeds without error
