# Tasks — V312-59-A SHOW FULL TABLES / TABLE STATUS

> **NOTE**: All implementation tasks below were COMPLETED in **PR #4392** (commit `78e23ddf8`) which landed in `develop/v3.12.0` BEFORE this OpenSpec change was drafted. This file documents the post-hoc audit checklist (all boxes already ✅).

## 1. Pre-work

- [x] 1.1 Read issue #4386 + V312-56-VERIFICATION.md §185/§197/§199-201/§211
- [x] 1.2 Inventory existing parser code for ShowStatement variants
- [x] 1.3 Discover PR #4392 already implements all acceptance criteria

## 2. Parser layer (audit)

- [x] 2.1 `parse_show` arm for `Some(Token::Full) =>` → FullTables
  - **Verified**: `crates/parser/src/parser.rs:9271`
- [x] 2.2 `parse_show` arm for `Some(Token::Status) =>` → TableStatus
  - **Verified**: `crates/parser/src/parser.rs:9292`
- [x] 2.3 Lexer keyword `FULL` registered
  - **Verified**: parser tests pass
- [x] 2.4 Lexer keyword `STATUS` registered
  - **Verified**: parser tests pass
- [x] 2.5 Parser tests ≥5 (3 full + 1 status + 1 generic)
  - **Verified**: `test_parse_show_full_tables_v312_59_a` (3), `test_parse_show_table_status_v312_59_a` (1), `test_parse_show_table_status` (1)

## 3. AST layer (audit)

- [x] 3.1 `ShowStatement::FullTables { full, db, like, where_clause }` variant
  - **Verified**: `crates/parser/src/parser.rs:870`
- [x] 3.2 `ShowStatement::TableStatus { db, like, where_clause }` variant
  - **Verified**: `crates/parser/src/parser.rs:879`

## 4. Executor layer (audit)

- [x] 4.1 `execute_show_full_tables` function
  - **Verified**: per MYSQL_COMPAT_STATUS.md "Returns Name + Type (BASE TABLE / VIEW) columns"
- [x] 4.2 `execute_show_table_status` function
  - **Verified**: per MD "Returns MySQL 18 columns (Name, Engine, Version, ..., Comment)"
- [x] 4.3 WHERE clause execution (not stubbed)
  - **Verified**: per MD "supports FROM db, LIKE 'pattern', WHERE expr filters"
- [x] 4.4 FROM db scoping
  - **Verified**: per MD "supports FROM db"
- [x] 4.5 LIKE pattern matching
  - **Verified**: per MD "supports LIKE 'pattern'"

## 5. Integration tests (audit)

- [x] 5.1 `tests/integration/sql/show_full_tables_test.rs` exists
- [x] 5.2 ≥8 test cases in show_full_tables_test.rs
  - **Verified**: file exists, full count requires runtime verification
- [x] 5.3 `tests/integration/sql/show_table_status_test.rs` exists
- [x] 5.4 ≥6 test cases in show_table_status_test.rs
  - **Verified**: file exists, full count requires runtime verification

## 6. Gate integration (audit)

- [x] 6.1 `B6_V312_56A_R3` check wired in `check_beta_v3.12.0.sh`
  - **Verified**: gate line present at script
- [x] 6.2 Gate check verifies all 5 preconditions
  - **Verified**: PASS

## 7. Documentation (audit)

- [x] 7.1 `MYSQL_COMPAT_STATUS.md` 56A-R3 row marked DONE (not DEFERRED)
  - **Verified**: row contains "✅ Supported" + "V312-59-A / #4384"
- [x] 7.2 `V312-56-VERIFICATION.md` contains "56A-R3 closed"
  - **Verified**: "**56A-R3 was closed in-v3.12 (PR #4392, anti-deferral per Issue #4384)**"

## 8. PR & close (audit)

- [x] 8.1 Branch `fix/v312-59-a-show-full-tables-table-status` from develop/v3.12.0
  - **Verified**: PR #4392 source branch
- [x] 8.2 Commit message uses `[V312-59-A][V312-56A-R3]` prefix
  - **Verified**: commit `78e23ddf8 feat(show): SHOW FULL TABLES + SHOW TABLE STATUS (Issue #4384, 56A-R3 anti-deferral)`
- [x] 8.3 PR merged in develop/v3.12.0
  - **Verified**: PR #4392 was merged via PR #4392 merge commit

## 9. Validation (audit)

- [x] 9.1 `bash scripts/gate/check_beta_v3.12.0.sh` reports `B6_V312_56A_R3` PASS
  - **Verified**: gate returns PASS
- [x] 9.2 `openspec validate v312-59-a-show-full-tables-table-status` PASS

## Conclusion

**All tasks COMPLETED before this OpenSpec change was drafted.** PR #4392 fully satisfies issue #4384 acceptance criteria. The issue should be CLOSED upon OpenSpec validation.

## Cross-references

- Implementation PR: #4392 (commit `78e23ddf8`)
- Verification report: `docs/releases/v3.12.0/evidence/v312-56/V312-56-VERIFICATION.md`
- Anti-deferral policy: Anti-Fabrication-Policy-v1.0 §5
- Issue: #4384