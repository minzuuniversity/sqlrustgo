## Context

v3.10.0 GA shipped with 22 integration tests disabled via `#![cfg(any())]` because internal API refactoring during v3.10.0 made them uncompilable. These tests live across `tests/anomaly/`, `tests/integration/`, `tests/stress/`, `tests/e2e/` and cover storage, executor, WAL, parser, optimizer, and transaction paths.

The test patterns that broke fall into two categories:

**Pattern A — WAL API changes (`WalManager` → trait, `WalWriter` methods removed)**  
Several WAL tests use the old `WalManager::new(...)` constructor and `log_begin`/`log_insert`/`log_commit` methods. The fix is to use `LegacyWalManager::new(...)` and `WalWriter::append(&WalEntry)`.

**Pattern B — execute/parse/Value/IndexScanExec API changes**  
The remaining tests use `execute(statement)` (now `execute(&str)`), `Value::Date`/`Value::Timestamp` (removed variants), `IndexScanExec::new(...)` (changed signature), `storage.write().map_err(...)` (parking_lot RwLock returns guard, not Result), and various struct field renames.

All fix patterns are known and already validated in other tests. No production code changes are needed.

## Goals / Non-Goals

**Goals:**
- Re-enable all 22 disabled test files so `cargo test -p sqlrustgo --all-features` passes
- Remove every `#![cfg(any())]` guard
- Verify each test compiles and passes in isolation
- Maintain per-crate coverage baselines

**Non-Goals:**
- No production API changes
- No new test coverage beyond what the disabled tests previously covered
- No behavioral spec changes — tests should verify the same contracts as before, just with new API calls

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Fix pattern for WalManager | Replace `WalManager::new(...)` with `LegacyWalManager::new(...)` | `WalManager` is now a trait; `LegacyWalManager` is the concrete struct. Import from `crate::wal_legacy` |
| Fix pattern for WalWriter logging | Replace `log_begin`/`log_insert`/`log_commit` with `WalWriter::append(&WalEntry)` | The new WAL API uses a single `append` method with typed `WalEntry` structs |
| Fix pattern for execute() | Change `execute(parse("...").unwrap())` to `execute("...")` | `execute` now accepts `&str` directly |
| Fix pattern for parking_lot | Remove `.map_err()` calls on `.write()`/`.read()` | `parking_lot::RwLock` doesn't poison; no error type |
| Fix pattern for removed Value variants | Replace `Value::Date(val)` / `Value::Timestamp(val)` with appropriate current representation | Variants were merged/removed in types crate refactor |
| Fix pattern for insert methods | `on_duplicate` → `on_duplicate_key_update`, `replace` → `is_replace` | Field renames in InsertStatement |
| Tests too far from current API | Keep `#![cfg(any())]` and document as permanently obsolete | optimizer_cost_test, q21_perf_bench reference completely removed APIs with no equivalent |
| Test ordering | Fix easier tests first (batch pattern), then harder ones individually | Faster feedback: compile-check each fix immediately |

## Risks / Trade-offs

- **[Compiler-only fix misses runtime regressions]** → Run each test after fixing to confirm it passes, not just compiles
- **[Test proves wrong contract]** → Review test assertions after fixing to ensure they still test the original intent, not an accidental pass
- **[Batch sed creates wrong patterns]** → Review each sed-based fix before committing; prefer manual per-file edits for complex cases
- **[Some tests permanently obsolete]** → Accept that 2-3 tests may remain disabled if the APIs they test have no replacement (documented in COVERAGE_REPORT.md known limitations)

## Migration Plan

1. **Batch 1 — Simple sed patterns** (execute, map_err, WalManager rename): ~10 files
2. **Batch 2 — Medium complexity** (WalWriter append, insert field renames): ~6 files
3. **Batch 3 — Hard cases** (Value types, IndexScanExec, tx_id scope): ~6 files
4. **Verification** — Full cargo test + cargo llvm-cov after each batch
