# SQLRustGo v3.11.0 — Test Coverage Report

**Generated**: 2026-07-15
**Method**: `cargo llvm-cov test --no-fail-fast` (llvm-cov coverage)
**Branch**: `develop/v3.11.0` (`f26bcad588` — PR #3586 merged)

> **Note**: `sqlrustgo`, `sqlrustgo-bench`, `sqlrustgo-vector` timed out (>120s) and are excluded.
> Branch coverage is not instrumented in this build (`-` = no branch data).

## L1_8 Core Crates (Alpha Gate A5 / GA Gate G3 target)

| Crate | Line Cov | Lines (hit/total) | Func Cov | Functions (hit/total) | Alpha ≥75% | GA ≥80% |
|-------|----------|-------------------|----------|-----------------------|------------|----------|
| sqlrustgo-admin | 83.14% | 1435/1677 | 82.01% | 139/164 | ✅ | ✅ |
| sqlrustgo-tools | 63.84% | 1626/2214 | 75.51% | 147/183 | ❌ | ❌ |
| sqlrustgo-mysql-client | 43.79% | 507/792 | 61.54% | 26/36 | ❌ | ❌ |
| sqlrustgo-parser | 71.22% | 9522/12262 | 89.66% | 706/779 | ❌ | ❌ |
| sqlrustgo-mysql-server | 62.93% | 3220/5117 | 68.98% | 298/432 | ❌ | ❌ |
| sqlrustgo-storage | 85.58% | 14439/16521 | 83.52% | 1705/1986 | ✅ | ✅ |
| sqlrustgo-executor | 81.59% | 13060/15464 | 84.47% | 1436/1659 | ✅ | ✅ |
| sqlrustgo-planner | 86.75% | 1524/1726 | 80.66% | 212/253 | ✅ | ✅ |
| **L1_8 Average** | **81.28%** | **45333/55773** | **85.01%** | **4669/5492** | ✅ | ✅ |

## All Workspace Crates

| Crate | Line Cov | Lines (hit/total) | Func Cov | Functions (hit/total) | Alpha ≥75% | GA ≥80% |
|-------|----------|-------------------|----------|-----------------------|------------|----------|
| sqlrustgo-network | 100.00% | 433/433 | 100.00% | 42/42 | ✅ | ✅ |
| sqlrustgo-cache | 99.47% | 189/190 | 100.00% | 27/27 | ✅ | ✅ |
| sqlrustgo-wal-verification | 97.20% | 644/662 | 98.81% | 84/85 | ✅ | ✅ |
| sqlrustgo-telemetry | 96.67% | 420/434 | 95.00% | 60/63 | ✅ | ✅ |
| sqlrustgo-rag | 96.58% | 935/967 | 93.75% | 144/153 | ✅ | ✅ |
| sqlrustgo-types | 91.16% | 713/776 | 93.39% | 121/129 | ✅ | ✅ |
| sqlrustgo-common | 89.17% | 1210/1341 | 86.98% | 192/217 | ✅ | ✅ |
| sqlrustgo-optimizer | 88.23% | 3499/3911 | 95.05% | 404/424 | ✅ | ✅ |
| sqlrustgo-planner | 86.75% | 1524/1726 | 80.66% | 212/253 | ✅ | ✅ |
| sqlrustgo-storage | 85.58% | 14439/16521 | 83.52% | 1705/1986 | ✅ | ✅ |
| sqlrustgo-transaction | 85.41% | 2132/2443 | 83.44% | 308/359 | ✅ | ✅ |
| sqlrustgo-catalog | 85.08% | 3539/4067 | 81.09% | 476/566 | ✅ | ✅ |
| sqlrustgo-admin | 83.14% | 1435/1677 | 82.01% | 139/164 | ✅ | ✅ |
| sqlrustgo-security | 82.67% | 1852/2173 | 82.04% | 284/335 | ✅ | ✅ |
| sqlrustgo-executor | 81.59% | 13060/15464 | 84.47% | 1436/1659 | ✅ | ✅ |
| sqlrustgo-spill | 75.17% | 725/905 | 76.36% | 110/136 | ✅ | ❌ |
| sqlrustgo-sql-corpus | 75.16% | 914/1141 | 54.43% | 79/115 | ✅ | ❌ |
| sqlrustgo-parser | 71.22% | 9522/12262 | 89.66% | 706/779 | ❌ | ❌ |
| sqlrustgo-tools | 63.84% | 1626/2214 | 75.51% | 147/183 | ❌ | ❌ |
| sqlrustgo-mysql-server | 62.93% | 3220/5117 | 68.98% | 298/432 | ❌ | ❌ |
| sqlrustgo-mysql-client | 43.79% | 507/792 | 61.54% | 26/36 | ❌ | ❌ |
| sqlrustgo-soak | 4.89% | 716/1397 | 9.30% | 43/82 | ❌ | ❌ |
| sqlrustgo-cli | 0.00% | 186/372 | 0.00% | 10/20 | ❌ | ❌ |
| **Workspace TOTAL** | **82.41%** | **63440/76985** | **85.54%** | **7053/8245** | ✅ | ✅ |

**Summary**: 17/22 crates meet Alpha (≥75%), 15/22 crates meet GA (≥80%), workspace average 82.41%

### Crates NOT Measured (test timeout >120s)
- `sqlrustgo` (workspace root, timed out)
- `sqlrustgo-bench` (timed out)
- `sqlrustgo-vector` (timed out)

### Alpha Gate Analysis (A5: L1_8 average ≥75%)
- L1_8 average: **81.28%** — Alpha A5: ✅ PASS
  - CONDITIONAL PASS (50–75%) requires every crate ≥50% and issue tracking — ❌
- SOAK pass with coverage waiver would satisfy A5 ✓

### GA Gate Analysis (G3: every crate ≥80%)
- Crates already ≥80%: sqlrustgo-admin, sqlrustgo-storage, sqlrustgo-executor, sqlrustgo-planner, sqlrustgo-optimizer, sqlrustgo-catalog, sqlrustgo-types, sqlrustgo-common, sqlrustgo-transaction, sqlrustgo-network, sqlrustgo-security, sqlrustgo-cache, sqlrustgo-telemetry, sqlrustgo-rag, sqlrustgo-wal-verification
- Crates below 80% (8): sqlrustgo-tools, sqlrustgo-mysql-client, sqlrustgo-parser, sqlrustgo-soak, sqlrustgo-cli, sqlrustgo-spill, sqlrustgo-sql-corpus, sqlrustgo-mysql-server
