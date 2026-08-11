# V312-22 Execution Architecture 与 Optimizer Debt — Round-12 真实性评估

> **provenance:** generated_by=v3.12.0-remediation-round-12, generated_at=2026-08-11T00:15:00Z, commit=7961c4d846bb8d3426f4e27dcc4211944981a5e4, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0

> **Source Issue**: #3909 (V312-22 Execution Architecture 与 Optimizer Debt Close-out)
> **Authority**: #3887 strict 7-condition closure (codex + Claude+M3)

---

## 1. Executive Summary

| Sub-Task | Round-11 Claim | Round-12 Reality | Disposition |
|----------|----------------|------------------|-------------|
| DML Path Integrity (ARCH-3/G4) | PASS | **PASS** (4/4 gate checks) | ✅ CLOSED |
| C-ARCH invariants (5) | 5/5 PASS | **4/5 PASS, 1 FAIL** (C-ARCH-05 regressed 1594→1762) | ⚠️ REGRESSION |
| AntiJoin (HashAntiJoin) | IMPLEMENTED (4/4 tests) | **IMPLEMENTED** (4/4 tests verified) | ✅ CLOSED |
| Subquery Decorrelation | IMPLEMENTED (13/13 tests) | **IMPLEMENTED** (13/13 tests verified) | ✅ CLOSED |
| Hash Semi Join | NOT IMPLEMENTED | **NOT IMPLEMENTED** (zero hits in crates/) | ⚠️ DEFERRED |
| CBO/Histogram | PARTIAL | **PARTIAL** (cost model 28/28 tests, histogram TODO comments only) | ⚠️ PARTIAL |

**Verdict**: #3909 cannot close under #3887 7-condition:
- 3 sub-tasks fully closed (DML Path, AntiJoin, Decorrelation)
- 3 sub-tasks require follow-up Issues (already created #4027, #4032, #4033)
- 1 critical regression: C-ARCH-05 was PASS at report commit, now FAIL

---

## 2. Sub-Task Evidence (TDD Verification)

### 2.1 DML Path Integrity (ARCH-3/G4) — ✅ CLOSED

```
$ bash scripts/gate/check_arch3_no_bypass.sh
=== G4 Gate: ARCH-3 (#3169) VtuGuard main-path enforcement ===
  [1/4] ✅ PASS: 3 VtuGuard marker calls in src/execution_engine.rs
  [2/4] ✅ PASS: 2 VtuGuard marker calls in openclaw_endpoints.rs
  [3/4] bypass patterns in DML paths: 0 (informational)
  [4/4] ✅ PASS: VtuGuard::assert_path_for_dml is public
=== G4 Gate: PASS ===
```

**Real evidence**: `crates/storage/src/vtu_guard.rs` SHA256 `b6fcbb1571042fa50a0f213b35f08d413944cf94fd262ce9e40586359673af30`

### 2.2 C-ARCH Invariants — ⚠️ REGRESSION on C-ARCH-05

```
$ bash scripts/gate/check_arch_invariants.sh
[C-ARCH-01] PASS: LocalExecutor has NO txn_manager field
[C-ARCH-02] PASS: LocalExecutor has NO write_buffer field
[C-ARCH-03] INFO (14 storage operations in business crates — allowed per AD-002)
[C-ARCH-04] PASS: no eng.execute(raw_sql) outside parser
[C-ARCH-05] FAIL: execution_engine.rs has 1762 lines (limit: 1600)
=== Summary ===
PASSED: 4
FAILED: 1
Result: FAIL
```

**Regression timeline**:
- Report commit (1903545df, 2026-08-09): execution_engine.rs = **1594 lines** (PASS)
- Current HEAD (7961c4d84, 2026-08-10): execution_engine.rs = **1762 lines** (FAIL)
- Delta: +168 lines from 3 commits:
  - 71488b9bd (V312-24 round-4)
  - 79e9c883f (V312-13 reopen fix)
  - d19c0f8a2 (V312-18 NOT NULL+CTAS merge)

**Follow-up**: #4027 [V312-F-4] (executor-agent, expiry 2026-09-15)

### 2.3 Anti Join (HashAntiJoin) — ✅ IMPLEMENTED

```
$ cargo test --package sqlrustgo-executor --lib join::hash_anti_join
running 4 tests
test join::hash_anti_join::tests::test_unique_keys ... ok
test join::hash_anti_join::tests::test_pure_static_residual_short_circuit ... ok
test join::hash_anti_join::tests::test_add_and_probe_basic ... ok
test join::hash_anti_join::tests::test_bloom_short_circuit ... ok
test result: ok. 4 passed; 0 failed
```

**Real evidence**: `crates/executor/src/join/hash_anti_join.rs` SHA256 `bfdfe6f3da089e16b0ae8bf82602f947259a35e0d62480ad4a077cd9793c9f57`

### 2.4 Subquery Decorrelation — ✅ IMPLEMENTED

```
$ cargo test --package sqlrustgo-optimizer --lib decorrelate
running 13 tests
... (all 13 PASS)
test result: ok. 13 passed; 0 failed; 0 ignored
```

**Real evidence**: `crates/optimizer/src/decorrelate.rs` SHA256 `0ccf5c1029018eac82919e2b3e17ccd9fa7f97b9e80c8697850b3f70b8055a67`

### 2.5 Hash Semi Join — ⚠️ NOT IMPLEMENTED

```
$ find crates -name "*semi*" -type f
(no results)
$ grep -rn "HashSemiJoin|hash_semi_join" --include="*.rs" crates/
(no results)
```

**Follow-up**: #4032 [V312-22a] (optimizer-agent, expiry 2026-09-30)

### 2.6 CBO/Histogram — ⚠️ PARTIAL

```
$ grep -rn "histogram|Histogram" --include="*.rs" crates/optimizer crates/executor crates/storage
crates/optimizer/src/unified_cost.rs:194:  /// Heuristic - real implementation would use table histograms.
crates/optimizer/src/stats.rs:98:        // In a real implementation, this would use histogram data
```

**Only 2 mentions, both TODO comments.** UnifiedCostModel has 28/28 tests pass (cost framework OK), but histogram collection is NOT implemented.

```
$ cargo test --package sqlrustgo-optimizer --lib unified_cost
test result: ok. 28 passed; 0 failed
```

**Real evidence**: `crates/optimizer/src/unified_cost.rs` SHA256 `0003ce5ea9c212774fd18913a259fa034335779e4a80ab137b501b7fec8a9dab`

**Follow-up**: #4033 [V312-22b] (optimizer-agent, expiry 2026-09-30)

---

## 3. Disposition Summary

| Item | Round-12 Status | Follow-up Issue | Expiry | Closure Boundary |
|------|-----------------|-----------------|--------|------------------|
| DML Path Integrity | ✅ CLOSED | - | - | - |
| C-ARCH-01/02/04 | ✅ PASS | - | - | - |
| C-ARCH-05 (line limit) | ❌ **FAIL** | #4027 | 2026-09-15 | execution_engine.rs ≤ 1500 lines |
| Hash Anti Join | ✅ IMPLEMENTED | - | - | - |
| Subquery Decorrelation | ✅ IMPLEMENTED | - | - | - |
| Hash Semi Join | ⚠️ NOT IMPL | #4032 | 2026-09-30 | hash_semi_join.rs + ≥4 tests + EXISTS rewrites |
| CBO/Histogram | ⚠️ PARTIAL | #4033 | 2026-09-30 | histogram collection + ≥5 tests |

---

## 4. #3887 7-Condition Evaluation

| Condition | Status | Notes |
|-----------|--------|-------|
| 1. PR merged | ❌ N/A | #3909 还未尝试关闭 |
| 2. Issue comment with evidence | ✅ MET | Round-12 evidence comment posted (this report) |
| 3. Real tests | ⚠️ PARTIAL | AntiJoin 4/4, Decorrelate 13/13, UnifiedCost 28/28 PASS; C-ARCH-05 FAIL |
| 4. FAIL/DEFERRED owner/expiry/boundary | ✅ MET | 3 follow-up issues: #4027, #4032, #4033 |
| 5. Fixture/reproducibility | ✅ MET | gate scripts reproducible |
| 6. Real gate output | ✅ MET | check_arch3_no_bypass.sh, check_arch_invariants.sh, cargo test outputs |
| 7. Master body update | ⏳ PENDING | Round-12 update to #3887 (this report) |

**Recommendation**: #3909 cannot close directly. Options:
1. Accept 3 closed sub-tasks + 3 follow-up issues; mark #3909 PARTIAL
2. Wait until #4027 (C-ARCH-05) is fixed before re-evaluating close

---

## 5. Evidence Hashes (real SHA256, verified 2026-08-11)

| Item | SHA256 |
|------|--------|
| `crates/storage/src/vtu_guard.rs` | `b6fcbb1571042fa50a0f213b35f08d413944cf94fd262ce9e40586359673af30` |
| `crates/executor/src/join/hash_anti_join.rs` | `bfdfe6f3da089e16b0ae8bf82602f947259a35e0d62480ad4a077cd9793c9f57` |
| `crates/optimizer/src/decorrelate.rs` | `0ccf5c1029018eac82919e2b3e17ccd9fa7f97b9e80c8697850b3f70b8055a67` |
| `crates/optimizer/src/unified_cost.rs` | `0003ce5ea9c212774fd18913a259fa034335779e4a80ab137b501b7fec8a9dab` |
| `execution-architecture-debt-report.md` | `0f0ca61eed32937aa66d79144a98bd5c2e77908eaf3c3c7a4b354e6c7a1a4ec6` |

**Removed fake hashes**: 4 placeholder hashes (`7f8e9a1b2c3d4e5f`, `a1b2c3d4e5f6a7b8`,
`c9d0e1f2a3b4c5d6`, `e7f8a9b0c1d2e3f4`) — sequential patterns violating Anti-Fabrication Policy.

---

## 6. Verdict

Round-12 audit: 3 CLOSED + 1 REGRESSION + 2 DEFERRED/PARTIAL.

**#3909 cannot close under #3887 7-condition** because:
- C-ARCH-05 was PASSED at report time but currently FAIL (regression)
- 2 items NOT IMPLEMENTED/PARTIAL need follow-up issues (already created)

Anti-Fabrication Policy v1.0: real exec results, real SHA256, no false claims.
