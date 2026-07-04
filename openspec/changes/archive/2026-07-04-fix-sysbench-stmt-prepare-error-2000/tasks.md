## 1. Diagnose root cause

- [x] 1.1 Run `cargo test --test stmt_execute_repro -- --nocapture` and capture the byte-level EXECUTE response so we know exactly which packet sequence the server emits
- [x] 1.2 Add temporary `tracing::info!("STMT EXECUTE pre-splice sql=\"...\"", ...)` line right before `replace_placeholders`
- [x] 1.3 Add temporary `tracing::info!("STMT EXECUTE post-splice sql=\"...\"", ...)` immediately after `replace_placeholders`
- [x] 1.4 Re-run the repro test; confirm the spliced SQL is `WHERE id = 3` (correct parameter substitution)
- [x] 1.5 Remove the diagnostic `tracing::info!` lines

## 2. Fix binary row framing bug

- [x] 2.1 Located bug in `write_binary_row`: `null_bytes = (row.len() + 9) / 8` over-allocates by 1 byte
- [x] 2.2 Fixed: changed math to `(row.len() + 7) / 8` per MySQL binary-protocol spec
- [x] 2.3 `write_column_def` already uses correct `lenenc_str` for catalog/schema/table fields per spec
- [x] 2.4 `cargo test --test stmt_execute_repro -- --nocapture` → `repro_stmt_execute_returns_malformed_packet` PASSES
- [x] 2.5 `cargo test -p sqlrustgo-mysql-server` — 141 tests PASS
- [x] 2.6 `cargo test --test prepared_stmt_params_test` — 8 tests PASS (no regression)
- [x] 2.7 `cargo test --test mysql_wire_protocol_test` — 28 tests PASS

## 3. Fix test client parser

- [x] 3.1 `tests/stmt_execute_repro.rs`: replaced ad-hoc first-EOF-exits with 4-phase state machine
- [x] 3.2 Phase 0 = col count; phase 1 = col defs; phase 2 = separator (EOF or OK); phase 3 = rows; terminator detected by `first_byte == 0xFE && len == 5` (EOF) or `first_byte == 0x00 && len == 7 && bytes[1..7] match OK pattern` (OK terminator)

## 4. Verify trailing-packet format

- [ ] 4.1 Check that `make_eof_packet(seq, 0x0002)` produces a 5-byte payload starting with `0xFE` ✓ (verified by code reading)
- [x] 4.2 Check that `make_deprecate_eof_ok_packet(seq, 0, 0, 0x0002, 0)` produces a 7-byte payload starting with `0x00` ✓ (verified by code reading)
- [ ] 4.3 In the COM_STMT_PREPARE handler, condition is correctly inverted ✓
- [x] 4.4 `*server_last_sent_seq = seq; seq = seq.wrapping_add(1);` ordering is BEFORE `write_to` to avoid lost sequence updates ✓

## 5. Investigate remaining sysbench 2000 error

- [x] 5.1 Verified that sysbench sends TLS upgrade request despite `--mysql-ssl=off` (capability 0x00bfaa8d has CLIENT_SSL bit set)
- [x] 5.2 Captured bytes via TLS proxy: sysbench does send STMT_PREPARE but MariaDB Connector C rejects with error 2000
- [ ] 5.3 **Outstanding investigation**: unit test passes but MariaDB Connector C rejects over TLS. Suspected either TLS framing issue (the existing PR #3694/#3696 fixes this) or a deeper protocol mismatch in PREPARE response format. Needs packet-level debugging with TLS key log to compare decrypted PREPARE response bytes against what MariaDB Connector C expects.
- [ ] 5.4 Future: rebuild MariaDB Connector C with debug logging or use frida/wireshark with TLS key log

## 6. Commit, push, archive

- [x] 6.1 `git add crates/mysql-server/src/lib.rs tests/stmt_execute_repro.rs`
- [x] 6.2 `git commit -m "fix(mysql-server): write_binary_row null bitmap sizing — makes COM_STMT_EXECUTE return rows"`
- [x] 6.3 `git push 252 fix-sysbench-stmt-prepare-error-2000 --force`
- [x] 6.4 PR #3699 created, merged at bcc8503d941b
- [x] 6.5 openspec change lives in `openspec/changes/fix-sysbench-stmt-prepare-error-2000/`

## Summary

The PRIMARY bug was the binary row null bitmap sizing (`write_binary_row`). That is fixed:
- `cargo test --test stmt_execute_repro` PASS (test was previously FAILED)
- `cargo test -p sqlrustgo-mysql-server` PASS (141 tests, no regression)
- PR merged into develop/v3.9.0

**Outstanding issue (tracked as followup, NOT blocking PR #3699)**:
sysbench 1.0.20 over TLS still fails with the same `mysql_stmt_prepare()` error 2000.
This is the original Issue #3694 territory (TLS drain of cipher records) and likely
requires server-side TLS key logging or packet-level tracing to nail down.
The previous PRs #3694 (root cause) + #3696 + #3698 + #3699 (this PR) together
close the unit-test gap and improve behavior but do not yet close sysbench end-to-end.
