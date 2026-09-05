## 1. Code change

- [ ] 1.1 In `crates/storage/src/file_storage.rs:3199` (`fn add_column`), add a row-backfill loop between `data.info.columns.push(column)` and `self.save_table(table, &table_data)?` (or equivalent):
  ```rust
  if let Some(data) = self.tables.get_mut(table) {
      data.info.columns.push(column);
      // #4647: backfill every existing row with the new column's
      // DEFAULT (or Value::Null when no default is specified) so the
      // schema and row layout stay aligned.
      let fill = super::super::engine::default_fill_value(
          &data.info.columns.last().map(|c| c.default_value.clone()).unwrap_or(None),
      );
      for row in data.rows.iter_mut() {
          row.push(fill.clone());
      }
      let table_data = data.clone();
      self.save_table(table, &table_data)?;
  }
  Ok(())
  ```
  Note: `data.info.columns` already has the new column pushed, so `last()` returns the new one. `default_fill_value` is `pub(crate)` accessible from `crate::storage::file_storage`.

## 2. Tests

- [ ] 2.1 Add 4 integration tests in `tests/integration/sql/v312_72_alter_table_add_column_test.rs` covering the spec scenarios.

## 3. Documentation and verification

- [ ] 3.1 Run `cargo build --all-features` and confirm clean.
- [ ] 3.2 Run `cargo test -p sqlrustgo-cli --all-features --lib` and confirm 73/73.
- [ ] 3.3 Run `cargo test --test v312_72_alter_table_add_column_test` and confirm 4/4.
- [ ] 3.4 Manual CLI repro from issue body and confirm `SELECT *` shows both columns.
