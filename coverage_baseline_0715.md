# Coverage Baseline — 2026-07-15

## Method
`cargo llvm-cov test --no-clean --ignore-run-fail -p <crate>`

## Crates ≥85% ✅ (6)
| Crate | Line % | Lines |
|-------|--------|-------|
| sqlrustgo-network | 100.00% | 433 |
| sqlrustgo-telemetry | 97.38% | 420 |
| sqlrustgo-types | 90.65% | 713 |
| sqlrustgo-planner | 88.70% | 1524 |
| sqlrustgo-catalog | 87.67% | 3507 |
| sqlrustgo-optimizer | 86.16% | 3499 |

## Crates <85% ❌ (7)
| Crate | Line % | Lines | Gap |
|-------|--------|-------|-----|
| sqlrustgo-mysql-server | 42.75% | 3587 | ~2050 lines |
| sqlrustgo-admin | 60.73% | 1289 | ~500 lines |
| sqlrustgo-parser | 71.01% | 9169 | ~2660 lines |
| sqlrustgo-executor | 82.92% | 13024 | ~2220 lines |
| sqlrustgo-storage | 83.82% | 14742 | ~2380 lines |
| sqlrustgo-common | 83.09% | 1102 | ~185 lines |

## Pre-existing Build Errors Fixed
- `Value::Point` added to `recovery_engine.rs` test match
- `TableInfo { compression: None }` added to `file_storage.rs` (was duplicated)
- `compression: None` added to `parallel_group_by_test.rs`
- Dead test file `disk_io_fault_injection.rs` deleted (imported non-existent types)
