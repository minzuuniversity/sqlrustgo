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
