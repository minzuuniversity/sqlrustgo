# ADR-008: Test Claim Transparency + No-Ignore Gate Tests Policy

> **Status**: PROPOSED (2026-06-17)
> **Deciders**: Hermes Agent (claude-macmini) + User
> **Date**: 2026-06-17
> **Supersedes**: None
> **Related**: [ADR-006 — meta-governance P11-P15](ADR-006-meta-governance.md), [ADR-007 — 5-PR truthfulness recovery](ADR-007-truthfulness-recovery-sequence.md), [`docs/audit/status/2026-06-17-truthfulness-current-state.md`](../../audit/status/2026-06-17-truthfulness-current-state.md)

## Context

### Two related truthfulness gaps

The 2026-06-17 truthfulness audit (in the morning) plus followup investigation (in the afternoon) revealed **two distinct, related problems** in how SQLRustGo reports test results:

#### Gap 1: "PASS" claims are ambiguous

A `#[test]` that is `#[ignore]`-marked produces:
- `cargo test`: 0 passed, 0 failed, 0 ignored=1, **exit code 0** (test "succeeds" because it's not run)
- The reported "test result: ok" + the "PASS" claim is **technically true but semantically misleading**

Examples found in the codebase:
- `docs/releases/v3.9.0/E2E_MIGRATION_MASTER_PLAN.md:326`:
  | 默认 `cargo test` (TPCH 13 个) | 0/13 跑 (全部 `#[ignore]`) | **13/13 PASS** | — |
  The "13/13 PASS" is correct mathematically (0 failed = pass) but visually misleading without context.
- `docs/releases/v3.9.0/EVALUATION_REPORT.md:13`:
  | TPC-H in-process PASS | 20/22 | **22/22** | +2 |
  The "22/22" reads as "all TPC-H queries pass" but doesn't disclose whether 22/22 = 22 queries (in-process) or 22 test functions (in some benchmark).

#### Gap 2: Gate tests can be silently ignored

The 2026-06-06 authenticity audit (`docs/audit/status/2026-06-06-test-authenticity-analysis-v390.md`) and the 2026-06-13 GA report disclosed that 42 tests are `#[ignore]`'d. The current SPEC-005 requires every `#[ignore]` to have an explicit `= "..."` reason. **But there is no rule that PROHIBITS ignoring gate-referenced tests.**

A future contributor could:
1. Add a new gate script in `scripts/gate/` that references `cargo test --test foo_test`
2. Mark all tests in `foo_test.rs` as `#[ignore]` (e.g., "long-running")
3. The gate would PASS (exit 0) but the test wouldn't actually run

This is **exactly the V8 vulnerability** (gate summary uses `grep -c` without PIPESTATUS) — a gate can silently pass without running the test it claims to run.

### Current state verification (2026-06-17)

| Test File | `#[ignore]` count | Gate-referenced? | Verdict |
|-----------|------------------|------------------|---------|
| `tests/qps_benchmark_test.rs` | 10 | No (perf, not gate) | ✅ Allowed |
| `tests/bench_v380_point_agg.rs` | 6 | No (perf benchmark) | ✅ Allowed |
| `tests/tx_wal_contract_tests.rs` | 6 | No (known broken, tracked #2870) | ✅ Allowed |
| `tests/tpch_q9_audit.rs` | 1 | No (missing baseline) | ✅ Allowed |
| `tests/long_run_stability_72h_test.rs` | 1 | No (long-run, 72h) | ✅ Allowed |
| `tests/perf_eng_batched_insert_test.rs` | 2 | No (perf gate) | ✅ Allowed |
| `tests/stored_proc_catalog_test.rs` | 3 | No (feature gap) | ✅ Allowed |
| `tests/boundary_test.rs` | 2 | No (edge-case) | ✅ Allowed |
| `tests/crash_monkey_test.rs` | 1 | No (long-running) | ✅ Allowed |
| `tests/recovery_fuzzer_test.rs` | 1 | No (long-running) | ✅ Allowed |
| `crates/vector/src/hnsw.rs` | 4 | No (vector perf) | ✅ Allowed |
| `crates/vector/src/parallel_knn.rs` | 2 | No (vector perf) | ✅ Allowed |
| `crates/storage/src/mmap_vector_store.rs` | 1 | No (macOS mmap bug) | ✅ Allowed |
| `crates/parser/src/parser.rs` | 1 | No (NOT syntax gap) | ✅ Allowed |
| `crates/executor/tests/hash_join_left_null_test.rs` | 1 | No (NOT syntax gap) | ✅ Allowed |
| **Total** | **42** | **0 violations** | ✅ |

**26 gate-referenced test files (e.g., `tpch_gate_test`, `tpch_full_22_test`, `tpch_hash_test`, `tpch_sf01_inprocess_test`, `oltp_test`, `soak_test`, `hash_chain_test`, `time_travel_test`, etc.) — ALL run by default. No `#[ignore]` on any gate test.**

The current state **satisfies the policy** but no explicit policy exists. **This ADR creates that policy + enforcement mechanism.**

## Decision

Adopt **two policies** + **one enforcement mechanism**:

### Policy 1: Test Claim Transparency (P-Claim)

**Every "PASS" claim in public-facing docs MUST be qualified** with:
- The number of tests actually run (vs trivial-PASS due to `#[ignore]`)
- Or an explicit "0 `#[ignore]`" badge if all tests ran
- A reference to the meta-gate that verified the claim (P11 Gate Self-Verification)

#### Required phrasing templates

| Original | Required replacement |
|----------|---------------------|
| `TPC-H 22/22 ✅` | `TPC-H 22/22 in-process ✅` + `0 #[ignore] in this gate` |
| `9-Dim Gate 8/8 PASS` | `9-Dim Gate 8/8 PASS (D9 fully run; G1-G15 have N #[ignore] — see ADR-008)` |
| `Soak 10/10 PASS` | `Soak 10/10 PASS (10/10 are `#[ignore]` — see ADR-008 §Why)` |
| `36/36 Substance tests PASS` | `36/36 Substance tests PASS (36/36 = 36/36, 0 #[ignore])` |

#### Why

If a reader sees "10/10 PASS" they assume the test ran. Without disclosure, **the number is misleading**. A reader who knows the tests are `#[ignore]` will be confused; a reader who doesn't will have false confidence.

#### Exceptions

- Internal code comments: not required
- Commit messages: not required
- Test output itself (`cargo test` already shows `0 ignored`): not required

### Policy 2: No-Ignore Gate Tests (P-Gate)

**A test file referenced by any script in `scripts/gate/*.sh` is a GATE TEST. Gate tests MUST NOT be `#[ignore]`-marked.**

#### Definition of "gate test"

A test file is a gate test iff its filename is referenced by `cargo test ... --test X` in any of:
- `scripts/gate/*.sh` (the standard gate location)
- `scripts/gate/*/*.sh` (gate subdirectories)
- `.gitea/workflows/*.yml` (CI workflow steps that run `cargo test`)

The 26 currently-gate-referenced test files are listed in `tests/baseline/gate_test_baseline.json` (captured by P16 meta-gate).

#### Exception process

A gate test MAY be temporarily ignored only via:
1. An ADR amendment (e.g., "ADR-008-exception: skip TPC-H cross-engine for 30 days while fixing Q9")
2. With explicit deadline + owner + success criteria
3. Tracked in the gate's own log

The exception ADR is bound to the gate script via header comment:
```bash
# scripts/gate/check_g1_tpch_baseline.sh
# ADR-008-exception: 2026-07-17 (hermes-agent, owner of #3424) — Q9 timeout
# until PR #3447 lands. Re-evaluate 2026-07-17.
```

#### Why

The current 42 `#[ignore]`'d tests are all **non-gate** (perf benchmarks, long-run stability, known-broken tracked in issues). This is **correct behavior**. The policy prevents the **next** contributor from accidentally ignoring a gate test for convenience.

### Policy 3: P16 Meta-Governance (enforcement)

Add **P16 Gate Test Integrity** to the meta-governance framework (PR #3438):
- **P16 Gate Test Integrity** (`scripts/gate/check_gate_test_integrity.sh`):
  - Extract all `cargo test ... --test X` references from `scripts/gate/`
  - For each, verify the test file exists and has NO `#[ignore]` attribute
  - Capture baseline in `tests/baseline/gate_test_baseline.json`
  - First run: create baseline
  - Subsequent runs: detect regression (any new gate test that's `#[ignore]`-marked = FAIL)

## Consequences

### Positive

1. **Auditable "PASS" claims**: Every PASS in public docs is qualified with run count vs `#[ignore]` count. Readers can verify.
2. **Future-proof gate integrity**: A contributor who tries to add a new gate that ignores its tests will be caught by P16 in CI.
3. **Aligned with meta-governance**: P16 extends the P11-P15 framework to cover the specific V8-grep / V6-`|| true` / V1-exit-only-code vulnerability patterns.
4. **No regression risk for current 42 ignores**: All 42 are non-gate (verified 2026-06-17), so the policy doesn't require removing any existing ignores.

### Negative

1. **README + 7 reports need updates**: README badges, EVALUATION_REPORT.md, E2E_MIGRATION_MASTER_PLAN.md, V390_COMPREHENSIVE_ASSESSMENT.md, CHANGELOG.md, etc. — qualified PASS claims required.
2. **Documentation overhead**: Every future doc must use the templates. Reviewer must enforce.
3. **P16 meta-gate requires a script + baseline file**: ~50 lines of bash + 1 JSON file.

### Risks

1. **The 26 gate-referenced tests may be hard-coded in CI** — the P16 meta-gate reads `scripts/gate/` only. CI workflow steps in `.gitea/workflows/` that run `cargo test` directly are NOT covered by the current P16. **Mitigation**: future scope expansion to also parse CI workflows. Documented as out-of-scope here.

2. **A gate test could be ignored via "soft" paths** — e.g., a test could have a conditional `#[cfg(...)]` that makes it disappear entirely. P16 only checks `#[ignore]` attribute. **Mitigation**: out-of-scope; conditional compilation is rare and intentional when used.

3. **Template enforcement** — "qualified phrasing" is a soft requirement that humans may skip. **Mitigation**: P11 (Gate Self-Verification) + P16 (Gate Test Integrity) + human review. Long-term: a docs-check tool that flags unqualified PASS claims.

## Implementation

### Files to create

| File | Purpose |
|------|---------|
| `docs/governance/adr/ADR-008-test-claim-transparency.md` | This ADR |
| `scripts/gate/check_gate_test_integrity.sh` | P16 meta-gate (executable) |
| `tests/baseline/gate_test_baseline.json` | P16 baseline (first run) |

### Files to update

| File | Change |
|------|--------|
| `README.md` | Add `#[ignore]` count to badges, qualify PASS claims |
| `AGENTS.md` | Add P16 + Policy 1 + Policy 2 to Essential Commands / Gate section |
| `tests/baseline/ignore_registry.json` (P12) | Add note that ignore is for non-gate tests only |
| `docs/governance/adr/ADR-006-meta-governance.md` | Add P16 to the principles list |
| `docs/governance/META_GATE_AUDIT_2026-06.md` | Add P16 to the meta-governance section |

### Template updates (in P11 meta-gate output)

Add P11's "report card" output:
```
=== Meta-Gate Suite (P11-P15, P16) ===
  P11 Gate Self-Verification              PASS (4/4 detectors)
  P12 No Implicit Tolerance                 PASS (42 #[ignore], 0 on gate tests)
  P13 Test Count Monotonicity              PASS (115 cargo_tests, 6136 active, 93 ignored*)
  P14 DRIFT != PASS                        PASS (12 anti-patterns in baseline)
  P15 Oracle Required                       PASS (8 gates without oracle in baseline)
  P16 Gate Test Integrity (NEW)            PASS (26 gate tests, 0 #[ignore] on gate tests)
  
  * P12 baseline has known false-positives (51 in comments); see ADR-008
```

## Verification

### End-to-end test of P16

```bash
# First run: create baseline
bash scripts/gate/check_gate_test_integrity.sh
# Output: PASS, baseline created

# Verify: 26 gate tests detected, 0 ignored
cat tests/baseline/gate_test_baseline.json | python3 -m json.tool

# Simulate violation: add #[ignore] to a gate test
echo '#[ignore = "test"]' >> tests/tpch_gate_test.rs

# Re-run: should FAIL
bash scripts/gate/check_gate_test_integrity.sh
# Output: FAIL with violation list

# Clean up: revert
git checkout tests/tpch_gate_test.rs
```

### 2026-06-17 baseline (expected)

```json
{
  "created_at": "2026-06-17T...",
  "generator": "scripts/gate/check_gate_test_integrity.sh",
  "gate_referenced_tests": {
    "_smoke_test": 0,
    "audit_log_test": 0,
    "backup_restore_test": 0,
    "compatibility_harness": 0,
    "...": "...",
    "total_gate_tests": 26,
    "total_with_ignore": 0,
    "violations": []
  },
  "status": "PASS"
}
```

## Refs

- [`docs/audit/status/2026-06-17-truthfulness-current-state.md`](../../audit/status/2026-06-17-truthfulness-current-state.md) — Morning + afternoon audit
- [`docs/releases/v3.9.0/E2E_MIGRATION_MASTER_PLAN.md:326`](../../releases/v3.9.0/E2E_MIGRATION_MASTER_PLAN.md) — The "13/13 PASS" misleading table that motivated this ADR
- [`docs/releases/v3.8.0/specs/gate/SPEC-005-ignore-test-reasons.md`](../../releases/v3.8.0/specs/gate/SPEC-005-ignore-test-reasons.md) — Predecessor that requires reasons on `#[ignore]`
- [ADR-006 — meta-governance P11-P15](ADR-006-meta-governance.md) — Foundational ADR (P11-P15)
- [ADR-007 — 5-PR truthfulness recovery](ADR-007-truthfulness-recovery-sequence.md) — Sister ADR documenting the 5-PR sequence

## Future Work (Out of Scope)

1. **P16 expansion to CI workflows**: Parse `.gitea/workflows/*.yml` for `cargo test` invocations and treat those as gate tests too. Out of scope here (CI workflow parsing is more complex than bash grep).
2. **Doc-check tool**: An automatic scanner that flags unqualified "PASS" claims in docs. Out of scope; would be a new project.
3. **Meta-gate for the meta-gate (P17)**: Self-test the P11-P16 meta-gates. Already partially handled by P11 Gate Self-Verification.

---

*Authored by: Hermes Agent (claude-macmini) as part of 2026-06-17 truthfulness recovery. Per project policy: "Truthfulness above all" — every "PASS" claim in this ADR is backed by a verifiable data point (commit SHA, file path, or test count).*
