# Z6G4 Hand-off Manual (v3.9.0 GA-blocking Tasks)

> **Date**: 2026-06-18 (post PR #3507, #3508 merge)
> **Branch**: `develop/v3.9.0` @ `784fe34447` (PR #3507 merged 03:36:15Z, PR #3508 merged 03:37:37Z)
> **Audience**: Z6G4 agent (or operator with 250 access)
> **Goal**: Complete all GA-blocking real-world tests; capture results; enable GA decision

---

## 0. Pre-requisites (Local P0 — All ✅ PASS)

Before Z6G4 hand-off, the following must be confirmed locally:

- [x] **V9 Coverage Gate 漏洞修复** — `check_coverage.sh` parametrized, `--skip` removed
- [x] **G17 Coverage Gate 定义** — ≥80% line coverage in `GATE_CONDITIONS.md`
- [x] **8 Oracle Gaps closed** — 8/8 gate scripts invoke inline oracle
- [x] **4 new oracle test files** — `oracle_g14_real_crash.rs`, `oracle_p22_time_travel.rs`, `oracle_p23_hash_chain.rs`, `oracle_p34_parallel_executor.rs`
- [x] **TPC-H 22/22 maintained**
- [x] **Bash syntax verified** on all 8 modified gate scripts
- [x] **RC8 docs created** — `RC8_GATE_REPORT.md`, `RC8_RELEASE_NOTES.md`, `CHANGELOG.md v1.3`
- [x] **PR #3507 merged** — RC8 docs + V9 fix + 8 oracle (closed 03:36:15Z)
- [x] **PR #3508 merged** — G5-B savepoint name case + P22 delete-isolation fix (closed 03:37:37Z)
- [x] **#3498 closed as duplicate of #3499** — `aggregate_smoke_test` 3/3 PASS verified

**Open P0 / hand-off items** (NOT pre-requisites, deferred to Z6G4):
- 🟡 **#3474 (P0)** — libmysqlclient 8.0.46 trailing terminator 60s hang (no fix yet)
- 🟡 **#3484** — sysbench 1h soak
- 🟡 **#3225 / #3229 / #3265 / #3266** — 24h/72h/168h wall-clock soaks

**If any pre-requisite fails**, do NOT proceed to Z6G4 — fix locally first.

---

## 1. Issue #3484: Sysbench 1h Soak

**Goal**: Run sysbench OLTP for 1 hour and verify **0 errors**.

### Steps

```bash
# 1.1 Verify sysbench installed
command -v sysbench && sysbench --version

# 1.2 Start SQLRustGo server
cargo run --bin sqlrustgo-server --release &> /tmp/sqlrustgo-server.log &
SERVER_PID=$!
sleep 5

# 1.3 Prepare sysbench data (10 tables × 100K rows)
sysbench oltp_read_write \
    --tables=10 --table-size=100000 \
    --db-driver=mysql --mysql-host=127.0.0.1 --mysql-port=3306 \
    --mysql-user=root --mysql-password= \
    prepare

# 1.4 Run 1h soak
START=$(date +%s)
sysbench oltp_read_write \
    --tables=10 --table-size=100000 \
    --db-driver=mysql --mysql-host=127.0.0.1 --mysql-port=3306 \
    --mysql-user=root --mysql-password= \
    --time=3600 --threads=16 --rate=1000 \
    --report-interval=60 \
    run 2>&1 | tee /tmp/sysbench_1h.log
END=$(date +%s)

# 1.5 Stop server
kill $SERVER_PID 2>/dev/null

# 1.6 Verify 0 errors
ERRORS=$(grep -cE "ERROR|FATAL" /tmp/sysbench_1h.log || echo 0)
QPS=$(grep "transactions:" /tmp/sysbench_1h.log | tail -1 | grep -oE "[0-9]+\.[0-9]+ per sec" | head -1)
DURATION=$((END - START))

echo "Soak result: ${DURATION}s, ${QPS} qps, ${ERRORS} errors"
```

### Expected Results

- Duration: 3600s ± 60s
- QPS: stable (no cliff drop)
- Errors: **0**

### Result Capture

Save to `docs/releases/v3.9.0/logs/sysbench_1h_<timestamp>/`:
- `sysbench_1h.log` — full sysbench output
- `RESULT.txt` — one-line summary
- `server.log` — server logs (sanitized)

---

## 2. Issue #3474: libmysqlclient 8.0.46 60s Hang

**Goal**: Verify that libmysqlclient 8.0.46 client connects and runs queries in <60s.

### Steps

```bash
# 2.1 Verify libmysqlclient version
mysql_config --version
ldconfig -p | grep mysqlclient | head -1

# 2.2 Start server
cargo run --bin sqlrustgo-server --release &> /tmp/sqlrustgo-server.log &
SERVER_PID=$!
sleep 5

# 2.3 Run MySQL client 60s hang test
START=$(date +%s)
mysql -h 127.0.0.1 -P 3306 -u root -e "
    SELECT 1;
    SELECT COUNT(*) FROM information_schema.tables;
    SHOW DATABASES;
    SELECT * FROM (SELECT 1) AS t1 JOIN (SELECT 2) AS t2;
" 2>&1 | tee /tmp/mysql_client_test.log
END=$(date +%s)

# 2.4 Stop server
kill $SERVER_PID 2>/dev/null

DURATION=$((END - START))
echo "MySQL client test: ${DURATION}s"
```

### Expected Results

- DURATION: < 60s
- Output: 4 successful query results

### If DURATION ≥ 60s

The DEPRECATE_EOF fix is incomplete. File a new issue and DO NOT cut GA.

---

## 3. Issue #3225: 24h Real Soak

**Goal**: Run real wall-clock 24h soak with no errors and no leaks.

### Steps

```bash
# 3.1 Start server
cargo run --bin sqlrustgo-server --release &> /tmp/sqlrustgo-soak.log &
SERVER_PID=$!

# 3.2 Run 24h soak (use built-in soak command)
cargo run --bin sqlrustgo-mysql-server --release -- soak \
    --duration 24h \
    --qps 1000 \
    --output /tmp/soak_24h.jsonl \
    --sample-interval-s 60 \
    --rss-warn-mb 512 \
    --seed 42 \
    2>&1 | tee /tmp/soak_24h_runner.log

# 3.3 Stop server
kill $SERVER_PID 2>/dev/null

# 3.4 Generate report
python3 scripts/soak/generate_report.py \
    /tmp/soak_24h.jsonl \
    > docs/releases/v3.9.0/logs/soak_24h_<timestamp>/SOAK_24H_REPORT.md
```

### Expected Results

- Duration: 86400s ± 60s
- Transactions: ≥ 80M (1000 qps × 86400s)
- Errors: **0**
- RSS growth: < 100MB (no leak)
- p99 latency: stable (no degradation)

### Leak Detection

If RSS growth > 100MB:
```bash
# Check for FD leak
ls /proc/$SERVER_PID/fd | wc -l  # should be stable

# Check for lock leak
# (custom metric, see SOAK_24H_REPORT.md)
```

### Result Capture

Save to `docs/releases/v3.9.0/logs/soak_24h_<timestamp>/`:
- `soak_24h.jsonl` — full time-series
- `soak_24h_runner.log` — runner output
- `SOAK_24H_REPORT.md` — generated report
- `RESULT.txt` — one-line summary

---

## 4. Coverage ≥80% Verification

**Goal**: Verify Coverage Gate passes (≥80% line coverage).

### Steps

```bash
# 4.1 Run coverage
cargo llvm-cov --workspace --all-features --tests \
    --lcov --output-path /tmp/lcov.info 2>&1 | tail -5

# 4.2 Generate summary
cargo llvm-cov report --summary-only 2>&1 | tee /tmp/coverage_summary.txt

# 4.3 Check threshold
LINE_COV=$(grep -oE "line coverage: [0-9]+\.[0-9]+%" /tmp/coverage_summary.txt | grep -oE "[0-9]+\.[0-9]+")
THRESHOLD=80.0
echo "Line coverage: ${LINE_COV}% (threshold: ${THRESHOLD}%)"

if (( $(echo "$LINE_COV >= $THRESHOLD" | bc -l) )); then
    echo "✅ Coverage Gate PASS"
else
    echo "❌ Coverage Gate FAIL"
fi
```

### Expected Results

- Line coverage: ≥ 80.0%

### If Line Coverage < 80%

DO NOT cut GA. Either:
- Add tests to raise coverage
- Lower the G17 threshold (and document the decision)

### Result Capture

Save to `docs/releases/v3.9.0/coverage-summary.md` (generated by `check_coverage.sh`).

---

## 5. Result Capture Summary

All Z6G4 outputs go to `docs/releases/v3.9.0/logs/z6g4_<timestamp>/`:

```
docs/releases/v3.9.0/logs/z6g4_<timestamp>/
├── sysbench_1h/
│   ├── sysbench_1h.log
│   ├── server.log
│   └── RESULT.txt
├── mysql_client_hang/
│   ├── mysql_client_test.log
│   └── RESULT.txt
├── soak_24h/
│   ├── soak_24h.jsonl
│   ├── soak_24h_runner.log
│   ├── SOAK_24H_REPORT.md
│   └── RESULT.txt
├── coverage/
│   ├── coverage_summary.txt
│   └── RESULT.txt
└── SUMMARY.md  # all 4 results in one file
```

---

## 6. GA Decision Tree

After all 4 Z6G4 tasks complete, apply this decision tree:

```
24h soak 0 errors?
├─ Yes → Coverage ≥80%?
│        ├─ Yes → sysbench 1h 0 errors?
│        │        ├─ Yes → MySQL client <60s?
│        │        │        ├─ Yes → GA ✅  → Cut v3.9.0 tag
│        │        │        └─ No  → RC9 + #3474 patch
│        │        └─ No  → RC9 + perf investigation
│        └─ No  → RC9 + add tests
└─ No  → Leak detected?
         ├─ Yes → RC9 + leak fix
         └─ No  → RC9 + investigate
```

---

## 7. Z6G4 Failure Recovery

If Z6G4 is unavailable for an extended period:

1. **Defer 24h/72h/168h soak** to a later window
2. **Do NOT cut v3.9.0-rc9** until soak completes
3. **Status**: RC8 = current best, GA blocked on Z6G4
4. **Documentation**: Update `GA_GATE_REPORT.md` with "Z6G4 unavailable" status

The RC8 tag can be cut based on local P0 completion (✅ done). GA tag requires Z6G4.

---

## 8. Contact / Handoff

- **Operator**: Z6G4 agent (or 250 admin)
- **Slack/IM**: (project-defined)
- **Issues to reference**: #3484, #3474, #3225, #3229, #3265, #3266
- **Branch**: `develop/v3.9.0` @ `784fe34447`
- **Target branch for merge**: `develop/v3.9.0` (already merged)

If you encounter issues not covered here, check:
- `docs/releases/v3.9.0/GA_GATE_REPORT.md` — overall GA status
- `docs/releases/v3.9.0/rc/RC8_GATE_REPORT.md` — RC8 details
- `docs/releases/v3.9.0/TEST_TRUTHFULNESS_REPORT.md` — V4/V6/V9 status
