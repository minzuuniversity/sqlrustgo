# V312-F-2 Design: e2e_wire_protocol SERVER_POOL Isolation

## Problem

9 of 46 e2e_wire_protocol tests fail with row counts matching the
previous test's state (e.g., test_e2e_delete expects 2 rows but gets 56
which matches test_e2e_in_operator's data).

The tests share a process-global `SERVER_POOL` (likely a `LazyLock` or
`OnceCell<Vec<...>>` in the test harness). Each test reuses the same
engine instance, and the previous test's tables persist.

## Fix Strategy

The minimal fix is **per-test isolation via DROP TABLE pre-cleanup**:
- Add a `cleanup_all_tables()` helper at the start of each test
- OR add `tear_down` fixture that drops all tables in the engine
- OR refactor the harness to spin up a fresh server per test

Since refactoring the harness is high-risk, the simplest fix is to ensure
each test drops any pre-existing tables matching its table names. This
requires the engine to support listing tables (which it does — `SHOW
TABLES` is a real SQL feature).

## Alternative

If the server pool holds an open transaction or accumulated WAL state,
we may need to call `engine.reset()` between tests. This requires
investigating the public API of the engine.

## Verification

Before: 37 passed, 9 failed.
After: 46 passed, 0 failed.