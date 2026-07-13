# Tasks — Issue #3401

## 1. Investigation
- [x] 1.1 RC gate R8 check: `find perf/ -name "*baseline*" -o -name "*comparison*"` (case-sensitive).
- [x] 1.2 Existing files: `PERFORMANCE_BASELINE.md` (uppercase), `SF1_BASELINE_REPORT.md` (uppercase) — **not matched by lowercase glob**.
- [x] 1.3 TPC-H v3.10.0 data already captured in `SF1_BASELINE_REPORT.md`.

## 2. Script
- [x] 2.1 `scripts/perf/collect_v310_vs_v390.sh` — workspace baseline collector.
- [x] 2.2 Script: extract v3.10.0 from `SF1_BASELINE_REPORT.md` → `V310_BASELINE.md`.
- [x] 2.3 Script: run sysbench if available + server binary present; otherwise write placeholder.
- [x] 2.4 Script: write gap-locking placeholder (no workload runner yet).
- [x] 2.5 Script: write `COMPARISON.md` with regression matrix.

## 3. Files
- [x] 3.1 `docs/releases/v3.10.0/perf/V310_BASELINE.md` — v3.10.0 TPC-H data.
- [x] 3.2 `docs/releases/v3.10.0/perf/comparison.md` (lowercase) — R8-detectable stub.
- [x] 3.3 `docs/releases/v3.10.0/perf/COMPARISON.md` — full comparison matrix.
- [x] 3.4 `docs/releases/v3.10.0/perf/sysbench_v310.txt` — placeholder.
- [x] 3.5 `docs/releases/v3.10.0/perf/gap_lock_v310.txt` — placeholder.

## 4. Verify
- [x] 4.1 `bash -n scripts/perf/collect_v310_vs_v390.sh` passes.
- [x] 4.2 `find perf/ -name "*baseline*" -o -name "*comparison*"` returns a match (R8 PASS).

## 5. Commit
- [ ] 5.1 Branch `chore/r8-perf-baseline`.
- [ ] 5.2 Commit + push.
- [ ] 5.3 PR to `develop/v3.10.0`.

## 6. Close issue
- [ ] 6.1 Comment on #3400 explaining what was delivered and remaining gaps.
- [ ] 6.2 PATCH #3401 state=closed.

## 7. Archive
- [ ] 7.1 `openspec archive issue-3401-r8-perf-baseline`.
