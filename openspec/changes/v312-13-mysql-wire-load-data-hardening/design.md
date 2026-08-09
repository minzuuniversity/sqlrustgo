## Design

### 1. Test surface (`tests/common/mod.rs` — `MySqlTestClient`)

Extend the existing client (already a raw-protocol client per `wire-protocol-execution` spec) with:

- `pub fn prepare(&mut self, sql: &str) -> Result<u32, MysqlError>` — sends COM_STMT_PREPARE, returns statement id; asserts server replies with 0x00 (OK) followed by column/param definitions, parsing them.
- `pub fn execute(&mut self, stmt_id: u32, params: &[Value]) -> Result<Vec<Vec<String>>, MysqlError>` — sends COM_STMT_EXECUTE with binary-encoded params; reads back rows; supports NULL marker.
- `pub fn close_stmt(&mut self, stmt_id: u32) -> Result<(), MysqlError>` — sends COM_STMT_CLOSE; no response expected.
- `pub fn reset_connection(&mut self) -> Result<(), MysqlError>` — sends COM_RESET_CONNECTION; verifies OK; verifies subsequent `SELECT @@autocommit` returns the server default, not the pre-reset value.
- `pub fn force_tls(&mut self) -> Result<(), MysqlError>` — negotiates TLS via `mysql_native_password` + SSLRequest byte after handshake; cipher echoed into test log.
- `pub fn force_compress(&mut self) -> Result<(), MysqlError>` — sends compressed payload for COM_QUERY; asserts compressed_len < raw_len and decompressed result matches.
- `pub fn expect_err(&mut self, sql: &str) -> Result<MysqlError, MysqlError>` — issues a query, expects ERR packet (0xFF) with non-zero error code and SQLSTATE.

### 2. Server-side minimal touches (`crates/mysql-server/src/lib.rs`)

The existing `do_command_loop` already dispatches the 0x03 / 0x16 / 0x17 / 0x19 / 0x1F command bytes. The change here is **observability and testability**, not new functionality:

- Add a `RUST_LOG=sqlrustgo_mysql_server=info` trace line for each command *after* parameter substitution (already in place per `binary-prepared-statement-roundtrip` spec).
- Emit a counter `wire_command_total{cmd="..."}` so the gate script can assert each command executed at least N times in a test run.
- TLS/compression: when client requests either capability, log the negotiated cipher/algorithm and assert the request succeeded (not silently fell back).

### 3. LOAD DATA SF=1 / SF=10 fixtures

- Reuse the existing TPC-H lineitem generator (`scripts/gate/generate_tpch_sf.py`) to produce `lineitem_sf1.tsv` and `lineitem_sf10.tsv` with `|` separator (matching MySQL LOAD DATA default).
- A new `tests/load_data/sf1_ingest.rs` and `sf10_ingest.rs`:
  1. Start ephemeral server via `start_ephemeral`.
  2. `CREATE TABLE lineitem_ld (...)` with the TPC-H schema.
  3. `LOAD DATA LOCAL INFILE '<path>' INTO TABLE lineitem_ld FIELDS TERMINATED BY '|'`.
  4. `SELECT COUNT(*), SHA256(...) FROM lineitem_ld` and compare against the generator's reference hash.
  5. Assert peak RSS via `rss::peak_rss_mb()` ≤ `LOAD_DATA_SF1_PEAK_RSS_MB` (default 4096) / `LOAD_DATA_SF10_PEAK_RSS_MB` (default 12288).
  6. Assert duration ≤ `LOAD_DATA_SF1_DURATION_S` (default 600) / `LOAD_DATA_SF10_DURATION_S` (default 3600).
- The memory cap is enforced in the executor: any batch over the cap fails with a typed error and the test asserts the error message contains "memory cap exceeded".

### 4. Gate script

`scripts/gate/check_v312_13_wire_load_data.sh` runs in this order:

1. `cargo build -p sqlrustgo-mysql-server -p sqlrustgo-mysql-client` (fail fast).
2. `cargo test -p sqlrustgo-mysql-server --test wire_com_query` (COM_QUERY E2E).
3. `cargo test -p sqlrustgo-mysql-server --test wire_stmt_prepare_execute_close` (COM_STMT_*).
4. `cargo test -p sqlrustgo-mysql-server --test wire_error_packet`.
5. `cargo test -p sqlrustgo-mysql-server --test wire_reset_connection`.
6. `cargo test -p sqlrustgo-mysql-server --test wire_tls_handshake -- --ignored` (TLS is feature-gated).
7. `cargo test -p sqlrustgo-mysql-server --test wire_compression -- --ignored`.
8. `cargo test -p sqlrustgo-mysql-server --test load_data_sf1 -- --ignored` (slow).
9. `cargo test -p sqlrustgo-mysql-server --test load_data_sf10 -- --ignored` (very slow, only on tag).
10. Emit `docs/releases/v3.12.0/evidence/wire_load_data/V312-13-REPORT.md` with per-test status, SHA256 of each captured log, timestamp, source_run = `minimax-v312-13-<short_sha>`.

### 5. Failure mode

If any of the 9 steps fails, the gate emits a non-zero exit and the script *still writes* the report with the failure status. Per the v3.12 governance rule: "不允许用文档 claim 替代实跑证据" — a missing report or a report with `n/a` rows is itself a gate failure.
