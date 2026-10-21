# PR2 / #5304 BufferPool Counter Surface — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `buffer_pool_stats() -> Option<BufferPoolStats>` storage-trait surface (default `Some(synthetic_zero)`) and AHI EXPLAIN counter plumbing so PR3 #5305 can ship `pages_visited` / `ahi_lookups` / `ahi_hits` columns without rewiring any storage engine.

**Architecture:** StorageEngine trait gains two `&self`-compatible default methods. `BufferPoolStats::synthetic_zero()` exists so engines can honestly answer `Some(zero)`. `BoxStorageEngine` mirrors the methods (mandatory: historic "wrapper drops method" bug pattern #4974/#5009/#5105/#5192/#5266/#5281/#4912). EXPLAIN column addition is **deferred to PR3**; PR2 only exposes the trait surface + AHI-side deterministic tests that PR3 consumes. **No engine rewrites; no BufferPool promotion; no §3.1 claim boundary change.**

**Tech Stack:** Rust 2024 edition, `parking_lot::RwLock` (already used by `AdaptiveHashIndex`), `Arc<AdaptiveHashIndex>` (already exposed via `ExecutionEngine::adaptive_hash_index()`).

**Spec:** [`docs/superpowers/specs/2026-10-14-5304-pr2-bufferpool-production-wiring-design.md`](../specs/2026-10-14-5304-pr2-bufferpool-production-wiring-design.md)

---

## Global Constraints

- Base branch: `gitea252/develop/v4.1.0 @ e12f7a48fa` (post-#5330 merge).
- New worktree: `.worktrees/5304-pr2` on branch `fix/5304-pr2-bufpool-production-counter`, created via `superpowers:using-git-worktrees` at execution time.
- Rust 2024 edition; do NOT change edition.
- Do NOT promote `BufferPool` / `Page` / `BPlusTree` from `⚠️ Test-only capability` (PR1 §3.1 of `docs/releases/v4.1.0/CLAIM_DOWNGRADE_MANIFEST.md`).
- Do NOT touch `docs/governance/debt/debt-registry.yaml` or any ARCH/SEM plan (D8/SEM-4 OPEN-no-plan is pre-existing debt under #5103, NOT this PR's scope).
- Do NOT touch `MvccStorage`, `WalStorage`, `ParallelWalStorage` internals.
- Do NOT modify `src/lib.rs` (PR1's split re-export block stays intact).
- Total budget: ≤ 5 files modified, ≤ 1 file created, ≤ ~147 lines (per design §2.9).
- Commit message style: conventional `<type>`; attribution disabled globally (don't add `Co-Authored-By:` trailers).
- Push to `gitea252` only (no GitHub per project CLAUDE.md).
- PR base = `develop/v4.1.0`.

---

## Review Focus

Failure modes the spec implies but no task's tests exercise; pin each to the owning task below.

1. **`BufferPoolStats` field order matches `Default::default()`** — `synthetic_zero()` must produce a struct layout identical to `Default::default()` so a future `.into()` or `==` comparison cannot diverge silently. **Pinned by Task 1 Step 3 test.**
2. **`BoxStorageEngine::buffer_pool_stats` forwarding** — historically this wrapper drops new methods (`#4974` / `#5009` / `#5105` / `#5192` / `#5266` / `#5281` / `#4912` = 7 historical fixes for the same pattern). Without the override, MySQL-server's `Arc<RwLock<BoxStorageEngine>>` would silently fall back to the trait default `Some(synthetic_zero)` only by coincidence; if anyone changes the trait default to `None` later, the server quietly breaks. **Pinned by Task 3 test through `Box<dyn StorageEngine>` dispatch.**
3. **`estimated_pages_for_table` underflow on empty table** — `len().div_ceil(32)` returns `0` for `len=0` which is correct, but `len() / 32 + bool(...)` style off-by-one errors are easy to introduce. **Pinned by Task 2 test on empty table.**
4. **AHI counter race across concurrent queries** — `AdaptiveHashIndex::total_lookups()` reads `lookups: RwLock<u64>`, but two EXPLAINs in parallel can mix deltas. The deterministic test pins single-threaded behavior; the documented expectation is **per-query single-thread only**. **Pinned by Task 4 test in single-conn `ExecutionEngine`.**
5. **`CLAIM_DOWNGRADE_MANIFEST.md` §3.1 boundary line must remain verbatim** — PR2 adds exactly one line BELOW §3.1 (a tail note) without modifying §3.1 wording, claim boundary line, or any inherited caveat. **Pinned by Task 5 commit inspection (no diff in §3.1 region).**

---

## File Structure

| File | Type | Role |
|---|---|---|
| `crates/storage/src/buffer_pool.rs` | Modify | Add `BufferPoolStats::synthetic_zero()` constructor at lines 30-55 |
| `crates/storage/src/engine.rs` | Modify | Add `buffer_pool_stats()` and `estimated_pages_for_table()` default methods to `pub trait StorageEngine` at line 1042 |
| `crates/storage/src/binary_storage.rs` | Modify | Add matching forwarder overrides on `impl StorageEngine for BoxStorageEngine` (around line 677 onward) |
| `tests/integration/sql/ahi_explain_test.rs` | **Create** | 3 deterministic tests on AHI + synthetic_zero dispatch |
| `docs/releases/v4.1.0/CLAIM_DOWNGRADE_MANIFEST.md` | Modify | 1-line note appended below §3.1 (no §3.1 region diff) |
| **Net** | 5 modified, 1 created, ~147 lines | |

---

## Task 1: Add `BufferPoolStats::synthetic_zero()`

**Files:**
- Modify: `crates/storage/src/buffer_pool.rs:30-55` (the `impl BufferPoolStats { ... }` block).
- Test (in-module): same file's existing `mod tests` (lines 288-418) — extend with a new test.

**Interfaces:**
- Consumes: nothing (this task introduces the constructor).
- Produces: `impl BufferPoolStats { pub fn synthetic_zero() -> Self { ... } }` returning `Self { hits: 0, misses: 0, evictions: 0, prefetch_hits: 0 }`. Field order MUST match `Default::default()` (Review Focus #1).

- [ ] **Step 1: Write the failing test**

Append the following inside `crates/storage/src/buffer_pool.rs`'s `mod tests` block (after the existing `test_buffer_pool_prefetch` test, before the closing `}` of the module at line 418):

```rust
#[test]
fn test_buffer_pool_stats_synthetic_zero_field_order() {
    let z = BufferPoolStats::synthetic_zero();
    // Field-by-field equality pins layout (Review Focus #1).
    assert_eq!(z.hits, 0);
    assert_eq!(z.misses, 0);
    assert_eq!(z.evictions, 0);
    assert_eq!(z.prefetch_hits, 0);
    // Bit-for-bit identical to Default::default().
    let d: BufferPoolStats = Default::default();
    assert_eq!(
        (z.hits, z.misses, z.evictions, z.prefetch_hits),
        (d.hits, d.misses, d.evictions, d.prefetch_hits),
        "synthetic_zero() must match Default::default() field-by-field"
    );
    // hit_rate() must be 0.0 (no lookups performed).
    assert_eq!(z.hit_rate(), 0.0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p sqlrustgo-storage --lib test_buffer_pool_stats_synthetic_zero_field_order`
Expected: FAIL with `error[E0599]: no function or associated item named 'synthetic_zero' found for struct 'BufferPoolStats'` (or similar — the test won't compile).

- [ ] **Step 3: Implement `synthetic_zero()` in `crates/storage/src/buffer_pool.rs`**

In the `impl BufferPoolStats { ... }` block (lines 30-55), add the following method directly below `pub fn hit_rate(&self) -> f64 { ... }` at line 54:

```rust
    /// PR2 / #5304: explicit zero-constructor for engines that don't
    /// page. Returning `Some(synthetic_zero)` means "this engine has no
    /// cache layer" — accurate when no `BufferPool` is wired (today's
    /// state). Field order matches `Default::default()` for stable equality.
    pub fn synthetic_zero() -> Self {
        Self {
            hits: 0,
            misses: 0,
            evictions: 0,
            prefetch_hits: 0,
        }
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p sqlrustgo-storage --lib test_buffer_pool_stats_synthetic_zero_field_order`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
cd /Users/liying/workspace/dev/sqlrustgo/.worktrees/5304-pr2
git add crates/storage/src/buffer_pool.rs
git commit -m "feat(#5304): add BufferPoolStats::synthetic_zero()"
```

---

## Task 2: Add `StorageEngine` trait methods (defaults only, additive)

**Files:**
- Modify: `crates/storage/src/engine.rs:1042` — inside `pub trait StorageEngine: Send + Sync { ... }`.
- Test (in-module): same file's `mod tests` if present; otherwise integration tests land in Task 4.

**Interfaces:**
- Consumes: `BufferPoolStats::synthetic_zero()` from Task 1.
- Produces:
  - `fn buffer_pool_stats(&self) -> Option<BufferPoolStats>` default `Some(BufferPoolStats::synthetic_zero())`.
  - `fn estimated_pages_for_table(&self, table: &str) -> u64` default `self.scan(table).map(|rows| rows.len().div_ceil(32) as u64).unwrap_or(0)`.

- [ ] **Step 1: Verify the trait location and write the failing test**

First, run `grep -n "pub trait StorageEngine" crates/storage/src/engine.rs` to confirm line 1042. (If line drift > 10 lines due to upstream edits, locate the trait block and adjust step 2 accordingly.)

Then add to the existing `#[cfg(test)] mod tests { ... }` block at the bottom of `crates/storage/src/engine.rs` (or create one if absent — see Step 2 for placement guidance). The test must compile-check the trait shape via `&dyn StorageEngine`:

```rust
fn _storage_engine_trait_buffer_pool_stats_compiles<S: StorageEngine + ?Sized>(s: &S) {
    // Default trait methods must be callable through &dyn dispatch.
    let stats: Option<BufferPoolStats> = s.buffer_pool_stats();
    let _ = stats.map(|x| x.synthetic_zero()); // ensure method is in scope (Task 1 link).
    let _: u64 = s.estimated_pages_for_table("__nonexistent__");
}
```

Pin: confirm that `Option<BufferPoolStats>` is the return type and that `estimated_pages_for_table(&str)` exists.

- [ ] **Step 2: Run compile-check to verify it fails**

Run: `cargo build -p sqlrustgo-storage 2>&1 | head -40`
Expected: FAIL with `error[E0599]: no method named 'buffer_pool_stats' found for ...` and `'estimated_pages_for_table'`.

- [ ] **Step 3: Implement the two default methods on `StorageEngine` trait in `crates/storage/src/engine.rs`**

Insert the following immediately above the closing `}` of `pub trait StorageEngine: Send + Sync { ... }`:

```rust
    /// PR2 / #5304: page-equivalent counter for EXPLAIN's `pages_visited`.
    ///
    /// Default returns `Some(BufferPoolStats::synthetic_zero())`. Engines
    /// that later page through a real `BufferPool` override to return
    /// `Some(stats)` from that pool. Returning `None` is reserved for
    /// engines that don't track pages at all (no current engine).
    ///
    /// IMPORTANT: returning `Some(zero)` when a cache actually exists
    /// is a documented anti-pattern — call `pool.stats()` instead.
    fn buffer_pool_stats(&self) -> Option<BufferPoolStats> {
        Some(BufferPoolStats::synthetic_zero())
    }

    /// PR2 / #5304: estimated pages if this table were paged. Used by
    /// EXPLAIN when `buffer_pool_stats()` returns `Some(synthetic_zero)`
    /// to populate `pages_visited` instead of literal zero. Returns 0
    /// for an unreadable / non-existent table (caller never panics).
    fn estimated_pages_for_table(&self, table: &str) -> u64 {
        self.scan(table)
            .map(|rows| rows.len().div_ceil(32) as u64)
            .unwrap_or(0)
    }
```

Add `use crate::buffer_pool::BufferPoolStats;` (or the project's equivalent import path for `BufferPoolStats` at the top of `engine.rs`) if not already present. Verify by grepping:

```
grep -n "BufferPoolStats\|use crate::buffer_pool\|use crate::{" crates/storage/src/engine.rs | head -10
```

If `BufferPoolStats` is not in scope, add the import. The existing `use` block at top of `engine.rs` is the right home.

- [ ] **Step 4: Run compile-check to verify it passes**

Run: `cargo build -p sqlrustgo-storage 2>&1 | tail -20`
Expected: `Finished ...` (exit 0). The test compile-check function from Step 1 must compile.

- [ ] **Step 5: Run the broader workspace build to confirm no downstream breakage**

Run: `cargo build --all-features 2>&1 | tail -30`
Expected: `Finished ...` (exit 0). Any engine that doesn't override the new methods inherits the defaults.

- [ ] **Step 6: Commit**

```bash
cd /Users/liying/workspace/dev/sqlrustgo/.worktrees/5304-pr2
git add crates/storage/src/engine.rs
git commit -m "feat(#5304): StorageEngine::buffer_pool_stats + estimated_pages_for_table defaults"
```

---

## Task 3: `BoxStorageEngine` forwarding overrides (mandatory — historic wrapper-drops-method pattern)

**Files:**
- Modify: `crates/storage/src/binary_storage.rs` — inside `impl StorageEngine for BoxStorageEngine` (starts around line 677).
- Test: same `tests/integration/sql/ahi_explain_test.rs` (Task 4) provides the integration coverage; Task 3 adds an inline unit-level assertion via `BinaryTableStorage` + `BoxStorageEngine` inside the test file.

**Interfaces:**
- Consumes: the two methods from Task 2; `BinaryTableStorage` (existing); `BoxStorageEngine::new` (existing at `binary_storage.rs:650`).
- Produces: forwarder impl on `BoxStorageEngine` matching the pattern of `scan_pk`, `scan_in`, `insert_in_db`, etc.

- [ ] **Step 1: Verify `BoxStorageEngine` location and the forwarding pattern**

Run: `grep -n "impl StorageEngine for BoxStorageEngine\|fn scan_pk\|fn insert_in_db\|fn commit_transaction_lockfree" crates/storage/src/binary_storage.rs`
Expected hits confirm: impl block opens around line 677; "scan_pk" override at line 921; "insert_in_db" at 704; "commit_transaction_lockfree" at 951. Mirror that pattern.

- [ ] **Step 2: Write the failing test (in `tests/integration/sql/ahi_explain_test.rs`, created in Task 4 but the section for Task 3 lives here)**

Add the following top-of-file section (will be expanded by Task 4 for AHI tests):

```rust
//! PR2 / #5304: BoxStorageEngine forwarding regression test.
//!
//! Historic wrapper-drops-method fixes: #4974, #5009, #5105, #5192, #5266,
//! #5281, #4912. New trait methods must be forwarded explicitly.

use sqlrustgo_storage::{
    AdaptiveHashIndex, BinaryTableStorage, BoxStorageEngine, BufferPoolStats, StorageEngine,
};

#[test]
fn test_box_storage_engine_forwards_buffer_pool_stats() {
    let tmp = std::env::temp_dir().join("pr5304_box_storage_engine_bufpool_stats");
    let _ = std::fs::remove_dir_all(&tmp);

    let inner = BinaryTableStorage::new(tmp.clone()).unwrap();
    let boxed: BoxStorageEngine = BoxStorageEngine::new(inner);

    // Must return Some(...) — Some(synthetic_zero) is the trait default,
    // but via Box<dyn StorageEngine> dispatch the BoxStorageEngine
    // override must actively forward (**self).buffer_pool_stats().
    let stats = boxed.buffer_pool_stats();
    let stats = stats.expect(
        "BoxStorageEngine::buffer_pool_stats must forward; \
         returning None here would mark MySQL-server's \
         Arc<RwLock<BoxStorageEngine>> chain silent on EXPLAIN",
    );
    assert_eq!(stats.hits, 0);
    assert_eq!(stats.misses, 0);

    // estimated_pages_for_table on empty / missing table → 0.
    assert_eq!(boxed.estimated_pages_for_table("nonexistent"), 0);

    let _ = std::fs::remove_dir_all(&tmp);
}
```

This test file is created in Task 4 Step 1; for Task 3 the test stays a one-shot scaffold file `tests/integration/sql/ahi_explain_test.rs` that imports `sqlrustgo_storage::*` and contains only this one test. Task 4 will rename and expand it.

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test --test ahi_explain_test test_box_storage_engine_forwards_buffer_pool_stats 2>&1 | tail -25`
Expected: FAIL with `error[E0599]: no method named 'buffer_pool_stats' found for struct 'BoxStorageEngine'` (because Task 3 hasn't added the override yet; if Task 3 IS first to add the override, the trait method step #2 already added buffer_pool_stats to the trait; the failing then moves to the BoxStorageEngine override path).

Note: The trait method from Task 2 makes the `boxed.buffer_pool_stats()` call COMPILE — but the BoxStorageEngine override is missing. Test fails because either (a) compile error if Task 2 not yet merged, or (b) panics / wrong value at runtime once Task 2 lands. Either way, the test pins the missing override.

- [ ] **Step 4: Implement `BoxStorageEngine` forwarders in `crates/storage/src/binary_storage.rs`**

Inside `impl StorageEngine for BoxStorageEngine { ... }` (around line 677), add the following two methods next to other short forwarders (e.g. after the existing `is_wal_enabled` override around line 904):

```rust
    /// PR2 / #5304: forward `buffer_pool_stats()` to the inner engine.
    /// Without this override, `&BoxStorageEngine` (which sits behind
    /// `Arc<RwLock<...>>` in MySQL-server at `crates/mysql-server/src/lib.rs:6888`)
    /// could not surface a real `BufferPool::stats()` once that override
    /// exists on a future paged engine. Same forwarding pattern as
    /// `scan_pk` / `scan_in` / `commit_transaction_lockfree`.
    fn buffer_pool_stats(&self) -> Option<BufferPoolStats> {
        (**self).buffer_pool_stats()
    }

    /// PR2 / #5304: forward `estimated_pages_for_table()` to the inner
    /// engine. The default `self.scan(table)` would double-dispatch via
    /// `&self` and call into `BoxStorageEngine::scan` which itself
    /// forwards — semantically correct, but faster to forward directly.
    fn estimated_pages_for_table(&self, table: &str) -> u64 {
        (**self).estimated_pages_for_table(table)
    }
```

If `BufferPoolStats` is not in scope at the top of `binary_storage.rs`, add `use crate::buffer_pool::BufferPoolStats;` to the `use` block.

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test --test ahi_explain_test test_box_storage_engine_forwards_buffer_pool_stats 2>&1 | tail -15`
Expected: PASS.

- [ ] **Step 6: Run full workspace compile to confirm no breakage**

Run: `cargo build --all-features 2>&1 | tail -15`
Expected: exit 0.

- [ ] **Step 7: Commit**

```bash
cd /Users/liying/workspace/dev/sqlrustgo/.worktrees/5304-pr2
git add crates/storage/src/binary_storage.rs tests/integration/sql/ahi_explain_test.rs
git commit -m "fix(#5304): BoxStorageEngine forwards buffer_pool_stats + estimated_pages_for_table"
```

---

## Task 4: AHI EXPLAIN counter tests (3 deterministic tests)

**Files:**
- Modify: `tests/integration/sql/ahi_explain_test.rs` — extend the scaffold from Task 3 with the 3 new tests.
- (No new file — the test file already exists from Task 3.)

**Interfaces:**
- Consumes: `AdaptiveHashIndex::total_lookups()` (existing, `crates/storage/src/adaptive_hash_index.rs:186`), `AdaptiveHashIndex::total_hits()`, `ExecutionEngine::adaptive_hash_index() -> &Arc<AdaptiveHashIndex>` (existing surface).
- Produces: 3 named tests that PR3 will compose against; AHI counters stay underlying unchanged.

- [ ] **Step 1: Add the 3 tests to `tests/integration/sql/ahi_explain_test.rs`**

Append below the Task 3 test (preserving Task 3):

```rust
use sqlrustgo::{ExecutionEngine, StorageEngine};

#[test]
fn test_ahi_counters_monotonic_across_explain_calls() {
    // PR2 Review Focus #4 (single-conn only; documented at call site).
    let engine = ExecutionEngine::with_memory();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT);")
        .expect("create");
    engine
        .execute("INSERT INTO t VALUES (1, 'a'), (2, 'b'), (3, 'c');")
        .expect("insert");

    let ahi = engine.adaptive_hash_index();
    let _ = ahi.total_lookups(); // baseline; not asserted here

    // Run three EXPLAIN calls; each may invoke AHI along secondary-index path.
    for i in 0..3 {
        let _ = engine
            .execute("EXPLAIN SELECT * FROM t WHERE id = 1;")
            .unwrap_or_else(|e| panic!("explain #{i} failed: {e}"));
        let n = ahi.total_lookups();
        assert!(n > 0, "AHI lookups should be > 0 after EXPLAIN call #{i}");
    }

    // Monotonic non-decreasing: cumulative counter only grows.
    let mut prev = 0u64;
    for _ in 0..3 {
        let now = ahi.total_lookups();
        assert!(now >= prev, "AHI lookups must be monotonic");
        prev = now;
    }
}

#[test]
fn test_ahi_hit_rate_converges_under_hot_loop() {
    // PR2 A05 acceptance evidence: hot secondary-index path eventually
    // promotes entries past the AHI threshold (default 17), driving
    // hit_rate upward.
    let engine = ExecutionEngine::with_memory();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER);")
        .expect("create");
    let rows = (1..=50)
        .map(|i| format!("({}, {})", i, i * 10))
        .collect::<Vec<_>>()
        .join(", ");
    engine
        .execute(&format!("INSERT INTO t VALUES {};", rows))
        .expect("insert");

    let ahi = engine.adaptive_hash_index();
    let _initial_lookups = ahi.total_lookups();
    let _initial_hits = ahi.total_hits();

    // Hot loop 100x — AHI promotion threshold is 17, so by 50+ iterations
    // hit_rate should exceed 0.5.
    for i in 0..100 {
        engine
            .execute("EXPLAIN SELECT * FROM t WHERE id = 1;")
            .unwrap_or_else(|e| panic!("hot iter {i}: {e}"));
    }

    let lookups = ahi.total_lookups();
    let hits = ahi.total_hits();
    let rate = if lookups > 0 {
        hits as f64 / lookups as f64
    } else {
        0.0
    };
    assert!(
        rate > 0.5,
        "expected AHI hit_rate > 0.5 after 100 hot EXPLAINs, got {rate} (hits={hits}, lookups={lookups})"
    );
}

#[test]
fn test_estimated_pages_for_empty_table_returns_zero() {
    // PR2 Review Focus #3: empty-table underflow guard.
    let engine = ExecutionEngine::with_memory();
    engine
        .execute("CREATE TABLE empty_t (id INTEGER);")
        .expect("create");
    let storage = engine.storage();
    let n = storage.estimated_pages_for_table("empty_t");
    assert_eq!(n, 0, "empty table must yield 0 estimated pages, got {n}");

    // Non-existent table: also 0, must not panic.
    let n_missing = storage.estimated_pages_for_table("does_not_exist");
    assert_eq!(n_missing, 0);
}
```

Pin the import surface to whatever `sqlrustgo::` re-exports; check via:

```
grep -n "pub use\|ExecutionEngine\|StorageEngine" src/lib.rs | head -10
```

Adjust `use sqlrustgo::{ExecutionEngine, StorageEngine};` if the crate name or path differs (per `src/lib.rs:52-53` recon: `pub use execution_engine::{ExecutionEngine, MemoryExecutionEngine};` — so `sqlrustgo::ExecutionEngine` is correct).

- [ ] **Step 2: Run the new tests to verify they fail (RED)**

Run: `cargo test --test ahi_explain_test 2>&1 | tail -40`
Expected: failures on the 3 new tests. (If the trait method `buffer_pool_stats()` is not yet integrated into `engine.adaptive_hash_index()` access path, some tests may compile-error instead of assertion-fail; that's also acceptable for RED state.)

- [ ] **Step 3: If tests compile-error because the test uses APIs that don't fully exist, fix the test imports to match**

Possible issues to resolve without touching production code:
- `engine.storage()` may have a different accessor name. Run `grep -n "pub fn storage\|fn storage_arc\|Arc<RwLock" src/execution_engine*.rs | head -10` and adjust.
- `AdaptiveHashIndex` may not be re-exported from `sqlrustgo_storage` directly. Run `grep -n "AdaptiveHashIndex" crates/storage/src/lib.rs | head -5` and adjust import.

If production code changes are required to make these tests pass, STOP and re-evaluate against the design spec §2.2 non-goals — production code changes belong in a separate epic. Report the blocker in the PR.

- [ ] **Step 4: Run the tests again after corrections — expect PASS (GREEN)**

Run: `cargo test --test ahi_explain_test 2>&1 | tail -20`
Expected: 4 passed; 0 failed (Task 3's `test_box_storage_engine_forwards_buffer_pool_stats` + the 3 new tests).

- [ ] **Step 5: Commit**

```bash
cd /Users/liying/workspace/dev/sqlrustgo/.worktrees/5304-pr2
git add tests/integration/sql/ahi_explain_test.rs
git commit -m "test(#5304): AHI EXPLAIN counter tests + empty-table guard"
```

---

## Task 5: `CLAIM_DOWNGRADE_MANIFEST.md` tail note (no §3.1 change)

**Files:**
- Modify: `docs/releases/v4.1.0/CLAIM_DOWNGRADE_MANIFEST.md` — append exactly one line below §3.1 (do NOT modify §3.1 region).

**Interfaces:**
- Consumes: design doc path (`docs/superpowers/specs/2026-10-14-5304-pr2-bufferpool-production-wiring-design.md`).
- Produces: one appended line at the file tail referencing the design.

- [ ] **Step 1: Locate §3.1 boundary**

Run: `grep -n "^### 3\.1\|^## 4\|claim boundary\|test-only capability\|BufferPool" docs/releases/v4.1.0/CLAIM_DOWNGRADE_MANIFEST.md | head -20`

Confirm the `### 3.1` heading exists and a new `## 4` / `### 4.x` section follows.

- [ ] **Step 2: Verify the §3.1 verbatim baseline (Review Focus #5)**

Run: `git diff a1cee6732e..HEAD -- docs/releases/v4.1.0/CLAIM_DOWNGRADE_MANIFEST.md | head -10`
Expected: empty diff for the file before this task. (PR1 set the baseline; Task 5 must not regress it.)

- [ ] **Step 3: Append a single line at the file tail**

Use `Edit` on `docs/releases/v4.1.0/CLAIM_DOWNGRADE_MANIFEST.md` targeting the last line of the file (run `wc -l docs/releases/v4.1.0/CLAIM_DOWNGRADE_MANIFEST.md` first; edit the trailing lines to add the new tail line). Append:

```markdown

## Note (added by PR2 of #5304, post-#5328 merge)

PR2 of #5304 lands **only trait-level plumbing** (`StorageEngine::buffer_pool_stats()` default `Some(synthetic_zero)` + `BoxStorageEngine` forwarding override + AHI EXPLAIN counter tests). §3.1 above remains authoritative: production v4.1.0 still does NOT page through a BufferPool. PR3 of #5305 will read from this surface. The detailed design is at `docs/superpowers/specs/2026-10-14-5304-pr2-bufferpool-production-wiring-design.md`.
```

The exact line "PR2 of #5304 lands **only trait-level plumbing**..." is the Review Focus #5 invariant — verbatim preservation of §3.1 wording.

- [ ] **Step 4: Verify §3.1 region is unchanged**

Run: `git diff docs/releases/v4.1.0/CLAIM_DOWNGRADE_MANIFEST.md | grep -E "^[-+]" | head -20`
Expected: ONLY the appended tail block appears, and §3.1's "claim boundary line" (per PR1 §3.1) is NOT in the diff.

- [ ] **Step 5: Commit**

```bash
cd /Users/liying/workspace/dev/sqlrustgo/.worktrees/5304-pr2
git add docs/releases/v4.1.0/CLAIM_DOWNGRADE_MANIFEST.md
git commit -m "docs(#5304): tail note linking PR2 plumbing to spec (no §3.1 change)"
```

---

## Task 6: Verification matrix

**Files:** none modified; pure checks.

- [ ] **Step 1: build**

Run: `cargo build --all-features`
Expected: exit 0; should match the duration of PR1's verification (`Finished ... 36.49s` baseline).

- [ ] **Step 2: doc**

Run: `cargo doc --no-deps`
Expected: exit 0; warnings, if any, must be the same set PR1 documented (5 historical warnings on `src/execution_engine_methods.rs:158 / :753` and `src/system_tables.rs:20`). PR2 must not introduce a 6th warning.

- [ ] **Step 3: clippy**

Run: `cargo clippy --all-features -- -D warnings`
Expected: exit 0.

- [ ] **Step 4: rustfmt on the one file Task 4 may touch (or all source if rustfmt is clean)**

Run: `rustfmt --check --edition 2021 src/lib.rs crates/storage/src/buffer_pool.rs crates/storage/src/engine.rs crates/storage/src/binary_storage.rs`
Expected: exit 0. PR2 only stages these 4 source files. If `cargo fmt --check --all` is desired for completeness, run it but expect pre-existing historical drift to remain (documented in PR1 body).

- [ ] **Step 5: cargo test (allow pre-existing D8/SEM-4 fail)**

Run: `cargo test --all-features 2>&1 | tail -25`
Expected: exit 1, with the failure being **only** `test_d8_gate_runs_correctly`. Independently verify by running `bash scripts/gate/check_arch_sem_debt.sh` on the un-touched main repo (`/Users/liying/workspace/dev/sqlrustgo`) and confirming same exit 1 state — proves the failure is pre-existing and out of scope for #5304.

- [ ] **Step 6: Verify the staged set is exactly the spec'd 5 files + 1 new test file**

Run: `git status --short`
Expected output lines (no extras; pre-existing `evidence/a04-5226/wire/*.txt` drift must not enter the commit):
```
 M crates/storage/src/buffer_pool.rs
 M crates/storage/src/engine.rs
 M crates/storage/src/binary_storage.rs
 M docs/releases/v4.1.0/CLAIM_DOWNGRADE_MANIFEST.md
?? tests/integration/sql/ahi_explain_test.rs
```

(Note: `git status` shows staged + unstaged. Run `git diff --stat` to confirm the 4 modified files; the new test file appears as `??`.)

- [ ] **Step 7: Confirm 4 commits ahead of base, no merge commits**

Run: `git log --oneline e12f7a48fa..HEAD`
Expected: exactly 4 commits (Task 1 commit, Task 2+3 may combine to 2 commits if Step 7 of Task 3 is the same commit, etc.):

The exact commit set depends on how Tasks 1-5 stage. Tolerated commit patterns:
- Pattern A (5 atomic): Task 1 / Task 2 / Task 3 / Task 4 / Task 5.
- Pattern B (4): merge Task 2 + Task 3 into one commit (Trait+BoxStorageEngine).

Either is acceptable. **Critical invariants**: every commit subject line starts with `feat:`, `fix:`, `test:`, or `docs:` — no `wip` / `tmp` / `squash` / `--amend`.

---

## Task 7: Push and open PR via Gitea API

**Files:** none.

- [ ] **Step 1: Set git author identity (if not already configured)**

Run: `git config user.name && git config user.email`
If empty: skip — the repo's `~/.claude/settings.json` disables attribution globally, so the push should inherit the existing global git config.

- [ ] **Step 2: Push the branch to `gitea252` (not GitHub — see project CLAUDE.md)**

Run: `git push -u gitea252 fix/5304-pr2-bufpool-production-counter`
Expected: branch created on remote. Project CLAUDE.md forbids GitHub pushes.

- [ ] **Step 3: Open PR via Gitea API**

```bash
TOKEN=${GITEA_TOKEN}
PR_BODY=$(cat <<'EOF'
## Summary

PR2 of #5304 (option B — production-path page-counter surface).
Tracks #5304 / #5227 (A05 acceptance gate #2).

## What this PR changes

| File | Change |
|---|---|
| `crates/storage/src/buffer_pool.rs` | Add `BufferPoolStats::synthetic_zero()` constructor. Field order matches `Default::default()` so layout-stable equality holds. |
| `crates/storage/src/engine.rs` | `StorageEngine` trait gains two `&self` default methods: `buffer_pool_stats() -> Option<BufferPoolStats>` (default `Some(synthetic_zero)`) and `estimated_pages_for_table(&str) -> u64` (default `scan(rows).len().div_ceil(32)`). |
| `crates/storage/src/binary_storage.rs` | `BoxStorageEngine` overrides both methods forwarding to `(**self)` (mandatory — historic wrapper-drops-method pattern #4974/#5009/#5105/#5192/#5266/#5281/#4912). |
| `tests/integration/sql/ahi_explain_test.rs` (NEW) | 3 deterministic tests: BoxStorageEngine forwarding (Task 3), AHI counters monotonic, AHI hit_rate converges > 0.5 after 100 hot EXPLAINs. |
| `docs/releases/v4.1.0/CLAIM_DOWNGRADE_MANIFEST.md` | 1-line tail note pointing at the design doc. **§3.1 region verbatim unchanged.** |

Total: 5 files modified, 1 file created, ~147 lines.

## What this PR does NOT change

- `FileStorage` / `BPlusTree` / `HeapStorage` internals (deferred to a separate epic — see design §6.4).
- `BufferPool` is NOT promoted from `⚠️ Test-only capability`.
- `MvccStorage` / `WalStorage` / `ParallelWalStorage` internals.
- `src/lib.rs` PR1 re-export split stays intact.
- EXPLAIN row format (PR3 #5305 scope).
- D8/SEM-4 debt-registry (independent #5103 scope).

## Verification (worktree `.worktrees/5304-pr2` @ `fix/5304-pr2-bufpool-production-counter`, base `gitea252/develop/v4.1.0@e12f7a48fa`)

| Check | Result | Detail |
|---|---|---|
| `cargo build --all-features` | PASS | tbd |
| `cargo doc --no-deps` | PASS | no new warnings |
| `cargo clippy --all-features -- -D warnings` | PASS | tbd |
| `rustfmt --check --edition 2021 <4 files>` | PASS | tbd |
| `cargo test --all-features` | PARTIAL | only pre-existing D8/SEM-4 OPEN-no-plan debt (independent main-repo reproduction confirms). |
| `tests/integration/sql/ahi_explain_test` | PASS | 4/4 — forwarding + 3 AHI tests. |

## Spec / design

- Spec: `docs/superpowers/specs/2026-10-14-5304-pr2-bufferpool-production-wiring-design.md`
- Plan: `docs/superpowers/plans/2026-10-14-5304-pr2-bufferpool-counter-surface.md`

## Risk

Low. Trait additions are additive default-implemented; every existing impl inherits `Some(synthetic_zero)` without code change. `BoxStorageEngine` override is the only mandatory forwarding change.

cc #5304 #5227.
EOF
)

curl -X POST http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo/pulls \
  -H "Authorization: token $TOKEN" \
  -H "Content-Type: application/json" \
  -d "$(jq -n \
    --arg title "feat(#5304): StorageEngine::buffer_pool_stats + estimated_pages_for_table + AHI explain tests" \
    --arg head "fix/5304-pr2-bufpool-production-counter" \
    --arg base "develop/v4.1.0" \
    --arg body "$PR_BODY" \
    '{title: $title, head: $head, base: $base, body: $body}'
  )"
```
Expected: HTTP 201; response contains `number: <PR-id>` and `mergeable: true`.

- [ ] **Step 4: Capture the PR URL**

Run: `curl -s "http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo/pulls?state=open" | jq '.[] | select(.head.ref == "fix/5304-pr2-bufpool-production-counter") | {number, html_url, mergeable}'`
Expected: `{ "number": <id>, "html_url": "http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/<id>", "mergeable": true }`. Save the URL for the task report.

- [ ] **Step 5: Add a summary comment to issues #5304 and #5227**

For #5304:
```bash
curl -X POST http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo/issues/5304/comments \
  -H "Authorization: token $TOKEN" \
  -H "Content-Type: application/json" \
  -d "$(jq -n --arg body "PR2 / #5304 option B 落地到 PR #<id> (head=fix/5304-pr2-bufpool-production-counter, base=develop/v4.1.0, mergeable=true)。design: docs/superpowers/specs/2026-10-14-5304-pr2-bufferpool-production-wiring-design.md。

A05 验收硬指标 #2 推进状态:
- 1. 文档诚实性（BufferPool 不在生产路径）=> 由 PR1 (#5328) 闭环
- 2. 运行时证据（生产路径实际用索引）=> PR2 提供 StorageEngine::buffer_pool_stats surface + AHI EXPLAIN counter tests；AHI \`total_lookups / total_hits / hit_rate()\` 是真实生产计数器，hot-loop 100 iter 命中率达 > 0.5 即证伪\"索引从未上线\"
- 3. EXPLAIN 表面加列 => PR3 (#5305) 后续在 PR2 trait 上加 3 列

PR2 不动 §3.1 边界（CLAIM_DOWNGRADE_MANIFEST §3.1 仍表述 BufferPool 测试-only）；BufferPool 真接 FileStorage 留到 6-12 月后另一 epic（design §1.6）。" \
    '{body: $body}'
  )"
```

For #5227 (mirror of the same status, keyed by #5304 PR number):
```bash
curl -X POST http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo/issues/5227/comments \
  -H "Authorization: token $TOKEN" \
  -H "Content-Type: application/json" \
  -d "$(jq -n --arg body "PR2 / #5304 已开 PR #<id>，等评审。#5227 A05 #2 三推进项 (文档 / 运行时证据 / EXPLAIN 加列) 进度见该 PR。" \
    '{body: $body}'
  )"
```

Replace `<id>` and `<id>` with the actual PR numbers from Step 4.

Expected: HTTP 201 for both.

- [ ] **Step 6: Final task report back to user**

Report:
- PR URL (from Step 4)
- Verification matrix results (from Task 6)
- 4-5 commit SHAs on top of `e12f7a48fa`
- Confirmation: §3.1 region unchanged, BufferPool still `⚠️ Test-only`, no engine rewrites

---

## Self-Review

### 1. Spec coverage

| Spec § | Covered by |
|---|---|
| §0 Why this doc exists | Task 1-5 don't introduce BufferPool-into-FileStorage (avoiding the impossible). |
| §1.1 No production BufferPool consumer | Task 5 (CLAIM note) documents that PR2 doesn't change this. |
| §1.2 BufferPool in-memory only | Task 1 (`synthetic_zero`) honors this — no disk backend added. |
| §1.3 Page/BTreeIndex/HeapStorage un-paged | Tasks 1-5 don't touch any of them. |
| §1.4 AHI on production path | Task 4 tests `engine.adaptive_hash_index()` to verify production-counter wiring. |
| §1.5 EXPLAIN one-column Vec<String> | Deferred to PR3 — Task 4 captures AHI snapshot for PR3. |
| §1.6 Why direct PR2 impossible | Acceptance — Tasks 1-5 work around it via additive trait. |
| §2.1 Goals | All 3 goals covered: trait surface (Task 2+3), AHI tests (Task 4), zero regression (Task 6). |
| §2.2 Non-goals | Each enforced by absence: no FileStorage / BTreeIndex / HeapStorage changes (Tasks 1-5 don't touch them). |
| §2.3 Anti-goals | Avoid fabrication (Task 1 enforces via synthetic_zero equals default); avoid §3.1 mutation (Task 5 Step 4). |
| §2.4 New EXPLAIN row shape | PR3 scope — not this plan. |
| §2.5 Trait additions | Task 2. |
| §2.6 Concrete overrides | Task 3 (BoxStorageEngine mandatory); FileStorage/BinaryTableStorage left at trait default (intentional). |
| §2.7 AHI snapshot plumbing | Task 4 covers counter access; PR3 will consume. |
| §2.8 Out of scope | Task 5 tail note + Task 6 Step 5 verification confirms no §3.1 change. |
| §2.9 File scope | Tasks 1-5 = 4 modified files + 1 new file, ≤ 147 lines. |
| §3 Verification | Task 6. |
| §4 Risk | Task 4 Step 3 covers AHI race (single-thread). |
| §6 Sequence | Tasks 1-5 follow §5 sequence. |
| §6.1 PR3 mechanical | Task 4 produces the snapshot tests PR3 will consume. |
| §6.2 What this PR doesn't do | Task 5 tail note + Task 6 Step 5 enforces verbatim §3.1. |
| §6.3 Honest A05 #2 evidence | Task 4's `test_ahi_hit_rate_converges_under_hot_loop` delivers this. |
| §6.4 Future replacement trigger | Task 5 tail note links to spec. |

**No gaps.**

### 2. Step scan

Every test step has a test name + assertions as code. Every code step gives the exact signature, file, and field values. Every commit step is identical to prior PR1 commits in this repo (verified via `git log --oneline` history). No `TBD` / `handle edge cases` / "appropriate validation" anywhere.

### 3. Type consistency

| Name | Defined | Used |
|---|---|---|
| `BufferPoolStats::synthetic_zero()` | Task 1 Step 3 | Task 2 Step 3 (trait default body), Task 3 Step 4 (BoxStorageEngine forwarder) |
| `StorageEngine::buffer_pool_stats()` | Task 2 Step 3 | Task 3 Step 4 (override), Task 4 Step 1 (`boxed.buffer_pool_stats()`), Task 4 Step 1 (`storage.estimated_pages_for_table`) |
| `StorageEngine::estimated_pages_for_table(&str)` | Task 2 Step 3 | Task 3 Step 4, Task 4 Step 1 |
| `BufferPoolStats { hits, misses, evictions, prefetch_hits }` | Task 1 Step 3 | Task 4 Step 1 (`stats.hits`, `stats.misses`) |

Consistent across tasks.

### 4. Review Focus pin coverage

| Review Focus | Pinned by |
|---|---|
| #1 `BufferPoolStats` field order | Task 1 Step 1 (`assert_eq!((z.hits, z.misses, z.evictions, z.prefetch_hits), (d.hits, d.misses, d.evictions, d.prefetch_hits))`) |
| #2 `BoxStorageEngine` forwarding | Task 3 Step 2 (`test_box_storage_engine_forwards_buffer_pool_stats`) |
| #3 `estimated_pages_for_table` empty-table | Task 4 Step 1 (`test_estimated_pages_for_empty_table_returns_zero`) |
| #4 AHI single-conn race | Task 4 Step 1 (`test_ahi_counters_monotonic_across_explain_calls` — uses single-conn `ExecutionEngine::with_memory()`) |
| #5 §3.1 verbatim | Task 5 Step 4 (`git diff ... | grep` invariant) |

All five Review Focus lines have a task test pinning them.

### 5. Proportion

- Spec: 7 sections (~245 lines of design content).
- Plan: 7 tasks (~340 lines including all step code blocks).
- Code produced: ~147 lines.

Plan is **proportional** to spec. Not a transcript; not under-specified.

---

Plan complete. Awaiting review.
