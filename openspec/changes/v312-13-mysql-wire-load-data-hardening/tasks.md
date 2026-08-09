## 1. MySqlTestClient extensions

- [ ] 1.1 Add `prepare` (COM_STMT_PREPARE) returning `Result<u32, MysqlError>` in `tests/common/mod.rs`
- [ ] 1.2 Add `execute` (COM_STMT_EXECUTE) with binary-param encoding and row decode
- [ ] 1.3 Add `close_stmt` (COM_STMT_CLOSE) no-op-on-success
- [ ] 1.4 Add `reset_connection` (COM_RESET_CONNECTION) + post-reset `SELECT @@autocommit` verification
- [ ] 1.5 Add `force_tls` (capability negotiation + cipher echo)
- [ ] 1.6 Add `force_compress` (zlib compressed COM_QUERY, assert `compressed_len < raw_len`)
- [ ] 1.7 Add `expect_err` returning `(error_code, sqlstate, message)` for the ERR packet

## 2. Server observability

- [ ] 2.1 Add `wire_command_total{cmd=...}` counter in `do_command_loop`
- [ ] 2.2 Confirm `RUST_LOG=sqlrustgo_mysql_server=info` emits post-substitution SQL for COM_STMT_EXECUTE (per existing `binary-prepared-statement-roundtrip` spec)
- [ ] 2.3 Log negotiated cipher/algorithm when TLS or compression capability is granted

## 3. COM_QUERY E2E

- [ ] 3.1 New test `tests/wire_com_query.rs`: SELECT/INSERT/UPDATE/DELETE/DDL round-trip with row-count assertion
- [ ] 3.2 Assert at least 1 of each DML/DDL produced a wire `OK` packet

## 4. COM_STMT_* E2E

- [ ] 4.1 New test `tests/wire_stmt_prepare_execute_close.rs`: prepare `SELECT ?, ?`, execute with (1, 'a'), (2, 'b'), close
- [ ] 4.2 Reuse case: bad param count → ERR packet with code `ER_WRONG_ARGUMENTS`
- [ ] 4.3 Reuse case: parameter type mismatch → ERR packet with `ER_INVALID_PARAMETER_TYPE`

## 5. ERR packet contract

- [ ] 5.1 New test `tests/wire_error_packet.rs`: syntax error → ERR with sqlstate `42000`
- [ ] 5.2 Constraint violation (PK duplicate) → ERR with `23000`
- [ ] 5.3 Auth failure → ERR with `28000`

## 6. COM_RESET_CONNECTION

- [ ] 6.1 New test `tests/wire_reset_connection.rs`: set `@@autocommit=0`, run reset, verify `@@autocommit` reverts to server default
- [ ] 6.2 Verify prepared statements from before reset are no longer accessible

## 7. TLS handshake

- [ ] 7.1 New test `tests/wire_tls_handshake.rs` with `--ignored` (TLS is feature-gated)
- [ ] 7.2 Run an authenticated `SELECT 1` over the encrypted channel

## 8. Compression

- [ ] 8.1 New test `tests/wire_compression.rs` with `--ignored`
- [ ] 8.2 Assert `compressed_len < raw_len` for a query whose result set is > 1KB
- [ ] 8.3 Decompressed result matches the uncompressed result

## 9. LOAD DATA SF=1

- [ ] 9.1 Generate `lineitem_sf1.tsv` via `scripts/gate/generate_tpch_sf.py` (already exists; verify SF=1 row count = 6,001,215)
- [ ] 9.2 New test `tests/load_data_sf1.rs` with `--ignored`: load via `LOAD DATA LOCAL INFILE`, assert `COUNT(*) = 6001215`, `SHA256(...)` matches reference, peak RSS ≤ `LOAD_DATA_SF1_PEAK_RSS_MB` (default 4096), duration ≤ `LOAD_DATA_SF1_DURATION_S` (default 600)
- [ ] 9.3 Enforce memory cap in the executor batch loader: assert error message contains "memory cap exceeded" if cap is set to 1MB

## 10. LOAD DATA SF=10

- [ ] 10.1 Generate `lineitem_sf10.tsv` (60,013,775 rows)
- [ ] 10.2 New test `tests/load_data_sf10.rs` with `--ignored`: same as SF=1 with thresholds `LOAD_DATA_SF10_PEAK_RSS_MB` (default 12288) and `LOAD_DATA_SF10_DURATION_S` (default 3600)
- [ ] 10.3 Tag-only: gate runs SF=10 only on `v3.12.0-rc*` and `v3.12.0-ga*` tags

## 11. Gate script

- [ ] 11.1 New `scripts/gate/check_v312_13_wire_load_data.sh` orchestrating all 9 test invocations
- [ ] 11.2 Emit `docs/releases/v3.12.0/evidence/wire_load_data/V312-13-REPORT.md` with per-test status, SHA256, timestamp, source_run

## 12. PR

- [ ] 12.1 Open PR on Gitea 252 against `develop/v3.12.0`
- [ ] 12.2 Get 1 reviewer approval (governance requires ≥1 for develop-v3.12.0)
- [ ] 12.3 Force-merge (admin)
- [ ] 12.4 Sync to gitcode + gitee
- [ ] 12.5 Update ISSUE #3900 with PR link + V312-13-REPORT.md SHA256
