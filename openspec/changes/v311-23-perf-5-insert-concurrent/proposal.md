## Why

PERF-5 was discovered during v3.10.0 168h SOAK (Issue #3434, 2026-07-15 06:32 UTC). v3.10.0 server's parser does NOT support `INSERT IGNORE` syntax - all `INSERT IGNORE INTO ...` statements return `ERROR 1064 (42000): Parse error: Expected Into, got Identifier("IGNORE")`.

This means the standard SOAK OLTP workload (which uses `INSERT IGNORE` to avoid PK conflicts from concurrent threads picking overlapping RANDOM IDs) produces **100% parse error rate** in v3.10.0. The actual error is parse failure, not "Lost connection" as originally reported.

Related issues found during investigation:
- MemoryStorage inserts work correctly (5/5 ALTER TABLE operations verified in V311-13)
- High-concurrency INSERTs succeed when IDs are unique (16 threads × 320 ops succeeded)
- The 0% INSERT rate in v3.10.0 SOAK was caused by 8 threads using same RANDOM%100000 range

## What Changes

- **Add `INSERT IGNORE` syntax to the parser** (`crates/parser/src/parser.rs:4364`)
  - Add `Token::Ignore` or use existing token recognition
  - Add flag `is_ignore: bool` to `InsertStatement` struct
- **Pass `is_ignore` through to executor** (`src/engine_dml.rs`)
  - When `is_ignore=true`, use storage.insert() that doesn't fail on duplicate
  - Or add new `insert_ignore` method to `StorageEngine` trait
- **Improve OLTP script to use unique IDs per thread** (`orchestrator_v2.sh`)
  - Each thread should have a unique ID range to avoid PK conflicts
- **Add regression test for concurrent INSERT** (`tests/stress/concurrent_insert_test.rs`)
  - 4 threads × 60 seconds with 100% INSERT success rate
  - Tests with overlapping IDs (must use INSERT IGNORE)
  - Tests with unique IDs (normal mode)

## Capabilities

### New Capabilities

- `parser-insert-ignore`: `INSERT IGNORE INTO ...` syntax is accepted by the parser and produces semantically correct execution (skip on duplicate key)

## Impact

- **Modified**: `crates/parser/src/parser.rs` — add `IGNORE` keyword handling in `parse_insert` (line 4364+)
- **Modified**: `crates/parser/src/parser.rs` — add `is_ignore: bool` field to `InsertStatement` struct
- **Modified**: `src/engine_dml.rs` — pass `is_ignore` to executor
- **Modified**: `src/execution_engine.rs` — destructure `is_ignore` from `InsertStatement`
- **New**: `tests/stress/concurrent_insert_test.rs` — 4-thread concurrent INSERT test, 60s
- **Modified**: `/tmp/soak_v310/orchestrator_v2.sh` — use unique ID ranges per thread
- **No new dependencies** — uses existing INSERT execution path

## Acceptance Criteria

- 4-thread concurrent OLTP for 1 minute: 0% parse errors, 0% connection errors
- Customer count increases by ~2400 rows (4 threads × 60 ops × 10 inserts/min = 2400)
- INSERT IGNORE / INSERT ... ON DUPLICATE KEY UPDATE / REPLACE INTO all work
- Server stays stable, no memory growth, no FD leak
- v3.10.0 SOAK test no longer reports PERF-5 issue
