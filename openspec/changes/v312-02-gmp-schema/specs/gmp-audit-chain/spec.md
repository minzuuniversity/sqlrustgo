# GMP Audit Chain

## ADDED Requirements

### Requirement: Audit row hash chaining

Each `gmp_audit_log` row contains `previous_hash` and `event_hash`. The `previous_hash` of row N equals the `event_hash` of row N-1. The genesis row has `previous_hash = NULL` and `event_hash = SHA-256(row_content_excluding_event_hash)`.

#### Scenario: Audit chain integrity
- **WHEN** two consecutive audit rows exist
- **THEN** row N's `previous_hash == SHA-256(row N-1 content)` holds

### Requirement: Event hash computation

`event_hash` is computed as SHA-256 of the concatenation of: `id | timestamp | user_id | action | table_name | record_id | old_value | new_value | ip_address | session_id | previous_hash`. The `event_hash` field itself is excluded from its own computation.

#### Scenario: Event hash determinism
- **WHEN** computing `event_hash` for a row with known field values
- **THEN** the hash is deterministic (same input always produces same output)

### Requirement: Chain verification function

A function `verify_audit_chain(storage)` returns `(ok: bool, broken_at: Option<i64>)` indicating whether the hash chain is intact or broken at a specific row ID.

#### Scenario: Intact chain verification
- **WHEN** `verify_audit_chain` is called on an unbroken chain
- **THEN** it returns `(true, None)`

#### Scenario: Broken chain detection
- **WHEN** row 5's `previous_hash` is tampered with
- **THEN** `verify_audit_chain` returns `(false, Some(5))`

### Requirement: Audit entry on all GMP mutations

Every INSERT/UPDATE/DELETE on GMP tables (`gmp_documents`, `gmp_chunks`, `gmp_relations`, `gmp_embeddings`, `gmp_document_versions`) creates an audit row.

#### Scenario: Document insert audit
- **WHEN** a new document is inserted
- **THEN** an audit row with `action = 'INSERT'`, `table_name = 'gmp_documents'` exists

#### Scenario: Chunk insert audit
- **WHEN** a chunk is inserted
- **THEN** an audit row with `action = 'INSERT'`, `table_name = 'gmp_chunks'` exists

### Requirement: Audit tamper detection

Modifying an existing audit row's `event_hash` or `previous_hash` causes chain verification to fail at the next read.

#### Scenario: Tampered event hash detection
- **WHEN** row 5's `event_hash` is manually updated
- **THEN** calling `verify_audit_chain` returns `(false, broken_at >= 5)`

### Requirement: Audit query with filters

Audit log can be queried by `start_time`, `end_time`, `user_id`, `action`, `table_name`.

#### Scenario: Time-bounded audit query
- **WHEN** querying audit logs for `start_time = T1`, `end_time = T2`
- **THEN** only rows with `timestamp` in `[T1, T2]` are returned
