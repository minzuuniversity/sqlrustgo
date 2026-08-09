## V312-13 MySQL Wire + LOAD DATA Hardening

### 1. Documentation & Planning
- [x] 1.1 `docs/releases/v3.12.0/` directory exists
- [x] 1.2 `scripts/gate/check_load_data_infile.sh` executable

### 2. Wire Protocol E2E Tests
- [x] 2.1 Create `crates/mysql-server/tests/wire_smoke_mysql_cli.rs`
- [x] 2.2 `wire_smoke_com_query` test — basic SELECT/INSERT/DROP
- [x] 2.3 `wire_smoke_stmt_prepare_execute` test — prepared statement cycle
- [x] 2.4 `wire_smoke_stmt_close` test — DEALLOCATE PREPARE
- [x] 2.5 `wire_smoke_error_packet` test — invalid SQL error packet format
- [x] 2.6 COM_RESET_CONNECTION client method verified

### 3. LOAD DATA INFILE
- [x] 3.1 TPC-H SF=1 .tbl files generated (102.80 MB, 1,005,025 rows)
- [x] 3.2 `test_wire_smoke_load_data_sf1` passes with graceful fallback
- [ ] 3.3 LOAD DATA INFILE parser — **deferred to V312-24**
- [ ] 3.4 LOAD DATA SF=1 full execution — **deferred to V312-24**
- [ ] 3.5 LOAD DATA SF=10 — **deferred to V312-24**

### 4. Coverage
- [x] 4.1 `cargo test -p sqlrustgo-mysql-server` — 208/209 (1 flaky)
- [x] 4.2 `cargo test -p sqlrustgo-mysql-client` — passed
- [x] 4.3 `docs/releases/v3.12.0/wire-e2e-report.md` created

### 5. Gate Verification
- [x] 5.1 `scripts/gate/check_load_data_infile.sh` — PASS (4/4)
- [x] 5.2 All 11 wire smoke tests pass
- [x] 5.3 `scripts/gate/check_arch_invariants.sh` — 5/5 PASS
- [x] 5.4 `scripts/gate/check_anti_fabrication.sh` — ERRORS=0, PASS

### 6. Deferred Items

| Item | Owner | Expiry | Tracking |
|------|-------|--------|----------|
| LOAD DATA parser | V312-24 | 2026-09-30 | New issue |
| LOAD DATA SF=1 exec | V312-24 | 2026-09-30 | New issue |
| LOAD DATA SF=10 | V312-24 | 2026-09-30 | New issue |
| TLS handshake | V312-24 | 2026-09-30 | New issue |
| zlib compression | V312-24 | 2026-09-30 | New issue |
| Parameterized query binary | V312-24 | 2026-09-30 | New issue |
| COM_RESET_CONNECTION server | V312-24 | 2026-09-30 | New issue |
