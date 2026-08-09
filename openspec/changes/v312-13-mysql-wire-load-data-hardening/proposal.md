## Why

V312-13 / ISSUE #3900 closes the remaining v3.11.0 MySQL wire-protocol and `LOAD DATA` production-risk gaps before v3.12.0 RC. v3.11.0's `ARCHITECTURE.md` (line 55) explicitly states the open surface: "prepared statement、error packet、TLS/compression 需补强". The `G4_WIRE_TEST_CLOSE_OUT_PLAN.md` (`docs/releases/v3.11.0/`) and `COMPREHENSIVE_ASSESSMENT_REPORT.md` confirm the wire path lacks E2E artifacts for the six commands below, and `LOAD DATA LOCAL INFILE` (gated by `scripts/gate/check_load_data_infile.sh`) has no SF=1 / SF=10 production evidence (row count, SHA256, memory cap, duration). Without these, SQLRustGo cannot honestly claim MySQL client compatibility for the GMP internal-audit workload.

This change delivers the production hardening needed to unblock V312-19 (issue #3906) which makes the wire/load-data test surface a v3.12 RC/GA blocking gate.

## What Changes

- **COM_QUERY (0x03) E2E artifact**: produce an executable script + log that runs `SELECT`/`INSERT`/`UPDATE`/`DELETE`/`DDL` over `MySqlTestClient`, captures row counts, and verifies against an in-memory oracle.
- **COM_STMT_PREPARE (0x16) / EXECUTE (0x17) / CLOSE (0x19) E2E artifact**: extend the existing `binary-prepared-statement-roundtrip` spec scenarios; capture wire-format traces and parameter-substituted SQL via `RUST_LOG=sqlrustgo_mysql_server=info`.
- **Error packet (0xFF) E2E artifact**: force syntax errors, constraint violations, auth failures, and verify the client receives a well-formed ERR packet with correct error code, SQLSTATE, and message.
- **COM_RESET_CONNECTION (0x1F) E2E artifact**: send reset, verify session state (autocommit, isolation, prepared statements, session variables) is cleared without closing the connection.
- **TLS handshake (--ssl-mode) E2E artifact**: negotiate TLS, verify cipher, then run an authenticated query over the encrypted channel.
- **Compression (--compress) E2E artifact**: negotiate zlib compression, verify payload bytes compress, query still returns correct rows.
- **`LOAD DATA LOCAL INFILE` SF=1 fixture**: load ~6M lineitem rows, capture duration, peak RSS, row count, and SHA256 of imported table; gate must pass with `LOAD_DATA_SF1_DURATION_S` threshold.
- **`LOAD DATA LOCAL INFILE` SF=10 fixture**: load ~60M lineitem rows, capture duration, peak RSS, row count, SHA256; gate must pass with `LOAD_DATA_SF10_DURATION_S` and `LOAD_DATA_SF10_PEAK_RSS_MB` thresholds.
- **Memory cap invariant**: a Rust `assert!` or test that fails CI if `LOAD DATA` RSS exceeds the configured cap.
- **Wire e2e shell script** at `scripts/gate/check_v312_13_wire_load_data.sh` that runs all the above and produces a single artifact at `docs/releases/v3.12.0/evidence/wire_load_data/V312-13-REPORT.md`.

## Capabilities

### New Capabilities

- `mysql-wire-stmt-reset-tls-compression`: E2E coverage for COM_STMT_*, COM_RESET_CONNECTION, TLS handshake, and zlib compression on the existing `MySqlTestClient`.
- `mysql-wire-error-packet-contract`: shape and error-code contract for the ERR packet (0xFF) under syntax/auth/constraint failure paths.
- `load-data-sf1-sf10-memory-cap`: production-scale `LOAD DATA LOCAL INFILE` for SF=1 (~6M rows) and SF=10 (~60M rows) with row count, SHA256, duration, peak RSS, and hard memory cap.

### Modified Capabilities

- `binary-prepared-statement-roundtrip`: extend to cover error packets on bad parameter types, param count mismatch, and `COM_STMT_CLOSE` lifecycle.
- `wire-protocol-execution`: add scenarios for TLS handshake and zlib compression; add a "reset connection clears session state" requirement.

## Impact

- **Modified**: `crates/mysql-server/src/lib.rs` (do_command_loop) — ensure all six commands produce testable artifacts; structured logging consistent with the existing `info!` pattern.
- **Modified**: `crates/mysql-server/src/handshake.rs` (or equivalent) — TLS and compression capability flags.
- **Modified**: `tests/common/mod.rs` (`MySqlTestClient`) — add `prepare_stmt`, `execute_stmt`, `close_stmt`, `reset_connection`, `force_tls`, `force_compress` helpers.
- **New**: `tests/load_data_sf1.rs`, `tests/load_data_sf10.rs` — production-scale ingest with memory cap.
- **New**: `scripts/gate/check_v312_13_wire_load_data.sh` — driver script for the gate.
- **New**: `docs/releases/v3.12.0/evidence/wire_load_data/V312-13-REPORT.md` — generated artifact.
- **Dependencies**: no new crate deps; the `sha1` crate already in use for native_password.
