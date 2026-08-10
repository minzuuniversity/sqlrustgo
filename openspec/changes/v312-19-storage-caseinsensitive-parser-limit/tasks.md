## 1. Storage case-insensitive fix

- [x] 1.1 Modify `crates/storage/src/engine.rs` `get_table_info` to use `to_lowercase()` on key
- [x] 1.2 Modify `has_table` similarly
- [x] 1.3 Modify `rename_table` similarly
- [x] 1.4 Modify any other `table_infos.get/remove/insert` (drop_column, add_column, ALTER)
- [x] 1.5 Verify no regression in existing storage tests

## 2. Parser LIMIT expression fix

- [x] 2.1 Modify `crates/parser/src/parser.rs` LIMIT parser to accept expression
- [x] 2.2 Add `constant_fold_u64` helper to evaluate expression at parse time
- [x] 2.3 Apply same fix to OFFSET
- [x] 2.4 Verify no regression in existing parser tests

## 3. Test verification

- [x] 3.1 `case_insensitive_alter.test` PASS
- [x] 3.2 `order__test_limit.test` PASS
- [x] 3.3 `cargo test -p sqlrustgo-storage` all pass
- [x] 3.4 `cargo test -p sqlrustgo-parser` all pass

## 4. PR + merge

- [x] 4.1 Create PR with both fixes
- [x] 4.2 hermes-z6g4 review
- [x] 4.3 Merge to develop/v3.12.0

## 5. Closure

- [x] 5.1 Post evidence comment to #3972
- [x] 5.2 Close #3972
- [x] 5.3 Update #3887 master checklist
- [x] 5.4 Sync 252↔250
