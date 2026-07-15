## Context

SEM-3 (ALTER TABLE RENAME/MODIFY 完整) has been OPEN since v3.0.0 era. v3.10.0 added the `MODIFY` keyword to the lexer (PR #3773) but the executor remained a stub for several operations.

Current state of `src/engine_ddl.rs::execute_alter_table` (lines 467-515):
- ✅ `AddColumn` — calls `storage.add_column()` (works in MemoryStorage)
- ✅ `DropColumn` — calls `storage.drop_column()` (works in MemoryStorage)
- ✅ `ModifyColumn` — calls `storage.modify_column()` (works in MemoryStorage)
- ✅ `RenameTo` — calls `storage.rename_table()` (works in MemoryStorage)
- ✅ `RenameColumn` — calls `storage.rename_column()` (works in MemoryStorage)

All 5 ALTER TABLE operations are already wired up at the executor level, AND MemoryStorage implements all of them. The remaining work is:
1. **Verify** the implementations work correctly via tests
2. **Audit** FileStorage for parity (production engine)
3. **Add** integration tests for the new operations
4. **Update** debt-registry to close SEM-3

**Architecture overview**: 
- `crates/executor` (in `src/engine_ddl.rs`) is the entry point for DDL
- `crates/storage/src/engine.rs` defines the `StorageEngine` trait and `MemoryStorage` impl
- `crates/storage/src/file_storage.rs` provides the production `FileStorage` impl
- `tests/integration/ddl/alter_table_test.rs` is the existing integration test file

## Goals / Non-Goals

**Goals:**
- Close SEM-3 debt by verifying all 5 ALTER TABLE operations work end-to-end
- Add 6 new integration tests covering RENAME/MODIFY
- Ensure FileStorage parity with MemoryStorage for these operations
- Update `debt-registry.yaml` to mark SEM-3 CLOSED

**Non-Goals:**
- No new ALTER TABLE features (e.g., ALTER TABLE...SET TABLESPACE)
- No GUI/schema migration tooling
- No breaking changes to the StorageEngine trait signature
- No changes to MySQL wire protocol response format

## Decisions

### 1. Use `cargo test --test alter_table_test` for verification

The existing test file `tests/integration/ddl/alter_table_test.rs` already uses the `run_repl()` helper to drive a real `sqlrustgo-mysql-server` process via stdin. Reuse this infrastructure to test the new operations end-to-end.

### 2. Test positional semantics for `rename_column`

In `MemoryStorage`, records are stored as `Vec<Vec<Value>>` (positional). When a column is renamed, the data does NOT need migration — only the column name in `TableInfo` changes. The existing implementation correctly handles this. Tests must verify that SELECT * after RENAME COLUMN returns data with the new column name and same data.

### 3. FileStorage parity via shared trait methods

The `StorageEngine` trait has default implementations for `drop_column`, `rename_column`, `modify_column` that return an error. `MemoryStorage` overrides these. If `FileStorage` doesn't override them, the default error will be returned, which is the correct conservative behavior. We document this but don't necessarily fix it (FileStorage tests use MemoryStorage by default in most integration tests).

### 4. No schema migration for column type changes

`modify_column` in `MemoryStorage` just updates the `ColumnDefinition` but does NOT migrate existing row data. This is consistent with MySQL 5.7 behavior when the change is type-compatible (e.g., INT to BIGINT). For incompatible changes, the server may fail at insert/select time. This is acceptable for v3.11.0 scope.

### 5. Test isolation via fresh REPL

Each integration test spawns a fresh `sqlrustgo-mysql-server repl` process via `Command::new()`, so tests don't interfere with each other. The `run_repl()` helper handles this automatically.

## Risks

| Risk | Mitigation |
|------|------------|
| FileStorage missing impls (returns error from trait default) | Tests use MemoryStorage by default; document behavior |
| ALTER TABLE on table with data — could lose data if not careful | Test data preservation explicitly |
| Column rename in SELECT — clients may cache schema | Add explicit DESC test after rename |

## Migration Plan

1. Audit FileStorage for parity (Phase 2 of tasks)
2. If gaps exist, implement them in FileStorage using `crates/storage/src/file_storage.rs`
3. Run existing test suite to verify no regression
4. Add 6 new tests for RENAME/MODIFY operations
5. Update debt-registry.yaml to mark SEM-3 CLOSED
6. Update v3.11.0 plan docs to mark V311-13 DONE
