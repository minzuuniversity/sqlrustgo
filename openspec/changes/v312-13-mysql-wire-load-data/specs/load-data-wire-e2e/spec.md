# V312-13 MySQL Wire + LOAD DATA Hardening Spec

## Wire Protocol E2E Tests

### COM_QUERY
- Connect via mysql CLI, execute SELECT 1, verify result set
- Execute CREATE TABLE / INSERT / SELECT / DROP cycle
- Verify error packets for invalid SQL

### COM_STMT_PREPARE / EXECUTE / CLOSE
- PREPARE stmt FROM 'SELECT ? + ?'
- EXECUTE stmt WITH (1, 2) → expect 3
- EXECUTE stmt WITH (10, 20) → expect 30
- DEALLOCATE PREPARE stmt
- Verify prepared statement reuse across connections

### Error Packet
- Send invalid SQL, verify ERR packet with MySQL error code
- Send COM_STMT_EXECUTE with wrong param count, verify error

### Reset Connection
- COM_RESET_CONNECTION resets session state
- Verify user variables cleared, prepared statements deallocated

### TLS / Compression (if available)
- TLS handshake succeeds
- Compression boundary tested with large result set

## LOAD DATA INFILE

### SF=1 Test
- Load TPC-H SF=1 lineitem.tbl (~6M rows)
- Record: row count, SHA256 of first/last 100 rows, memory peak, duration
- Verify data integrity after load

### SF=10 Test (if feasible)
- Load TPC-H SF=10 lineitem.tbl (~60M rows)
- Record: row count, SHA256, memory peak, duration
- Fail closed on OOM or silent truncation

## Acceptance Criteria

- Wire protocol e2e report exists at `docs/releases/v3.12.0/wire-e2e-report.md`
- LOAD DATA report exists at `docs/releases/v3.12.0/load-data-report.md`
- mysql-server and mysql-client coverage trend improves or has explicit non-blocking rationale
