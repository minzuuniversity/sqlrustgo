# v3.12.0 GA-2 168h SOAK — Z6G4 Operator Runbook

> **provenance**: generated 2026-08-28 by claude-macmini, post-#4563 merge
> **status**: READY TO RUN (Dockerfile.soak + run_168h_ci.sh fixed)
> **preconditions**: Z6G4 Docker host with 80 cores / 231 GB free disk

## Goal

Run `scripts/soak/Dockerfile.soak` on Z6G4 for 168 hours against the v3.12.0
release binary, producing `SOAK_168H_REPORT.md` for the v3.12.0 GA-2 gate.

## Quick start

```bash
# On Z6G4 (Linux x86_64, 80 cores):

# 1. Clone (or rsync) the workspace at the GA-candidate commit
git clone https://gitea-macmini:openclaw/sqlrustgo.git
cd sqlrustgo
git checkout ca5649d42   # or the latest pre-GA commit
# (Currently post-#4563 merge, HEAD = 4d8e8d44e7 — use that)

# 2. Build the Docker image
docker build -f scripts/soak/Dockerfile.soak -t sqlrustgo-soak:ga2 .

# 3. Launch 168h SOAK in background (logs persist via volume mount)
mkdir -p /tmp/sqlrustgo-soak-data /tmp/sqlrustgo-soak-results
docker run --rm --detach --name sqlrustgo-soak \
  -p 3396:3396 -p 9300:9300 \
  -v /tmp/sqlrustgo-soak-data:/data \
  -v /tmp/sqlrustgo-soak-results:/results \
  -e SOAK_HOURS=168 \
  sqlrustgo-soak:ga2

# 4. Tail logs
docker logs -f sqlrustgo-soak

# 5. Prometheus metrics scrape
curl http://localhost:9300/metrics
```

## Expected timeline (168h = 7 days)

| Day | Event | Action |
|---|---|---|
| 0 (T+0) | Container start; server ready in ~2s | tail docker logs |
| 0 (T+1m) | Schema + sysbench prepare complete | verify `sysbench_prepare.log` |
| 0 (T+2m) | Sysbench run starts (8 threads, --db-ps-mode=disable) | verify QPS > 0 |
| 1-7 | Sample every 60s; snapshot every 6h; status print every hour | monitor metrics.csv |
| 7 (T+168h) | SOAK wall time elapsed; sysbench killed INT | read SOAK_168H_REPORT.md |

## Key changes from prior version

| Issue | Old behavior | New behavior |
|---|---|---|
| `soak` subcommand missing | Script called `$BINARY soak` (no-op) | Uses `mysql` CLI for probes + sysbench for load |
| TLS handshake 4/8 stall | Default `--db-ps-mode=auto` (sysbench prepared statements) | `--db-ps-mode=disable` — uses plain COM_QUERY |
| Sysbench not part of 168h loop | Only `soak` subcommand looped | sysbench runs full 168h; metrics.csv tracks both server + sysbench liveness |
| No docker image | Plain bash script, required mysql-client on host | Dockerfile.soak installs sysbench + mysql-client in Alpine image |

## Exit criteria for GA-2 gate

1. **168h wall time elapsed** without `SERVER DIED` event in `errors.log`
2. **`QUERIES_OK` (probes) ≥ 8000** out of 10080 (1/min × 168h = 10080) — 80% probe success rate
3. **Sysbench QPS > 0 sustained** for ≥ 90% of the 168h duration
4. **RSS peak < 4 GB** (buffer pool + WAL + catalog)
5. **No FD exhaustion** (peak FD < ulimit -n / 2)
6. **WAL size ≤ 10 GB at any snapshot** (auto-cycling should keep this bounded)
7. **Zero OOM / panic / core dump** events in `server.log`

When all 7 met, copy `SOAK_168H_REPORT.md` + `metrics.csv` + `server.log`
to `docs/releases/v3.12.0/evidence/v312-59/soak/` and file a PR titled
`docs(v312-59 / #4499): GA-2 168h SOAK evidence — Z6G4 run`.

## Anti-deferral guard

Per Anti-Fabrication-Policy-v1.0:
- Do NOT report PASS without the 7 criteria met
- Do NOT re-run on a different host and report aggregate
- Do NOT claim "infrastructure PASS" as "SOAK PASS"

The 1h local smoke (via `scripts/soak/run_soak_loop.sh`) is a separate
non-GA-gate exercise; only the Z6G4 168h run satisfies GA-2.

## Related

- issue #4499 — GA-2 umbrella
- issue #4497 — V312-59-D v2 promotion cycle
- issue #4560 — reopened with corrected diagnosis (4/8 TLS-handshake stall)
- issue #4564 — independent investigation ticket for the 4/8 stall root cause
- PR #4563 — corrected GA_GATE_REPORT GA-2 row
- scripts/soak/run_soak_loop.sh — local 1h smoke harness
- scripts/soak/run_168h_ci.sh — this runbook's Docker entrypoint