# Issue #4847 follow-up — an open EXPLICIT transaction must not be silently committed by a second `BEGIN`

- **Date**: 2026-10-07
- **Branch**: `fix/4847-nested-begin-silent-commit`
- **Base**: `gitea252/develop/v4.1.0` = `d065c94f28`
- **Source of the work**: the `executor` full-suite re-run requested after PR #5069 reported
  `exit=101`, `ok: 35`, `FAILED: 1`.
- **Status**: fix + regression tests + mutation evidence complete at time of writing.

---

## 1. Why this work started

PR #5069 (`d065c94f28`) was merged and pushed to all 5 remotes. The follow-up
`cargo test -p sqlrustgo-executor --all-features` run reported one failing target:

```
Running tests/issue_4847_transaction_semantics_test.rs
test test_issue_4847_a_comment_line_before_begin_via_cli_dispatch ... FAILED
thread '...' panicked at crates/executor/tests/issue_4847_transaction_semantics_test.rs:54:39:
called `Result::unwrap_err()` on an `Ok` value: ExecutorResult { rows: [], affected_rows: 0 }
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

Every other target in that run was green (36 targets, `778 passed; 0 failed` for the
library target). This is the only failure.

## 2. Ruling out a regression from v4.1.0 work first

The failing assertion is `engine.execute("BEGIN").unwrap_err()` after a bare
`INSERT`. Before concluding anything about my own changes, the provenance of the
current engine behaviour was established:

| Commit | Date | What |
|---|---|---|
| `634c1f257d` | 2026-09-11 | added `issue_4847_transaction_semantics_test.rs` (CLI fix, part A) |
| `d3457e9c7c` | 2026-09-29 | added the V312-85 / Issue #4519 "drain the implicit TX" block |

```
$ git merge-base --is-ancestor d3457e9c7c 1f8ed79203   # 1f8ed79203 = PR #5064 merge
YES_drain_predates_my_prs
```

`d3457e9c7c` is an ancestor of the PR #5064 merge commit, i.e. it predates every
fix PR in this batch (#5064 / #5065 / #5066 / #5067 / #5068 / #5069).
**The failure is pre-existing on `develop/v4.1.0` and was not caused by v4.1.0 work.**

## 3. Root cause of the *test failure*

`d3457e9c7c` changed the engine contract: an `INSERT` in autocommit mode is
committed, so a following explicit `BEGIN` is legitimate and now succeeds. That
is correct MySQL / PostgreSQL behaviour.

The test pinned the **pre-#4519 symptom** as if it were a contract. Its own header
comment describes that symptom as the bug:

> *"the engine's begin_transaction would then reject with 'Transaction already in
> progress'; if it is ROLLBACK, the engine's rollback_transaction would reject"*

AFP lesson applied: a test that encodes a defect is not evidence. The test was
rewritten to assert the contract that actually protects the user — **a `ROLLBACK`
must undo the work done inside its transaction** — rather than asserting that a
legitimate statement fails.

## 4. The real defect found while re-testing that scenario

Rewriting the test required re-establishing what `BEGIN` actually does, which
surfaced a separate and worse defect that the stale test had been hiding.

`ExecutionEngine::begin_transaction` (`src/execution_engine_methods.rs`) drained
*whatever* transaction was open before starting the new one:

```rust
if self.tx_session.lock().current_tx_id.is_some() {
    // V312-85 / Issue #4519: drain the implicit TX.
    ...
    let _ = storage.commit_transaction();
    ...
}
```

The drain is correct for an **implicit** autocommit transaction. It was
unconditional, so it also fired for an **explicit** one:

```sql
BEGIN;
INSERT INTO t VALUES (1);
BEGIN;        -- silently COMMITs the open explicit transaction
ROLLBACK;     -- nothing left to undo
```

Measured on `develop/v4.1.0` before the fix:

```
P5 BEGIN#2        -> Ok("ExecutorResult { rows: [], affected_rows: 0 }")
P5 ROLLBACK       -> true
P5 after RB       -> ["Integer(1)"]     <- row survived a ROLLBACK
P5 ROLLBACK_LOST  -> true
```

No dialect this engine targets commits an open transaction because the user typed
`BEGIN` again — MySQL raises 1568, PostgreSQL warns "there is already a
transaction in progress", SQLite raises "cannot start a transaction within a
transaction". The drain silently destroyed the user's rollback boundary.

### Contributing factor: a dead flag

`TxSession::is_explicit_transaction` (`src/execution_engine.rs:223`) already
existed. It is initialised to `false` at all eight `TxSession` construction sites
(`execution_engine.rs:318`, `engine_builder.rs:39/74/109/155/206/263/314`) and
**was never assigned `true` and never read anywhere in `src/`**. It was dead.

`tests/integration/sql/v312_77_explicit_tx_semantics_test.rs` claims in its header:

> *"Path D (engine): `commit_implicit_dml_tx` unconditionally consumed the current
> tx even when inside an explicit BEGIN ... Fixed by adding
> `is_explicit_transaction` flag that prevents implicit commits inside explicit
> transactions."*

That is not what the code does. Path D is actually handled by the separate
`started_implicit` boolean threaded through `begin_implicit_dml_tx` /
`commit_implicit_dml_tx`. The documented fix mechanism does not exist. (The
header is left as-is here; correcting it is a separate docs change.)

## 5. The fix

`src/execution_engine_methods.rs`:

1. `begin_transaction` now distinguishes the two cases using the flag, and refuses
   the nested explicit `BEGIN` instead of committing it:

```rust
let drains_implicit = {
    let sess = self.tx_session.lock();
    match (sess.current_tx_id.is_some(), sess.is_explicit_transaction) {
        (false, _) | (true, false) => true,
        (true, true) => false,
    }
};
if !drains_implicit {
    return Err(SqlError::ExecutionError(
        "Transaction already in progress".to_string(),
    ));
}
```

2. The flag is set on a successful explicit `BEGIN`.
3. The flag is cleared by `COMMIT` and by `ROLLBACK`.

`begin_transaction` is called only from the two `TransactionStatement::Begin`
handlers (`execution_engine_methods.rs:1513`, `:1555`) — it has no internal
callers, so refusing a nested explicit `BEGIN` cannot break an engine-internal
path.

## 6. Test changes

### 6.1 Rewritten — `crates/executor/tests/issue_4847_transaction_semantics_test.rs`

| Before | After |
|---|---|
| `engine.execute("BEGIN").unwrap_err()` — asserts the pre-#4519 symptom | asserts `BEGIN` succeeds after an implicit autocommit `INSERT` |
| asserts the error message | asserts the **surviving row set** after `ROLLBACK`: `["Integer(1)"]`, i.e. the #4847 data loss does not occur |
| 1 test | 2 tests (the second proves the transaction opened by the drain really commits) |

### 6.2 New — `crates/executor/tests/explicit_tx_not_silently_committed_4847.rs` (6 tests)

- `nested_begin_must_not_commit_the_open_explicit_transaction` — the core regression
- `transaction_survives_a_refused_nested_begin` — the refusal must not corrupt the session
- `implicit_autocommit_transaction_is_still_drained` — the #4519 drain must keep firing (guards against "fix" == "start rejecting again")
- `begin_after_commit_plus_leaked_tx_is_not_treated_as_nested`
- `begin_after_rollback_plus_leaked_tx_is_not_treated_as_nested`
- `implicit_dml_inside_explicit_tx_does_not_clear_the_flag`

## 7. Mutation evidence

Method: revert one decision at a time, run the two #4847 test targets with
`--no-fail-fast`, restore the fixed file from a `sha256` checkpoint
(`e4cd46435b6573d901bb7d677a2bcba8c9dd8835bf73b33178e92c329fb527ac`) after each
run.

| ID | Mutation | Result | Caught by |
|---|---|---|---|
| M17 | revert the fix — drain unconditionally (`drains_implicit = true`) | **CAUGHT** — 3 FAILED | the 3 nested-BEGIN tests |
| M18 | drop `is_explicit_transaction = false` from `commit_transaction` | **CAUGHT** — 1 FAILED | `begin_after_commit_plus_leaked_tx_...` |
| M19 | drop `is_explicit_transaction = false` from `rollback_transaction` | **CAUGHT** — 1 FAILED | `begin_after_rollback_plus_leaked_tx_...` |
| M20 | drop `is_explicit_transaction = true` from `begin_transaction` | **CAUGHT** — 3 FAILED | the 3 nested-BEGIN tests |

No invalid mutations: every mutation changes observable behaviour and is caught
by at least one assertion. M18/M19 each fail exactly one test — the one written
for them — so the tests are not over-coupled.

### 7.1 M18 and M19 survived the first round — and that was correct to chase

The first version of this suite used a *successful* implicit DML to re-open a
transaction after `COMMIT` / `ROLLBACK`:

```rust
e.execute("COMMIT").unwrap();
e.execute("INSERT INTO t VALUES (2)").unwrap();   // succeeds → self-commits
e.execute("BEGIN").expect(...);
```

M18 and M19 both **survived**. The reason is not a wrong hypothesis but a wrong
fixture: a successful implicit DML calls `commit_implicit_dml_tx`, which resets
`current_tx_id` to `None` before returning. At `BEGIN` time `current_tx_id` is
`None`, so `begin_transaction` short-circuits on `(false, _) => true` and never
reads the flag at all.

Reaching the flag requires an implicit DML that **fails** — a duplicate-key
`INSERT` returns through `?` without the implicit commit and leaks the
transaction. Confirmed by probe before rewriting the tests:

```
# with the fix in place
D dup err            -> Some("Duplicate entry '1' for key 'PRIMARY'")
D BEGIN after leak   -> None            (BEGIN succeeded — drained)
# with M18 applied (COMMIT reset removed)
D BEGIN after leak   -> Some("Execution error: Transaction already in progress")
```

M18's missing reset is a genuine user-visible defect (a legitimate `BEGIN`
rejected), not a theoretical one. Both tests were rewritten to use a
`seeded_pk()` table plus `leak_implicit_tx()`.

This is the second time in this batch that a surviving mutation turned out to be a
real coverage gap rather than a wrong fix (cf. M10 on #5049, which was a fixture
defect). Both outcomes are recorded here rather than resolved by weakening the
mutation.

## 8. Behaviour after the fix

Measured, same probes:

```
P1 INSERT;BEGIN          -> Ok          (autocommit, #4519 preserved)
P2 BEGIN;INSERT;BEGIN     -> Err "Transaction already in progress"
P3 BEGIN;INSERT;ROLLBACK  -> row 1 survives only (rollback works)
P4 #4847 data-loss script -> no data loss
P5 BEGIN;INSERT;BEGIN;RB  -> []          (rollback boundary preserved; was ["Integer(1)"])
```

## 9. Full-suite verification

| Suite | Result |
|---|---|
| `cargo test -p sqlrustgo-storage --all-features --lib` | **889 passed, 0 failed** |
| `cargo test -p sqlrustgo-storage --all-features --tests` | **1394 passed, 0 failed** (4 consecutive runs, see 9.1) |
| `cargo test -p sqlrustgo-executor --all-features --no-fail-fast` | **1555 passed, 4 failed** — all 4 pre-existing, see 9.2 |
| `cargo clippy -p sqlrustgo-storage --all-features` | **0 warnings** (was 19) |
| `cargo fmt --all` | clean |

### 9.1 A flaky wall-clock test found while verifying (NOT caused by this change)

`crates/storage/src/wal_legacy.rs:1473` — `test_wal_perf_1000_insert` asserts

```rust
// Target: <2s
assert!(elapsed.as_secs_f64() < 2.0, "WAL INSERT too slow: {:?}", elapsed);
```

A hard wall-clock budget inside a unit test. It failed 1 run in 4 during this
session, while a concurrent `cargo test --workspace` from another agent pushed the
box into swap thrashing (11.1 GB of 12 GB swap in use):

```
run1 exit=101  passed 1393 failed 1
  wal_legacy::tests::test_wal_perf_1000_insert ... FAILED
  panicked at crates/storage/src/wal_legacy.rs:1473:9
run2 exit=0    passed 1394 failed 0
run3 exit=0    passed 1394 failed 0
```

`crates/storage/src/wal_legacy.rs` is not in this change's diff, and none of the
edits made to storage can affect WAL insert throughput (they are borrow
removals, an equivalent `match` guard, `unwrap_or_default`, a deleted dead
method, and `scoped_key` now referencing the identical `'\u{1}'` constant it
previously inlined).

**Reported, not fixed.** Loosening the threshold to make a red go away is exactly
the kind of unearned PASS this repo's `ANTI_FABRICATION_POLICY.md` forbids; the
right resolution (move to a bench, or make the budget load-aware) is a separate
decision. It is recorded here because it materially undermines any "0 FAILED"
claim made on a loaded machine — including the ones in this document.

### 9.2 The 4 executor failures are pre-existing

```
test_select_extract_year_projects_expression   left: Integer(1995)  right: Text("1995")
call_body_raw_sql_runs_through_dispatcher      left: 0               right: 1
v312_55d_after_insert_trigger_survives_commit  left: Null            right: Text("sku-A")
crates/executor/src/instrumentation.rs - instrumentation (line 12)  hook.seq_scan_count() >= 1
```

None is transaction-related (they are type-coercion and return-shape drifts).

* The 3 integration failures were reproduced with `src/execution_engine_methods.rs`
  reverted (`git stash push src/execution_engine_methods.rs`, then
  `cargo test --offline -p sqlrustgo-executor --all-features --no-fail-fast
  --test select_projection_test --test test_stored_proc
  --test v312_55d_trigger_rollback_test`) — **all 3 still FAILED**. Restored
  afterwards; `sha256` re-checked to `e4cd46435b6573d901bb7d677a2bcba8c9dd8835bf73b33178e92c329fb527ac`.
* The doc-test lives in `crates/executor`, which `sqlrustgo` **depends on**.
  A change to the `sqlrustgo` root crate cannot affect `sqlrustgo-executor`'s own
  doc-tests, so it is independent of this change by construction. The snippet is
  illustrative — its body is `// ...` comments, so it compiles and then asserts
  `seq_scan_count() >= 1` without ever running a query.

**Why they were invisible until now.** The first post-#5069 run used
`--no-fail-fast`-less default behaviour and aborted at
`issue_4847_transaction_semantics_test`. Targets sort alphabetically;
`issue_4847_*` < `select_projection_test` < `test_stored_proc` <
`v312_55d_*`, so **none of these four had ever been reached** by that run. The
"1 FAILED" that started this work was masking 4 more. Switching to
`--no-fail-fast` is what surfaced them.

## 10. Side finding: the clippy gate is already red on `develop/v4.1.0`

CI (`.gitea/workflows/ci.yml:50`) runs `cargo clippy --all-features -- -D warnings`.

`sqlrustgo-storage` had **19** warnings that became errors under `-D warnings`,
all blameable to 2026-10-06 storage work (`247d2d79ec` #5025 table-space
isolation, `654488601` #5055 real WAL, `5204e98f` #5025 step 1, `eafce9729` #5057
step 1). Since they sit in a crate this change touches and they block the gate,
they were cleared here:

| Lint | Sites | Resolution |
|---|---|---|
| needless borrow `self.tbl(&x)` | 5 + 3 | drop the `&` |
| needless borrow `&table` | 6 | drop the `&` |
| `let mut file` unneeded | 1 | drop `mut` |
| `unwrap_or_else(\|\| Vec::new())` | 1 | `unwrap_or_default()` |
| `DB_SEP` never used | 1 | **wired into `scoped_key`** rather than deleted — it was dead only because `scoped_key` inlined the same `'\u{1}'` literal; deleting it would drop the rationale comment and keep the duplication |
| `MemoryStorage::db` never used | 1 | deleted (private, dead under `--all-features`) |
| collapsible `if` in `match` | 1 | `Rollback if open.remove(..) =>` guard — equivalent, the `_` arm already absorbs the guard-false case |
| too many arguments | 1 | `#[allow]` + rationale; bundling the binlog slave handler's params is a larger refactor than a lint cleanup should carry |

Two of the `self.tbl(&table_name)` sites were deliberately **left alone**:
`table_name` there is an owned `String` consumed by a later
`save_index(&table_name, ..)`, so the borrow is real — replacing it produced
`E0382: borrow of moved value`. Clippy had not flagged them.

**The gate is still red.** The root `sqlrustgo` crate carries **34** warnings of
its own (23 in `execution_engine_methods.rs` — a stale `use` block — plus 8 in
`engine_dml.rs` and 3 in `engine_select.rs`). Verified pre-existing: reverting
this change's `src/` edit gives **35**, i.e. the change *reduces* the count by
one. Those 34 need their own change; they are not touched here.

Note the trap in that list: several flagged "unused" imports are only used by
`#[cfg(test)]` code. CI's command omits `--all-targets`, so it flags them while
`cargo test` needs them. Deleting them wholesale would break the test build, so
that set needs per-name analysis rather than a blanket `cargo clippy --fix`.

## 11. Scope

- `src/execution_engine_methods.rs` — `begin_transaction` gate, flag set/clear
- `crates/executor/tests/issue_4847_transaction_semantics_test.rs` — assertions corrected
- `crates/executor/tests/explicit_tx_not_silently_committed_4847.rs` — new

Not in scope, recorded for follow-up:
- `tests/integration/sql/v312_77_explicit_tx_semantics_test.rs` header still credits
  Path D to a flag that was dead. Doc-only correction.
- `tests/integration/sql/v312_62_issue_batch_test.rs:303` is `#[ignore]`d with the
  reason *"develop HEAD reports 'Transaction already in progress' on the second
  BEGIN"* — that reason describes the symptom this PR removes. Needs a re-check
  before the `#[ignore]` is lifted.

## 12. Evidence fields (ADR-014)

- `source_agent`: mcode (MCode desktop session)
- `source_run`: mutation matrix `bg_928c508c-560d-4090-9b24-4a26129eecea`;
  baseline `explicit_tx_not_silently_committed_4847` 6 passed / 0 failed,
  `issue_4847_transaction_semantics_test` 2 passed / 0 failed; storage lib 889
  passed / 0 failed; storage --tests 1394 passed / 0 failed; executor 1555
  passed / 4 failed (all 4 pre-existing, §9.2)
- `timestamp`: 2026-10-07
- `evidence_hash`: `e4cd46435b6573d901bb7d677a2bcba8c9dd8835bf73b33178e92c329fb527ac`
  (`src/execution_engine_methods.rs`, post-mutation restore checkpoint)
- `conflict_resolution`: two conflicting behaviours were asserted by existing code
  and tests — the stale #4847 test demanded `INSERT; BEGIN` fail, while the #4519
  fix made it succeed. Resolved in favour of the engine, on MySQL/PostgreSQL
  semantics, and the test was corrected rather than the engine reverted.