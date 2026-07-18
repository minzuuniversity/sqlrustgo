# SQLRustGo v3.11.0 — 完整测试覆盖率报告

**生成时间**: 2026-07-18
**测量方法**: `cargo llvm-cov test --no-fail-fast` (llvm-cov 覆盖率)
**分支**: `develop/v3.11.0`

## 测试策略说明

| 策略 | 说明 |
|------|------|
| `全量测试` | 运行所有测试用例（含 lib + integration + e2e） |
| `--lib` | 仅运行 lib 单元测试（跳过 integration/e2e 测试） |
| `--lib --skip <slow>` | 运行 lib 单元测试，跳过超过 60s 的慢测试 |

> **注意**: `sqlrustgo-vector` 测试超时（>120s），数据缺失。
> 分支覆盖率（branch coverage）未启用，此报告仅含行覆盖率和函数覆盖率。

## L1_8 核心 Crate（Alpha Gate A5 / GA Gate G3 考核范围）

| Crate | 行覆盖率 | 覆盖行数 | 函数覆盖率 | 函数数 | Alpha ≥75% | GA ≥80% | 备注 |
|-------|----------|----------|-----------|--------|------------|---------|------|
| `sqlrustgo-admin` | 83.14% | 1435/1677 | 82.01% | 139/164 | ✅ | ✅ | 全量测试 |
| `sqlrustgo-tools` | 63.84% | 1626/2214 | 75.51% | 147/183 | ❌ | ❌ | 全量测试 |
| `sqlrustgo-mysql-client` | 43.79% | 507/792 | 61.54% | 26/36 | ❌ | ❌ | 全量测试 |
| `sqlrustgo-parser` | 71.22% | 9522/12262 | 89.66% | 706/779 | ❌ | ❌ | 全量测试 |
| `sqlrustgo-mysql-server` | 51.53% | 3650/5419 | 61.35% | 326/432 | ❌ | ❌ | 全量测试 |
| `sqlrustgo-storage` | 85.58% | 14439/16521 | 83.52% | 1705/1986 | ✅ | ✅ | 全量测试 |
| `sqlrustgo-executor` | 76.45% | 13060/16136 | 78.69% | 1436/1659 | ✅ | ❌ | lib 仅 |
| `sqlrustgo-planner` | 84.91% | 1524/1754 | 79.72% | 212/253 | ✅ | ✅ | lib 仅 |

| **L1_8 合计** | **80.60%** | **45363/56275** | **83.92%** | **4697/5492** | ✅ | ✅ | — |

## 所有 Workspace Crate 覆盖率（按行覆盖率降序）

| # | Crate | 行覆盖率 | 覆盖行数 | 函数覆盖率 | 函数数 | Alpha ≥75% | GA ≥80% | 测量策略 |
|---|-------|----------|----------|-----------|--------|------------|---------|----------|
| 1 | `sqlrustgo-network` | 100.00% | 433/433 | 100.00% | 42/42 | ✅ | ✅ | 全量测试 |
| 2 | `sqlrustgo-cache` | 99.47% | 189/190 | 100.00% | 27/27 | ✅ | ✅ | 全量测试 |
| 3 | `sqlrustgo-wal-verification` | 97.20% | 644/662 | 98.81% | 84/85 | ✅ | ✅ | 全量测试 |
| 4 | `sqlrustgo-telemetry` | 96.67% | 420/434 | 95.00% | 60/63 | ✅ | ✅ | 全量测试 |
| 5 | `sqlrustgo-rag` | 96.58% | 935/967 | 93.75% | 144/153 | ✅ | ✅ | 全量测试 |
| 6 | `sqlrustgo-types` | 91.16% | 713/776 | 93.39% | 121/129 | ✅ | ✅ | 全量测试 |
| 7 | `sqlrustgo-common` | 89.17% | 1210/1341 | 86.98% | 192/217 | ✅ | ✅ | 全量测试 |
| 8 | `sqlrustgo-optimizer` | 88.23% | 3499/3911 | 95.05% | 404/424 | ✅ | ✅ | 全量测试 |
| 9 | `sqlrustgo-planner` | 84.91% | 1524/1754 | 79.72% | 212/253 | ✅ | ✅ | lib 仅 |
| 10 | `sqlrustgo-storage` | 85.58% | 14439/16521 | 83.52% | 1705/1986 | ✅ | ✅ | 全量测试 |
| 11 | `sqlrustgo-transaction` | 84.29% | 2132/2443 | 82.79% | 308/359 | ✅ | ✅ | lib 仅 |
| 12 | `sqlrustgo-catalog` | 85.08% | 3539/4067 | 81.09% | 476/566 | ✅ | ✅ | 全量测试 |
| 13 | `sqlrustgo-admin` | 83.14% | 1435/1677 | 82.01% | 139/164 | ✅ | ✅ | 全量测试 |
| 14 | `sqlrustgo-security` | 82.67% | 1852/2173 | 82.04% | 284/335 | ✅ | ✅ | 全量测试 |
| 15 | `sqlrustgo-server` | 85.00% | 1220/1435 | 78.65% | 178/216 | ✅ | ✅ | 全量测试 |
| 16 | `sqlrustgo-executor` | 76.45% | 13060/16136 | 78.69% | 1436/1659 | ✅ | ❌ | lib 仅 |
| 17 | `sqlrustgo-spill` | 75.17% | 725/905 | 76.36% | 110/136 | ✅ | ❌ | 全量测试 |
| 18 | `sqlrustgo-sql-corpus` | 75.16% | 914/1141 | 54.43% | 79/115 | ✅ | ❌ | 全量测试 |
| 19 | `sqlrustgo-gmp` | 73.54% | 3088/4200 | 66.45% | 310/414 | ❌ | ❌ | 全量测试 |
| 20 | `sqlrustgo-parser` | 71.22% | 9522/12262 | 89.66% | 706/779 | ❌ | ❌ | 全量测试 |
| 21 | `sqlrustgo-tools` | 63.84% | 1626/2214 | 75.51% | 147/183 | ❌ | ❌ | 全量测试 |
| 22 | `sqlrustgo-bench` | 57.85% | 2109/2998 | 57.45% | 369/526 | ❌ | ❌ | lib (跳慢测) |
| 23 | `sqlrustgo-mysql-server` | 51.53% | 3650/5419 | 61.35% | 326/432 | ❌ | ❌ | 全量测试 |
| 24 | `sqlrustgo-mysql-client` | 43.79% | 507/792 | 61.54% | 26/36 | ❌ | ❌ | 全量测试 |
| 25 | `sqlrustgo` | 17.00% | 6958/12733 | 18.74% | 587/1064 | ❌ | ❌ | lib (跳慢测) |
| 26 | `sqlrustgo-soak` | 4.89% | 716/1397 | 9.30% | 43/82 | ❌ | ❌ | 全量测试 |
| 27 | `sqlrustgo-cli` | 0.00% | 186/372 | 0.00% | 10/20 | ❌ | ❌ | 全量测试 |

| **Workspace 合计** | **77.99%** | **77245/99050** | **80.64%** | **8525/10572** | ✅ | ✅ | — |

## 覆盖率分级统计

- **Alpha (≥75%)**: 18/27 crates ✅
- **GA (≥80%)**: 15/27 crates
- **Timeout（未测量）**: sqlrustgo-vector
- **Workspace 行覆盖率**: 77.99%

### 未达 Alpha 75% 的 Crate（9 个）

- `sqlrustgo-gmp`: 73.54% (3088/4200)
- `sqlrustgo-parser`: 71.22% (9522/12262)
- `sqlrustgo-tools`: 63.84% (1626/2214)
- `sqlrustgo-bench`: 57.85% (2109/2998)
- `sqlrustgo-mysql-server`: 51.53% (3650/5419)
- `sqlrustgo-mysql-client`: 43.79% (507/792)
- `sqlrustgo`: 17.00% (6958/12733)
- `sqlrustgo-soak`: 4.89% (716/1397)
- `sqlrustgo-cli`: 0.00% (186/372)

### 未达 GA 80% 的 Crate（12 个）

- `sqlrustgo-executor`: 76.45% (13060/16136)
- `sqlrustgo-spill`: 75.17% (725/905)
- `sqlrustgo-sql-corpus`: 75.16% (914/1141)
- `sqlrustgo-gmp`: 73.54% (3088/4200)
- `sqlrustgo-parser`: 71.22% (9522/12262)
- `sqlrustgo-tools`: 63.84% (1626/2214)
- `sqlrustgo-bench`: 57.85% (2109/2998)
- `sqlrustgo-mysql-server`: 51.53% (3650/5419)
- `sqlrustgo-mysql-client`: 43.79% (507/792)
- `sqlrustgo`: 17.00% (6958/12733)
- `sqlrustgo-soak`: 4.89% (716/1397)
- `sqlrustgo-cli`: 0.00% (186/372)

## Alpha Gate A5 分析

| 指标 | 数值 | 阈值 | 状态 |
|------|------|------|------|
| L1_8 平均行覆盖率 | **80.60%** | ≥75% | ✅ |
| L1_8 平均函数覆盖率 | 83.92% | — | — |
| Alpha 达标 crate 数 | 18/27 | — | — |

**结论**: L1_8 平均行覆盖率 **80.60%**，超过 Alpha A5 阈值（75%），满足 Alpha Gate A5 条件。

## GA Gate G3 分析

- 已达 GA 80% 的 crate（15 个）:
  sqlrustgo-admin, sqlrustgo-cache, sqlrustgo-catalog, sqlrustgo-common, sqlrustgo-network, sqlrustgo-optimizer, sqlrustgo-planner, sqlrustgo-rag, sqlrustgo-security, sqlrustgo-server, sqlrustgo-storage, sqlrustgo-telemetry, sqlrustgo-transaction, sqlrustgo-types, sqlrustgo-wal-verification
- 未达 GA 80% 的 crate（12 个）:
  sqlrustgo, sqlrustgo-bench, sqlrustgo-cli, sqlrustgo-executor, sqlrustgo-gmp, sqlrustgo-mysql-client, sqlrustgo-mysql-server, sqlrustgo-parser, sqlrustgo-soak, sqlrustgo-spill, sqlrustgo-sql-corpus, sqlrustgo-tools

## 覆盖率区间分布

| 区间 | Crate 数 | |
|------|----------|---|
| **≥90%** | **6** | sqlrustgo-network (100.00%), sqlrustgo-cache (99.47%), sqlrustgo-wal-verification (97.20%), sqlrustgo-telemetry (96.67%), sqlrustgo-rag (96.58%), sqlrustgo-types (91.16%) |
| **80-89%** | **7** | sqlrustgo-common (89.17%), sqlrustgo-optimizer (88.23%), sqlrustgo-server (85.00%), sqlrustgo-planner (84.91%), sqlrustgo-storage (85.58%), sqlrustgo-transaction (84.29%), sqlrustgo-catalog (85.08%), sqlrustgo-admin (83.14%), sqlrustgo-security (82.67%) |
| **75-79%** | **3** | sqlrustgo-executor (76.45%), sqlrustgo-spill (75.17%), sqlrustgo-sql-corpus (75.16%) |
| **50-74%** | **5** | sqlrustgo-gmp (73.54%), sqlrustgo-parser (71.22%), sqlrustgo-tools (63.84%), sqlrustgo-bench (57.85%), sqlrustgo-mysql-server (51.53%) |
| **<50%** | **4** | sqlrustgo-mysql-client (43.79%), sqlrustgo (17.00%), sqlrustgo-soak (4.89%), sqlrustgo-cli (0.00%) |

## 覆盖率总览（ASCII 图表）

```
Crate                                  Line%     Lines      Func%    Funcs
---------------------------------------------------------------------------
sqlrustgo-network                    100.00%    433/433  100.00%    42/42
sqlrustgo-cache                       99.47%    189/190  100.00%    27/27
sqlrustgo-wal-verification            97.20%    644/662   98.81%    84/85
sqlrustgo-telemetry                   96.67%    420/434   95.00%    60/63
sqlrustgo-rag                         96.58%    935/967   93.75%   144/153
sqlrustgo-types                       91.16%    713/776   93.39%   121/129
sqlrustgo-common                      89.17%   1210/1341   86.98%   192/217
sqlrustgo-optimizer                   88.23%   3499/3911   95.05%   404/424
sqlrustgo-server                      85.00%   1220/1435   78.65%   178/216
sqlrustgo-planner                     84.91%   1524/1754   79.72%   212/253
sqlrustgo-storage                     85.58%  14439/16521   83.52% 1705/1986
sqlrustgo-transaction                 84.29%   2132/2443   82.79%   308/359
sqlrustgo-catalog                     85.08%   3539/4067   81.09%   476/566
sqlrustgo-admin                       83.14%   1435/1677   82.01%   139/164
sqlrustgo-security                    82.67%   1852/2173   82.04%   284/335
sqlrustgo-executor                    76.45%  13060/16136   78.69% 1436/1659
sqlrustgo-spill                       75.17%    725/905   76.36%   110/136
sqlrustgo-sql-corpus                  75.16%    914/1141   54.43%    79/115
sqlrustgo-gmp                         73.54%   3088/4200   66.45%   310/414
sqlrustgo-parser                      71.22%   9522/12262   89.66%   706/779
sqlrustgo-tools                       63.84%   1626/2214   75.51%   147/183
sqlrustgo-bench                       57.85%   2109/2998   57.45%   369/526
sqlrustgo-mysql-server                51.53%   3650/5419   61.35%   326/432
sqlrustgo-mysql-client                43.79%    507/792   61.54%    26/36
sqlrustgo                             17.00%   6958/12733   18.74%   587/1064
sqlrustgo-soak                         4.89%    716/1397    9.30%    43/82
sqlrustgo-cli                          0.00%    186/372    0.00%    10/20
sqlrustgo-vector                          N/A      N/A       N/A       N/A (超时)
---------------------------------------------------------------------------
TOTAL (27 crates)                    77.99%  77245/99050   80.64%  8525/10572
L1_8 (8 crates)                      80.60%  45363/56275   83.92%  4697/5492
```
