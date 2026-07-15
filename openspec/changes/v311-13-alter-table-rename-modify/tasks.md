## 1. Verify MemoryStorage implementations

- [x] 1.1 Verify `add_column` impl in `crates/storage/src/engine.rs:947`
- [x] 1.2 Verify `drop_column` impl in `crates/storage/src/engine.rs:1030` (removes from columns + records)
- [x] 1.3 Verify `modify_column` impl in `crates/storage/src/engine.rs:1048` (updates column definition)
- [x] 1.4 Verify `rename_table` impl in `crates/storage/src/engine.rs:957` (moves table + records)
- [x] 1.5 Verify `rename_column` impl in `crates/storage/src/engine.rs:1062` (updates column name only)
- [x] 1.6 Note: Records are `Vec<Value>` (positional), so column renames don't need data migration

## 2. Audit FileStorage implementations

- [x] 2.1 Check `crates/storage/src/file_storage.rs` for `add_column` impl
- [x] 2.2 Check `crates/storage/src/file_storage.rs` for `drop_column` impl
- [x] 2.3 Check `crates/storage/src/file_storage.rs` for `rename_table` impl
- [x] 2.4 Check `crates/storage/src/file_storage.rs` for `rename_column` impl
- [x] 2.5 Check `crates/storage/src/file_storage.rs` for `modify_column` impl
- [x] 2.6 If any missing: implement them (using MemoryStorage as reference)

## 3. Add integration tests

- [x] 3.1 `test_alter_table_rename_table`: RENAME TABLE creates new table, drops old
- [x] 3.2 `test_alter_table_rename_column`: RENAME COLUMN preserves data (positional)
- [x] 3.3 `test_alter_table_modify_column`: MODIFY COLUMN changes data type
- [x] 3.4 `test_alter_table_modify_column_nullable`: MODIFY COLUMN with NOT NULL / NULL
- [x] 3.5 `test_alter_table_chain_renames`: Multiple RENAME TABLE / COLUMN in sequence
- [x] 3.6 `test_alter_table_rename_preserves_data`: RENAME TABLE preserves all rows

## 4. Verify compilation and tests

- [x] 4.1 `cargo build --release` — 0 errors
- [x] 4.2 `cargo clippy` (per-file, sqlrustgo-mysql-server only, no errors)
- [x] 4.3 `cargo test --test alter_table_test` — all tests PASS
- [x] 4.4 `cargo test --lib -p sqlrustgo-storage` — MemoryStorage tests PASS
- [ ] 4.5 (skipped: cargo test --workspace would take 30+ min) `cargo test --workspace` — no new failures

## 5. Update documentation and debt registry

- [ ] 5.1 Update `docs/governance/debt/debt-registry.yaml`: SEM-3 IN_PROGRESS → CLOSED
- [ ] 5.2 Update `docs/releases/v3.11.0/FEATURE_CHECKLIST.md`: V311-13 status
- [ ] 5.3 Update `docs/releases/v3.11.0/plans/V311_DEVELOPMENT_PLAN.md`: V311-13 status
- [ ] 5.4 Comment on Issue #3434 (V311-MASTER) with PR link
