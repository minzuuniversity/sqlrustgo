# v4.1.0 — CHANGELOG

> **Date**: 2026-09-29
> **Status**: DRAFT (per `docs/releases/v4.1.0/STAGE.yaml`)
> **Format**: v4.1.0-specific commits only; for v4.0.0 history, see `docs/releases/v4.0.0/CHANGELOG.md`

## [Unreleased] — DRAFT phase

### Phase 0 (DRAFT doc scaffolding) — 2026-09-29

- `STAGE.yaml` — initial DRAFT entry
- `VERSION_PLAN.md` — v4.1.0 deltas over v4.0.0
- `DEV_PLAN.md` — v4.1.0 dev workflow
- `ROADMAP.md` — v4.1.0 milestones
- `TEST_PLAN.md` — v4.1.0 test scope
- `ISSUES_PLAN.md` — v4.1.0 issue catalog
- `LEGACY_ISSUES.md` — v4.1.0 inherited + new legacy items
- `README.md` — v4.1.0 entry point

### Phase 0/1 (5-remote sync infrastructure) — 2026-09-20..2026-09-28

- `scripts/sync/5remotes_sync.sh` — push a ref to all 5 remotes; uses
  SSH container `git update-ref` for Gitea protected branches
- `scripts/sync/5remotes_drift_check.sh` — read-only TSV output for
  cron / log scraping
- `scripts/sync/README.md` — operator documentation

### Review queue closure — 2026-09-22..2026-09-26

#### Empty-merge graph repair (2026-09-22)

- `fca71cb525` chore(v4.1.0): bring v4.0.0 commit 6d504d1b3c (public mirror cleanup) into v4.1.0 graph
- `a65497e276` chore(v4.1.0): bring v4.0.0 merge 1d2588d937 (gitea250 SYNC_AUDIT final) into v4.1.0 graph
- `7e30fbc0cb` chore(v4.1.0): bring v4.0.0 merge f350eb13a5 (gitea252 SYNC_AUDIT final sync) into v4.1.0 graph
- `2bd69b223f` docs(v4.1.0): V400_TO_V410_REVIEW_QUEUE — 4 commits awaiting manual review

#### Bugfix cherry-picks (2026-09-23)

- `6603820f1e` fix(v4.0.0 alpha gate): executor BINARY collation, admin Windows compat, remove tpch_hash_test
- `daad2c687d` fix(storage): checkpoint JSON escapes Windows paths; recovery tolerates unknown prefix as NULL
- `82772919e2` fix: cross-platform /proc and filename compatibility for Windows

#### Sync audit + queue closure (2026-09-23..2026-09-26)

- `136f232643` docs(v4.0.0): SYNC_AUDIT_v4.1.0_2026-09-20 — 5-remote convergence report
- `9c6767a512` merge: bring v4.0.0 Windows compat + BINARY collation review-queue commits into v4.1.0
- `0bbb044da3` docs(v4.1.0): close V400_TO_V410_REVIEW_QUEUE — all 4 commits resolved

### Bugfix carry-forward — 2026-09-21

- `f3595e7361` v4.0.0 / tests: fix 4 pre-existing DML/storage regressions
- `40e07f7342` Merge pull request 'v4.0.0 / tests: fix 4 pre-existing DML/storage regressions (port from 4.0.0)' (#4905)
- `17b459969f` Merge pull request 'v4.0.0 / tests: fix 4 pre-existing DML/storage regressions (port from 4.0.0)' (#3792)
- `b4d46e6e18` fix(v4.1.0): restore workers.push wrapper in ServerThreadPool::start
- `7502ee4cd4` Merge pull request 'fix(v4.1.0): restore workers.push wrapper in ServerThreadPool::start (follow-up to PR #4905)' (#4906)
- `d2e3a3bf98` Merge pull request 'fix(v4.1.0): restore workers.push wrapper in ServerThreadPool::start (follow-up to PR #4905)' (#3793)
- `67b624cbd2` v4.1.0 / zombie-fix core: apply non-graph portion of bulk-insert + DLM fix
- `d6dd4fab28` Merge pull request 'v4.1.0 / zombie-fix core: apply non-graph portion of bulk-insert + DLM fix' (#3794)

### v4.0.0 GA gate (parent commit) — 2026-09-20

- `2e1f9bd44d` docs(v4.0.0-ga): GA_GATE_REPORT.md FINAL — promotes v4.0.0 to GA
- `cceba7f330` docs(v4.0.0-ga): TAG_PROTECTION_v4.0.0.md — branch pinning strategy
- `3005413953` docs(v4.0.0): FORCE_PUSH_AUDIT_2026-09-19 — 5 force-pushes, no content loss
- `38566af0f8` docs(v4.0.0): update SYNC_AUDIT with final convergence (3 PRs + 1 push)

## Status

- [ ] v4.1.0-alpha1 tag (blocked by 3 inherited alpha-gate FAILs)
- [ ] v4.1.0-beta1 tag
- [ ] v4.1.0-rc1 tag
- [ ] v4.1.0-ga tag

## References

- `docs/releases/v4.0.0/CHANGELOG.md` — v4.0.0 history
- `docs/releases/v4.1.0/STAGE.yaml` — stage progression
- `docs/releases/v4.1.0/ISSUES_PLAN.md` — issue catalog
- `scripts/sync/README.md` — 5-remote sync tooling