# ADR-008 Exception: T4 `BinaryTableStorageV2` 4 RED-frozen tests registered as GAP

> **Status**: PROPOSED (2026-11-26)
> **Deciders**: openclaw + executor-agent
> **Date**: 2026-11-26
> **Supersedes**: None (3rd exception under [ADR-008 §Policy 2](./ADR-008-test-claim-transparency.md))
> **Authorising ADR**: [ADR-008-test-claim-transparency.md](./ADR-008-test-claim-transparency.md) §Policy 2
> **Related**:
> - `crates/storage/tests/tf_t4_ai_candidate_v1.rs` (the 4 frozen-red tests)
> - `crates/storage/src/binary_storage_v2.rs` (the stub backend)
> - [`tests/baseline/ignore_registry.json`](../../../tests/baseline/ignore_registry.json) (4 new entries, category `GAP_DOCUMENTED`, status `ACTIVE`)
> - [`tests/baseline/test_count.json`](../../../tests/baseline/test_count.json) (`ignored_tests` 136 → 140)

## Context

`crates/storage/tests/tf_t4_ai_candidate_v1.rs` was added by the T4 pilot as a
**frozen-red** counter-example set: it asserts the behaviour the T4 storage
slice is supposed to have, while `BinaryTableStorageV2` is still a stub:

| Test | Stub behaviour today | Wanted behaviour |
|---|---|---|
| `test_insert_then_scan_visible` | `insert()` returns `Ok`, `scan()` returns `Ok(vec![])` ⇒ the rows are never visible | the written rows come back from `scan()` |
| `test_delete_returns_real_count` | `delete()` returns `Ok(0)` and deletes nothing | the real affected-row count, the wrong row survives |
| `test_update_returns_real_count_and_persists` | `update()` returns `Ok(0)` and applies nothing | the real affected-row count, the update is persisted |
| `test_transaction_methods_reject_on_non_tx_backend` | `begin_transaction()` returns `Ok(0)` on a backend that has no transaction support | the call is rejected instead of silently "succeeding" |

The commit that introduced them froze them as red on purpose
(`7aed31db27 test(test-factory): 反例#1 修复 + 75 场景复跑证据 + D-001 RED 固化`).
What was **not** done at the time is the accounting step: the four markers were
**not** `#[ignore]`-attributed and **not** registered in
`tests/baseline/ignore_registry.json`. Consequences:

- `cargo test -p sqlrustgo-storage` was **red on the release line**
  (`develop/v4.1.0`) — 4 failures that no gate attributed to anybody. This is
  the same class as the P2-08 finding of 2026-11-26 (unregistered failing
  tests), and it is consistent with `EVALUATION_REPORT.md` D10 ("the CI `test`
  job has never completed successfully").
- A red suite hides real regressions: any *new* failure in the storage crate
  is indistinguishable from the four known ones.

## Decision

Under ADR-008 §Policy 2, register the four frozen-red tests as an explicit,
tracked GAP instead of leaving them as unattributed red:

1. Add `#[ignore = "<reason>"]` to each of the four tests. The reason names
   the unimplemented behaviour (the stub method and what it does instead).
2. Add the four entries to `tests/baseline/ignore_registry.json` with
   `status: ACTIVE`, `category: GAP_DOCUMENTED` — ACTIVE because the reason
   names unimplemented behaviour, i.e. exactly the debt the ceilings bound.
3. Re-record the P13 baseline (`tests/baseline/test_count.json`):
   `ignored_tests` 136 → 140, with this ADR as the authorising record.

The RED signal is not lost: it moves from *"unexpected red in a suite"* to
*"a registered gap with a named owner and a lifting condition"* — which is
what ADR-008 asks for. The `ACTIVE` status keeps it inside the ceiling that
bounds debt (`active_max` 92; this change moves the ACTIVE count 81 → 85).

### Exception scope

- **Gate affected**: P13 (`check_test_count_monotonic.sh`) — the only gate that
  fails on a growing `#[ignore]` count, and it is satisfied by this ADR plus
  the re-recorded baseline.
- **Allowed state for PASS**: 140 `#[ignore]` attributes (136 pre-existing + 4
  T4 entries), of which 85 are `ACTIVE`.
- **Bound to**: the T4 storage slice landing the `BinaryTableStorageV2`
  read/write and transaction semantics.

### Acceptance criteria for lifting

The exception is lifted when **all** of:

1. `BinaryTableStorageV2::scan` returns the rows written by `insert`, and
   `delete` / `update` return the real affected-row counts;
2. `begin_transaction` (and the other transaction methods) reject the call on a
   backend without transaction support;
3. the four `#[ignore]` markers are removed and the four tests pass under
   `cargo test -p sqlrustgo-storage --test tf_t4_ai_candidate_v1`;
4. the four registry entries are removed from
   `tests/baseline/ignore_registry.json` and `total_allowed` drops back to 136.

### Expiry

**2026-12-26** (30 days). If the T4 slice has not landed by then, the
exception is reviewed: either the T4 slice is re-planned with a new expiry
recorded here, or the four tests are deleted as obsolete (they would then be
covered by the T4 slice's own test suite).

## Consequences

### Positive

- `cargo test -p sqlrustgo-storage` is green again — the release line no
  longer ships with four unattributed failures;
- a *new* failure in the storage crate is now visible as new;
- the debt is visible in the registry with a named reason, an owner and a
  lifting condition.

### Negative

- The T4 read/write path has **no running coverage** in the default suite
  until the slice lands; the four tests only run with
  `--include-ignored`, and they fail there by design;
- the P13 baseline moves from 136 to 140 `#[ignore]`, so the ceiling is
  consumed 4 slots earlier (140 / 150 max).

### Neutral

- ADR-008 exception count grows from 2 to 3;
- `tests/baseline/ignore_registry.json` grows 136 → 140 entries;
- the ACTIVE count grows 81 → 85 (ceiling 92).

## Compliance evidence

| Evidence | Path | sha256 |
|---|---|---|
| The 4 `#[ignore]` markers | `crates/storage/tests/tf_t4_ai_candidate_v1.rs:24,38,56,87` | (commit-pinned) |
| The stub backend they pin | `crates/storage/src/binary_storage_v2.rs:528,536,544,771` | (commit-pinned) |
| Registry entries (4 × `GAP_DOCUMENTED` / `ACTIVE`) | `tests/baseline/ignore_registry.json` | (commit-pinned) |
| P12 gate (all `#[ignore]` registered) | `scripts/gate/check_ignore_count.sh` → PASS | (commit-pinned) |
| G19 gate (registry ↔ tree recount) | `scripts/gate/check_anti_ignore_gate.sh` → PASS, total 140 / active 85 | (commit-pinned) |
| Storage suite after the change | `cargo test -p sqlrustgo-storage` → 0 failed | (commit-pinned) |

---

**Authorised**: openclaw + executor-agent
**Effective**: 2026-11-26
**Expires**: 2026-12-26
