# V312-13 Design

## Wire Protocol E2E Tests

### Test File Location
`crates/mysql-server/tests/wire_smoke_mysql_cli.rs`

### Approach
- Use the existing `testing::EphemeralConfig` test harness in `crates/mysql-server`
- Spawn ephemeral server, use `mysql` crate or raw TCP to drive wire protocol
- Test COM_QUERY by sending SQL and parsing result set
- Test COM_STMT_PREPARE/EXECUTE/CLOSE using `mysql::远方::prepared` or raw packet building
- Test error packets by sending malformed SQL and checking ERR packet structure
- Test reset by sending COM_RESET_CONNECTION and verifying session state cleared

### Key Files to Modify
- `crates/mysql-server/src/lib.rs` — add COM_STMT_CLOSE handler if missing
- `crates/mysql-server/src/lib.rs` — ensure COM_RESET_CONNECTION resets session state
- `crates/mysql-server/src/lib.rs` — ensure error packet format matches MySQL 8.0

### Test Naming Convention
- `wire_smoke_com_query` — basic query
- `wire_smoke_stmt_prepare_execute` — prepared statement cycle
- `wire_smoke_stmt_close` — deallocate
- `wire_smoke_error_packet` — invalid SQL error
- `wire_smoke_reset_connection` — session reset

## LOAD DATA INFILE Tests

### Test File Location
`crates/mysql-server/tests/load_data_sf_test.rs`

### Approach
- Use TPC-H .tbl files (SF=1: ~6M rows, SF=10: ~60M rows if available)
- Execute `LOAD DATA LOCAL INFILE` via MySQL client
- Record: row count from `SELECT COUNT(*)`, SHA256 of `SELECT * ORDER BY 1 LIMIT 100`
- Monitor memory via `/proc/{pid}/status` PeakVmData
- Duration from `time` command
- Output saved to `docs/releases/v3.12.0/load-data-report.md`

### Gate Script Update
- Update `scripts/gate/check_load_data_infile.sh` to also check for load-data-report.md with SF=10 section
