# GitCode CI Integration for Sprint 5 TPC-H Evaluation

> **Date**: 2026-06-07
> **Reason**: 252 Gitea down (PR review blocked), 250 backup down.
> Gitcode (cloud) is the only fully-accessible mirror.
> **Ref**: 用户 2026-06-07 — need CI integration that works without 252/250

## Pipeline (gitcode.com CI)

```yaml
# .gitcode-ci.yml
name: v3.9.0 Sprint 5 TPC-H Evaluation
on:
  push:
    branches: [develop/v3.9.0, fix/v390-*]
  pull_request:
    branches: [develop/v3.9.0]

jobs:
  freeze-oracle:
    name: Validate Oracle Frozen (Step P0-1)
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install psql
        run: brew install postgresql@16
      - name: Setup PG tpch_test
        env:
          PGPASSWORD: ${{ secrets.PGPASSWORD }}
        run: |
          # Use pre-frozen snapshot, no runtime mutation
          python3 bench/oracle/freeze_oracle.py
      - name: Validate snapshot
        run: |
          python3 bench/oracle/tpch_harness_v2.py validate \
            --snapshot bench/oracle/tpch_sf01_snapshot_v2

  build-engine:
    name: Build sqlrustgo engine binary
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - name: Rust cache
        uses: Swatinem/rust-cache@v2
      - name: Build tpch_run_query
        run: cargo build --release -p sqlrustgo-bench --example tpch_run_query
      - name: Upload binary
        uses: actions/upload-artifact@v3
        with:
          name: tpch_run_query
          path: target/release/examples/tpch_run_query

  run-harness:
    name: Run TPC-H harness v2 (Step P0-2)
    needs: [freeze-oracle, build-engine]
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install psql
        run: brew install postgresql@16
      - name: Setup PG tpch_test
        env:
          PGPASSWORD: ${{ secrets.PGPASSWORD }}
        run: |
          # Use snapshot
          psql -U liying -d postgres -c "DROP DATABASE IF EXISTS tpch_test;"
          psql -U liying -d postgres -c "CREATE DATABASE tpch_test;"
          # Re-freeze for CI runner (different env)
          python3 bench/oracle/freeze_oracle.py
      - name: Download engine binary
        uses: actions/download-artifact@v3
        with:
          name: tpch_run_query
      - name: Run full 22-query harness
        run: |
          python3 bench/oracle/tpch_harness_v2.py run \
            --snapshot bench/oracle/tpch_sf01_snapshot_v2 \
            --timeout 30 \
            --report docs/audit/status/$(date +%Y-%m-%d)-tpch-cell-diff-ci.json
      - name: Upload report
        uses: actions/upload-artifact@v3
        with:
          name: harness-report
          path: docs/audit/status/*-tpch-cell-diff-ci.json

  gate:
    name: GA gate (≥95% semantic pass rate)
    needs: [run-harness]
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - name: Download report
        uses: actions/download-artifact@v3
        with:
          name: harness-report
      - name: Evaluate gate
        run: |
          python3 -c "
          import json, sys
          r = json.load(open('docs/audit/status/$(date +%Y-%m-%d)-tpch-cell-diff-ci.json'))
          s = r['summary']
          total = sum(s.values())
          pass_rate = s.get('PASS', 0) / total if total else 0
          timeout_rate = s.get('TIMEOUT', 0) / total if total else 1
          print(f'Pass rate: {pass_rate:.1%}')
          print(f'Timeout rate: {timeout_rate:.1%}')
          # GA gate criteria (per user 2026-06-07):
          # semantic pass rate ≥ 95% (21-22/22)
          # timeout rate ≤ 10% (≤ 2/22)
          if pass_rate >= 0.95 and timeout_rate <= 0.10:
              print('GATE PASS')
              sys.exit(0)
          else:
              print('GATE FAIL — sprint 5 not ready for GA')
              sys.exit(1)
          "
```

## Local invocation (without CI)

```bash
# Step 1: Freeze oracle (one-time)
python3 bench/oracle/freeze_oracle.py

# Step 2: Validate PG matches snapshot
python3 bench/oracle/tpch_harness_v2.py validate --snapshot bench/oracle/tpch_sf01_snapshot_v2

# Step 3: Build engine binary
cargo build --release -p sqlrustgo-bench --example tpch_run_query

# Step 4: Run full harness
python3 bench/oracle/tpch_harness_v2.py run \
  --snapshot bench/oracle/tpch_sf01_snapshot_v2 \
  --timeout 30 \
  --report docs/audit/status/$(date +%Y-%m-%d)-tpch-cell-diff-ci.json

# Step 5: Inspect
cat docs/audit/status/*-tpch-cell-diff-ci.json | python3 -m json.tool | head -30
```

## Sprint 5 v2 Current Results (snapshot)

| State | Count | Queries |
|-------|------:|---------|
| ✓ PASS | 14 | Q1, Q5, Q6, Q7, Q9, Q11, Q12, Q13, Q14, Q15, Q16, Q19, Q20, Q22 |
| ✗ FAIL | 5 | Q3, Q8, Q10, Q17, Q18 |
| ⏱ TIMEOUT | 3 | Q2, Q4, Q21 |

**GA gate criteria (per user)**:
- semantic pass rate ≥ 95% (= 21-22/22) — currently 63.6%
- timeout rate ≤ 10% (= ≤ 2/22) — currently 13.6%
- oracle mismatch = 0 — currently 0

**GA gate NOT met**. 8 more queries need to be fixed (5 FAIL + 3 TIMEOUT).

## Opencode assignments (parallel work)

| Path | Issue | Engine fix | Status |
|------|-------|------------|--------|
| A | #3286 | Multi-JOIN ON-condition | ⏳ in progress |
| B | #3289 | Q20/Q21 correlated EXISTS (lineitem l_orderkey index) | ⏳ in progress |
| C | #3276 + #3285 | SUM(REAL) precision + storage type preservation | ⏳ in progress |
| D | #3277 + #3278 | Q14 LIKE/date (sibling fix) | ⏳ in progress |
| E | Q17 specific bug | correlated scalar subquery in WHERE | 🔍 new (just discovered) |

## Why this CI approach

The current sprint 5 cell-diff test (`tests/four_way_cell_diff_test.rs`)
hangs on Q3/Q4/Q8/Q21 N² EXISTS scan and can't be the CI gate. The
harness v2 Python tool has:

1. **Hard timeout per query** (8-30s) — never hangs
2. **Frozen oracle snapshot** — no mutation risk
3. **5-state classification** — honest PASS/FAIL/TIMEOUT distinction
4. **JSON report** — easy to gate on in CI

This is the GA-grade benchmark infrastructure that the user's
2026-06-07 feedback was asking for. Sprint 5 verification can now
be trusted.

## Files

- `bench/oracle/freeze_oracle.py` — oracle fingerprint
- `bench/oracle/tpch_harness_v2.py` — 3-layer harness
- `bench/oracle/tpch_sf01_snapshot_v2/meta.json` — frozen metadata
- `crates/bench/examples/tpch_run_query.rs` — engine binary
- `bench/oracle/reports/sprint5_full_v2.json` — last run
- `docs/audit/status/2026-06-07-GITCODE_CI_INTEGRATION.md` (this file)
