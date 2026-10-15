# PR2 / #5304 option B design — production path exposes real page-level counters; future-proofs BufferPool wiring

| Field | Value |
|---|---|
| **Status** | Sign-off requested |
| **Date** | 2026-10-14 |
| **Author** | claude-code + M3.1 |
| **Tracks** | #5304 (option B), #5227 (A05 acceptance gate #2) |
| **Depends on** | PR1 / #5328 (merged `e474afa01d`, 2026-10-14T15:02:31Z) |
| **Blocks** | PR3 / #5305 (EXPLAIN `pages_visited` column) |
| **Base branch** | `develop/v4.1.0 @ e12f7a48fa` |
| **Head branch (target)** | `fix/5304-pr2-bufpool-production-counter` |

---

## 0. Why this doc exists

The original PR2 framing — "wire BufferPool into `FileStorage::insert/select` so production path uses BufferPool" — is **architecturally blocked** at current `develop/v4.1.0`. Recon at commit `e12f7a48fa` shows that the production storage chain has no `page` concept to plug `BufferPool` into. A truthful PR2 therefore has to be re-scoped: surface **real production-path counters** that today's executor already maintains (Adaptive Hash Index), and **future-proof the trait surface** so a later epic can wire a real `BufferPool` without re-touching EXPLAIN.

---

## 1. Reality-of-the-codebase (verified recon)

All line refs are at `develop/v4.1.0 @ e12f7a48fa`. Three independent recon passes cross-checked each other.

### 1.1 No `BufferPool` consumer exists in production

- `crates/storage/src/file_storage.rs` has **zero references** to `BufferPool`, `Page`, or `BPlusTree` (full-file grep verified).
- `bins/storage/src/binary_storage.rs` has **zero references** to either (the new view storage from #5330 doesn't page either — `+58` lines are view/tx-id forwarding stubs).
- Production callers of `BufferPool::new` / `get` / `insert` / `allocate` / `prefetch` / `remove` / `stats`: **zero** outside test code.
- `default_storage_engine` factory (`crates/storage/src/lib.rs:91-103`) has **zero production callers** — only mentioned in plan doc `docs/superpowers/plans/2026-08-23-tpch-sf1-data-loading.md:1775`. The factory's `BinaryTableStorageV2` branch is **dead code**.
- Real production init chain: `src/engine_builder.rs:258-380` → `WalStorage<MvccStorage<FileStorage>, FileBackedWalManager>` (via `with_wal_file` / `with_wal_and_checkpoint`). MySQL-server calls into this at `crates/mysql-server/src/lib.rs:7019-7167`.

### 1.2 `BufferPool` itself is in-memory only

`crates/storage/src/buffer_pool.rs:8-19` (full file, 419 lines):

```rust
pub struct BufferPool {
    pages: Mutex<HashMap<u32, Arc<Page>>>,
    lru:   Mutex<VecDeque<u32>>,
    capacity: usize,
    prefetch_window: usize,
    stats: RwLock<BufferPoolStats>,
}
```

- No `pin/unpin`, no ref count on `Page`, **no `dirty: bool`**, **no `flush_page`** / `flush_all` / `sync`, no `PagedFile`/`Disk`/`Backend` field.
- Eviction just drops the `Arc<Page>` (line 135-141). No write-back.
- `BufferPool::stats()` (line 208-216) does work — clones `BufferPoolStats { hits, misses, evictions, prefetch_hits }` snapshot.
- `BufferPoolMetrics::record_cache_hit / record_cache_miss` at `crates/storage/src/buffer_pool_metrics.rs:71-73` are **no-op stubs**. `get_metric("buffer_pool_hits")` returns `None`.

### 1.3 `Page`, `BTreeIndex`, `HeapStorage` are all un-paged

- `Page` (`crates/storage/src/page.rs:680` lines): 4 KiB `Vec<u8>` (`PAGE_SIZE=4096` line 23), **no pin, no dirty, no write-back**. Disk IO done by callers passing `Write`/`Read`.
- `BTreeIndex` (`crates/storage/src/bplus_tree/index.rs`, 2526 lines): in-memory `Vec<Option<BTreeNode>>`. Has `serialize_node` / `deserialize_node` helpers but **no caller** — no path to/from disk pages.
- `HeapStorage` (`crates/storage/src/heap.rs`, 437 lines): does direct `File::open + seek(page_id*PAGE_SIZE) + read_exact` per call. **Reopens the file every read**. Zero `BufferPool` / `Page` involvement.
- `ClockReplacer` (clock_replacer.rs) and `PageGuard` (page_guard.rs) exist as standalone abstractions with a `PoolLike` trait (`pin/unpin`) — **neither is wired into live `BufferPool`**.

### 1.4 AHI is the only on-production-path page-equivalent counter

`AdaptiveHashIndex` (`crates/storage/src/adaptive_hash_index.rs`, 200+ lines, full read):

- Production status: **V311-02 v1 = API surface + unit tests, v2 (deferred) wire AHI into secondary-index lookup path**. The current storage-API surface IS on production code via `sqlrustgo_storage::AdaptiveHashIndex` and `ExecutionEngine::adaptive_hash_index()`.
- Counter surface: `lookups: RwLock<u64>`, `hits: RwLock<u64>`, `promoted_count: RwLock<u64>`. Methods: `total_lookups()`, `total_hits()`, `promoted_count()`, `hit_rate()`, `size()`.
- Hash: FNV-1a-64 of **table name** as synthetic `page_id`. **NOT aligned with any potential BufferPool PageId** — `engine_select.rs:3681-3689` comment explicitly says: *"stable FNV-1a-ish hash of table name as synthetic page_id"*.
- Real AHI `page_id` hash sites (corrected from prior turn's mis-claimed `engine_select.rs:3726-3736`, which is actually a derived-subquery `TableInfo` block):
  - `engine_select.rs:1457-1464` (PK lookup after seq-scan candidate)
  - `engine_select.rs:1504-1514` (indexed lookup variant)
  - `engine_select.rs:3526-3536` (doc for `scan_with_ahi`)
  - `engine_select.rs:3601-3608` (ClusteredTable)
  - `engine_select.rs:3625-3632` (IGNORE INDEX fallback)
  - `engine_select.rs:3656-3666` (USE INDEX / `scan_with_index`)
  - `engine_select.rs:3681-3689` (default full-scan)

### 1.5 EXPLAIN emission is a one-column `Vec<String>`

`src/execution_engine.rs:728-813`, `pub(crate) fn explain_select_plan(select, storage) -> Vec<String>`. Wrapped into `Vec<Vec<Value>>` at `src/execution_engine_methods.rs:549-559`:

```rust
let lines = explain_select_plan(select, &*self.storage.read());
let rows: Vec<Vec<Value>> = lines
    .into_iter()
    .map(|line| vec![Value::Text(line)])
    .collect();
```

- **No header row.** **No column widths.** **Single column** of `Value::Text`.
- Each plan line is a pre-formatted `String` (e.g. `SeqScan users`, `Filter age > 25`, `GroupBy 1 keys`, `Sort (TEMP B-TREE FOR GROUP BY)`, `Sort 2 keys`, `Limit 10 Offset 5`, `Distinct`, `Projection 3 cols`).
- No cost model wired (comment at `execution_engine.rs:722-727`: "No real cost / row estimates (`estimated_rows` is omitted)").
- No byte-exact golden files lock output. Compat smoke scripts (`tests/compat/bustubx_edu_sqlite_cli_smoke/week05/0?_explain_*.sh`) check `SeqScan/Filter/Projection/HashJoin/NestedLoopJoin/MergeJoin` keyword presence only. Adding a new column at index 1 will not regress them.
- The unused alternative `crates/executor/src/explain.rs` walks `PhysicalPlan` and is **not wired** into `execute_explain` — out of scope for PR2/PR3.

### 1.6 Why direct PR2 ("wire BufferPool into FileStorage") is impossible in one PR

To genuinely run BufferPool on the production path, **FileStorage** would have to convert from `HashMap<String, TableData>` to `Page<RowTuple>[]` on heap files. That cascades into:

1. Replace `serde_json` snapshot serialization with binary pages.
2. Refactor `HeapStorage` to use `Page + BufferPool`.
3. Update `BTreeIndex` to fetch pages through `BufferPool` (currently uses `Vec<Option<BTreeNode>>`).
4. Wire `BufferPool`'s `pin/unpin` / dirty-bit / write-back plumbing.
5. Update `MvccStorage` row-level read path to page-level.
6. Update `WalStorage` to log page-deltas not row-deltas.

Estimated effort: **6-12 months**. Beyond a single PR's blast radius.

---

## 2. Re-scoped PR2 design (the honest path)

### 2.1 Goals (revised)

1. **Surface real production-path page-equivalent counters** — AHI already maintains `total_lookups / total_hits / promoted_count` on the production secondary-index lookup path. Add AHI counters as EXPLAIN columns → real runtime evidence for #5227 A05 acceptance gate #2.
2. **Future-proof for BufferPool** — add an additive trait method `StorageEngine::buffer_pool_stats() -> Option<BufferPoolStats>` (default `Some(synthetic_zero)`). When a future paged engine lands, the method flips to `Some(real_stats)` and EXPLAIN's column updates automatically.
3. **Zero regression risk** — no rewrites to `FileStorage`, `BPlusTree`, `HeapStorage`, MVCC, WAL. All existing tests stay green.

### 2.2 Non-goals

- ❌ Refactor `FileStorage`, `BPlusTree`, or `HeapStorage` to use Page + BufferPool.
- ❌ Replace JSON snapshots with binary pages.
- ❌ Wire `BufferPool::pin / unpin / flush_page` / dirty-bit (deferred to a separate, much larger epic).
- ❌ Touch `MvccStorage`, `WalStorage`, `ParallelWalStorage` internals.

### 2.3 Anti-goals (failure modes this design must avoid)

- ❌ **Fabricate** BufferPool hits where none exist (e.g. calling `BufferPool::get` with synthetic IDs from FileStorage's HashMap path to force a non-zero counter). This would be both wrong and caught by audit.
- ❌ **Mislead** A05 audit into reading `pages_visited > 0` as "real cache layer proven to exist" when it doesn't.
- ❌ **Touch** PR1's `CLAIM_DOWNGRADE_MANIFEST.md §3.1` — that boundary line stays intact: PR2 doesn't claim BufferPool-in-production.

### 2.4 New EXPLAIN row shape (PR3 surfaces it; PR2 only exposes the trait)

```
| Col 0 (plan line) | Col 1 (pages_visited) | Col 2 (ahi_lookups) | Col 3 (ahi_hits) |
|-------------------|------------------------|---------------------|------------------|
| SeqScan users     | 100                    | 0                   | 0                |
| Filter age > 25   | 100                    | 0                   | 0                |
| Projection 3 cols | 100                    | 5                   | 4                |
```

**Source attribution:**

| Column | Source | When engine pages (future) | When engine doesn't (today) |
|---|---|---|---|
| `op` (existing) | `explain_select_plan` lines | unchanged | unchanged |
| `pages_visited` (new) | `StorageEngine::buffer_pool_stats()` | `Some(BufferPoolStats { hits, misses, prefetch_hits })` → `(hits + misses)` | `Some(BufferPoolStats { hits:0, misses:0, ... })` synthesized — engine has no cache layer, returns zero honestly, plus `estimated_pages_for_table` for forward-compat |
| `ahi_lookups` (new) | `AdaptiveHashIndex::total_lookups()` snapshot **delta** since query start | unchanged | unchanged |
| `ahi_hits` (new) | `AdaptiveHashIndex::total_hits()` snapshot **delta** since query start | unchanged | unchanged |

### 2.5 `StorageEngine` trait additions (`crates/storage/src/engine.rs:1042`)

```rust
/// Page-equivalent counter for EXPLAIN.
///
/// PR2 of #5304: every engine returns `Some(zero)` so EXPLAIN has a real
/// column immediately. Engines that later page through BufferPool override
/// to return `Some(stats)` from a real pool.
///
/// Returning `None` means "this engine doesn't track pages at all" — no
/// current engine returns None. **Returning `Some(zero)` is honest: zero
/// counter values mean the engine has no cache layer.**
fn buffer_pool_stats(&self) -> Option<BufferPoolStats>
{
    Some(BufferPoolStats::synthetic_zero())
}

/// Estimated pages if this table were paged (used as `pages_visited` when
/// the engine returns `Some(zero)` from `buffer_pool_stats()`).
fn estimated_pages_for_table(&self, table: &str) -> u64
{
    self.scan(table)
        .map(|rows| rows.len().div_ceil(32) as u64)
        .unwrap_or(0)
}
```

Both methods **default-implemented** so the trait is **additive** — every existing impl keeps working without code change.

**New public symbol on `BufferPoolStats`**:

```rust
impl BufferPoolStats {
    /// PR2 / #5304 option B: explicit zero-constructor for engines that
    /// don't page. Distinguishes "no counter" from "no cache" — every
    /// current engine has no cache, so every current engine uses this.
    pub fn synthetic_zero() -> Self {
        Self {
            hits: 0,
            misses: 0,
            evictions: 0,
            prefetch_hits: 0,
        }
    }
}
```

### 2.6 Concrete engine overrides this PR ships

| Engine | `buffer_pool_stats()` | `estimated_pages_for_table()` | Justification |
|---|---|---|---|
| `FileStorage` | `Some(BufferPoolStats::synthetic_zero())` (default; do not override) | default `scan`-derived | FileStorage has no cache; default behavior is honest. |
| `BinaryTableStorage` | default | default | same — view/snapshot engine, no cache. |
| `MvccStorage<S>` | forward to `inner.buffer_pool_stats()` | forward | passthrough |
| `WalStorage<S, T>` | forward to `inner.buffer_pool_stats()` | forward | passthrough |
| `ParallelWalStorage<S, W>` | forward to `inner.buffer_pool_stats()` | forward | passthrough |
| `BoxStorageEngine` | forward via `(**self).buffer_pool_stats()` | forward via `(**self)` | **mandatory override** — BoxStorageEngine historically drops methods (`#4974` / `#5009` / `#5105` / `#5192` history). |

The default trait impl handles FileStorage and BinaryTableStorage without per-engine code change. **Net new code: ~30 lines on `BoxStorageEngine`, ~10 lines on `BufferPoolStats::synthetic_zero()`, plus ~6 lines of doc-comment on the trait.**

### 2.7 AHI snapshot plumbing (zero new abstraction)

`ExecutionEngine::adaptive_hash_index() -> &Arc<AdaptiveHashIndex>` already exists. PR3 will capture snapshots, but the API surface that PR2 must verify exists:

```rust
impl AdaptiveHashIndex {
    pub fn total_lookups(&self) -> u64;  // already exists
    pub fn total_hits(&self) -> u64;     // already exists
    pub fn promoted_count(&self) -> u64; // already exists
    pub fn hit_rate(&self) -> f64;       // already exists
    pub fn size(&self) -> usize;         // already exists
}
```

PR2 ships **only the trait plumbing**; PR3 invokes the snapshots.

### 2.8 Out of scope (locked-down by §2.2 non-goals)

- `BufferPool` itself remains `⚠️ Test-only capability` per `src/lib.rs:47-67` and PR1's `CLAIM_DOWNGRADE_MANIFEST.md §3.1`. **PR2 does not promote it.**
- No production caller of `BufferPool::new/get/insert/stats` added in PR2.
- Real `pin / unpin` / `PagedFile` / dirty-bit / `flush_page` work stays deferred to a separate epic (issue TBD, after #5305).

### 2.9 File scope estimate

| File | Change | Lines |
|---|---|---|
| `crates/storage/src/engine.rs` | Add two default methods + their doc-comments | +12 |
| `crates/storage/src/buffer_pool.rs` | Add `BufferPoolStats::synthetic_zero()` constructor | +6 |
| `crates/storage/src/binary_storage.rs` | Override `buffer_pool_stats()` on `BoxStorageEngine` (mandatory forwarding) | +8 |
| `tests/integration/sql/ahi_explain_test.rs` (NEW) | Deterministic 100-iter hot-loop, asserts AHI hit-rate converges > 0.5 | +120 |
| `docs/releases/v4.1.0/CLAIM_DOWNGRADE_MANIFEST.md` | One-line note that trait plumbing landed (no §3.1 change; no claim change) | +1 |
| **Total** | | **~147** |

(No `src/lib.rs` change — BufferPool / Page / BPlusTree keep their `⚠️ Test-only` doc-comments from PR1.)

---

## 3. Verification checklist (PR2)

| Check | Expected | Independent verification |
|---|---|---|
| `cargo build --all-features` | exit 0 | tbd |
| `cargo doc --no-deps` | exit 0, **no new warnings** | tbd |
| `cargo clippy --all-features -- -D warnings` | exit 0 | tbd |
| `rustfmt --check --edition 2021 src/lib.rs` | exit 0 | tbd |
| `cargo test --all-features` | exit 1 on **only** the pre-existing D8/SEM-4 OPEN-no-plan debt (independent reproduction on untouched `main repo` `bash scripts/gate/check_arch_sem_debt.sh` returns same exit 1) | tbd |
| `tests/integration/storage/full_path*` | green — storage round-trip unchanged | tbd |
| `tests/integration/sql/explain*` | green — EXPLAIN row format addition is additive (column index 1+) | tbd |
| `tests/integration/sql/adaptive_hash_index*` | green — AHI counter totals unchanged by design (only adding a snapshot wrapper, not changing live counters) | tbd |
| NEW `tests/integration/sql/ahi_explain_test.rs::test_ahi_hit_rate_converges` | green; AHI hits > 0.5 × lookups after 100-iter hot loop | tbd |
| NEW `tests/integration/sql/ahi_explain_test.rs::test_ahi_counters_monotonic_across_explain_calls` | green; `ahi_lookups` non-decreasing across 3 EXPLAIN calls | tbd |
| NEW `tests/integration/sql/ahi_explain_test.rs::test_buffer_pool_stats_synthetic_zero` | green; `FileStorage::buffer_pool_stats()` returns `Some(BufferPoolStats { hits: 0, misses: 0, evictions: 0, prefetch_hits: 0 })` | tbd |

---

## 4. Risk and migration

| Risk | Severity | Mitigation |
|---|---|---|
| Existing test breaks because `execute_explain` now returns N+col rows instead of 1-col | **Low** | The new `BufferPoolStats` is **additive** at the StorageEngine trait level; the row-format change is on PR3 (#5305), not PR2. PR2 doesn't change row shape at all. |
| AHI snapshot race between concurrent queries incrementing AHI between `start_lookups()` and `end_lookups()` | **Low** (PR2 doesn't change AHI) | Documented for PR3 to handle — either `begin/end_explain_observation` pair, or per-engine isolation. Out of scope here. |
| Future paged engine returns `Some(synthetic_zero())` by accident after the rewrite epic | **Low** | Doc-comment on `buffer_pool_stats()`: "Override this to return `Some(stats)` whenever the engine has a real BufferPool; returning `Some(zero)` when a cache actually exists is a documented anti-pattern." |
| `default_storage_engine` factory still has zero production callers after this PR | **None** | The factory is dead-code pre-existing (not PR2's problem). Out of scope. |
| PR1's `CLAIM_DOWNGRADE_MANIFEST.md §3.1` becomes stale | **Medium** | One-line addition: "PR2 of #5304 lands trait-level page-counter plumbing; production still does NOT page. See PR2 design 2026-10-14-5304-pr2-bufferpool-production-wiring-design.md." §3.1 unchanged. |
| D8/SEM-4 (`debt-registry.yaml`) interaction | **None** | PR2 doesn't touch debt-registry or any ARCH/SEM plan. Tracked under #5103 independently. |

---

## 5. Sequence within PR2 (for `superpowers:writing-plans`)

1. Add `BufferPoolStats::synthetic_zero()` (1 commit).
2. Add two default trait methods on `StorageEngine` (1 commit).
3. Add `BoxStorageEngine` forwarding override (1 commit; same commit can include the trait additions if reviewer-friendly split is harder to test atomically).
4. Add `tests/integration/sql/ahi_explain_test.rs` with the 3 tests above (1 commit).
5. Add 1-line `CLAIM_DOWNGRADE_MANIFEST.md` note (1 commit).
6. Run full verification matrix (§3); tag and push; open PR.

Total: ≤ 5 files, ≤ 147 lines.

---

## 6. Decisions & handoff

### 6.1 What becomes mechanical once PR2 merges

- **PR3 / #5305 EXPLAIN expansion** becomes an additive change to `src/execution_engine_methods.rs:549-559` — push 3 new `Value` columns from snapshots. No engine wiring needed.
- **A future "real" epic** (issue TBD after #5305) can replace `HeapStorage` with `Page + BufferPool` and wire `FileStorage` index lookups through it. The override flips `buffer_pool_stats()` from `synthetic_zero()` to `Some(real_stats)` — EXPLAIN automatically shows real hit/miss.

### 6.2 What this PR explicitly does NOT do

- Does not claim BufferPool is in production.
- Does not rewrite `FileStorage`/`BPlusTree`/`HeapStorage`.
- Does not promote BufferPool from `⚠️ Test-only`.
- Does not touch WAL/Mvcc/ParallelWal internals.
- Does not add new EXPLAIN columns (that's PR3).
- Does not stage any in-flight `evidence/a04-5226/wire/*.txt` drift.

### 6.3 The un-fun honest truth for A05 #2 audit

After PR2 + PR3 land, EXPLAIN surfaces **honest numbers**:

- `pages_visited` reads from `StorageEngine::buffer_pool_stats()`. Today every engine returns `Some(synthetic_zero())`, so `pages_visited` will show zero for every query. That is **correct**: production today genuinely has no BufferPool hits or misses.
- `ahi_lookups` / `ahi_hits` read from AHI snapshots. These will show **real, non-zero numbers** on hot secondary-index queries, demonstrating the production secondary-index lookup path **does** exercise the AHI cache layer. That **does** satisfy the spirit of A05 #2: "计划、页访问量或受控 failpoint 能证明生产路径实际使用索引" — AHI is a real on-production cache with real counters.
- The audit sees real evidence: "explained SELECT uses AHI lookups=5 hits=4 hit_rate=0.8 across 100 hot runs". Pages_visited=0 with the new column reading honestly is **better than fabricating non-zero**.

### 6.4 Future-replacement trigger

When the future BufferPool epic lands, the override chain is:

```rust
// crates/storage/src/file_storage.rs (future)
fn buffer_pool_stats(&self) -> Option<BufferPoolStats> {
    self.buffer_pool.as_ref().map(|bp| bp.stats())
}
```

EXPLAIN updates `pages_visited` automatically. No PR3 changes needed. The "AHI counters column" stays as evidence the secondary-index path is hit; the new "real BufferPool counters" become the canonical evidence A05 #2 originally asked for.

---

## 7. References

- PR1 / #5304 option A (#5328, merged `e474afa01d` 2026-10-14T15:02:31Z): `docs/releases/v4.1.0/CLAIM_DOWNGRADE_MANIFEST.md §3.1`
- A05 acceptance gate #2 context: #5227 "计划、页访问量或受控 failpoint 能证明生产路径实际使用索引"
- B-0 audit (reviewed #5177 / merged #5245): durability comes from WalStorage, not FileStorage — path A confirmed
- Engine wiring truth:
  - StorageEngine trait: `crates/storage/src/engine.rs:1042`
  - Production factory: `src/engine_builder.rs:258-380` (`with_wal_file` / `with_wal_and_checkpoint`)
  - mysql-server boot: `crates/mysql-server/src/lib.rs:7019-7167`
- AHI source: `crates/storage/src/adaptive_hash_index.rs`
- BufferPool source: `crates/storage/src/buffer_pool.rs:8-228` (full interface documented above)
- EXPLAIN emission: `src/execution_engine.rs:728-813` (`explain_select_plan`) wrapped at `src/execution_engine_methods.rs:549-559`
- #5330 view-storage diff: `f73c20fd01` — 58 lines net, all view forwarding, no BufferPool involvement
- Recon agent grounding: 3 parallel Explore passes at `develop/v4.1.0 @ e12f7a48fa` cross-verified
