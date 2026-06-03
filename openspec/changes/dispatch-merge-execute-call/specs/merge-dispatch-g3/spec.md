# Spec: merge-dispatch-g3

## ADDED Requirements

### Requirement: LocalExecutorDml MUST Dispatch MERGE to MergeExecutor

The `LocalExecutorDml::execute_dml(sql: &str)` method at `crates/executor/src/local_executor_dml.rs` MUST detect SQL starting with the `MERGE` keyword (case-insensitive) and route the call to `MergeExecutor::execute_merge()` instead of returning an error or doing nothing.

#### Scenario: execute_dml routes MERGE statement to MergeExecutor
- **WHEN** `execute_dml("MERGE INTO target t USING source s ON t.id = s.id WHEN MATCHED THEN UPDATE SET t.val = s.val")` is called
- **THEN** the method MUST:
  1. Parse the SQL using the G2 parser (`sqlrustgo_parser::parse`)
  2. Extract the `Statement::Merge(MergeStatement)` AST
  3. Convert to `sqlrustgo_planner::MergeStatement`
  4. Construct `MergeExecutor::new(self.storage.clone(), self.engine.clone())`
  5. Call `merge_executor.execute_merge(&planner_merge)`
  6. Return the `ExecutorResult` from execute_merge

#### Scenario: execute_dml rejects non-MERGE statements with helpful error
- **WHEN** `execute_dml("INSERT INTO t VALUES (1)")` is called
- **THEN** the method MUST return `Err(SqlError)` indicating that only MERGE is currently supported by this dispatch path (not INSERT/UPDATE/DELETE — those are out of scope for G3)

#### Scenario: execute_dml returns parse error for malformed MERGE
- **WHEN** `execute_dml("MERGE INTO")` is called (incomplete statement)
- **THEN** the method MUST return `Err(SqlError::ExecutionError(...))` with a parse error message
- **AND** it MUST NOT call `execute_merge`

### Requirement: LocalExecutorDml MUST Hold Arc<RwLock<dyn StorageEngine>>

The `LocalExecutorDml` struct MUST include a `storage: Arc<RwLock<dyn StorageEngine>>` field to pass to `MergeExecutor::new(storage, ...)`. This is required because MergeExecutor needs owned `Arc<RwLock<>>` storage.

#### Scenario: LocalExecutorDml exposes storage Arc
- **WHEN** `LocalExecutorDml` is constructed via `new_with_storage(storage, engine)`
- **THEN** the struct MUST store the exact provided `storage` Arc
- **AND** `local_executor_dml.storage()` (or internal access) MUST return a cloneable `Arc<RwLock<dyn StorageEngine>>`

#### Scenario: local_executor_dml.storage.clone() usable for MergeExecutor
- **WHEN** the dispatch path constructs `MergeExecutor::new(local_executor_dml.storage.clone(), local_executor_dml.engine.clone())`
- **THEN** the call MUST compile (storage type matches MergeExecutor's expected `Arc<RwLock<dyn StorageEngine>>`)

### Requirement: Parser-to-Planner MergeStatement Conversion

A private `convert_parser_merge_to_planner` function MUST exist to convert `sqlrustgo_parser::MergeStatement` (with `Vec<MergeWhenClause>`) to `sqlrustgo_planner::MergeStatement` (with `Option<MergeClause>` × 2). The conversion MUST:
- Map `target_table` directly
- Map `source: MergeSource::Table{name}` → `source_table: name` (Subquery not yet supported)
- Map `on_condition: Expression` → `on_condition: Expr` (lossy: see Known Limitations)
- Map first `when_clauses[i]` where `is_matched=true` → `matched_clause`
- Map first `when_clauses[j]` where `is_matched=false` → `not_matched_clause`
- Map `MergeAction::Update{set_clauses}` → `MergeClause{update_columns, update_values}`
- Map `MergeAction::Insert{columns, values}` → `MergeClause{insert_columns, insert_values}`

#### Scenario: Simple MERGE with matched UPDATE only
- **WHEN** parser produces a MergeStatement with one `when_clauses[0]` where `is_matched=true` and `action=Update{set_clauses=[("t.val", sval)]}`
- **THEN** the conversion MUST produce a planner MergeStatement with:
  - `matched_clause: Some(MergeClause{update_columns=["t.val"], update_values=[sval], ...})`
  - `not_matched_clause: None`

#### Scenario: MERGE with both matched and not matched
- **WHEN** parser produces a MergeStatement with two `when_clauses` (one matched UPDATE, one not matched INSERT)
- **THEN** the conversion MUST produce a planner MergeStatement with both clauses populated

## Out of Scope

- INSERT/UPDATE/DELETE DML execution (only MERGE in this dispatcher)
- MERGE with DELETE action (parser supports, planner doesn't)
- MERGE with subquery source (parser supports, conversion not implemented)
- Replacing orphan `local_executor.rs` (separate refactor)
- Production caller for `execute_dml` (e.g., REPL/server integration)
