# ADR-008 Exception: v3.12.0 V312-F-2 e2e_wire_protocol 9 Tests Deferral

> **⚠️ STATUS: SUPERSEDED — 2026-08-12 (Round-15 close-out)**
>
> This ADR-008 exception is **SUPERSEDED** at `origin/develop/v3.12.0` (`ac4364a046`; local HEAD `ed3083a59b` post-rebase).
>
> **Reason for supersession**:
> - All 4 lifting criteria (see "Acceptance criteria for lifting" below) are satisfied:
>   1. ✅ Issue #4025 closed via PR #4081 (commit `5a85a5184e`) merged into develop/v3.12.0
>   2. ✅ All 9 e2e_wire_protocol tests pass under `cargo test -p sqlrustgo-mysql-server --test e2e_wire_protocol`
>      — verified 2026-08-12: `test result: ok. 46 passed; 0 failed; 0 ignored; 0 measured`
>   3. ✅ All 9 `#[ignore = "V312-F-2 DEFERRED: ..."]` markers removed by commit `7aef7d407e` (PR #4081)
>   4. ✅ Pass log captured; sha256 verified
> - `crates/mysql-server/tests/e2e_wire_protocol.rs` currently has **0 `#[ignore]` markers** (was 9 at PROPOSED time).
> - `tests/baseline/gate_test_baseline.json` adr_exceptions[1] removed (Round-15 reconciliation).
> - `tests/baseline/ignore_registry.json` entry for `crates/mysql-server/tests/e2e_wire_protocol.rs` removed (Round-15 reconciliation).
>
> **Root cause fix chain** (referenced for audit, not as blocking issues):
> 1. PR #4014 (V312-32): `ACTIVE_CONFIG` → `Arc<EphemeralConfig>` for concurrent `start_ephemeral` safety
> 2. Commit `5611a95b5` (V312-F-2): `start_ephemeral` passes resolved `data_dir` (not `Option`) to server thread — guarantees each ephemeral server has unique on-disk data_dir
> 3. Commit `c0a4da530` (V312-F-2): `data_dir` uniqueness hardened (port+pid → port+pid+nanos) so SERVER_POOL slot reuse can no longer collide
>
> **Close-out evidence**: `docs/releases/v3.12.0/evidence/v312_f3_closure/v312_f3_strict_proof_audit.md` §1.4, §2.2
>
> **Effective lifetime**: PROPOSED 2026-08-11 → SUPERSEDED 2026-08-12 (1 day).
>
> ---

> **Original Status**: PROPOSED (2026-08-11)
> **Deciders**: openclaw + executor-agent
> **Date**: 2026-08-11
> **Supersedes**: None (2nd exception under [ADR-008 §Policy 2](./ADR-008-test-claim-transparency.md))
> **Authorising ADR**: [ADR-008-test-claim-transparency.md](./ADR-008-test-claim-transparency.md) §Policy 2
> **Related**:
> - [Issue #4025](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4025) (V312-F-2 e2e_wire_protocol 9 tests FAIL — SERVER_POOL state pollution)
> - [Issue #3959](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3959) (V312-24 MySQL Wire Hardening Deferred Items)
> - [PR #4035](http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/4035) (V312-F-1..F-6 follow-up ISSUE closures, d4ab1592a5)
> - [tests/baseline/gate_test_baseline.json](../../../tests/baseline/gate_test_baseline.json) adr_exceptions entry
> - [tests/baseline/ignore_registry.json](../../../tests/baseline/ignore_registry.json) entry for `crates/mysql-server/tests/e2e_wire_protocol.rs` (category: `deferred_e2e_server_bug`, issue_link: #4025)

## Context

V312-F-2 (Issue #4025) recorded 9 e2e_wire_protocol tests failing due to server-side executor bugs:

| 测试 | 期望 | 实际 | 行号 |
|------|------|------|------|
| test_e2e_delete | 2 rows | 56 | e2e_wire_protocol.rs:493 |
| test_e2e_drop_table | 1 row | 28 | e2e_wire_protocol.rs:235 |
| test_e2e_group_by_aggregates | "300" | "8400" | e2e_wire_protocol.rs:634 |
| test_e2e_in_operator | 2 rows | 56 | e2e_wire_protocol.rs:1541 |
| test_e2e_insert_multiple_rows | 3 rows | "84" | e2e_wire_protocol.rs:1647 |
| test_e2e_is_null | 2 rows | "56" | e2e_wire_protocol.rs:1575 |
| test_e2e_null_handling | 1 row | 28 | e2e_wire_protocol.rs:342 |
| test_e2e_order_by | 5 rows | 270 | e2e_wire_protocol.rs:540 |
| test_e2e_update | 3 rows | 61 | e2e_wire_protocol.rs:439 |

Root cause: `SERVER_POOL` (process-global) reuses slot 0 (port 9001) data_dir across tests. Even with `--test-threads=1`, prior tests' INSERT/DELETE/DROP residuals pollute subsequent test expectations.

PR #4014 (V312-32 round-10) migrated process-global `ACTIVE_CONFIG` → per-handle `Arc<EphemeralConfig>` but did not propagate the change to `SERVER_POOL` slot reuse, so same-slot data_dir accumulation persists.

PR #4035 (commit `d4ab1592a5`) chose to **defer** with `#[ignore = "V312-F-2 DEFERRED: ... V312-24 (#4025)"]` markers on all 9 tests to unblock v3.12.0 RC, tracking the real fix in #4025 / V312-24.

Without an explicit ADR-008 exception, P16 (`check_gate_test_integrity.sh`) fails as a regression: baseline was 1 `#[ignore]` (tpch_sf1_22_vs_3engines_test), now 10 (1 + 9 from e2e_wire_protocol deferral).

## Decision

Under ADR-008 §Policy 2, grant a **time-bounded exception** allowing the P16 gate to PASS with 10 `#[ignore]` markers, contingent on the close-out plan in [Issue #4025](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4025) (and re-exported as [Issue #3959](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3959) V312-24 deferred items).

### Exception scope

- **Gate affected**: P16 (`check_gate_test_integrity.sh`) only
- **Allowed state for PASS**: 10 `#[ignore]` total (1 pre-existing tpch_sf1_22_vs_3engines + 9 e2e_wire_protocol deferral)
- **Bound to**: Issue #4025 closure (real fix + re-enable all 9 tests)
- **Bound to**: `tests/baseline/ignore_registry.json` entry exists with issue_link=#4025 + per-test reason

### Acceptance criteria for lifting

The exception is lifted (and P16 baseline reverts to ≤ 1 `#[ignore]`) when ALL of:

1. Issue #4025 is closed via PR merged into develop/v3.12.0
2. All 9 e2e_wire_protocol tests pass under `cargo test -p sqlrustgo-mysql-server --test e2e_wire_protocol -- --include-ignored`
3. The 9 `#[ignore = "V312-F-2 DEFERRED: ..."]` markers are removed
4. Real evidence log captured under `docs/releases/v3.12.0/evidence/wire_load_data/05-e2e-wire-protocol.log` with sha256 of pass log

### Expiry

**2026-09-15** (matches V312-FOLLOWUP-INDEX F-2 deadline 2026-08-25 + 21-day buffer for executor-agent regression review).

If expiry is reached without lifting, the exception is **revoked** and P16 returns to FAIL until criteria met.

## Consequences

### Positive

- v3.12.0 RC unblocked; integration gate (`check_integration_gate.sh`) remains PASS (4/4)
- 9 failing tests no longer mask real PR-merge progress on unrelated work
- #4025 remains the single source of truth for tracking the real fix
- Round-16 work (PR #4046) can land without P16 regression

### Negative

- 9 e2e_wire_protocol tests are formally excluded from gate coverage until #4025 closure
- TPC-H SF=1 wire E2E coverage (28 tests in this file) is reduced from 37 pass / 9 fail to 28 pass / 9 ignore
- Risk that #4025 fix does not land by 2026-09-15 → exception revoked → v3.12.0 GA blocked

### Neutral

- ADR-008 exception count grows from 1 to 2; ADR document inventory updated
- `tests/baseline/ignore_registry.json` adds 1 new entry (74 → 75 total, source count 94)
- `tests/baseline/gate_test_baseline.json` adr_exceptions grows from 1 to 2; `total_ignore_hits` 1 → 10

## Compliance evidence

| Evidence | Path | sha256 |
|---|---|---|
| 9 `#[ignore]` markers in e2e_wire_protocol.rs | `crates/mysql-server/tests/e2e_wire_protocol.rs:211,321,414,474,526,621,1558,1594,1669` | (commit-pinned) |
| Registry entry | `tests/baseline/ignore_registry.json` (`crates/mysql-server/tests/e2e_wire_protocol.rs`) | (commit-pinned) |
| Gate baseline update | `tests/baseline/gate_test_baseline.json` (`adr_exceptions[1]`) | (commit-pinned) |
| Issue linkage | http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4025 | (Gitea API) |
| Deferral commit | PR #4035 / `d4ab1592a5 fix(V312-F-1,F-2): typed-wrappers test fix + e2e_wire_protocol deferral` | (commit-pinned) |

## Notes

- This exception follows the pattern of ADR-008-exception-v311-tpch-sf1 (G4 deferral, expiry 2026-09-01)
- Owner is `executor-agent` per V312-FOLLOWUP-INDEX F-2 assignment
- The #3887 V312-MASTER close condition #4 (FAIL/DEFERRED split to follow-up) is satisfied via Issue #4025
- Reverts to baseline (≤ 1 `#[ignore]`) requires: (a) #4025 fix merged, (b) ignore markers removed, (c) `cargo test ... --include-ignored` passes, (d) sha256 of pass log added to this ADR

---

**Authorised**: openclaw + executor-agent
**Effective**: 2026-08-11 (merge of round-16 fixup)
**Expires**: 2026-09-15