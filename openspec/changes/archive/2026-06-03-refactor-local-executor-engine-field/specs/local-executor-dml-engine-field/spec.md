# Spec: local-executor-dml-engine-field

## ADDED Requirements

### Requirement: LocalExecutorDml MUST Hold Arc<Mutex<dyn ExecutionEngine>>

The `LocalExecutorDml` struct at `crates/executor/src/local_executor_dml.rs` MUST include an `engine` field of type `Arc<Mutex<dyn ExecutionEngine>>`. This field MUST be accessible (cloneable) so it can be passed to `MergeExecutor::new(engine, ...)` in G3 (#2810).

#### Scenario: LocalExecutorDml exposes engine Arc
- **WHEN** `LocalExecutorDml::new()` is invoked (default constructor)
- **THEN** the returned struct MUST have a valid `engine` field
- **AND** `local_executor_dml.engine()` MUST return a cloneable `Arc<Mutex<dyn ExecutionEngine>>`
- **AND** the returned Arc MUST be referenceable by another consumer (e.g., G3's MergeExecutor::new)

#### Scenario: LocalExecutorDml::with_engine accepts explicit engine
- **WHEN** `LocalExecutorDml::with_engine(engine)` is invoked with `engine: Arc<Mutex<dyn ExecutionEngine>>`
- **THEN** the struct MUST store the exact provided Arc (no replacement)
- **AND** `local_executor_dml.engine()` MUST return an Arc pointing to the same underlying engine

#### Scenario: engine Arc is cloneable (sharing semantics)
- **WHEN** two callers invoke `local_executor_dml.engine()` separately
- **THEN** both returned Arcs MUST point to the same underlying engine
- **AND** `Arc::ptr_eq(&arc1, &arc2)` MUST return `true`

### Requirement: NoopExecutionEngine Adapter for Default Constructor

A private `NoopExecutionEngine` struct MUST be defined to satisfy the `ExecutionEngine` trait for the default `new()` constructor. This adapter returns `Ok(ExecutionResult::ok(0))` for `execute()` and `Err` for `begin/commit/rollback`. G3 (#2810) will replace this default with a real engine via `with_engine()`.

#### Scenario: NoopExecutionEngine::execute returns Ok
- **WHEN** `NoopExecutionEngine::execute(&mut self, _ctx)` is called
- **THEN** it MUST return `Ok(ExecutionResult::ok(0))`

#### Scenario: NoopExecutionEngine::begin returns Err
- **WHEN** `NoopExecutionEngine::begin(&mut self)` is called
- **THEN** it MUST return `Err(SqlError)` indicating not-yet-implemented

#### Scenario: NoopExecutionEngine::commit returns Err
- **WHEN** `NoopExecutionEngine::commit(&mut self, _txn)` is called
- **THEN** it MUST return `Err(SqlError)` indicating not-yet-implemented

#### Scenario: NoopExecutionEngine::rollback returns Err
- **WHEN** `NoopExecutionEngine::rollback(&mut self, _txn)` is called
- **THEN** it MUST return `Err(SqlError)` indicating not-yet-implemented

### Requirement: Existing Send Test Adapts to Engine Field

The `test_send_sync` test MUST acknowledge that `LocalExecutorDml` is no longer `Send` (because `Arc<Mutex<dyn ExecutionEngine>>` requires `ExecutionEngine: Send`, which the trait does not bound). The test MUST only verify `LocalExecutorDmlArc` for Send + Sync.

#### Scenario: test_send_sync only checks LocalExecutorDmlArc
- **WHEN** `test_send_sync` runs
- **THEN** only `LocalExecutorDmlArc` is checked for `Send + Sync`
- **AND** `LocalExecutorDml` is NOT checked (acknowledged as potentially !Send due to engine trait bound)

## Out of Scope

- The actual `execute_merge()` call from executor dispatch (G3 #2810)
- Migrating the orphan `local_executor.rs` (2579 lines) into lib.rs
- Adding `Send + Sync` to `ExecutionEngine` trait (would change API across all callers)
