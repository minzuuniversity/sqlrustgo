## Why

ISSUE #4024 (V312-F-1): v312_13 typed-wrappers test FAIL.

Per `check_v312_13_wire_load_data.sh` step 02 (2026-08-10T15:50:54Z):
- Test: `v312_13_reset_connection_ok`
- Location: `tests/integration/wire/v312_13_typed_wrappers_test.rs:44:37`
- Error: `post-reset SELECT 1: Error("unexpected response (seq=1, first=0x01): ")`
- Root cause: COM_RESET_CONNECTION (0x1F) handler does not properly reset packet sequence and session state in server, causing the next query packet to be parsed with stale state.

Evidence hash: `3c11bd118249eea2755fefc3a7d569473cddec48d42a34810b4ac75b1440eb6b`

## What Changes

- **Server**: `crates/mysql-server/src/lib.rs` — fix COM_RESET_CONNECTION handler to reset:
  - Packet sequence counter (server-side and client-expected)
  - Connection state (charset, status flags, prepared statements)
  - Session variables
- **Client**: `crates/mysql-client/src/lib.rs` — after COM_RESET_CONNECTION response, re-send handshake init packets correctly
- **Test**: add regression test that verifies SELECT 1 works immediately after COM_RESET_CONNECTION

## Impact

- Modified: `crates/mysql-server/src/lib.rs` (~30 lines for reset handler)
- Modified: `crates/mysql-client/src/lib.rs` (~10 lines for re-handshake)
- Modified: `tests/integration/wire/v312_13_typed_wrappers_test.rs` (add 1 regression test)
- No spec-level API changes

## Acceptance criteria

- [ ] `cargo test --test v312_13_typed_wrappers_test -- --test-threads=1` 全部 PASS (22/22)
- [ ] `bash scripts/gate/check_v312_13_wire_load_data.sh` step 02 status=pass
- [ ] 重新生成 evidence_hash 并提交 PR
- [ ] ISSUE #4024 comment 含 PR#, SHA, evidence_hash, 修复说明

## Risk

Low. The change is localized to COM_RESET_CONNECTION handler and the corresponding client-side packet handling. The MySQL protocol reset is well-documented and the fix follows the standard pattern (sequence=0, status=0x02, charset=server default).

## References

- ISSUE #4024 (F-1)
- ISSUE #3887 (V312-MASTER) 关闭条件 #4
- ISSUE #3959 (V312-24) deferred items tracker
- log: `docs/releases/v3.12.0/evidence/wire_load_data/02-typed-wrappers.log`