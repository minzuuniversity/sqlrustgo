# v3.6.0 Alpha Gate Report

## Meta
- Date: 2026-05-29
- Branch: develop/v3.6.0
- Commit: 86ff2a3d3
- Executor: hermes-agent (Z6G4)

## Results

| ID | Check | Result | Evidence |
|----|-------|--------|----------|
| A1 | Build (release) | ✅ PASS | cargo build --release --workspace |
| A2 | Test (lib) | ✅ PASS | cargo test --lib --workspace --exclude sqlrustgo-mysql-server |
| A3 | Clippy | ✅ PASS | cargo clippy --all-features -- -D warnings |
| A4 | Format | ✅ PASS | cargo fmt --all -- --check |
| A5 | Coverage | ✅ PASS (81.97%) | L1 8 crates avg, threshold 75% |

## Coverage Detail

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

## Known Issues

1. **mysql-server: 30 compile errors** (pre-existing v2.5.0 legacy)
   - 11x old_password_hash missing
   - 4x verify_old_password_response missing
   - 8x function signature mismatches
   - A2 excludes this crate
2. **parser coverage 47.16%** — below GA threshold (85%)
3. **executor coverage 72.04%** — below GA threshold

## Fixes Applied in This Version

| Fix | File | Status |
|-----|------|--------|
| WAL verification workspace integration | crates/wal-verification/ | ✅ |
| SIMD vectorization module | crates/executor/src/vec_simd.rs | ✅ |
| SIMD aggregate dispatch | crates/executor/src/local_executor.rs | ✅ |
| qmd-bridge test import fix | crates/qmd-bridge/src/hybrid.rs | ✅ |
| mysql-server is_select_query | crates/mysql-server/src/lib.rs | ✅ |
| mysql-server duplicate tests | crates/mysql-server/src/lib.rs | ✅ |
| Format fixes | various | ✅ |

## Status
**CONDITIONAL PASS** — all Alpha Gate thresholds met. Coverage at 81.97% exceeds 75% Alpha threshold.
