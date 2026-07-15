## 1. Root cause investigation

- [x] 1.1 Reproduce "Lost connection" error locally (FAILED - no such error found)
- [x] 1.2 Investigate INSERT failure - found parse error on INSERT IGNORE
- [x] 1.3 Verify plain INSERT works in concurrent context
- [x] 1.4 Identify the 3 contributing factors: parser, ID range, OLTP script

## 2. Parser fix: Add INSERT IGNORE support

- [x] 2.1 Add `Token::Ignore` variant in `crates/parser/src/token.rs:50`
- [x] 2.2 Add `Token::Ignore => write!(f, "IGNORE")` in Display impl
- [x] 2.3 Add `"IGNORE" => Some(Token::Ignore)` in keyword map
- [x] 2.4 Add test assertion in token.rs
- [x] 2.5 Add `"IGNORE" => Token::Ignore` in lexer keyword map `crates/parser/src/lexer.rs:277`
- [x] 2.6 Build - 0 errors

## 3. AST change: Add is_ignore field

- [x] 3.1 Add `pub is_ignore: bool` to `InsertStatement` in parser.rs:507
- [x] 3.2 Update `parse_insert` to consume `Token::Ignore` after Insert
- [x] 3.3 Update InsertStatement construction to set is_ignore

## 4. Executor fix: Handle is_ignore

- [x] 4.1 Update `src/engine_dml.rs` duplicate check to skip when is_ignore
- [x] 4.2 Add `odku_handled_indices.insert(new_idx)` to mark as handled
- [x] 4.3 Build - 0 errors

## 5. OLTP orchestrator improvement

- [x] 5.1 Add per-thread script copy in orchestrator_v2.sh
- [x] 5.2 Use THREAD_ID for unique ID range in OLTP workload
- [x] 5.3 Use INSERT IGNORE in OLTP script for safety

## 6. Regression test suite

- [x] 6.1 Create `tests/integration/stress/concurrent_insert_test.rs`
- [x] 6.2 Add 6 integration tests covering all INSERT variants
- [x] 6.3 Add to Cargo.toml `[[test]]` section
- [x] 6.4 6/6 tests PASS

## 7. Documentation updates

- [x] 7.1 Update `debt-registry.yaml`: PERF-5 state OPEN → CLOSED
- [x] 7.2 Update `FEATURE_CHECKLIST.md`: V311-23 status ✅ DONE
- [x] 7.3 Update `V311_DEVELOPMENT_PLAN.md`: V311-23 status
- [x] 7.4 Comment on Issue #3434 (V311-MASTER) with completion link

## 8. PR and merge

- [x] 8.1 Create PR on Gitea 250
- [ ] 8.2 Get 2 approvals
- [ ] 8.3 Force-merge (admin)
- [ ] 8.4 Sync to gitcode + gitee
- [ ] 8.5 Update Issue #3434 with PR link
