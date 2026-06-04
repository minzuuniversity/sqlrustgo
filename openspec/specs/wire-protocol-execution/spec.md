# wire-protocol-execution Specification

## Purpose
TBD - created by archiving change mysql-server-canonical-entry. Update Purpose after archive.
## Requirements
### Requirement: Raw-protocol test client

`MySqlTestClient` SHALL be a raw-protocol MySQL client in
`tests/common/mod.rs` that:

- Reads HandshakeV10 from the server and extracts the 20-byte
  scramble.
- Computes a `mysql_native_password` auth response from the
  scramble and the configured password.
- Sends HandshakeResponse41 with `PROTOCOL_41 | SECURE_CONNECTION
  | LONG_PASSWORD` capabilities and reads the server's OK /
  ERR response.
- Sends COM_QUERY (0x03) packets and parses the result set
  (column count, column definitions, single EOF separator, row
  packets, terminal EOF / OK).
- Exposes `exec(&mut self, sql)`, `query_rows(&mut self, sql) ->
  Vec<Vec<String>>`, and `query_one_i64(&mut self, sql) -> i64`
  for the test surface.
- Exposes `connect_handle(handle)` for in-process ephemerals and
  `connect_at(addr, user, password)` for external (subprocess)
  servers.

The client SHALL NOT depend on the `mysql` crate; the
`mysql_native_password` math is implemented inline with the
`sha1` crate.

#### Scenario: Wire-protocol round-trip is GREEN

- GIVEN an ephemeral server started via `start_ephemeral`
- WHEN the test connects with `MySqlTestClient::connect_default`
  and runs CREATE + INSERT + SELECT
- THEN the test MUST observe the inserted rows via
  `query_rows` and `query_one_i64`

### Requirement: EphemeralConfig flags

`EphemeralConfig` SHALL expose two opt-in flags:

- `bootstrap_tables: bool` (default `true`) — pre-create the
  internal catalog tables `content`, `vectors`, `documents`
- `bootstrap_users: bool` (default `true`) — pre-create a
  `tester` user with password `tester`

A test that asserts on a clean catalog SHALL set
`bootstrap_tables: false` and keep `bootstrap_users: true` so
authentication still works.

#### Scenario: Clean catalog for SHOW TABLES

- GIVEN a test starts an ephemeral with
  `EphemeralConfig { bootstrap_tables: false, .. }`
- WHEN it runs `SHOW TABLES` against the empty database
- THEN the result set SHALL have zero rows

### Requirement: COM_QUERY routes every DDL/DML statement
The MySQL wire-protocol COM_QUERY handler in `sqlrustgo-mysql-server` MUST accept every DDL/DML statement that the in-process `ExecutionEngine` accepts and MUST route it through `StorageEngine` exactly as `ExecutionEngine::execute` would. No COM_QUERY path may short-circuit the storage layer or skip WAL.

#### Scenario: CREATE TABLE round-trip
- **WHEN** a client sends `CREATE TABLE t (id INT PRIMARY KEY, v TEXT)` and then `INSERT INTO t VALUES (1, 'x')` and then `SELECT * FROM t`
- **THEN** the second result set contains exactly one row `[1, "x"]` and the engine's WAL file contains both the CREATE and the INSERT entries in order

#### Scenario: UPDATE replay survives crash
- **WHEN** a client runs `BEGIN; UPDATE t SET v = 'y'; COMMIT;`, the server process is killed with `SIGKILL`, and a fresh server is started against the same `--data-dir`
- **THEN** a fresh client `SELECT v FROM t WHERE id = 1` returns `"y"`

#### Scenario: SELECT against vector index
- **WHEN** a client runs `SELECT id, distance FROM vector_index_search('idx_emb', VECTOR('[0.1, 0.2]'), 10)`
- **THEN** the result set contains up to 10 rows ordered by ascending `distance`, served by the `vector` crate (not by a SQL-side fallback)

### Requirement: Transaction lifecycle over the wire
`BEGIN`, `COMMIT`, `ROLLBACK`, and autocommit MUST behave identically over the wire as they do in-process. The server MUST reject statements that violate the TX lifecycle with the same `SqlError` mapping the in-process API returns, surfaced as a MySQL error packet.

#### Scenario: DML outside an explicit transaction
- **WHEN** a client sends `INSERT INTO t VALUES (1, 'a')` without a prior `BEGIN`
- **THEN** the server auto-commits (default MySQL behavior) and the row is durable

#### Scenario: DML inside an explicit transaction without COMMIT
- **WHEN** a client runs `BEGIN; INSERT INTO t VALUES (2, 'b'); <client disconnects without COMMIT>`
- **THEN** on server restart against the same data dir, `SELECT * FROM t WHERE id = 2` returns zero rows

### Requirement: Advanced subsystems reachable as SQL
The advanced subsystems (`vector`, `graph`, `rag`, `gmp`, `qmd-bridge`, `telemetry`, `spill`, `network`, `catalog`, `information-schema`) MUST be reachable from the wire as virtual tables or virtual functions. None of them may be reachable only in-process.

#### Scenario: Graph traversal
- **WHEN** a client runs `SELECT * FROM cypher_match('MATCH (n:Person) RETURN n.name LIMIT 5')`
- **THEN** the result set is the union over the `graph` crate's first five `Person` nodes, and the request was routed through the graph subsystem (verified by telemetry counter `graph_query_total` incrementing)

#### Scenario: GMP retrieval
- **WHEN** a client runs `SELECT doc_id, score FROM gmp_search('docs', 'hello world', 5)`
- **THEN** the result set contains up to 5 `(doc_id, score)` pairs served by the `gmp` crate, ordered by descending `score`

#### Scenario: Health probe
- **WHEN** a client runs `SELECT * FROM server_health`
- **THEN** the result set is a single row with columns `(uptime_secs, wal_lsn, open_connections, slow_query_total, cache_hit_ratio, feature_advanced_enabled)`

