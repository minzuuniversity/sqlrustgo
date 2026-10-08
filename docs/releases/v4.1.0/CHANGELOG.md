# v4.1.0 — CHANGELOG

> **Date**: 2026-09-29
> **Status**: DRAFT (per `docs/releases/v4.1.0/STAGE.yaml`)
> **Format**: v4.1.0-specific commits only; for v4.0.0 history, see `docs/releases/v4.0.0/CHANGELOG.md`

## [Unreleased] — DRAFT phase

### P0 修复 — 2026-10-08

- **#5099 并发事务静默丢行**（PR #5132）—— 🟡 **部分修复**。事务身份此前通过
  共享的 `current_tx_id` 槽位**隐式传递**，而该槽位位于所有连接共享的单个
  `FileStorage` 上，导致并发事务互相提交/回滚。修复前 8000 并发事务丢失
  **323 行（40.4%）**，且每次 `ROLLBACK` 都返回成功、无任何错误信号。
  **修复后丢行率降至 1% 量级，但 `ERROR 1062` 与偶发服务器卡死仍未解决，
  1h SOAK 仍无法跑完。** 见
  `evidence/CONCURRENT_TX_ROLLBACK_DATA_LOSS_5099_2026-10-08.md`。

  > 更正：PR #5132 的初版描述称「0 丢行、5/5 轮全 0」，该结论不成立 ——
  > 同一二进制复跑即复现错误。缺陷以高波动为特征（修复前丢行率在
  > 0.5%~54% 间跳），「若干轮全绿」不构成充分证据。

- **#5099 服务器卡死（TLS 读路径空转）** —— ✅ **已修复**。`impl Read for
  TlsStream` 的 `while self.conn.wants_read()` 循环在 socket 无数据时
  `break`，但 `wants_read()` 在该状态下**仍为 true**（它表示「rustls 想读」，
  不表示「此刻有数据」）。`read_exact` 重入后零进展地忙循环，每 worker
  100% CPU —— 实测 **1334%**。确定性复现：连上只发 1 个字节（半个包头）后停住。
  修复用 `complete_io` 的 `rdlen` 区分「无数据」（`Ok(0)` 会被 `read_exact`
  当作 EOF 而误断连接，不可采用）。

  **1h SOAK 跑满 3602s：0 FATAL / 45609 事务 / 912180 查询 / 0 errors /
  0 reconnects**，CPU 普查 237 样本最高 **178%**（空转会 >1000%），
  47 次 `SELECT 1` 探针全通。并发普查固定 seed **20 次重复 0 丢失**
  （200 行与 1000 行两档，后者 80,000 事务），且 oracle 经反向对照验证
  （主动删 37 行，精确报出 37）。见
  `evidence/TLS_READ_SPIN_HANG_5099_2026-10-09.md`。

- **#5099 顺带发现：`COUNT(*)` 与范围扫描返回 0（P0，待独立定位）** ——
  `SELECT COUNT(*)` / 范围条件 / `SUM` 聚合一律返回 0 或 NULL，**点查正常**。
  **不是丢行**（10000 行表 13 个抽样 id 全部命中，差额在 `.delta` insert
  buffer 内）。用 pristine 二进制复现结果完全相同，**与 TLS 修复无关**。
  影响：任何依赖计数返回值的下游会得到静默错误结论，本次所有行数统计均改用
  逐 id 点查。详见 `evidence/TLS_READ_SPIN_HANG_5099_2026-10-09.md` §6。

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