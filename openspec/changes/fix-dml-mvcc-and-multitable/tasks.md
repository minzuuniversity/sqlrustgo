## 1. MemoryStorage transaction support

- [ ] 1.1 Add `tx_log: Option<Vec<RowSnapshot>>` and `tx_active: bool`
      fields to `MemoryStorage` (with `Default` impl initialising both to
      `None` / `false`).
- [ ] 1.2 Replace `MemoryStorage::begin_transaction` so it returns the
      next `u64` handle on first call and errors on a nested `begin`.
      Initialise `tx_log` and set `tx_active = true`.
- [ ] 1.3 Make `MemoryStorage::commit_transaction` clear `tx_log`,
      set `tx_active = false`, and return `Ok(())`.
- [ ] 1.4 Make `MemoryStorage::rollback_transaction` restore every
      row snapshot recorded in `tx_log`, clear `tx_log`, and set
      `tx_active = false`.
- [ ] 1.5 Instrument `MemoryStorage::insert` / `update` / `delete` so
      each affected row is pushed onto `tx_log` while `tx_active` is
      true. For `insert` push the pre-image as `Insert { table, row }`
      so rollback can remove it. For `update` / `delete` push the prior
      row so rollback can replace / re-insert it.
- [ ] 1.6 Add `tx_id: u64` counter field so `begin_transaction`
      returns a unique id each call.

## 2. UPDATE/DELETE Statement multi-table AST

- [ ] 2.1 Add `pub struct TableRef { pub name: String, pub alias:
      Option<String> }` in `crates/parser/src/parser.rs`.
- [ ] 2.2 Change `UpdateStatement.table: String` to `UpdateStatement.tables:
      Vec<TableRef>`. Update the single constructor site in
      `parse_update` to wrap the existing string in a 1-element vec.
- [ ] 2.3 Change `DeleteStatement.table: String` to `DeleteStatement.tables:
      Vec<TableRef>` and add `DeleteStatement.using: Option<Vec<TableRef>>`.
      Update `parse_delete` accordingly.
- [ ] 2.4 Update `parse_update` to accept `<table>[, <table>]*` and
      `parse_delete` to accept `DELETE <tables> FROM <tables> [WHERE …]`
      (multiple comma-separated names in both positions).
- [ ] 2.5 Update `local_executor.rs` and any other consumer that reads
      `update.table` / `delete.table` to use `tables[0]` (single-table
      fast path stays correct because `parse_update` wraps in a
      1-element vec).

## 3. Multi-table UPDATE / DELETE executor

- [ ] 3.1 In `execute_update`, when `update.tables.len() > 1`, build a
      combined schema across the listed tables (re-using the helper
      already used by `engine_select::build_combined_schema` for
      cartesian joins). Resolve each SET clause's column index in the
      combined schema.
- [ ] 3.2 Iterate the cartesian product of rows from the listed
      tables; for each combined row that matches `update.where_clause`
      (after the existing pre-resolve step), build the new combined
      row by applying SET clauses; then split the new combined row
      back into per-table rows and write them to storage.
- [ ] 3.3 Same split-and-write strategy for the `update.tables.len() ==
      1` path so the code is uniform; the cartesian product collapses
      to a single scan.
- [ ] 3.4 Mirror steps 3.1–3.3 in `execute_delete`: build combined
      schema, scan joined rows, mark matches, remove from each per-table
      row-set.
- [ ] 3.5 Run `cargo test --test dml_integration_test update_multiple_tables
      delete_multiple_tables --all-features` and confirm both pass.

## 4. Un-ignore the four tests

- [ ] 4.1 In `tests/dml_integration_test.rs`, remove the `#[ignore = …]`
      attributes above `transaction_rollback_undoes_dml`,
      `transaction_update_then_rollback`, `update_multiple_tables`,
      and `delete_multiple_tables`.
- [ ] 4.2 Run `cargo test --test dml_integration_test --all-features`
      and confirm 24 passed / 0 failed / 0 ignored.

## 5. Commit and push

- [ ] 5.1 `cargo check --all-features --workspace` passes (no new
      warnings beyond the existing `dead_code` notes).
- [ ] 5.2 `cargo fmt --all` clean.
- [ ] 5.3 Commit with message referencing this change and issue
      #3621 (the original pre-existing-failures tracking issue).
- [ ] 5.4 Fast-forward push to `250/develop/v3.9.0` (admin override,
      since self-approve is blocked on Gitea).