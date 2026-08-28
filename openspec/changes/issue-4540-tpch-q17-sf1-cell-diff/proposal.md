# Proposal: TPC-H Q17 SF=1 cell-diff on CI/Z6G4 (verify elapsed ≤ 300s)

## Why

Issue #4432 (TPC-H SF=1 Q17 >300s) acceptance criteria #1 second half
(`elapsed <= 300s` on SF=1 dataset) is unverifiable on the current
developer machine because the `/tmp/tpch-sf1/*.tbl` fixture is absent.
This is a recurring blocker noted in
`evidence/v312-58/issue-4379-sprint4-step15-residual-has-subquery.md`
§"Verdict" 2026-08-25.

The decorrelation wire-up itself is already shipped to `develop/v3.12.0`
(commit `d705176ef`, see #4432 status comment 97763 / 97831) — the
remaining gap is **runtime evidence on a full SF=1 lineitem corpus**.

This change delivers the runbook + verification evidence for the
acceptance criteria, so that #4432 can close and #4502 (GA-5 TPC-H SF=1
22/22 oracle match) can advance.

## What changes

1. **Dev-machine runbook** (`openspec/changes/issue-4540-.../runbook.md`):
   step-by-step commands to generate SF=1 fixture, run
   `scripts/tpch_sf1_baseline.sh --sf1-dir /tmp/tpch-sf1`, capture
   Q17 elapsed/row-count/sha256 evidence.
2. **CI/Z6G4 runbook** (same file §3): alternate execution path on
   the production-aligned Docker runner where `devstack-gitea-1` is
   hosted.
3. **Evidence artifact**: write the captured cell-diff to
   `evidence/v312-58/issue-4540-sf1-cell-diff.md` per ADR-001 G-04.
4. **Issue closure**: once Q17 ≤ 300s on SF=1, post the report to
   #4432 and request closure via Gitea API.

## Impact

- **#4432** (TPC-H SF=1 Q17 perf): unblocked — closure boundary met.
- **#4502** (GA-5 TPC-H SF=1 22/22 oracle match): unblocked after
  #4540 evidence delivered.
- **#4497** (V312-59-D v2 umbrella): one less GA-N gate blocker.

## Non-goals

- Decorrelation correctness (already shipped in commit `d705176ef`,
  see #4432 status). This change only delivers the **runtime
  evidence** for the perf acceptance.
- Cross-engine (sqlrustgo vs MariaDB vs SQLite) — gated by #3474
  (external mysql client reliability), not on critical path.

## Risk

- **Local-machine timing**: SF=1 lineitem = 6M rows; tpch_data_gen
  in-process may take 20-40 min for generation; Q17 query up to 300s.
  Total wall-clock budget ~1h on a typical dev laptop. CI/Z6G4 has
  more RAM and faster disk.
- **Disk**: ~1.2 GB at `/tmp/tpch-sf1/*.tbl`. Project root
  `docs/TPC-H-FIXTURE-DATA-HANDLING.md` forbids committing fixtures
  to git (LFS not enabled on 252 Gitea). Use local `/tmp` only.
- **Q17 result variance**: even with the decorrelation fix, Q17
  elapsed may still exceed 300s on a cold disk-cache. Mitigation:
  warm file_storage cache once before measuring.

## Stage gate

- Current: **RC (2026-08-26, post PR #4483)**
- This change only produces **runtime evidence + runbook** — no
  production code change. RC stage permits documentation / evidence
  capture. No new feature, no API change.

## Provenance (ADR-014 5 evidence fields)

| 字段 | 值 |
|------|---|
| source_agent | claude-sonnet (Claude Code) |
| source_run | issue-4540-tpch-q17-sf1-cell-diff-20260827 |
| timestamp | 2026-08-27T22:00:00+08:00 |
| evidence_hash | local-git:`14f638d09` (post-merge of PR #4524 docs-unify) |
| conflict_resolution | N/A — single AI scope |

Refs: #4432 (Q17 perf), #4502 (GA-5 TPC-H SF=1), #4497 (umbrella),
#4426 (v3.13 decorrelation master), #3474 (mysql-client reliability).