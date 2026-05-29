# Gate Contract v3.6.0

## Version
- Name: v3.6.0
- Branch: develop/v3.6.0
- Status: Alpha (开发阶段)
- Created: 2026-05-29
- Previous: v3.5.0 GA (5720805b)

## Alpha Gate (入口)

| ID | Check | Method | Threshold | 
|----|-------|--------|-----------|
| A1 | Build | cargo build --release --workspace | PASS |
| A2 | Test | cargo test --lib --workspace | PASS (0 failure) |
| A3 | Clippy | cargo clippy --all-features -- -D warnings | PASS |
| A4 | Format | cargo fmt --all -- --check | PASS |
| A5 | Coverage | L1 8 crates 综合平均 | ≥75% |

## Beta Gate (目标)

| ID | Check | Threshold |
|----|-------|-----------|
| B1 | A1-A5 | All PASS |
| B2 | Full test | cargo test --workspace ≥90% |
| B3 | TPC-H SF=0.1 | 22/22 PASS |
| B4 | Coverage L1 | ≥85% |

## RC Gate (目标)

| ID | Check | Threshold |
|----|-------|-----------|
| R1 | B1-B4 | All PASS |
| R2 | TPC-H SF=1 | 22/22 PASS |
| R3 | Coverage L1 | ≥85% |
| R4 | QPS regression | ≤5% 退化 |

## L1 Coverage Crates
1. sqlrustgo-types
2. sqlrustgo-parser
3. sqlrustgo-planner
4. sqlrustgo-optimizer
5. sqlrustgo-executor
6. sqlrustgo-storage
7. sqlrustgo-transaction
8. sqlrustgo-catalog
