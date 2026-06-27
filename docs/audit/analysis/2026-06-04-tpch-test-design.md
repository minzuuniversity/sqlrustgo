# Analysis: TPC-H Test Design — Should TPC-H use the canonical `sqlrustgo-mysql-server`?

**Date**: 2026-06-04
**Author**: consolidation workstream
**Status**: Investigation (not a directive)
**Refs**:
- `tests/tpch_gate_test.rs` — the only `cargo test`-driven TPC-H suite
- `crates/bench-cli/src/commands/tpch.rs` — `bench-cli tpch` subcommand
- `benches/tpch_comprehensive.rs` — synthetic-data + SQLite comparison
- `benchmarks/baselines/sf01/tpch.json` — placeholder (now has real numbers in `benchmarks/results/sf01/tpch.json`)
- `docs/audit/issues/ISSUE-2768_tpch_sf01_sf1_real_execution.md` — sibling audit that re-ran SF1 and measured the actual numbers

## TL;DR

- The current TPC-H suite **does not** use the canonical `sqlrustgo-mysql-server` binary. It runs in-process via `ExecutionEngine::new(MemoryStorage)`.
- The suite can **not** prove **correctness**: assertions are limited to "no parse error" and "no timeout". No result value, row count, or column-value check.
- The suite can prove **performance in a narrow sense**: single cold-start latency. Not percentiles, not warmup, not throughput.
- The user's consolidation mandate says "all e2e / perf tests must use mysql-server". TPC-H fits both categories, so it should drive the wire protocol.
- **Recommendation**: keep the in-process test (it is fast and isolated) **and** add a wire-protocol TPC-H smoke that runs a subset of queries against the canonical subprocess binary. Do **not** force all 22 queries through the wire path — the 4 that already fail to parse in-process will fail in exactly the same way over the wire, and importing 6M lineitem rows over a TCP socket is hours, not seconds.

## What the current `tests/tpch_gate_test.rs` does

`tests/tpch_gate_test.rs:222-389` (`test_tpch_sf01_gate`):

1. **Storage**: `MemoryStorage` (lines 265-267). Not `FileStorage`, not `WalStorage`. The test cannot exercise the production write path.
2. **Engine construction**: `ExecutionEngine::new(storage.clone())` (line 267). The test cannot exercise the wire protocol, the bootstrap tables, or the UserStore of the production server.
3. **Schema**: 8 hand-written `CREATE TABLE` statements (lines 82-91). Matches TPC-H spec column names + types.
4. **Data import**: parses `.tbl` files line-by-line, builds `Vec<Vec<SqlValue>>` batches of 10 000 rows, calls `storage.insert(name, batch)` directly (lines 155-216). Bypasses the parser and the engine.
5. **Query set**: only 12 of the 22 TPC-H queries, all pre-simplified to avoid `arithmetic inside aggregates` and `EXTRACT(YEAR FROM ...)`. The comment at line 95-97 explicitly admits this:
   > v3.8.0 parser doesn't support arithmetic expressions inside aggregate functions. Simplified versions use pre-computed columns or simpler aggregates.
6. **Result assertions**: `result = engine.execute(q_sql)` — only the return value (`Ok(ExecutorResult { ... })`) is matched, not the rows inside. Lines 318-351 just check `Ok(_)` and time the call.
7. **Pass / fail criterion**: lines 374-387 require Q1 and Q6 to complete within `TPCH_TIMEOUT_S` (120s for SF=0.1, 300s for SF=1, 600s for SF=10). All other queries are reported but do not fail the test.
8. **Net result**: at SF=0.1 the test typically reports `6/12 passed, 4 parse errors, 2 timeouts` — but only Q1/Q6 failure fails the gate. The other 10 queries' pass/fail is advisory.

## What the in-process suite actually proves

| Question | Does the current suite answer it? |
|---|---|
| "Q1 parses" | Yes (parse errors are surfaced) |
| "Q1 completes within 120s on SF=0.1" | Yes |
| "Q1 returns the *correct* sum_qty" | **No** — no value comparison |
| "Q1 is faster than MySQL on SF=1" | **No** — single cold-start, no statistical aggregation |
| "Q1 throughput at 10 concurrent clients" | **No** — single-threaded, single client |
| "Q1 over the wire matches Q1 in-process" | **No** — wire path is not exercised at all |
| "Q1 WAL records are flushed correctly" | **No** — uses MemoryStorage, no WAL |
| "Q1 auth handshake works" | **No** — no auth |
| "Q1 result-set serialization works" | **No** — no wire protocol |

## What `crates/bench-cli/src/commands/tpch.rs` does (the `--bench` invocation)

- 22 queries are defined inline (lines 11-58) — these are the *full* TPC-H SQL text, not the simplified 12 from `tpch_gate_test.rs`. So `bench-cli tpch --queries Q1,Q3,Q6` exercises more queries than `cargo test tpch_gate_test`.
- But: the test still calls `engine.execute(sql)` directly. No wire protocol, no bootstrap, no auth, no WAL.
- For each query, it creates a **new** `ExecutionEngine` per iteration (line 60). Cold start on every iteration means the per-query latency is dominated by the *engine+storage construction* (a fraction of a millisecond on a 100-row dataset), not the query itself. The numbers are not meaningful for performance reporting.

## What `benches/tpch_comprehensive.rs` does (the `cargo bench` path)

- `run_all_scenarios` (line 524) iterates 4 scenarios (single-thread, multi-thread, cache-hit, cache-miss) and *generates synthetic data* in the engine via row-by-row INSERTs.
- `run_sf_comparison` (line 547) compares against **SQLite** opened via `rusqlite::Connection::open_in_memory()`. SQLite is the wrong baseline for an OLAP engine — comparing against MySQL or PostgreSQL would be meaningful.
- The SF scaling is *also* synthetic: `orders_rows = 1_500_000 * SF` is hand-coded, not derived from TPC-H spec.
- Real data from `tests/integration/data/tpch-tiny/` (2-5 rows per table) is not used.

## Cross-engine comparison: what we have today

`docs/audit/issues/ISSUE-2768_tpch_sf01_sf1_real_execution.md` documents the only MySQL-vs-SQLRustGo comparison:

- **Q1 SF1**: SQLRustGo 14.93s vs MySQL 7.08s (Rust 2.1× slower). The measurement is wall-clock cold-start; no warmup, no percentiles.
- **Q3 / Q6 SF0.01**: 4-180ms. Different SFs, so not directly comparable.
- **MySQL Q1 SQL text** is the standard TPC-H (uses `arithmetic in aggregates`); SQLRustGo uses the simplified Q1 from `tpch_gate_test.rs`. The 2× gap includes the cost of Rust's unoptimized vectorized execution *and* the absence of JIT, *and* the difference in SQL.

The `benchmarks/baselines/sf01/tpch.json` file was a **placeholder** (all zeros) until `ISSUE-2768` populated `benchmarks/results/sf01/tpch.json` with the real numbers.

## Why "use the canonical binary" is the right call for TPC-H (eventually)

The user's mandate — "all e2e / perf tests must use mysql-server" — is a code-path-mandate, not a "use the slow path" mandate. The wire protocol adds:

- A 4-byte packet header (3-byte length + 1-byte sequence) per packet.
- An RSA / SHA1 auth handshake.
- The full `do_command_loop` (statement dispatch, prepared statements, error mapping).
- `FileStorage` + `WalStorage` + `FileBackedWalManager` (the production write path) instead of `MemoryStorage`.

A perf number from the in-process engine is the **engine's** number, not the **system's** number. The system has cost the in-process test never pays. TPC-H that uses the canonical binary is the only TPC-H that proves what we ship.

## Why it is *not* free to do so today

| Obstacle | Impact | Where |
|---|---|---|
| 4 of 22 queries fail to parse in-process | The wire test would fail in exactly the same place | `tests/tpch_gate_test.rs:107-119` (Q7/Q8/Q9 parse error on `FROM (SELECT ... )` subquery); Q12 fails on `0 AS low_line_count` |
| `EphemeralConfig` data dir is hardcoded to `std::env::temp_dir() + port + pid` | Cannot pre-load a 6M-row lineitem.tbl into a shared dir between two `start_ephemeral` calls (one to import, one to query) | `crates/mysql-server/src/lib.rs:2441-2447` |
| `FileStorage` import path: 6M rows × ~150 bytes/row = ~900 MB of raw .tbl bytes. Parsing 6M INSERTs and sending them over the wire as 60 000 batched COM_QUERY packets would take **tens of minutes**, dominated by network round-trips, not by the engine | The test would exceed the 120s timeout in CI | back-of-envelope |
| No reference-result fixtures in the repo | A wire-protocol TPC-H still wouldn't prove correctness — it would just shift the question from "did the engine execute" to "did the engine execute the right thing" | `tests/data/` contains `tpch-tiny/` (handful of rows) but no `expected/Q1_sf01.json` |

## Recommendation

Three tracks, in priority order:

### Track 1 (immediate, ~half day): keep the in-process suite, harden it

- Keep `tests/tpch_gate_test.rs` as-is. It is fast, isolated, and does *not* require the canonical binary — making it a useful smoke test on every PR.
- Add a small fixture file `tests/data/tpch/expected/sf01.json` containing the **known correct** outputs of Q1, Q3, Q6 for SF=0.1 (one entry per query, listing the expected sum / count / row). At SF=0.1 these numbers are small and stable; they can be computed once from any reference engine (DuckDB, SQLite-with-tpch-extension) and committed.
- Add `assert_eq!(result.rows[0][0], expected_q1_sum_qty)` to the gate. The current test prints the value but does not check it.
- Mark this as the **correctness gate**.

### Track 2 (medium, ~1 day): add a wire-protocol TPC-H smoke

- Add `EphemeralConfig::data_dir: Option<PathBuf>` so a test can pre-stage the data directory before the server starts. (Trivial patch to `crates/mysql-server/src/lib.rs:2437-2447`.)
- Add a `EphemeralConfig::bootstrap_sql: Vec<String>` for the test to inject the 8 TPC-H `CREATE TABLE` DDLs. (Trivial patch to `run_server_with_listener_and_shutdown_with_bootstrap_and_tables`.)
- Write `tests/tpch_wire_smoke.rs` that:
  1. Stages `tests/integration/data/tpch-tiny/` into a temp dir.
  2. Starts an ephemeral server with that data dir and the 8 TPC-H tables pre-created.
  3. Connects via `MySqlTestClient::connect_at`.
  4. Runs the 12 simplified queries over the wire and asserts the `result.rows` count matches `expected/tpch_tiny/sf0.json` (a new fixture for the tiny data set — easy to compute by hand).
  5. Asserts wire protocol: client gets the expected packet sequence, the server returns OK/ERR correctly, the `result` struct round-trips the rows.
- This test runs in <1s on the tiny data set, and proves the canonical binary end-to-end for TPC-H.

### Track 3 (long, ~1 week): real-data TPC-H over the wire

- Use the `tpch_data_gen` crate (`crates/bench/examples/tpch_data_gen.rs`) to *generate* the .tbl files programmatically (no external `dbgen` dependency).
- Patch the wire-protocol INSERT path to support a server-side bulk loader (or accept a `LOAD DATA LOCAL INFILE`-style packet that streams .tbl bytes from the client). This is the only way to make 6M-row import practical.
- Run Q1 against SF=1 from a separate wire client and assert the result matches a reference engine's result to within a numerical tolerance.
- Report timing via `benchmarks/results/sf01/wire.json` and `benchmarks/results/sf1/wire.json`.

Track 3 is the *only* one that produces a meaningful perf comparison. Tracks 1 + 2 are correctness.

## What the L3 acceptance for the consolidation change is *not* testing

To be explicit: the 32 wire-protocol tests added by the consolidation PRs (`e2e_canonical_subprocess` etc.) prove that the **wire path** works for DDL/DML/DQL/transactional DML on a 3-row table. They do **not** prove that the engine is correct on real TPC-H queries at SF≥1. That gap is pre-existing and is what Track 1+2 above close.

## Conclusion

The current TPC-H suite is **insufficient** for both correctness and performance. It runs in-process on `MemoryStorage`, asserts only "no error", and times only a single cold start. The user's consolidation mandate points the right way (use the canonical binary) but doing it for the full SF=1 path requires non-trivial work — server-side bulk load, pre-stageable data dir, reference fixtures.

**Action**: Track 1 (harden the existing test with value comparison against fixtures) + Track 2 (add a wire-protocol smoke on the `tpch-tiny` data set) are achievable in a day. Track 3 (real-data wire-protocol TPC-H) is a follow-up issue, not a blocker for v3.8.0 GA.
