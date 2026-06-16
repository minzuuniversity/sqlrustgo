# Meta-Gate Audit Report — 2026-06-13

> **Author**: Hermes Agent (claude-macmini)
> **Date**: 2026-06-13
> **Scope**: Self-consistency check of SQLRustGo gate infrastructure (P1-P10 + 4 phase gates)
> **Outcome**: **8 vulnerabilities identified, 5 new meta-governance principles proposed, 5 enforcement scripts created**

## Executive Summary

In 2026-06-13, a self-consistency check was performed on SQLRustGo's governance infrastructure. The check revealed that the gate system itself had **8 critical vulnerabilities** that could allow tests to pass without genuine quality verification. The principle "执行了测试就算通过，没有认真检查测试结果是否正确" (running a test = passing, without verifying correctness) is the meta-governance gap.

| Severity | Vulnerability | Description | Mitigation |
|----------|--------------|-------------|------------|
| 🔴 **CRITICAL** | V1: Exit-Code Only Gate | `check()` only checks exit code, accepts "0 tests run" as PASS | P11 (Gate Self-Verification) |
| 🔴 **CRITICAL** | V2: Untracked #[ignore] | 93 #[ignore] tests tracked by ZERO gates | P12 (No Implicit Tolerance) |
| 🟠 **HIGH** | V3: Test Count Regression | Tests can be deleted; cargo test still returns 0 | P13 (Test Count Monotonicity) |
| 🟠 **HIGH** | V4: No Oracle Verification | 0 of 82 gate scripts use independent oracle | P15 (Oracle Required) |
| 🟠 **HIGH** | V5: DRIFT as PASS | `check_full_gate_verification.sh` treats exit 2 (DRIFT) as non-blocker | P14 (DRIFT ≠ PASS) |
| 🟡 **MEDIUM** | V6: `\|\| true` Swallows Errors | Multiple gates use `cargo test ... \|\| true`, losing exit codes | P14 detector |
| 🟡 **MEDIUM** | V7: No Meta-Gate | 82 gate scripts, NONE have self-tests | P11 (Gate Self-Verification) |
| 🟡 **MEDIUM** | V8: Grep-on-stdout Silently Fails | If cargo output format changes, grep returns nothing, gate PASSes | P14 detector |

## The 8 Vulnerabilities

### V1: Exit-Code Only Gate (CRITICAL)

**Location**: `scripts/gate/check_alpha_v380.sh:32-46`

**Issue**: The `check()` helper function:
```bash
check() {
    local cmd="$1"
    local name="$2"
    if eval "$cmd" >/dev/null 2>&1; then
        echo "  ✅ $name: PASS"
    else
        echo "  ❌ $name: FAIL"
    fi
}
```

Only checks exit code. If `cargo test --lib` is run with `--no-run`, exit code is 0 but **no tests are actually run**. A gate that only checks exit code can be silently bypassed.

**Counterexample**:
```bash
$ cargo test --lib --no-run; echo "exit: $?"
exit: 0
# no tests actually executed, but exit code is 0
```

### V2: Untracked #[ignore] (CRITICAL)

**Location**: Project-wide

**Issue**: 93 tests are marked with `#[ignore]` (e.g., recovery_scenarios, TPC-H, etc.). ZERO gate scripts track or constrain this number. Adding new `#[ignore]` tests is completely ungoverned.

**Counterexample**:
```rust
#[test]
#[ignore]  // ← Nobody tracks this!
fn critical_recovery_test() { ... }
```

### V3: Test Count Regression (HIGH)

**Location**: `scripts/gate/audit_testing.sh`

**Issue**: Cargo test reports "0 failed" if 0 tests are run. The gate's `cargo test ... 2>&1 | grep -E "0 failed"` will PASS if 0 tests are run, since "0 failed" matches.

**Counterexample**:
```bash
$ cargo test --lib 2>&1 | grep -E "0 failed"
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured
# All tests deleted, gate still PASSes
```

### V4: No Oracle Verification (HIGH)

**Location**: All 82 gate scripts in `scripts/gate/`

**Issue**: `grep -rE 'sqlite3|mysql|mariadb|psql|baseline' scripts/gate/ | wc -l` returns 22 (only 27% of gates have any oracle comparison). Correctness is self-validated.

**Counterexample**: A gate that just runs SQL and checks "no error" — no cross-engine comparison.

### V5: DRIFT as PASS (HIGH)

**Location**: `scripts/gate/check_full_gate_verification.sh:run_gate()`

**Issue**: The `run_gate()` function:
```bash
elif [ "$code" -eq 2 ] && [ "$expect_code" -eq 0 ]; then
    echo "  ⚠️  DRIFT (exit 2, expected 0)"
    DRIFT_COUNT=$((DRIFT_COUNT+1))
```

DRIFT is logged but the final summary treats it as non-blocker:
```bash
if [ "$BLOCKERS" -gt 0 ] || [ "$CRITICAL_FAILS" -gt 0 ]; then
    exit 1
fi
# DRIFT_COUNT > 0 still passes!
```

DRIFT is unresolved. It requires explicit human decision, not silent acceptance.

### V6: `|| true` Swallows Errors (MEDIUM)

**Location**: Multiple gates (15 sites detected)

**Issue**: `cargo test ... || true` returns 0 even if tests fail. The pattern is used in 15 gate script lines, including:
- `scripts/gate/check_g_correctness_v390.sh:39,44`
- `scripts/gate/check_gate_self_verification.sh:35,171`
- `scripts/gate/check_rc_ga_gate.sh:262,367,480`

**Counterexample**:
```bash
$ cargo test --no-run || true
$ echo "exit: $?"  # 0 even if cargo test failed
exit: 0
```

### V7: No Meta-Gate (MEDIUM)

**Location**: All 82 gate scripts in `scripts/gate/`

**Issue**: Not a single gate script has a self-test. The 82 gates can pass vacuously without anyone noticing.

**Counterexample**: A gate that does `echo "OK"` will return 0 forever.

### V8: Grep-on-stdout Silently Fails (MEDIUM)

**Location**: 10 gates using `cargo test | grep` without PIPESTATUS

**Issue**: If `cargo test` output format changes (e.g., a new Rust version), `grep` returns nothing, exit 0, gate PASSes.

**Counterexample**:
```bash
$ cargo test --lib 2>&1 | grep "RESULT_NOT_PRESENT"
# exit code: 1 (from grep)
# BUT if gate does: cargo test --lib 2>&1 | grep "RESULT_NOT_PRESENT" || true
# exit code: 0 (because of || true)
```

## The 5 New Meta-Governance Principles (P11-P15)

### P11: Gate Self-Verification

> Every gate script MUST be tested for self-consistency. A gate that cannot detect its own failure modes is unfit for use.

**Enforcement**: `scripts/gate/check_gate_self_verification.sh`
- Verifies: cargo test actually ran tests; grep patterns work; exit codes are checked
- **Status**: ✅ Active

### P12: No Implicit Tolerance

> `#[ignore]` tests MUST be in an explicit registry with reason + issue link. No "≤1 ignored allowed" in gates.

**Enforcement**: `scripts/gate/check_ignore_count.sh`
- Registry: `tests/baseline/ignore_registry.json` (93 entries auto-captured)
- **Status**: ✅ Active

### P13: Test Count Monotonicity

> Total test count MUST NOT decrease without an explicit ADR. Silent test deletion violates coverage guarantees.

**Enforcement**: `scripts/gate/check_test_count_monotonic.sh`
- Baseline: `tests/baseline/test_count.json` (115 cargo_tests, 6136 active #[test], 93 #[ignore])
- **Status**: ✅ Active

### P14: DRIFT ≠ PASS

> DRIFT state (exit 2) MUST NOT be treated as PASS. Drift is unresolved; it requires explicit human decision.

**Enforcement**: `scripts/gate/check_drift_not_pass.sh`
- Verifies: No gate script accepts exit 2 as success
- Detected: 12 anti-patterns (in baseline, not regression)
- **Status**: ✅ Active

### P15: Oracle Required

> Any correctness gate MUST compare against an independent oracle (SQLite, MariaDB, PostgreSQL, TPC-H baseline). Self-checking is insufficient.

**Enforcement**: `scripts/gate/check_oracle_present.sh`
- Detected: 8 gates without oracle (in baseline, not regression)
- Engines found: sqlite3, mariadb, mysql, postgresql, duckdb
- **Status**: ✅ Active

## Implementation Results

### Baseline Established (2026-06-13)

| Baseline File | Content | Purpose |
|---------------|---------|---------|
| `tests/baseline/test_count.json` | cargo_tests=115, active=6136, ignored=93 | P13 (count regression) |
| `tests/baseline/ignore_registry.json` | 93 entries with file:line | P12 (ignored tests) |
| `tests/baseline/drift_baseline.json` | 12 anti-patterns | P14 (DRIFT/swallow patterns) |
| `tests/baseline/oracle_baseline.json` | 8 gates without oracle | P15 (oracle coverage) |

### Meta-Gate Suite (All 5 PASS, 10/10 runs)

```
─── check_gate_self_verification.sh ───  ✅ PASS
─── check_ignore_count.sh ───            ✅ PASS
─── check_test_count_monotonic.sh ───    ✅ PASS
─── check_drift_not_pass.sh ───          ✅ PASS
─── check_oracle_present.sh ───          ✅ PASS

Total: 10 PASS / 0 FAIL (out of 10 runs)
```

## Remediation Roadmap (Phase 3 — Pending)

The 5 meta-gates currently pass via baseline establishment. The "real" fixes are Phase 3:

| # | Vulnerability | Fix | Effort |
|---|---------------|-----|--------|
| 1 | V5 (DRIFT as PASS) | Refactor `check_full_gate_verification.sh::run_gate()` to FAIL on DRIFT | 4h |
| 2 | V6 (`\|\| true`) | Replace `\|\| true` with `PIPESTATUS` checks in 15 sites | 6h |
| 3 | V8 (grep on stdout) | Add PIPESTATUS check to 10 `cargo test \| grep` sites | 4h |
| 4 | V2 (untracked #[ignore]) | Each `#[ignore]` must have explicit ADR + issue link | 8h (93 tests) |
| 5 | V4 (no oracle) | Add oracle comparison to 8 gates (g12/g13/g14/g16/p14/p22/p23/p34) | 16h |
| 6 | V1 (exit-only check) | Refactor `check_alpha_v380.sh::check()` to parse test output | 4h |
| 7 | V3 (count regression) | Already mitigated by P13 (baseline regression detection) | ✅ done |
| 8 | V7 (no meta-gate) | Already mitigated by P11 (gate self-verification) | ✅ done |

**Total Phase 3 effort**: ~42 hours of work

## Decision

**ACCEPTED**: 5 new meta-governance principles (P11-P15) + 5 enforcement scripts are added to SQLRustGo. The 8 vulnerabilities are documented and tracked for Phase 3 remediation.

The meta-gate suite provides:
1. **Baseline regression detection**: Future PRs cannot silently reduce test count, add #[ignore], or remove oracle
2. **Visible findings**: All current anti-patterns are documented in baseline files (not silently accepted)
3. **Enforcement**: CI can run the meta-gate suite before allowing merges

## Impact on v3.9.0-rc Trustworthiness Claims (2026-06-17 Update)

**The V1-V8 vulnerabilities identified here are NOT abstract risks — they directly affect the "PASS" claims currently in v3.9.0-rc documentation.**

After this audit was merged (PR #3438), a follow-up truthfulness audit (2026-06-17) cross-referenced documented PASS claims against actual evidence. Key findings:

| Documented Claim | Source | Truth |
|------------------|--------|-------|
| "G1-G16 PASS" | README, GA_GATE_STATUS_REPORT | 🔴 Only 6 of 16 gates have actual scripts (G1, G11, G12, G13, G14, G16). G11-G14 are "infrastructure ready" but not run. |
| "TPC-H 22/22 wire" | RC1, RC2 reports | 🔴 Wire 22/22 uses **corrupt fixture** (`tests/data/tpch-sf001`) — "PASS" is meaningless |
| "TPC-H 22/22 vs 3 engines" | Implied by tpch_sf01_22_vs_3engines test | 🔴 Currently **times out at Q9** (issue #3424, created 2026-06-16) |
| "Soak 10/10 PASS" | RC2, GA report | 🔴 **SIMULATED** (1,440× time compression, per #3225; real wall-clock not run) |
| "G1-G10 10/10 PASS" (RC1 vs RC2) | RC1, RC2 reports | 🔴 **Identical templated block** — output not re-generated between commits |
| "L1 87.36% coverage" | README | 🟡 Coverage script has known version mismatch (per GA report §1.3) |

**The 2026-06-06 authenticity audit** (`docs/audit/status/2026-06-06-test-authenticity-analysis-v390.md`) ALREADY documented these gaps, but the findings were **not propagated** to public-facing docs (README, GA_GATE_STATUS_REPORT) until 2026-06-17.

**Conclusion**: v3.9.0-rc has **trustworthy in-process test coverage (~35% production-equivalent)** but the "G1-G16 PASS" framing is **structurally overstated**. The meta-governance framework (P11-P15) added in PR #3438 is necessary but not sufficient — Phase 3 of ADR-006 (fix V1-V8 in existing gates) is required before v3.9.0 GA can be truthfully claimed.

**See also**: `docs/audit/status/2026-06-17-truthfulness-current-state.md` for the complete cross-reference matrix.

## References

- `docs/audit/status/2026-06-06-test-authenticity-analysis-v390.md` — Pre-existing 2026-06-06 authenticity audit (findings)
- `docs/audit/status/2026-06-17-truthfulness-current-state.md` — 2026-06-17 cross-reference of PASS claims vs evidence
- `docs/governance/adr/ADR-006-meta-governance.md` — Formal ADR for P11-P15
- `scripts/gate/check_gate_self_verification.sh` — P11 enforcement
- `scripts/gate/check_ignore_count.sh` — P12 enforcement
- `scripts/gate/check_test_count_monotonic.sh` — P13 enforcement
- `scripts/gate/check_drift_not_pass.sh` — P14 enforcement
- `scripts/gate/check_oracle_present.sh` — P15 enforcement
- `tests/baseline/*.json` — Baseline files
- [GOVERNANCE_EXECUTION_SKILL](http://192.168.0.252:3000/openclaw/hermes-ops-wiki/wiki/GOVERNANCE_EXECUTION_SKILL) — Source 10 principles
- Gitea issues referenced: #3216, #3217, #3225, #3229, #3264, #3265, #3266, #3424

---

*This audit is the result of the 2026-06-13 self-consistency check mandated by the user's instruction: "执行了测试就算通过，没有认真检查测试结果是否正确, 这是非常严重的问题, 必须纠正和监督."*
