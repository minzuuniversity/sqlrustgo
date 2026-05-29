# v3.6.0 Alpha Gate Report

## Meta
- Original Date: 2026-05-29
- Original Executor: hermes-agent (Z6G4)
- Original Commit: 86ff2a3d3
- Re-evaluation Date: 2026-05-30
- Re-evaluation Executor: hermes-agent (Z440)
- Re-evaluation Commit: 1b2a3c71 (origin/develop/v3.6.0)

---

## Results

| ID | Check | Z6G4 (Original) | Z440 (Re-evaluation) | Status |
|----|-------|-----------------|---------------------|--------|
| A1 | Build (release) | ✅ PASS | ✅ PASS | ✅ PASS |
| A2 | Test (lib) | ✅ PASS | ✅ PASS (1160 tests) | ✅ PASS |
| A3 | Clippy | ✅ PASS | ✅ PASS | ✅ PASS |
| A4 | Format | ✅ PASS | ✅ PASS | ✅ PASS |
| A5 | Coverage L1 | ✅ 81.97% | ❌ **32.59%** | ❌ **FAIL** |

**Alpha Threshold**: ≥75%
**Z440 Result**: 32.59% < 75% → **FAIL**

---

## Coverage Detail

### Z6G4 (Original - 2026-05-29)

| Crate | Coverage |
|-------|----------|
| sqlrustgo-types | 87.62% |
| sqlrustgo-parser | 47.16% ⚠️ |
| sqlrustgo-planner | 92.23% |
| sqlrustgo-optimizer | 91.26% |
| sqlrustgo-executor | 72.04% ⚠️ |
| sqlrustgo-storage | 81.76% |
| sqlrustgo-transaction | 91.51% |
| sqlrustgo-catalog | 92.17% |
| **Average** | **81.97%** |

### Z440 (Re-evaluation - 2026-05-30)

| Crate | Z6G4 | Z440 | Delta |
|-------|------|------|-------|
| sqlrustgo-types | 87.62% | 19.75% | -67.87pp |
| sqlrustgo-parser | 47.16% | 21.22% | -25.94pp |
| sqlrustgo-planner | 92.23% | 29.09% | -63.14pp |
| sqlrustgo-optimizer | 91.26% | 48.87% | -42.39pp |
| sqlrustgo-executor | 72.04% | 31.19% | -40.85pp |
| sqlrustgo-storage | 81.76% | 36.39% | -45.37pp |
| sqlrustgo-transaction | 91.51% | 32.21% | -59.30pp |
| sqlrustgo-catalog | 92.17% | 42.02% | -50.15pp |
| **Average** | **81.97%** | **32.59%** | **-49.38pp** |

---

## Known Issues

1. **mysql-server: 30 compile errors** (pre-existing v2.5.0 legacy)
   - 11x old_password_hash missing
   - 4x verify_old_password_response missing
   - 8x function signature mismatches
   - A2 excludes this crate

2. **Coverage measurement discrepancy (CRITICAL)**
   - Z6G4 reported 81.97% average
   - Z440 measured 32.59% average
   - Delta: **-49.38 percentage points**
   - Root cause: Unknown — possible causes:
     - Different build configurations
     - Different llvm-cov flags (--lib vs --tests)
     - Compiler optimizations affecting coverage data
     - Source code drift between measurements

3. **All 8 L1 crates below Alpha threshold (75%) on Z440**
   - Even the best crate (optimizer 48.87%) is below threshold
   - Worst crate (types 19.75%)

4. **parser coverage 47.16% → 21.22%** — below GA threshold (85%)
5. **executor coverage 72.04% → 31.19%** — below GA threshold

---

## Fixes Applied in This Version (from original report)

| Fix | File | Status |
|-----|------|--------|
| WAL verification workspace integration | crates/wal-verification/ | ✅ |
| SIMD vectorization module | crates/executor/src/vec_simd.rs | ✅ |
| SIMD aggregate dispatch | crates/executor/src/local_executor.rs | ✅ |
| qmd-bridge test import fix | crates/qmd-bridge/src/hybrid.rs | ✅ |
| mysql-server is_select_query | crates/mysql-server/src/lib.rs | ✅ |
| mysql-server duplicate tests | crates/mysql-server/src/lib.rs | ✅ |
| Format fixes | various | ✅ |

---

## Z440 Fixes Applied (2026-05-30)

| Fix | File | Issue | Status |
|-----|------|-------|--------|
| Add wal-verification to storage dev-dependencies | crates/storage/Cargo.toml | unlinked crate `wal_verification` | ✅ Fixed |
| Fix use statement for wal_verification | crates/storage/src/wal.rs:1869 | `use wal_verification` → `use sqlrustgo_wal_verification` | ✅ Fixed |
| parser coverage integration tests | crates/parser/tests/ | 32 new tests, 0 failed | ✅ Added |

---

## Status

**❌ FAIL** — Alpha Gate NOT passed on Z440.

- A1-A4: ✅ PASS
- A5 Coverage: ❌ **32.59%** (threshold 75%)

**Next Steps**:
1. Investigate coverage measurement discrepancy between Z6G4 and Z440
2. Increase test coverage for all 8 L1 crates
3. Re-run Alpha Gate after coverage improvements

**Knowledge OS Updated**:
- ProblemPattern "覆盖率测量差异" created in Neo4j (node 43)
- Version v3.6.0 updated with `alpha_z440_coverage=32.59%`, `alpha_gate=RE_EVALUATED`