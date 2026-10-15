# BufferPool Paged Engine Epic — Design Spec

> **Issue**: #5305 followup (PR2 spec §2.2 listed this as non-goal; PR2 PR-A actually punted it)
> **Date**: 2026-10-15
> **Status**: Draft for approval (HARD-GATE per superpowers:brainstorming)
> **Owner**: implementer will be the same agent / same workflow as PR2 + PR3

---

## Intent & Why

This is the **6-12 month architectural epic** to replace `FileStorage` HashMap with
`Page`-backed heap files and wire `BufferPool::pin/unpin/flush_page` into a real paged
engine. The goal is not raw throughput — it is to make `BufferPoolStats::pages_visited`
an honest signal and to put "this is a real DB" on the page in the code.

### Stated priority (user, 2026-10-15)

1. **(c) Architectural integrity** — the engine looks like a database in the code, even if
   some engines temporarily run alongside a stub.
2. **(b) Observability / EXPLAIN signal** — `pages_visited` (and friends) become real,
   not always-zero.
3. **(a) Production data capacity** — bonus, not the driver. If we get it, fine; if
   not, the epic still paid for itself.

### Why not the AHI lookup-side wiring alternative

The 1-3 month AHI lookup wiring was the other option. It was rejected because it does
not address (c): even with AHI wiring, the storage layer is still a HashMap in memory,
which is the deeper architectural gap.

---

## Goals

- `StorageEngine` trait exists; at least one production engine (`FileStorage`) returns
  real page-granularity I/O via `BufferPool` after PR-A.
- `BufferPoolStats::pages_visited` (and `cache_hits`, `cache_misses`, `disk_reads`,
  `disk_writes`) become honest EXPLAIN signals — currently always 0.
- `ExecutionEngine::with_memory()` and `ExecutionEngine::execute()` public API
  unchanged across the entire epic.
- All existing tests pass without modification across every slice PR.
- Each PR is independently mergeable; nothing requires a "big bang" cutover.

## Non-Goals

- Compete with Postgres / SQLite / DuckDB on throughput.
- Replace MVCC semantics, the WAL protocol, or the page-format choice of the existing
  in-mem Page module.
- Add new SQL features, new indexes, new planners.
- Multi-process / distributed anything.
- `ParallelWalStorage` rewrite before the basic WAL path is paged (PR-E only after PR-D).

## Anti-Goals (would be a failure even if "done")

1. **Public-API break.** Any commit that forces rewrites of `tests/` callers of
   `ExecutionEngine::with_memory()` / `execute()` is a failure — the spec violated its
   own goal.
2. **Durability regression.** MVCC semantics or WAL recovery breaks. A simulated crash
   that previously lost 0 committed rows now loses any committed row is a failure.
3. **Performance regression > 2x.** INSERT/SELECT on the existing micro-benchmarks
   regresses by more than ~2x is a failure. (We accept temporary regression while a
   slice is mid-flight, but not at slice-close.)
4. **Big-bang cutover.** Any commit where the suite is red because the migration is
   half-done is a failure. Each slice must land green before the next starts.
5. **PR3 contract violation.** The `| ahi_promoted=<u64>` snapshot semantics from
   PR3 / #5305 must remain exact — `pages_visited` is additive, not a replacement.

## Constraints

- **PR2 baseline already landed.** `BufferPool` exists with stats; PR2's EXPLAIN hook
  exposes `pages_visited` (always 0 today). PR-A must make it non-zero without
  breaking PR2's tests (`tests/integration/sql/ahi_explain_test.rs`).
- **PR3 baseline already landed.** The `ahi_promoted` EXPLAIN suffix must remain.
- **Gitea workflow.** PRs go to `gitea252` (currently unreachable; gitea250 is the
  fallback remote). PR comments on issues #5305 / #5304 / #5227 must be posted when
  Gitea recovers.
- **Toolchain.** Rust 2024, Tokio, all existing crate boundaries.
- **Existing test suite.** 100+ storage / DML / EXPLAIN tests must pass at every
  commit. No test rewrites allowed.

---

## Architecture (the how, decided)

### Single trait, engine-by-engine migration

A `StorageEngine` trait exists already in spirit (`FileStorage`, `HeapStorage`,
`MvccStorage`, `WalStorage`, `ParallelWalStorage` all implement `insert` / `scan` /
`update` / `delete` / `commit` / `rollback` with overlapping but not identical
signatures). The trait surface is **consolidated and frozen in PR-A**, then each
engine migrates to a `BufferPool`-backed implementation one PR at a time.

### Page / BufferPool contract

- `BufferPool::pin(page_id) -> PageGuard` already exists (PR2 work).
- `PageGuard::read() / write() / mark_dirty()` already exists.
- The migration per engine is: take its in-memory `HashMap<Key, Value>` (or its
  hand-rolled `Vec<Option<BTreeNode>>` for BPlusTree), translate `Key` → `page_id`
  via a configurable index scheme (linear for PR-A, B+ for PR-C), pin, read/write,
  unpin.

### Stats contract

`BufferPoolStats` already records `pages_visited`, `cache_hits`, `cache_misses`,
`disk_reads`, `disk_writes`. After PR-A these become non-zero for `FileStorage`.
After PR-D they become non-zero for the WAL path.

**EXPLAIN suffix format** (binding decision 2026-10-15, mirror PR3 row-end
append):

```
 | ahi_promoted=<u64> | pages_visited=<u64> | cache_hits=<u64> | cache_misses=<u64> | disk_reads=<u64> | disk_writes=<u64>
```

Snapshot at EXPLAIN entry (same semantics as PR3's `ahi_promoted`). Order of
keys is fixed (left to right). Each key is appended once the corresponding stat
is non-zero in that slice — pre-rollout keys are omitted. PR3's `ahi_promoted`
key stays first.

---

## Slices (PR-A through PR-E)

The user-approved slicing (2026-10-15). Sum of slice upper-bounds = 27, sum = 16.
This exceeds the original "6-12 month" PR2 spec §2.2 estimate; the new estimate
inherits from this spec. **Ruled: each slice duration is a working estimate, not
a contract — per-slice plans tighten or extend as discovery happens. The 6-12
month figure referred to the un-sliced whole; sliced, the per-PR targets below
are the operating numbers.**

### PR-A: `StorageEngine` trait + `FileStorage` → Page/BufferPool (1-2 months)

**Scope**:
- Freeze the `StorageEngine` trait (consolidate signatures).
- Reimplement `FileStorage::insert` / `select` / `update` / `delete` using
  `BufferPool::pin/unpin/flush_page`. Keep `FileStorage`'s existing public methods.
- Add tests proving `BufferPoolStats::pages_visited > 0` after `FileStorage` writes.
- Other 5 engines (`HeapStorage`, `BPlusTree`, `MvccStorage`, `WalStorage`,
  `ParallelWalStorage`) remain unchanged.

**Acceptance**:
- All existing `FileStorage` tests pass unchanged.
- New test: 1 INSERT + 1 SELECT through `FileStorage` results in `pages_visited >= 1`.
- PR2 `ahi_explain_test` still passes (no regression on `ahi_promoted` snapshot).
- PR3 `explain_ahi_promoted_test` still passes.

### PR-B: `HeapStorage` follows same pattern (2-3 months)

**Scope**:
- Reimplement `HeapStorage::insert` / `scan` / `update` / `delete` via
  `BufferPool`. (HeapStorage is the more commonly-hit engine in the SQL execution
  path; PR-B is when `pages_visited` first becomes non-zero in the SQL-path EXPLAIN.)
- Share the trait surface from PR-A.

**Acceptance**:
- All existing `HeapStorage` tests pass unchanged.
- New test: a multi-row `INSERT ... SELECT` through `ExecutionEngine::execute()`
  results in `pages_visited > 0` in PR3's EXPLAIN path.

### PR-C: `BPlusTree` uses BufferPool (3-4 months)

**Scope**:
- `BPlusTree` currently uses `Vec<Option<BTreeNode>>` in memory. Translate to
  `BufferPool`-backed pages; index pages and table pages share the same cache.
- BPlusTree-specific tests for split / merge / range scan must pass.

**Acceptance**:
- All existing BPlusTree tests pass unchanged.
- Index lookup EXPLAIN (when added later) shows non-zero `pages_visited` on cold
  cache.

### PR-D: `MvccStorage` / `WalStorage` adapt + crash recovery (4-6 months)

**Scope**:
- `MvccStorage` transaction-version pages go through BufferPool.
- `WalStorage` log records go through BufferPool with proper fsync semantics.
- New crash-recovery test suite with **real fault-injection** (SIGKILL
  mid-transaction, simulated power-cut via process kill + restart) — not
  unit-level mocks. CI-friendly: faults run as subprocess tests, not flaky
  in-process.

**Acceptance**:
- Existing MVCC tests pass.
- Existing WAL tests pass.
- New crash-recovery fault tests pass (kill mid-write → restart → verify
  committed rows preserved, uncommitted rows discarded).
- `pages_visited` non-zero in WAL-path EXPLAIN.

### PR-E: `ParallelWalStorage` + multi-pool partitioning + perf parity (6-12 months)

**Scope**:
- `ParallelWalStorage` multi-buffer-pool partitioning (per-shard BufferPool).
- Performance tuning: clock-sweep or LRU-k replacement policy choice; batch
  flushing; group commit.
- Benchmark parity: INSERT/SELECT within 2x of pre-PR-A HashMap baseline.

**Acceptance**:
- Existing benchmarks within 2x of baseline.
- Multi-pool partitioning tested with concurrent INSERT workload.

---

## Migration strategy (decided)

- **Trait-based, not big-bang.** PR-A freezes the trait; each subsequent slice
  migrates one engine behind the trait.
- **Shadow mode optional.** PR-A does not require a shadow mode (the existing
  `FileStorage` impl is removed in PR-A and replaced). PR-D adds a shadow mode for
  crash recovery (WAL writes go to both old and new path during one release, then
  old removed).
- **Per-slice feature flag.** Each engine has a `*_via_bufferpool()` constructor;
  the old `*::new()` constructor remains as a stub returning "not yet migrated"
  for engines that haven't been migrated yet, so the type system forces
  completeness.

---

## What's left to the builder (the spec author did NOT decide these)

These go in the per-slice plans, not in this epic spec:

- Exact `StorageEngine` trait method signatures (PR-A decision).
- Pin/unpin ownership model — RAII guard (current PageGuard) vs. explicit
  acquire/release (PR-A decision).
- Page format on disk — header schema, checksum algorithm, version field
  (PR-A decision; PR-D may add WAL-specific overlay).
- BufferPool replacement policy when going multi-pool — clock-sweep, LRU-k,
  or 2Q (PR-E decision).
- Whether shadow mode in PR-D is byte-level or logical (PR-D decision).

---

## "Theirs" pile (questions only the user can answer, surfaced for spec sign-off)

All four items resolved by the user (2026-10-15). Their answers are written
into the spec as binding decisions below; this section is preserved as the
decision log.

0. **Timeline reality check.** *Answer: accepted 16-27 months as the new total.*
   Working estimate, not a contract — per-slice plans tighten or extend.
1. **PR-E necessity.** *Answer: in scope.* PR-E remains in this epic.
2. **Crash-recovery tests in PR-D.** *Answer: real fault-injection framework.*
   PR-D ships a CI-friendly fault harness (SIGKILL mid-transaction / power-cut
   simulation / consistency verification on restart), not unit-level mocks.
3. **EXPLAIN suffix format for new stats.** *Answer: row-end append, mirror PR3.*
   Every EXPLAIN row will end with:
   ` | ahi_promoted=<u64> | pages_visited=<u64> | cache_hits=<u64> | cache_misses=<u64> | disk_reads=<u64> | disk_writes=<u64>`
   Order of keys is fixed (left to right). Each key is appended once the
   corresponding stat is non-zero in that slice; pre-rollout keys are omitted.
   PR3's `ahi_promoted` key stays first.

---

## Open questions deferred to per-slice plans

- Per-slice test fixtures (how to construct a minimal BufferPool for unit tests
  without standing up the whole engine).
- Per-slice benchmark selection (which existing benchmarks to track regression
  against).
- Per-slice EXPLAIN integration order (which stats get exposed in which slice).

---

## What this spec does NOT cover

- AHI lookup-side wiring (separate epic, deferred — was the 1-3 month alternative).
- Multi-process / distributed SQLRustGo.
- New SQL features (window functions, CTEs, JSON, etc.).
- Performance work beyond "within 2x of baseline".
- Storage format migration of any existing on-disk data (none exists; pre-PR-A
  is in-memory only).
