# GMP Relation Graph

## ADDED Requirements

### Requirement: Typed relation edges

A relation row stores a typed edge between a source and target. The `relation_type` must be one of: `SOP`, `CLAUSE`, `CAPA`, `DEVIATION`, `ROLE`, `EQUIPMENT`, `AUDIT_FINDING`.

#### Scenario: Valid relation type
- **WHEN** inserting a relation with `relation_type = 'CLAUSE'`
- **THEN** the insert succeeds

#### Scenario: Invalid relation type
- **WHEN** inserting a relation with `relation_type = 'INVALID_TYPE'`
- **THEN** the insert fails or is rejected

### Requirement: Relation references doc or chunk

A relation edge references either documents (`source_doc_id`, `target_doc_id`) or chunks (`source_chunk_id`, `target_chunk_id`). At least one side of the relation must be set.

#### Scenario: Document-to-document relation
- **WHEN** creating a relation between two documents
- **THEN** `source_doc_id` and `target_doc_id` are set; `source_chunk_id` and `target_chunk_id` are NULL

#### Scenario: Chunk-to-chunk relation
- **WHEN** creating a relation between two chunks
- **THEN** `source_chunk_id` and `target_chunk_id` are set; `source_doc_id` and `target_doc_id` may be NULL

### Requirement: Relation properties as JSON

The `properties` column stores a JSON object with type-specific attributes (e.g., `{"strength": "MUST", "regulatory_ref": "ICH Q10"}`).

#### Scenario: Properties JSON storage
- **WHEN** inserting a relation with `properties = '{"strength": "MUST"}'`
- **THEN** the relation can be retrieved and `properties` parses as valid JSON

### Requirement: Relation uniqueness constraint

Duplicate relations (same source, target, and type) are prevented.

#### Scenario: Duplicate relation prevention
- **WHEN** inserting a relation where `(source_doc_id, source_chunk_id, relation_type, target_doc_id, target_chunk_id)` already exists
- **THEN** the insert is rejected or replaces the existing row

### Requirement: Neighbor query by document

Given a `doc_id`, retrieve all related documents with their relation types, optionally filtered by relation type.

#### Scenario: All neighbors of a document
- **WHEN** querying neighbors of document `doc_id = 5`
- **THEN** all related documents are returned with their relation types

#### Scenario: Filtered neighbors by relation type
- **WHEN** querying neighbors of document `doc_id = 5` with `relation_type = 'CLAUSE'`
- **THEN** only relations of type `CLAUSE` are returned

### Requirement: Path traversal (depth ≤ 3)

Graph projection can traverse paths of up to 3 hops.

#### Scenario: 2-hop path query
- **WHEN** querying paths from document A to document C with depth = 2
- **THEN** if A→B→C exists, the path is returned with evidence bundle

### Requirement: Evidence bundle per path

Each path returned from graph traversal includes for each node: `doc_id`, `version`, `chunk_id`, `content_hash`, and for each edge: `relation_type`, `properties`.

#### Scenario: Evidence bundle completeness
- **WHEN** traversing a 2-hop path A→B→C
- **THEN** the response includes node data for A, B, C and edge data for A→B, B→C
