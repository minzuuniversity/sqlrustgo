# GMP Chunk Management

## ADDED Requirements

### Requirement: Document chunking on ingestion

A document imported via the ingestion API is split into chunks. Each chunk is stored in `gmp_chunks` with a unique `chunk_index` per `doc_id + version_number`. Chunk content is not empty.

#### Scenario: Document with 5 sections imported
- **WHEN** a document with 5 sections is ingested
- **THEN** 5 rows exist in `gmp_chunks` with `chunk_index` 0 through 4, each with non-empty `content_text`

### Requirement: Chunk content hash

Each chunk row stores `content_hash = SHA-256(content_text)`. Two chunks with identical `content_text` have identical `content_hash`.

#### Scenario: Content hash uniqueness
- **WHEN** two chunks have different content text
- **THEN** their `content_hash` values are different

### Requirement: Chunk references document and version

Each chunk references its parent document via `doc_id` and `version_number`.

#### Scenario: Chunk-parent relationship
- **WHEN** querying a chunk by its `id`
- **THEN** `doc_id` and `version_number` correctly identify the document version it belongs to

### Requirement: Chunk index uniqueness

For a given `(doc_id, version_number)`, each `chunk_index` value is unique.

#### Scenario: Chunk index uniqueness constraint
- **WHEN** attempting to insert a chunk with `chunk_index = 0` where one already exists for the same `(doc_id, version_number)`
- **THEN** the insert fails or replaces the existing chunk (upsert behavior)

### Requirement: Chunk retrieval by document

All chunks for a specific `(doc_id, version_number)` can be retrieved ordered by `chunk_index`.

#### Scenario: Retrieve all chunks for a version
- **WHEN** querying all chunks for document version `(doc_id=1, version_number=2)`
- **THEN** results are ordered by `chunk_index` ascending

### Requirement: Configurable chunk size

The ingestion pipeline accepts a `chunk_size` parameter (in characters or tokens) that controls maximum chunk content length. Chunks exceeding the limit are split.

#### Scenario: Chunk size boundary
- **WHEN** ingesting with `chunk_size = 500` characters
- **THEN** no chunk's `content_text` exceeds 500 characters (except when a single segment exceeds the limit and must be kept as-is)

### Requirement: Section name preservation

The `section_name` column stores the structural heading or section identifier (e.g., "CH01-1", "APPX-A-2") if derivable from the source document structure.

#### Scenario: Section name from markdown heading
- **WHEN** ingesting a markdown file with `## Section 2.1` heading
- **THEN** the corresponding chunk's `section_name` is "2.1" or the heading text
