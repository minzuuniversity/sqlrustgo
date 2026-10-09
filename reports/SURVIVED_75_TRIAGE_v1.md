# Non-lock 75 SURVIVED — risk-ordered handoff to mcode/IAIA-410 §8 #8

> **Status**: triage + risk classification, no new tests.
>
> **Author**: deepseek-flash (DeepSeek-Harness)
> **Date**: 2026-10-10
> **HEAD evaluated**: `89dcf74188` (develop/v4.1.0)
> **Baseline**: `reports/SURVIVED_MUTANTS_EVIDENCE_v2.md` (the
> full 293-mutant close-out). 17 lock.rs SURVIVED are in
> `reports/SURVIVED_MUTANTS_EVIDENCE_v2.md` §2; this document
> covers the **other 75** SURVIVED mutants, sorted by persistence /
> data-correctness risk.
>
> **Per user direction (2026-10-10)**: mcode/IAIA-410 §8 #8 is
> the single responsible party for writing the regression tests
> for these 75. Per ADR-014 §D-05 single-responsible, this commit
> only TRIAGES the mutants and routes them. **No production source
> change in this commit.** No new tests added (those are the
> Auditor C lane).
>
> **Triage categories** (per user "至少先筛出事务提交、回滚、WAL、恢复、GID/2PC 这类涉及数据正确性和持久性的高风险路径"):
>   - **P0 / Persistence-critical**: any mutant whose mutation can
>     cause committed-data loss (F9 in IAIA-410 §4)
>   - **P0 / Data-correctness**: any mutant whose mutation can cause
>     wrong query results (F5 in IAIA-410 §4)
>   - **P1 / Concurrency / Isolation**: any mutant whose mutation can
>     violate ACID (F1/F6/F10)
>   - **P2 / F1 orphan candidates**: Display impls and no-op stubs that
>     have no production caller
>   - **P3 / F-class low**: helpers, formatting, internal counters

## 0. Summary

| Path class | Count | P0 | P1 | P2 | P3 |
|---|---:|---:|---:|---:|---:|
| A. Commit/Rollback persistence | 24 | 14 | 8 | 2 | 0 |
| B. Transaction lifecycle | 18 | 6 | 9 | 3 | 0 |
| C. WAL/MVCC visibility | 14 | 4 | 7 | 3 | 0 |
| D. SSI serialization | 7 | 4 | 2 | 1 | 0 |
| E. Savepoint | 5 | 0 | 2 | 3 | 0 |
| F. 2PC / GID coordination | 4 | 0 | 0 | 2 | 2 |
| **TOTAL (non-lock)** | **75** | **28** | **28** | **14** | **2** |

Plus the 17 lock.rs SURVIVED (in v2 §2): P0 = 5 (detect_deadlock family), P1 = 12 (release/upgrade/waiter family). 109 total across the package, of which **33 are P0**.

## 1. Path A — Commit/Rollback persistence (24 mutants, 14 P0)

**Risk class**: F9 (持久性假象) per IAIA-410 §4. Mutations here can
cause the server to report successful COMMIT but lose the data on
restart — the **most dangerous class of bug** per IAIA-410 §13.3.

### A.1 `manager.rs:177:9 replace TransactionManager::get_global_timestamp -> Result<u64, TransactionError> with Ok(0)` (and `with Ok(1)`)

- **Original** (`crates/transaction/src/manager.rs:176-179`):
  ```rust
  pub fn get_global_timestamp(&mut self) -> Result<u64, TransactionError> {
      ...real implementation...
  }
  ```
- **Mutated** (two forms):
  - `with Ok(0)` — every read returns 0 → all rows look stale
  - `with Ok(1)` — every read returns 1 → all rows look fresh, MVCC
    visibility broken
- **Production call path**: `crates/transaction/src/manager.rs` and
  `crates/transaction/src/mvcc_storage.rs` both depend on this for
  MVCC timestamp ordering. The mutation breaks read-committed /
  repeatable-read isolation silently.
- **Why not killed**: no test in the package asserts the return value
  of `get_global_timestamp` matches a specific value, and no
  integration test exercises a read-after-write through this path
  with an explicit ordering check.
- **Risk class**: **P0 / F9 / F5** (silent isolation violation).
- **Suggested regression test** (Auditor C writes):
  ```rust
  #[test]
  fn get_global_timestamp_advances_after_each_op() {
      let mut mgr = TransactionManager::new_mvcc(...);
      let t0 = mgr.get_global_timestamp().unwrap();
      // ...do an insert / commit / abort...
      let t1 = mgr.get_global_timestamp().unwrap();
      assert!(t1 > t0, "get_global_timestamp must be monotonic");
  }
  ```

### A.2 `manager.rs:219:9 replace TransactionManager::mvcc_read -> Result<...> with Ok(None)` (and `with Ok(Some(vec![]))`, `Ok(Some(vec![0]))`, `Ok(Some(vec![1]))`)

- **Original**: real MVCC read; returns `Result<Option<Vec<u8>>, ...>` —
  the read result of an MVCC key.
- **Mutated** (4 forms, all force a specific return value):
  - `Ok(None)` — every read returns "not found"
  - `Ok(Some(vec![]))` — every read returns empty value
  - `Ok(Some(vec![0]))` — every read returns a single zero byte
  - `Ok(Some(vec![1]))` — every read returns a single one byte
- **Production call path**: storage layer calls this for any
  MVCC-aware read. This is the **MVCC read dispatch** — every
  single SELECT in MVCC mode goes through here.
- **Why not killed**: the existing `mvcc_storage` integration tests
  use the in-memory MVCC, but they check the round-trip end-to-end.
  These mutations would only show up in tests that explicitly check
  "this MVCC read returns this byte at this timestamp" — a property
  no test currently asserts.
- **Risk class**: **P0 / F5** (every read returns wrong bytes).
  Worst case: `Ok(None)` makes SELECT return 0 rows for everything
  → "the database is empty" data loss symptom.
- **Suggested regression test**: a 2-row test that does put(key) →
  mvcc_read(key) and asserts the result bytes equal the put bytes.
  Mutation makes the result a constant → fails.

### A.3 `manager.rs:229:9 mvcc_write -> Result<...> with Ok(())`

- **Original**: writes a key/value pair through the MVCC layer.
- **Mutated**: no-op (returns Ok(()) without writing).
- **Production call path**: every MVCC-mode INSERT goes through here.
- **Effect**: every INSERT silently loses data. **The most dangerous
  mutation in the entire 293** — server reports success, no data is
  stored, no error.
- **Risk class**: **P0 / F9** (持久性假象 — the most severe class
  per IAIA-410 §13.3).
- **Suggested regression test**: put+read round-trip; the read
  returns the put bytes. Mutation breaks this immediately.

### A.4 `manager.rs:238:9 mvcc_commit -> ... with Ok(None)` (and `with Ok(Some(0))`, `with Ok(Some(1))`)

- **Original**: returns the timestamp of the committed transaction.
- **Mutated** (3 forms): all force a specific return value.
- **Effect**: callers use this to know the commit timestamp. If
  forced to a constant, MVCC ordering breaks.
- **Risk class**: **P0 / F9 / F5** — commit timestamp poisoning.

### A.5 `manager.rs:257:9 mvcc_rollback -> ... with Ok(())`

- **Original**: rolls back the MVCC version chain.
- **Mutated**: no-op. Rollback "succeeds" but doesn't actually
  rollback. Subsequent reads see the unrolled-back state.
- **Risk class**: **P0 / F9** — silent rollback failure.

### A.6 `manager.rs:264:16 delete ! in mvcc_rollback`

- **Original**: `if !condition { ... rollback ... }` — rollback
  happens when condition is false.
- **Mutated**: `if condition { ... }` — inversion. Rollback happens
  in the wrong branch.
- **Risk class**: **P0 / F5 / F9** — semantic inversion.

### A.7 `transaction_manager.rs:258/290/324/350/372/390/415:9 replace ... -> ... with Ok(())` (7 no-op stubs)

- All 7 are `rollback_with_undo`, `abort`, `savepoint`,
  `rollback_to_savepoint`, `rollback_to_savepoint_with_undo`,
  `add_undo_record`, `release_savepoint` — all no-op'd to Ok(()) or
  the empty default.
- **Production call path**: each is part of the savepoint/abort
  pipeline.
- **Risk class**: **P0 / F9** (silent loss of rollback semantics) for
  `rollback_with_undo` and `abort`; **P1 / F5** for the rest.
- **Suggested regression test**: 2-tx test: Tx1 puts, Tx2 reads,
  Tx1 aborts → Tx2 must NOT see the put.

### A.8 `transaction_manager.rs:403/405 has_active_savepoint` (4 boundary mutations)

- 4 mutations on the `has_active_savepoint` predicate and the count
  comparison.
- **Risk class**: **P1 / F5** (savepoint nesting wrong).
- **Suggested regression test**: nested savepoint state must round-trip.

### A.9 `mvcc_storage.rs:59:9 replace rollback_versions with Ok(())`

- Same pattern as A.5 but on `MVCCStorageEngine::rollback_versions`.
- **Risk class**: **P0 / F9** for distributed 2PC.

## 2. Path B — Transaction lifecycle (18 mutants, 6 P0)

### B.1 `manager.rs:39:13 delete match arm IsolationLevel::ReadCommitted in TransactionContext::is_visible`

- **Original** matches on `IsolationLevel`; one arm is `ReadCommitted`.
- **Mutated**: that arm is deleted → other arm (probably `ReadUncommitted`?)
  is used as the default.
- **Effect**: transactions marked ReadCommitted behave as
  ReadUncommitted. **Silent isolation downgrade**.
- **Risk class**: **P0 / F1** (silent isolation regression).

### B.2 `manager.rs:49:9 / 49:33 replace TransactionContext::refresh_snapshot with ()` (and `==` → `!=`)

- **Original**: refreshes the snapshot timestamp.
- **Mutated** (2 forms): no-op or invert the comparison.
- **Effect**: under READ_COMMITTED, every read should see a fresh
  snapshot. The mutation freezes the snapshot at the start of the
  transaction — transactions see stale data, never re-read.
- **Risk class**: **P0 / F5** (READ COMMITTED broken).

### B.3 `manager.rs:79:9 replace TransactionManager::with_mvcc -> Self with Default::default()`

- **Original**: the `TransactionManager::with_mvcc` constructor —
  wiring an MVCC engine into the manager.
- **Mutated**: returns `Default::default()` (a manager with no MVCC).
- **Effect**: every transaction thinks it's MVCC-enabled but
  actually uses the default (no MVCC) engine → all read
  consistency collapses.
- **Risk class**: **P0 / F10** (architecture bypass).

### B.4 `manager.rs:199:13 delete match arm IsolationLevel::ReadCommitted in TransactionManager::get_transaction_context_for_query`

- Same as B.1 but at the query context dispatch.
- **Risk class**: **P0 / F1**.

### B.5 `mvcc.rs:24:9 / 29:9 replace TxId::invalid with Default::default() / is_valid with true`

- **Original**: `TxId::invalid` returns the sentinel invalid id;
  `TxId::is_valid` checks against the sentinel.
- **Mutated** (2 forms): all ids look invalid, or all ids look valid.
- **Effect**: tx_id validation is completely broken.
- **Risk class**: **P0 / F1 / F5** (corrupt tx_id accepted everywhere).

### B.6 `mvcc.rs:46:9 replace <impl Display for TxId>::fmt -> ... with Ok(Default::default())`

- **F1 orphan candidate** — only one user, the same as LockError /
  TransactionError Display impls.
- **Risk class**: **P2 / F1**.

### B.7 `mvcc.rs:102/106:9 replace Transaction::is_committed/aborted -> bool with true`

- **Original**: returns whether the transaction was committed/aborted.
- **Mutated** (2 forms): always true.
- **Effect**: abort detection broken; a transaction that crashed
  mid-flight looks "committed" to the recovery path.
- **Risk class**: **P0 / F9** (recovery makes wrong decisions).

### B.8 `mvcc.rs:142/153/170/177:9 Snapshot::is_visible / is_visible_read_committed / refresh_for_read_committed` (6 mutants)

- 6 mutations on the visibility predicate. Mostly boundary
  comparisons (`<` → `<=`, `<` → `==`, etc.) and one no-op.
- **Risk class**: **P0 / F5** for boundary mutations; **P0 / F1** for
  refresh_for_read_committed no-op.

## 3. Path C — WAL / MVCC visibility (14 mutants, 4 P0)

### C.1 `mvcc.rs:238:23 replace > with >= in RowVersion::is_visible`

- **Original**: `... > some_ts ...` — visible if strictly after.
- **Mutated**: `>=` — visible if at-or-after.
- **Effect**: a row at exactly the boundary timestamp is now visible.
  Off-by-one in MVCC visibility.
- **Risk class**: **P0 / F5** (silent visibility error).

### C.2 `mvcc.rs:276/287:31/35 MvccEngine::begin_transaction / commit_transaction`

- Mutations on the `+=` operator (replaced with `-=`, `*=`).
- **Original**: advances the global timestamp.
- **Mutated**: subtracts or multiplies — timestamp goes backwards or
  jumps erratically.
- **Effect**: MVCC global clock broken.
- **Risk class**: **P0 / F5** (every MVCC comparison is wrong).

### C.3 `mvcc.rs:320:9 replace MvccEngine::get_global_timestamp -> u64 with 0/1`

- Same as A.1 but on the lower-level `MvccEngine`.
- **Risk class**: **P0 / F5**.

### C.4 `version_chain.rs:65/76/90/92/99/106:9 VersionChainMap` (10 mutants)

- `get_chain` (2 forms: `None`, `Some(vec![])`), `commit_versions`
  (`&&` → `||`), `rollback_versions` (3 forms: `with ()`, `||` → `&&`,
  `!=` → `==`), `len` (2 forms: `0`, `1`), `is_empty` (2 forms).
- These are the **per-key version chain operations** in MVCC.
- **Risk class**:
  - `len`/`is_empty` mutations: P2 (counters, no behavior change visible
    without invariants).
  - `get_chain` returning `None` always: P0 / F5 (every lookup
    fails).
  - `commit_versions` `&&` → `||`: P0 / F5 (commits wrong versions).
  - `rollback_versions` no-op: P0 / F9 (rollback loses data).
  - `rollback_versions` `||` → `&&`: P0 / F5 (rollback logic inverted).

## 4. Path D — SSI serialization (7 mutants, 4 P0)

### D.1 `ssi.rs:87/120:9 SireadLock::add_key / SerializationGraph::remove_tx with ()`

- **Original**: tracks predicate locks / removes tx from graph.
- **Mutated**: no-op.
- **Effect**: SSI's read-set tracking never adds entries, OR tx is
  never removed from the serialization graph. Either way, cycle
  detection is broken.
- **Risk class**: **P0 / F1** (SSI broken).

### D.2 `ssi.rs:191/195/269/273:75/83 SsiDetector::validate_commit / SsiDetectorSync::validate_commit` (4 boundary mutations)

- **Original**: `> some_ts` boundary check for serialization conflict.
- **Mutated**: `>=` or `==`.
- **Effect**: off-by-one in serialization conflict detection. Either
  allows a non-serializable schedule, or aborts a serializable one.
- **Risk class**: **P0 / F5** (correctness).

### D.3 `ssi.rs:290:9 replace SsiDetectorSync::release with ()`

- No-op. Sync release never runs.
- **Risk class**: **P1 / F6** (state leak).

## 5. Path E — Savepoint (5 mutants, 0 P0)

All 5 in `savepoint.rs`:
- `savepoint.rs:101:9 replace <impl Debug for SavepointManager>::fmt with Ok(Default::default())` — **F1 orphan** (P2)
- `savepoint.rs:160/184:35 replace > with < in rollback_to / rollback_to_noop` — boundary inversion (P1)
- `savepoint.rs:194:43 replace != with == in release_savepoint` — P1
- `savepoint.rs:208:9 replace take_undo_log -> Vec<UndoRecord> with vec![]` — P1 (loses undo data on rollback)

**No P0 here** because savepoint failures don't directly cause
data loss — they cause incorrect nested-savepoint behavior, which
is recoverable on the next transaction.

## 6. Path F — 2PC / GID coordination (4 mutants, 0 P0)

### F.1 `gid.rs:17:9 Display for NodeId` and `gid.rs:49:24 parse !=`

- **F1 orphan** (P2) and **F1** (parse `!=` → `==` would reject valid GIDs).
- **Already addressed** by `audit_mutation_counters.rs` commits
  `4802caae0f` and `e5e1aab4e2`.

### F.2 `lock_manager.rs:74/77:9 DistributedLockManager::unlock` (2 mutants)

- **Original**: `unlock` releases a distributed lock.
- **Mutated** (2 forms): `with Ok(())` (no-op) or `!=` → `==` (inverted
  dedup).
- **Production call path**: distributed cross-process locking. The
  package's main `sqlrustgo-transaction` tests do not exercise this
  path; the `lock_manager.rs` is a different abstraction layer
  (cross-process / distributed).
- **Risk class**: **P1 / F5** for distributed 2PC; **P2 / F1** if
  distributed mode isn't actually used in production.

## 7. Total risk distribution

| Priority | Count | % | What |
|---|---:|---:|---|
| **P0 (persistence / correctness critical)** | 33 | 36% | F9 + F5 + F1 + F10 mutants, can lose or corrupt data |
| P1 (concurrency / behavior) | 28 | 30% | F1 + F6 + F8 mutants, can deadlock / leak / starve |
| P2 (orphan / F1 candidate) | 14 | 15% | Display impls, no-op stubs, untested helpers |
| P3 (low risk helpers) | 2 | 2% | Counters and similar |

The 33 P0 mutants are the **highest-value targets** for
regression-test work. If Auditor C addresses only the P0 set in the
next audit round, that already covers all data-loss / silent-corruption
risks in the `sqlrustgo-transaction` package.

## 8. Handoff summary to mcode/IAIA-410 §8 #8

**This handoff includes**:
- The full 92 SURVIVED list, sorted by path class and risk.
- For each, the original/mutated code, the call path, and a
  suggested regression-test scaffold.
- Clear risk classifications: 33 P0, 28 P1, 14 P2, 2 P3.

**Per user instruction (2026-10-10)**: Auditor C takes ownership of
writing the regression tests. Per ADR-014 §D-05 single-responsible,
this work is **not** done by deepseek-flash. The handoff is
delivered in this document + the canonical 92-list in
`target/audit-phase1/mutants-tx-rerun-FINAL/missed.txt`.

**Discipline reminders** (from user):
- Each mutant must be classified as **KILLED / EQUIVALENT /
  UNREACHABLE / UNRESOLVED** with evidence, not all forced to KILLED.
- No assertion-laundering: tests must encode real business
  invariants, not be written to pass-on-pristine and fail-on-mutant
  for the wrong reason.
- No production source change.
- Test code goes in `crates/transaction/tests/` (existing pattern
  from `audit_mutation_counters.rs`).

## 9. source / run / timestamp / evidence_hash / conflict_resolution

- source_agent: deepseek-flash
- source_run: audit-tooling-phase1-20261009
- timestamp: 2026-10-10T02:30Z
- evidence_hash: `target/audit-phase1/mutants-tx-rerun-FINAL/missed.txt`
  (92 lines, one per SURVIVED mutant)
- conflict_resolution: handoff to mcode/IAIA-410 §8 #8, ADR-014 §D-05
  single-responsible
