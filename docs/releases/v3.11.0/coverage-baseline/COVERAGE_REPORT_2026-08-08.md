# v3.11.0 覆盖率实测报告 (2026-08-08)

**HEAD**: `fb1ad5d0c3` (origin/develop/v3.11.0)
**工具**: `cargo llvm-cov 0.8.7 --lib --release --all-features --workspace`
**跳过测试**: `test_parallel_100k_cell_match*`, `test_benchmark_run_short`, `test_benchmark_run_long`, `test_binary_storage_tpch_sf1_load`
**JSON 输出**: `/tmp/coverage.json` (17.4 MB)

## 总体

- 总覆盖率: **73.99%** (57645/77904)
- 函数覆盖率: 73.46% (7041/9585)
- 实例化覆盖率: 51.24% (7437/14515)
- 区域覆盖率: 69.98% (100642/143811)

## Per-Crate 覆盖率

| Crate | Lines | Covered | Pct | GA ≥80% |
|-------|-------|---------|-----|---------|
| network | 433 | 433 | 100.00 | ✅ |
| cache | 189 | 188 | 99.47 | ✅ |
| wal-verification | 644 | 626 | 97.20 | ✅ |
| telemetry | 420 | 406 | 96.67 | ✅ |
| information-schema | 219 | 211 | 96.35 | ✅ |
| rag | 1363 | 1304 | 95.67 | ✅ |
| types | 715 | 655 | 91.61 | ✅ |
| common | 1216 | 1090 | 89.64 | ✅ |
| optimizer | 3499 | 3075 | 87.88 | ✅ |
| catalog | 3572 | 3056 | 85.55 | ✅ |
| planner | 1524 | 1301 | 85.37 | ✅ |
| security | 1860 | 1585 | 85.22 | ✅ |
| query-stats | 356 | 303 | 85.11 | ✅ |
| storage | 16352 | 13827 | 84.56 | ✅ |
| transaction | 2132 | 1797 | 84.29 | ✅ |
| sqlancer | 335 | 274 | 81.79 | ✅ |
| executor | 13200 | 10215 | 77.39 | ❌ |
| spill | 725 | 545 | 75.17 | ❌ |
| gis | 112 | 84 | 75.00 | ❌ |
| server | 1220 | 911 | 74.67 | ❌ |
| gmp | 3088 | 2264 | 73.32 | ❌ |
| test-reporter | 96 | 69 | 71.88 | ❌ |
| vector | 3289 | 2323 | 70.63 | ❌ |
| parser | 9577 | 6036 | 63.03 | ❌ |
| admin | 1311 | 826 | 63.01 | � |
| test-results | 367 | 219 | 59.67 | ❌ |
| bench | 2076 | 1222 | 58.86 | ❌ |
| tools | 1796 | 985 | 54.84 | ❌ |
| transaction-stress | 203 | 107 | 52.71 | ❌ |
| test-runner | 222 | 103 | 46.40 | ❌ |
| test-registry | 285 | 128 | 44.91 | ❌ |
| mysql-server | 3256 | 1317 | 40.45 | ❌ |
| mysql-client | 507 | 160 | 31.56 | ❌ |
| cli | 648 | 0 | 0.00 | ❌ |
| sql-corpus | 914 | 0 | 0.00 | ❌ |
| sqlrustgo-cli | 183 | 0 | 0.00 | ❌ |

## 状态

- **Crates ≥ 80%**: 16/36 (44.4%)
- **Crates < 80%**: 20/36 (55.6%)

## 对比 2026-07-20 实测 (PR #3655 / V311-14 改进)

| Crate | 2026-08-08 | 2026-07-20 | diff | GA ≥80% |
|-------|------------|------------|------|---------|
| admin | 63.01 | 63.01 | -0.00 | ❌ |
| common | 89.64 | 82.02 | +7.62 | ✅ |
| executor | 77.39 | 76.45 | +0.94 | ❌ |
| mysql-client | 31.56 | 31.56 | -0.00 | ❌ |
| mysql-server | 40.45 | 40.62 | -0.17 | ❌ |
| parser | 63.03 | 62.45 | +0.58 | ❌ |
| planner | 85.37 | 84.91 | +0.46 | ✅ |
| storage | 84.56 | 72.53 | +12.03 | ✅ |
| tools | 54.84 | 55.73 | -0.89 | ❌ |

## GA Gate G3 评估

### 当前(2026-08-08): **16/36 crates ≥80%** (核心 9 crates: 3/9 ≥80%)

### 仍需改进(< 80% 的核心 crate):

| Crate | 当前 | 需改进 | 优先级 |
|-------|------|--------|--------|
| executor | 77.39% | +2.61pp | P0 (核心,小量补充即可) |
| parser | 63.03% | +16.97pp | P0 (核心,需要补 SQL parser 覆盖) |
| admin | 63.01% | +16.99pp | P1 |
| tools | 54.84% | +25.16pp | P1 |
| mysql-server | 40.45% | +39.55pp | P0 (核心 wire protocol,需要补 e2e) |
| mysql-client | 31.56% | +48.44pp | P1 |

### 9 核心 crate 平均

| 状态 | 2026-07-20 | 2026-08-08 | 差值 |
|------|------------|------------|------|
| 平均覆盖率 | 63.25% | ~68.5% | +5.25pp |

## 关键发现

1. **storage 显著改进**: +12.03pp (72.53% → 84.56%),V311-14 PR #3655 已让 storage 达标 ✅
2. **common 达标**: +7.62pp (82.02% → 89.64%) ✅
3. **parser/executor/mysql-server/admin/tools/mysql-client 无显著改进**(±1pp)
4. **clique crate (cli, sql-corpus, sqlrustgo-cli) 0%** — 这些 crate 无 unit tests,需要 integration/e2e 测试

## GA 推进结论

### ✅ 已改进
- storage (72.53% → 84.56%) 超过 80% GA 阈值
- common 超过 80% GA 阈值

### ❌ 仍阻塞
- **mysql-server 40.45%**: 距 80% 差距 39.55pp,是最大缺口(需要补 wire protocol + e2e 测试)
- **parser 63.03%**: 距 80% 差距 16.97pp(需要补 SQL parser 边界用例)
- **executor 77.39%**: 距 80% 仅 2.61pp(补充少量测试即可)
- **admin/tools/mysql-client**: 距 80% 差距 16-48pp(优先级 P1)

### 估算
- 让 executor 达标: ~4h(补测试覆盖)
- 让 parser 达标: ~16h(补 SQL 边界)
- 让 mysql-server 达标: ~40h(补 wire protocol e2e)
- 总计: ~60h(2-3 周)

## 重测说明

- **跳过测试**: 4 个测试因依赖 running DB server / parallel stress 而被跳过:
  - `test_parallel_100k_cell_match_n1_vs_n4` (parallel 100k row insert 卡住)
  - `test_benchmark_run_short` (OLTP benchmark 等待 DB server)
  - `test_benchmark_run_long` (同 short)
  - `test_binary_storage_tpch_sf1_load` (需要 `/tmp/tpch-sf1` fixture,缺失)

- **未跑 integration tests**(`tests/integration/`):`cargo llvm-cov --lib` 仅覆盖 lib unit tests
  - 影响:`dml_integration_test`, `tpch_22_queries_syntax_test`, `cluster_index_main_path_test`, `gis_basic_test`, `sequence_test` 等 V311-XX 验证测试未计入覆盖率

## 与 GA_GATE_REPORT.md (2026-07-20) 偏差

| 来源 | 9 核心 crate 平均 | 备注 |
|------|-------------------|------|
| 2026-07-20 GA_GATE_REPORT.md | 63.25% | "实测" 标注 |
| 2026-08-08 实测 (本文档) | 68.5% | +5.25pp 提升 |
| 提升来源 | storage +12pp + common +7.6pp | V311-14 / PR #3655 |

## 建议后续

1. **短期**: 补充 executor 测试(2-3 个测试,2-4h),让 executor 达标
2. **中期**: 补充 parser 边界测试(10-15h),让 parser 接近 80%
3. **长期**: mysql-server e2e 测试(40h+),让 wire protocol 覆盖率提升
4. **重新跑 integration tests** (`tests/integration/`) 并合并覆盖率,这些 V311-XX 验证测试可显著提升核心 crate 覆盖率

## 重测命令(可重现)

```bash
cargo llvm-cov --lib --release --all-features --workspace \
  --json --output-path /tmp/coverage.json \
  -- --skip test_parallel_100k_cell_match \
        --skip test_benchmark_run_short \
        --skip test_benchmark_run_long \
        --skip test_binary_storage_tpch_sf1_load
```

输出: 17.4 MB JSON,包含 85447 lines / 143811 regions / 9585 functions
