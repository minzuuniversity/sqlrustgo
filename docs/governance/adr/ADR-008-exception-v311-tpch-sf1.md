# ADR-008 Exception: v3.11.0 G4 TPC-H SF=1 Wire Test Deferral

> **Status**: PROPOSED (2026-08-09)
> **Deciders**: Hermes Agent (claude-macmini) + User
> **Date**: 2026-08-09
> **Supersedes**: None (first exception under [ADR-008 §Policy 2](../../governance/adr/ADR-008-test-claim-transparency.md))
> **Authorising ADR**: [ADR-008-test-claim-transparency.md](ADR-008-test-claim-transparency.md) §Policy 2
> **Related**:
> - [CURRENT_VERSION.md §v3.11.0](../../../CURRENT_VERSION.md) (G4 row references this exception)
> - [docs/releases/v3.11.0/GA_GATE_REPORT.md](../../releases/v3.11.0/GA_GATE_REPORT.md) (G4 PASS row, line 27 / 58 / 74)
> - [docs/releases/v3.11.0/TPCH_SF1_VERIFICATION_REPORT.md](../../releases/v3.11.0/TPCH_SF1_VERIFICATION_REPORT.md)
> - [docs/releases/v3.11.0/G4_WIRE_TEST_CLOSE_OUT_PLAN.md](../../releases/v3.11.0/G4_WIRE_TEST_CLOSE_OUT_PLAN.md)

## Context

The v3.11.0 G4 GA gate requires 22/22 TPC-H SF=1 queries to PASS over the MySQL wire protocol.
Audit (2026-08-09) shows the per-query state on `develop/v3.11.0`:

| Bucket | Count | Queries |
|--------|-------|---------|
| Cell-level ✅ PASS | 5 | Q1, Q2, Q6 + parser-fixed Q5/Q21 (not wire-verified) |
| 🟡 WIRED / UNVERIFIED (in `tpch_sf1_22_vs_3engines_test.rs` loop, no SF=1 wire PASS artifact) | 13 | Q3, Q4, Q7, Q10–Q14, Q16–Q18, Q20, Q22 |
| ⚠️ Parser-only fix landed, no SF=1 wire E2E | 2 | Q5, Q21 |
| 🔴 Known wire-timeout at SF=0.1 (6–8h pushdown fix per CONVERGENCE_TRACKER.md:212-214) | 2 | Q8, Q9 |
| **Total** | **22** | |

The SF=1 fixture (1.1 GB, generated 2026-08-08, commit `d043d6bfd3`) lives at
`/var/tmp/tpch-sf1/` on host 250. The `tpch_sf1_22_vs_3engines_test.rs` harness
exists but has **never been logged as run successfully E2E**: the test is
`#[ignore]`-gated and infrastructure (P0-2 queries/*.sql files present in
git, but `scripts/tpch/run_sf1.sh` does not honor `TPCH_ONLY_Q`, no CI workflow,
default fixture path is `/tmp/tpch-sf1` not `/var/tmp/tpch-sf1`) is incomplete.

The G4 gate currently passes via two means:
1. SF=0.1 G1 wire gate `tpch_gate_test::tpch_gate_completes` (22/22 ✅ but on 60K lineitem, not 6M)
2. Parser/executor analysis showing no per-query regression

Neither demonstrates strict G4 compliance (SF=1 wire 22/22). Without an exception
the gate must strictly FAIL until a real SF=1 22/22 wire run is captured.

## Decision

Under ADR-008 §Policy 2, grant a **time-bounded exception** allowing the G4
TPC-H SF=1 wire gate to PASS with the current 🟡 ALMOST status, contingent on
the close-out plan in [`docs/releases/v3.11.0/G4_WIRE_TEST_CLOSE_OUT_PLAN.md`](../../releases/v3.11.0/G4_WIRE_TEST_CLOSE_OUT_PLAN.md).

### Exception scope

- **Gate affected**: G4 TPC-H SF=1 wire (only)
- **Allowed state for PASS**: 🟡 ALMOST PASS (per CURRENT_VERSION.md G4 row)
  - P0-1 fixture ✅ 1.1 GB (closed by `d043d6bfd3`)
  - P0-2 wire tests 🟡 mostly passing (per GA_GATE_REPORT.md)
- **What is NOT allowed by this exception**:
  - `#[ignore]`-marked SF=1 wire test claims PASS without captured log
  - Renaming the test from `tpch_sf1_22_vs_3engines_test` to drop the `#[ignore]` without first capturing a green log
  - Skipping the per-query log upload requirement
  - Modifying the test to silently skip on missing fixture (current `fixture_present()` is correct)

### Deadline

**2026-09-01** (23 days from 2026-08-09). On this date the exception auto-expires
and G4 must satisfy strict PASS unless renewed by a successor exception ADR.

### Owner

- **Primary**: Hermes Agent (claude-macmini)
- **Backup**: User (escalation if Hermes Agent is offline)
- **Tracking**: this ADR + the close-out plan + the per-query log artifacts

### Success criteria for close-out

Per [`docs/releases/v3.11.0/G4_WIRE_TEST_CLOSE_OUT_PLAN.md`](../../releases/v3.11.0/G4_WIRE_TEST_CLOSE_OUT_PLAN.md) §Phase C:

1. All 22 queries PASS wire on SF=1 with exact row counts (or documented accepted
   deltas with cell-level justification).
2. CI workflow `tpch-sf1-wire.yml` runs the 22 queries on every PR touching
   `crates/{mysql-server,mysql-client,executor,optimizer}/src/`.
3. `GA_GATE_REPORT.md` G4 row flips from 🟡 to ✅.
4. PR merged + pushed to gitea250 / gitea (gitcode + gitee remain blocked pending
   credential refresh; tracked separately).

### Auto-fail conditions (gate flips to FAIL even with exception)

The exception does NOT cover:
- Test claim violations per ADR-008 §Policy 1 (unqualified PASS claims)
- Adding a new `#[ignore]` to any gate-referenced test (covered by P16 Gate Test Integrity)
- Renaming the SF=1 test without keeping its `#[ignore]` gate

## Gate script binding

The exception is bound to any script in `scripts/gate/` that references
`tpch_sf1_22_vs_3engines_test` via the header comment format prescribed by
ADR-008 §Policy 2.

```bash
# scripts/gate/check_g4_tpch_sf1.sh  (when created)
# ADR-008-exception: 2026-09-01 (hermes-agent) — G4 SF=1 wire 22/22 close-out
# in flight per docs/releases/v3.11.0/G4_WIRE_TEST_CLOSE_OUT_PLAN.md.
# Re-evaluate 2026-09-01.
```

Until `scripts/gate/check_g4_tpch_sf1.sh` exists, the binding applies to the
test file itself:

```rust
// tests/integration/tpch/tpch_sf1_22_vs_3engines_test.rs
// ADR-008-exception: 2026-09-01 (hermes-agent) — G4 SF=1 wire 22/22 close-out
// in flight per docs/releases/v3.11.0/G4_WIRE_TEST_CLOSE_OUT_PLAN.md.
// Re-evaluate 2026-09-01.
```

(TODO: add the header comment to the test file before merging this ADR.)

## Consequences

### Positive

1. v3.11.0 GA gate can complete (🟡 G4) without blocking on SF=1 wire infra.
2. Close-out plan documented with concrete Phase A/B/C milestones.
3. ADR-008 §Policy 2 used for the first time — establishes the exception
   pattern for future gates.

### Negative

1. G4 is not strictly green; a future v3.11.0.x patch may need to fix
   regressions discovered post-GA.
2. ADR exceptions accumulate governance debt; the exception must be closed
   (success or renewal) by 2026-09-01.

### Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Q8/Q9 pushdown breaks other queries | M | M | Full 22-query regression after each pushdown |
| Wire test flakiness under load (1.1 GB fixture, 6 M lineitem) | H | M | timeout=1800 s; retry-once; per-query isolation |
| 1.1 GB fixture gets corrupted on 250 | L | H | Snapshot fixture as `.tar.gz` on 250; restore script in `scripts/tpch_sf1_baseline.sh` |
| 250 unreachable for re-run | M | M | Document manual-run fallback; CI on self-hosted 250 runner |
| Cell-diff on floating-point SUM/AVG | M | L | Tolerance `1e-6`; document per-query in `SF1_BASELINE_REPORT.md` |
| Server synthesizes VARCHAR(255) for all COM_QUERY cols | M | L | Document as known; not a 22-PASS blocker (harness discards metadata) |

## Implementation

### Files to create

| File | Purpose |
|------|---------|
| `docs/governance/adr/ADR-008-exception-v311-tpch-sf1.md` | This file |
| `.gitea/workflows/tpch-sf1-wire.yml` | CI job running the 22 queries on PR + nightly (per close-out plan §Phase A) |
| `docs/releases/v3.11.0/perf/SF1_BASELINE_REPORT.md` (rewrite) | Replace ⚠️ DATA INVALID content with real SF=1 data |

### Files to update

| File | Change |
|------|--------|
| `tests/integration/tpch/tpch_sf1_22_vs_3engines_test.rs` | Add ADR-008-exception header comment (binding) |
| `scripts/tpch/run_sf1.sh` | Honor `TPCH_ONLY_Q` (per close-out plan §Phase A.3) |
| `scripts/tpch_sf1_baseline.sh` | Fix report path → v3.11.0 (per close-out plan §Phase A.4) |
| `docs/releases/v3.11.0/GA_GATE_REPORT.md:74` | Replace placeholder reference with this file's path (already correct) |
| `docs/governance/debt/debt-registry.yaml` | Add ADR-008 exception entry (currently missing) |
| `docs/governance/adr/INDEX.md` | Add this exception ADR to the index |

### Renewal criteria (if 2026-09-01 not met)

A successor ADR must be opened by 2026-08-25 (T-7 days) with:
- Concrete list of which queries remain 🟡/🔴 and why
- Estimated effort to close
- New deadline (capped at +30 days from original; total exception window ≤ 60 days)

## Verification

### Acceptance for this exception to remain valid

1. Close-out plan checklist Phase A complete by 2026-08-12 (T+3).
2. First full 22-query SF=1 wire E2E log captured by 2026-08-13 (T+4).
3. Per-query status matrix updated in `TPCH_SF1_VERIFICATION_REPORT.md` by 2026-08-25 (T+16).
4. Final flip 🟡→✅ in `GA_GATE_REPORT.md` by 2026-08-29 (T+20).

### Revocation triggers

If any of the above slip by ≥3 days, the exception auto-reports as expiring on
2026-09-01 and the G4 gate flips to FAIL strictly. The CI workflow
`tpch-sf1-wire.yml` (when created) enforces the deadline.

## Refs

- [ADR-008 §Policy 2](ADR-008-test-claim-transparency.md) — Authorising policy
- [CURRENT_VERSION.md](../../../CURRENT_VERSION.md) — G4 row exception reference
- [docs/releases/v3.11.0/GA_GATE_REPORT.md](../../releases/v3.11.0/GA_GATE_REPORT.md) — G4 PASS row
- [docs/releases/v3.11.0/G4_WIRE_TEST_CLOSE_OUT_PLAN.md](../../releases/v3.11.0/G4_WIRE_TEST_CLOSE_OUT_PLAN.md) — Phase A/B/C close-out plan
- [docs/releases/v3.11.0/TPCH_SF1_VERIFICATION_REPORT.md](../../releases/v3.11.0/TPCH_SF1_VERIFICATION_REPORT.md) — P0-1/P0-2 status

---

*Authored by: Hermes Agent (claude-macmini) as first use of ADR-008 §Policy 2 exception mechanism. Per project policy: "Truthfulness above all" — every claim in this ADR is grounded in repo evidence (commit SHAs, file paths, audit dates).*