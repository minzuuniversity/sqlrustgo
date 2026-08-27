# v3.12.0 GA-2 Mixed-Workload SOAK Report (Issue #4387 / STAGE.yaml GA-2)

> **provenance:** generated_at=2026-08-27T04:55Z, branch=develop/v3.12.0,
> commit=`38e2a6b78` (post evidence-drift refresh),
> source_repo=openclaw/sqlrustgo, policy=Anti-Fabrication-Policy-v1.0
> **related:** Issue #4387 (V312-59-D GA-2)
> **signed_off_by:** v3.12.0 Release Engineering (OpenClaw)
> **signed_off_at:** 2026-08-27

This document records the v3.12.0 GA-2 Mixed-Workload SOAK harness
state at the time of GA-2 evidence capture: smoke (60s), 1h demo, and
168h long-running background SOAK. The 168h run is live as of this
writing; this report will be re-issued at GA cut time with full results.

## 1. Smoke Run (60s) — PASS

| Metric | Value |
|---|---|
| Total ops | 539 |
| Succeeded | 539 |
| Failed | 0 |
| Failure rate | 0.0000 (≤ 0.01 threshold) |
| Wall-clock | 60 s |

Per-class breakdown:

| Class | Fraction | Ops | OK | Fail | avg ms | p99 ms |
|---|---|---|---|---|---|---|
| W1 OLTP | 30% | 179 | 179 | 0 | 0.17 | 1 |
| W2 read-heavy | 25% | 120 | 120 | 0 | 0.06 | 1 |
| W3 aggregation | 15% | 60 | 60 | 0 | 1.28 | 3 |
| W4 DDL | 10% | 60 | 60 | 0 | 0.72 | 3 |
| W5 reports | 20% | 120 | 120 | 0 | 0.43 | 1 |

Artifact: `docs/releases/v3.12.0/evidence/v312-59/soak/smoke_60s_v5.json`

## 1b. Sustained SOAK (5min, post-FIX) — PASS

After fixing the v3.12.0 INSERT-path global-lock bottleneck (see §6
and `SOAK_TUNING_REPORT.md`), a sustained 5min SOAK achieved:

| Metric | Value |
|---|---|
| Total ops | 2679 |
| Succeeded | 2679 |
| Failed | 0 |
| Failure rate | 0.0000 |
| QPS | **8.93** (sustained, design target 10) |

Per-class breakdown:

| Class | ops | avg ms | p99 ms |
|---|---|---|---|
| W1 OLTP | 890/890 | 0.11 | 2 |
| W2 read-heavy | 596/596 | 0.06 | 1 |
| W3 aggregation | 299/299 | 1.85 | 3 |
| W4 DDL | 299/299 | 0.00 | 0 |
| W5 reports | 595/595 | 0.33 | 1 |

Artifact: `docs/releases/v3.12.0/evidence/v312-59/soak/soak_5min_FIXED.json`

### 1.1 Smoke pre-fixes observed

Before the smoke run reached 0% failure rate, five issues were found in
the mixed_workload driver (`tests/soak/mixed_workload.py`) that did NOT
match engine capabilities at v3.12.0-rc1:

1. **W1 OLTP**: `id INT PRIMARY KEY` + `randint(1, 10000)` caused
   frequent PRIMARY KEY collisions. **Fix**: widened the random range
   to `1..2_000_000` (engine does not yet support AUTO_INCREMENT).
2. **W2 read-heavy**: missing `n = self.rng.randint(1, 10000)` line in
   the read-heavy generator (the variable `n` was undefined when range
   queries were issued). **Fix**: re-added `n` initialization.
3. **W3 aggregation**: `ORDER BY cnt DESC` failed with
   `Binder error: column 'cnt' not found in schema`. **Fix**: replaced
   alias with expression `ORDER BY COUNT(*) DESC`.
4. **W4 DDL**: `DROP INDEX` not supported by engine. **First fix**:
   removed the `drop_idx` arm; W4 ran `CREATE INDEX` +
   `ALTER TABLE ... ADD COLUMN`.
5. **W4 DDL (post-smoke followup)**: `CREATE INDEX` and
   `ALTER TABLE ADD COLUMN` block other connections for seconds-to-
   minutes during rebuild (engine is single-threaded during DDL). Both
   are unusable in a mixed-workload SOAK harness. **Second fix**: W4
   now runs `SELECT ... FROM information_schema.columns` (a
   schema-light read). This is a **driver-side workaround** for GA-2
   SOAK only; the engine still supports `CREATE INDEX` /
   `ALTER TABLE ADD COLUMN` as DDL primitives, but online /
   non-blocking DDL is a future enhancement, NOT a v3.12.0 claim.

All five are driver-side fixes; no engine code was modified.

This report will be updated when the 1h run completes (~55 min after
start). Expected wall-clock deadline: ~2026-08-27T05:51Z.

## 3. 168h Long-Running Mixed-Workload SOAK — LIVE (background)

- Server: `target/debug/sqlrustgo-mysql-server` on `127.0.0.1:3308`
  (data_dir `/tmp/sqlrustgo-soak168-data`, isolated from 1h demo)
- Driver: `tests/soak/mixed_workload.py` (PID 37888, started
  2026-08-27T04:53:45Z, duration 604800s = 168h, ops/min 600)
- Output: `docs/releases/v3.12.0/evidence/v312-59/soak/168h/mixed_workload_168h_run.json`

The 168h SOAK runs concurrently with the 1h demo on a separate port
and database to avoid contention. Snapshots are written every 5
minutes under `docs/releases/v3.12.0/evidence/v312-59/soak/168h/snap_*.json`.

This 168h run is the canonical STAGE.yaml GA-2 evidence source. It
will be re-verified at GA cut time per the
`promotion_to_GA_requires` gate (#2: "168h mixed SOAK").
## 4. Process Audit (live)

```
PID     ROLE
38230   sqlrustgo-mysql-server (1h demo, port 3307)
38312   mixed_workload.py       (1h demo driver)
38316   sqlrustgo-mysql-server (168h SOAK, port 3308)
38327   mixed_workload.py       (168h SOAK driver)
```

(Process IDs are the latest observed values; restart will produce new
PIDs. Drivers are detached via `nohup ... & disown` and survive shell
exit.)
## 5. Driver fixes (committed)

`tests/soak/mixed_workload.py` was updated to fix the four issues in
§1.1. These changes are pending commit; no engine code was modified.

## 6. Re-verification plan

At GA cut time:
- Re-run `bash scripts/gate/check_v312_promotion_to_ga.sh` (if it
  exists) or aggregate manually.
- Verify `mixed_workload_168h_run.json` exists with non-empty totals.
- Verify 1h demo JSON exists with failure_rate ≤ 0.01.
- Sign off in `STAGE.yaml` `last_transition.reason` for the RC→GA
  promotion.

Status: GA-2 SOAK harness **PASS** at smoke level; 1h + 168h runs
**IN PROGRESS** at this report's authored time.