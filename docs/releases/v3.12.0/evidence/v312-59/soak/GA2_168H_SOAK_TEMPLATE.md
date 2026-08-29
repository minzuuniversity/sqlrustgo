[agent: claude-code] 168h SOAK COMPLETION TEMPLATE — Issue #4499 (placeholder).

> **status**: ⚠️ **DEPRECATED for Run #5685** (2026-08-30) — Run #5685 failed at step 0 (Checkout) due to act_runner egress firewall blocking github.com. FAIL-mode verdict posted to #4499 as comment 98718 (2026-08-29T19:05:04Z). Template retained for next dispatch attempt after CI infrastructure restored. **Do not publish as a PASS-mode completion report.**
>
> PRE-FILLED TEMPLATE — fields marked `<<>>` will be populated when a fresh workflow run reaches T+168h completion.

## Provenance (ADR-014 5 evidence fields)

| Field | Value |
|---|---|
| source_agent | claude-sonnet (Claude Code) |
| source_run | issue-4499-168h-soak-completion-<<RUN_ID>> |
| timestamp | <<ISO_TIMESTAMP>> (T+168h from dispatch 2026-08-29T07:24:42Z) |
| evidence_hash | git:HEAD=<<FINAL_HEAD_SHA>> on develop/v3.12.0; artifact SHA256=<<METRICS_CSV_SHA256>>; binary SHA256=<<RELEASE_BINARY_SHA256>> |
| conflict_resolution | <<N/A or note any concurrent PRs landed during the run window>> |

## Run summary

* **Workflow run ID**: <<RUN_ID>> (http://192.168.0.252:3000/openclaw/sqlrustgo/actions/runs/<<RUN_ID>>)
* **Started**: 2026-08-29T07:24:42Z (= 15:24:42 CST)
* **Completed**: <<ISO_TIMESTAMP>> (target 2026-09-05T07:24Z)
* **Duration**: <<HOURS>>h <<MINUTES>>m
* **Final status**: <<success / failure / cancelled>>
* **Runner**: z6g4-runner-demo (id=1721, host HP Z6 G4 = gaoyuanai-HPZ6G4)

## Metrics summary (168h aggregate)

| Metric | Value | Threshold | Verdict |
|---|---|---|---|
| Total sysbench transactions | <<N>> | n/a | n/a |
| Avg QPS | <<QPS>> | n/a | n/a |
| Avg p95 latency (ms) | <<LAT_MS>> | n/a | n/a |
| Peak RSS (MB) | <<PEAK_RSS>> | ≤ 800 | <<PASS/FAIL>> |
| WAL disk peak (MB) | <<WAL_PEAK>> | ≤ 800 | <<PASS/FAIL>> |
| Total restarts | <<N_RESTARTS>> | = 0 | <<PASS/FAIL>> |
| Total FATAL events | <<N_FATAL>> | = 0 | <<PASS/FAIL>> |
| Total sysbench errors | <<N_ERR>> | = 0 | <<PASS/FAIL>> |
| Total reconnects | <<N_RECONN>> | = 0 | <<PASS/FAIL>> |

## Hourly trend (sampled at 24h / 48h / 72h / 96h / 120h / 144h / 168h)

| Hour | QPS | p95 (ms) | RSS (MB) | WAL (MB) | restart | err |
|---|---|---|---|---|---|---|
| T+24h | <<>> | <<>> | <<>> | <<>> | <<>> | <<>> |
| T+48h | <<>> | <<>> | <<>> | <<>> | <<>> | <<>> |
| T+72h | <<>> | <<>> | <<>> | <<>> | <<>> | <<>> |
| T+96h | <<>> | <<>> | <<>> | <<>> | <<>> | <<>> |
| T+120h | <<>> | <<>> | <<>> | <<>> | <<>> | <<>> |
| T+144h | <<>> | <<>> | <<>> | <<>> | <<>> | <<>> |
| T+168h | <<>> | <<>> | <<>> | <<>> | <<>> | <<>> |

## Artifacts (per workflow `actions/upload-artifact@v4`)

* `results/soak-168h/metrics.csv` (full 168h time-series)
* `results/soak-168h/server.log` (full server log)
* `results/soak-168h/snap_*.json` (5-minute snapshots)
* `docs/releases/v3.12.0/evidence/v312-59/soak/GA2_168H_SOAK_REPORT.md` (workflow-generated scaffold)

Artifact retention: 30 days from upload.

## Verdict

* **168h SOAK run**: <<PASS / FAIL / MIXED>>
* **GA-2 evidence**: <<sufficient / insufficient — see §notes>>
* **GA promotion recommendation**: <<recommended / hold — see §notes>>

## Anti-Fabrication-Policy-v1.0 compliance

* All SHAs cited (no estimated SHAs).
* All verdict lines preceded by evidence reference.
* Any discrepancies between observed vs threshold disclosed.
* If verdict = PASS, includes reference to PR-merged CI gate evidence for the same develop/v3.12.0 head.

---

This is a TEMPLATE. Will be overwritten with real values when run #5685 reaches completion. Anti-Fabrication-Policy-v1.0 forbids publishing a partially-completed completion report.