## Why

SEM-3 (ALTER TABLE 不完整) 在 v3.10.0 仍是 OPEN 状态。`engine_ddl.rs::execute_alter_table` 已经引用了 `storage.rename_table()`, `storage.rename_column()`, `storage.modify_column()` 等方法, 而 `MemoryStorage` 也已经实现了这些方法 (在 `crates/storage/src/engine.rs:541-1055`)。但没有相应的测试覆盖, 且 `FileStorage` (生产用) 可能未实现这些方法 (issue #3575 报告路径)。Issue #3434 (V311-13) 需要:
- 验证 `MemoryStorage` 实现正确性 (单元测试)
- 确认 `FileStorage` 实现一致性
- 添加集成测试覆盖 RENAME TABLE, RENAME COLUMN, MODIFY COLUMN
- 解除 6 个 `#[ignore]` 测试的 IGNORE 状态

## What Changes

- **Verify** existing `MemoryStorage` implementations of `rename_table`, `rename_column`, `modify_column`
- **Audit** `FileStorage` for the same methods; implement if missing
- **Add** integration tests in `tests/integration/ddl/alter_table_test.rs` for:
  - `test_alter_table_rename_table`: RENAME TABLE old_name TO new_name
  - `test_alter_table_rename_column`: RENAME COLUMN old_name TO new_name
  - `test_alter_table_modify_column`: MODIFY COLUMN col_name data_type [NOT NULL | NULL]
  - `test_alter_table_rename_preserves_data`: Rename and verify rows are preserved
  - `test_alter_table_modify_preserves_data`: Modify and verify values
  - `test_alter_table_chain_renames`: Multiple renames in sequence
- **Unignore** 6 existing `#[ignore]` tests if they exist (per V311-13 acceptance)
- **Update** `debt-registry.yaml`: SEM-3 state IN_PROGRESS → CLOSED

## Capabilities

### Modified Capabilities

- `alter-table-rename-table`: SQL `ALTER TABLE old_name RENAME TO new_name` actually renames table in catalog and moves all data, not just a stub
- `alter-table-rename-column`: SQL `ALTER TABLE t1 RENAME COLUMN old_name TO new_name` actually renames column in schema
- `alter-table-modify-column`: SQL `ALTER TABLE t1 MODIFY COLUMN col_name NEW_TYPE [NOT NULL|NULL]` actually changes column type and nullability

## Impact

- **Modified**: `crates/storage/src/engine.rs` — verify or fix `MemoryStorage` methods (lines 541-1055)
- **Modified**: `crates/storage/src/file_storage.rs` — verify or add `rename_table`, `rename_column`, `modify_column` impls
- **Modified**: `tests/integration/ddl/alter_table_test.rs` — add 6 new tests
- **Modified**: `src/engine_ddl.rs:505-512` — verify error handling for non-existent columns
- **Modified**: `docs/governance/debt/debt-registry.yaml` — update SEM-3 state

## Acceptance Criteria

- All 6 new integration tests PASS
- C-4b / C-4c / C-4d (per V311-13 plan) all PASS
- `cargo test -p sqlrustgo` shows 0 new failures
- FileStorage renamed tables and columns correctly (if testable in unit test)
- SEM-3 state IN_PROGRESS → CLOSED in debt-registry.yaml
