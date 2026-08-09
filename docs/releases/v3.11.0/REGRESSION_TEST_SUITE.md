# v3.11.0 回归测试套件

## 1. 概览

本文件描述 v3.11.0 GA 相关自动回归测试套件。由于历史英文原文包含 fixture missing / PENDING 的旧状态，当前中文主文按最新综合评估明确测试边界：能证明的只按实跑证据声明，缺失的测试必须进入 v3.12 跟踪。

## 2. 测试组成

| 测试项 | 命令/方法 | 当前要求 |
|---|---|---|
| TPC-H Baseline (SF=1) | `bash scripts/tpch/run_sf1.sh` 或后续 gate | 必须有 fixture、row-count、checksum 和 per-query artifact |
| Chaos / SOAK | `bash scripts/soak/chaos_soak_test.sh` 或 168h SOAK 流程 | 必须记录时长、错误数、内存、恢复行为 |
| Upgrade Test v3.10.0 -> v3.11.0 | `bash scripts/test_upgrade_v310_to_v311.sh` | 必须记录数据一致性和 rollback 策略 |
| Sysbench OLTP | `bash scripts/sysbench/run_oltp.sh 600 8` | 需要 baseline 和可接受回归阈值 |
| Coverage Regression | `cargo llvm-cov ...` | 需要统一 coverage command，不能混用口径 |

## 3. 性能回归判断

建议使用 baseline 对比脚本：

```bash
bash scripts/tpch/compare_baseline.sh --output report.json
```

可接受标准应明确记录，例如 p95 latency 增长小于 10%。若没有 baseline、fixture 或输出 artifact，不得写成 PASS。

## 4. CI 集成要求

回归套件进入 CI 时，必须确保 gate 失败时返回非 0，不能使用 warn-only loop 或忽略失败。

```yaml
regression-tests:
  script:
    - bash scripts/tpch/run_sf1.sh
    - bash scripts/soak/chaos_soak_test.sh
    - bash scripts/test_upgrade_v310_to_v311.sh
```

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义。若英文附录与中文正文或 `COMPREHENSIVE_ASSESSMENT_REPORT.md` 冲突，当前正式判断以中文正文和综合评估报告为准。

# v3.11.0 Regression Test Suite

## Overview

Automated regression test suite for v3.11.0 GA release.

## Test Components

### 1. TPC-H Baseline (SF=1)

22/22 queries must pass at SF=1 once fixture is generated.

**Gate**: `bash scripts/tpch/run_sf1.sh` → ⚠️ PENDING (fixture at /tmp/tpch-sf1 missing)

```bash
cd /path/to/sqlrustgo
bash scripts/tpch/run_sf1.sh
```

### 2. Chaos Soak (2 hours)

Random fault injection with 2-hour soak.

**Gate**: `bash scripts/soak/chaos_soak_test.sh` → all PASS

```bash
cd /path/to/sqlrustgo
bash scripts/soak/chaos_soak_test.sh
```

### 3. Upgrade Test (v3.10.0 → v3.11.0)

Data migration verification.

**Gate**: `bash scripts/test_upgrade_v310_to_v311.sh` → all PASS

```bash
cd /path/to/sqlrustgo
bash scripts/test_upgrade_v310_to_v311.sh
```

### 4. Sysbench OLTP

Mixed read/write workload verification.

**Gate**: `bash scripts/sysbench/run_oltp.sh 600 8` → tps > baseline

### 5. Coverage Regression

Line coverage must not decrease.

**Gate**: L1_8 ≥ 80.60% (previous release baseline)

```bash
cargo llvm-cov test -p sqlrustgo --no-fail-fast
cargo llvm-cov test -p sqlrustgo-storage --no-fail-fast
cargo llvm-cov test -p sqlrustgo-executor --no-fail-fast
```

## Performance Regression Detection

Compare query times against baseline:

```bash
bash scripts/tpch/compare_baseline.sh --output report.json
```

**Acceptable**: p95 latency increase < 10%

## CI Integration

Add to CI pipeline:

```yaml
regression-tests:
  script:
    - bash scripts/tpch/run_sf1.sh
    - bash scripts/soak/chaos_soak_test.sh
    - bash scripts/test_upgrade_v310_to_v311.sh
```

## Gate Summary

| Test | Gate | Status |
|------|------|--------|
| TPC-H SF=1 | 22/22 PENDING (fixture missing) | Required |
| Chaos Soak | 2h all PASS | Required |
| Upgrade | All PASS | Required |
| Sysbench OLTP | tps > baseline | Required |
| Coverage | L1_8 ≥ 80.60% | Required |
