## Why

ISSUE #4025 (V312-F-2): v312_13 e2e_wire_protocol 9 tests FAIL.

Per `check_v312_13_wire_load_data.sh` step 05 (2026-08-10T15:50:54Z),
`crates/mysql-server/tests/e2e_wire_protocol.rs` produces 9 failures out of 46 tests:

| Test | Failure |
|------|---------|
| test_e2e_delete | assertion: 56 != 2 |
| test_e2e_drop_table | assertion: 28 != 1 |
| test_e2e_group_by_aggregates | assertion: "8400" != "300" |
| test_e2e_in_operator | assertion: 56 != 2 |
| test_e2e_insert_multiple_rows | assertion: "84" != "3" |
| test_e2e_is_null | assertion: "56" != "2" |
| test_e2e_null_handling | assertion: 28 != 1 |
| test_e2e_order_by | assertion: 270 != 5 |
| test_e2e_update | assertion: 61 != 3 |

Pattern: tests receive the row count from a PREVIOUS test's state because
they share a process-global `SERVER_POOL`. Tests that run after a test with
N affected rows incorrectly read N as their own row count.

Evidence hash: `f83397ab10bdf48130b8c8f8439a2f1a674f02f1bbb065333d3616f465d3fdb6`

## What Changes

- **Server tests**: `crates/mysql-server/tests/e2e_wire_protocol.rs` — add `DROP TABLE IF EXISTS` for any pre-existing table names before each test, OR use fully isolated schemas
- **Server pool**: investigate `SERVER_POOL` global state in test harness — if it's a `static` / `OnceCell`, ensure each test gets a fresh state

## Impact

- Modified: `crates/mysql-server/tests/e2e_wire_protocol.rs` (test-only)
- No production code changes
- No spec-level changes

## Acceptance criteria

- [ ] `cargo test -p sqlrustgo-mysql-server --test e2e_wire_protocol -- --test-threads=1` 46/46 PASS
- [ ] `bash scripts/gate/check_v312_13_wire_load_data.sh` step 05 status=pass
- [ ] 重新生成 evidence_hash 并提交 PR
- [ ] ISSUE #4025 comment 含 PR#, SHA, evidence_hash

## Risk

Medium. The fix involves test isolation which can have subtle side effects. However, since all tests already use unique table names (tdeld, t2, tgrp, etc.), the issue is likely the SERVER_POOL being a process-global that accumulates state.

## References

- ISSUE #4025 (F-2)
- ISSUE #3887 (V312-MASTER) 关闭条件 #4
- log: `docs/releases/v3.12.0/evidence/wire_load_data/05-e2e-wire-protocol.log`