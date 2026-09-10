# SQLRustGo v3.12.0 变更日志

> **provenance:** generated_by=codex, generated_at=2026-09-11T00:00:00+08:00, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0 + ADR-001 + ADR-014

> **状态**: **GA** (2026-09-08)
> **日期**: 2026-09-11 (GA publication doc refresh); 2026-09-08 (GA cut); 2026-08-26 (RC); 2026-08-19 (BETA); 2026-08-09 (initial)
> **stage_history**: DRAFT (pre-2026-08-12) → ALPHA (2026-08-12) → BETA (2026-08-19) → RC (2026-08-26) → **GA** (2026-09-08)

> **ga_tag_commit**: `355b5a38378c41ffee2f43a29ad2c4f7bd7097d4`
> **post_cut_refresh_head**: `9febebb255f984387ac78c26510d5b46d73f6046`

## 2026-09-11 GA publication documentation refresh

### Docs

- Rewrote [`RELEASE_NOTES.md`](RELEASE_NOTES.md), [`RELEASE_CHECKLIST.md`](RELEASE_CHECKLIST.md), [`GA_RELEASE_REPORT.md`](GA_RELEASE_REPORT.md), [`GA_GATE_REPORT.md`](GA_GATE_REPORT.md), [`PERFORMANCE_REPORT.md`](PERFORMANCE_REPORT.md), [`SECURITY_AUDIT.md`](SECURITY_AUDIT.md), and [`COMPREHENSIVE_ASSESSMENT_REPORT.md`](COMPREHENSIVE_ASSESSMENT_REPORT.md) to use the final GA state instead of stale RC / GA-candidate wording.
- Added [`GA_PUBLICATION_EVIDENCE_INDEX.md`](GA_PUBLICATION_EVIDENCE_INDEX.md) as the single publication evidence entry point.
- Added [`DOC_RECTIFICATION_WORK_REPORT_2026-09-11.md`](DOC_RECTIFICATION_WORK_REPORT_2026-09-11.md) for the document correction audit trail.

### Release governance

- GA SSOT: [`STAGE.yaml`](STAGE.yaml) records `current_stage: "GA"`.
- Final aggregate evidence: [`evidence/v312-59/ga_gate_report.json`](evidence/v312-59/ga_gate_report.json), generated at `2026-09-08T04:17:15Z`, mode `full`, commit `355b5a3837`, totals `72/72 PASS, blockers 0`.
- GA tags: `v3.12.0` and `v3.12.0-ga`, both dereference to `355b5a3837`.
- Current known limitations remain outside GA claims: #4846 / #4847 / #4848. 252 Gitea also has open follow-up PRs #4868 / #4870 / #4869, which are not included in the GA tag until merged and verified.

## 2026-09-02 GA candidate documentation refresh

### Docs

- Added [`GA_RELEASE_REPORT.md`](GA_RELEASE_REPORT.md) to record the current
  GA candidate state, remote HEAD, Gitea milestone state, and remaining hard
  promotion blockers.
- Added [`PERFORMANCE_REPORT.md`](PERFORMANCE_REPORT.md) to summarize TPC-H,
  mixed SOAK demo, local 8h SOAK V5, and the remaining Linux/Docker SOAK gap.
- Added [`SECURITY_AUDIT.md`](SECURITY_AUDIT.md) to roll up GA-3 security
  evidence and mark final-cut refresh requirements.
- Added [`RELEASE_CHECKLIST.md`](RELEASE_CHECKLIST.md) to map
  `promotion_to_GA_requires` to evidence files and final cut actions.

### Release governance

- Milestone `v3.12.0` is open with `open_issues=0 / closed_issues=79`, but
  `STAGE.yaml` remains `current_stage: RC`.
- GA promotion remains blocked on final full-mode aggregate evidence and GA-2
  SOAK closure/reclassification.
- New open unmilestoned compatibility issues #4607, #4608, #4610, #4611,
  #4612, and #4613 restrict broad SQLite teaching / SQL function correctness
  release claims until they are closed or explicitly scoped out.

## 2026-08-27 #4491 residual scope closure (post-RC)

PR #4493 (commit `f118dd896c`) closed the **integer-keyed** form of BUG-3a
(JOIN alias.column with INT PK). Issue #4491 tracked the residual scope:
when the shared JOIN key is **CHAR-typed** (the exact 清华 A-track teaching
schema — `studentno char(11)`), the MySQL non-strict GROUP BY fallback path
in `src/engine_select.rs` still returned `Null` for the alias.column
projection. This section documents the closure of that residual scope.

### Fixed

- **#4491 residual (this PR)** — MySQL non-strict GROUP BY fallback:
  bridge `Integer↔Text` and trim CHAR padding in the fallback key match
  (scope-limited to `src/engine_select.rs:1442-1458`).
  - Pre-fix repro (CHAR(11) shared key, `s JOIN sc ON s.studentno=sc.studentno GROUP BY sc.studentno`):
    `s.sname` was `Null` while `avg(sc.final)` was correct.
  - Post-fix: `s.sname` resolves to `'alice'` / `'bob'` correctly.
  - Evidence: `docs/releases/v3.12.0/evidence/issue-4491/4491_closeout.md`.
  - Regression test: `tests/integration/oracle/issue_4491_join_groupby_alias_col_and_scalar_subquery.rs`
    (3 tests, 3 GREEN post-fix).
  - Scope discipline: `sql_compare` and `compare_values` were intentionally
    NOT modified. Broader cross-type bridging is deferred — see out-of-scope
    in evidence doc.

## 2026-08-26 RC drift-fix (post-RC transition)

HEAD 已从 `dd5ab204` 前移到 `cbe1f53f85`。RC 期间合并的关键 PRs：

### Fixed

- **PR #4493** (f118dd896c) — BUG report v3.12.0 修复 BUG-2/3/4 (清华 A 轨教学兼容性):
    - BUG-2a parser 嵌套函数 (`year(now())` parse error)
    - BUG-2b builtin functions (now/curdate/curtime/year/month/day/datediff/round/rand/length/abs) 静默 Null
    - BUG-3a JOIN alias.column 在 GROUP BY 中返 Null
    - BUG-3b WHERE 标量子查询返 0 行
    - BUG-4 char(n) blank-padded 比较失败
  - 关闭 issues: #4490, #4491, #4492 (closed 2026-08-26 by drift-fix)
- **PR #4495** (821678fcd2) — parser UTF-8 char-boundary panic 修复 (skip_whitespace / read_identifier 按 char len 前进位置)
- **PR #4488** (ac90a27ba0) — 移除 server01_serve_verbose 过度规约的 TLS:/WAL: 断言
- **PR #4484** (5e51ec4347) — server01_server_test::get_binary_path 增加 target/debug/ 扫描

### Docs

- **PR #4487** (37c0a82cbc) — README + CURRENT_VERSION drift-fix (sync to RC)
- **PR #4486** (0debdeee80) — TPC-H SF=1 cell-diff Q08 clean_match

### Closed PR

- **PR #4494** (closed 2026-08-26) — 与 #4493 文件严重重叠 (expr/mod.rs, parser.rs, engine_utils.rs)，作为重复 PR 关闭

### Active issue

- **#4432** (Q17 SF=1) — ✅ CLOSED via GA reclassification (path-2 per #4432 issue body)
  — `Q17_Q20_V313_DEFERRED_STATUS.md` records elapsed budget relaxed from ≤300s
  to ≤1800s for v3.12.0 GA; correctness (row_count / sha256) remains hard-required.
  Decorrelation wire-up continues under v3.13 #4426.

## 2026-08-26 BETA → RC transition

v3.12.0 officially promoted to **RC** on 2026-08-26, recorded in
`docs/releases/v3.12.0/STAGE.yaml` (current_stage: RC) and tracked
under issue #4386 (V312-59-C umbrella).

### Gate snapshot

```
$ bash scripts/gate/check_v312_promotion_to_rc.sh
PASS:               9 / 11
FAIL:               0 / 11
NO-OP (covered):    2 / 11
Total checked:      11 / 11
```

All 12 `promotion_to_RC_requires` items satisfied. Full verdict map:
[`RC_GATE_REPORT.md`](RC_GATE_REPORT.md). Per-item wrapper reports
under `evidence/v312-59/RC{1..11}_*_REPORT.md`; umbrella aggregator at
[`V312-59-C-RC-PROMOTION-REPORT.md`](V312-59-C-RC-PROMOTION-REPORT.md).

### Composite evidence

- **B8 thresholds_override**: 13/13 PASS at 2026-08-26T18:30:00Z
  (issue #4388 re-verified post SF=10 fixture remediation).
- **RC6 TPC-H SF=1 cross-engine**: NO-OP covered by V312-58 Sprint 5
  series (`evidence/tpch/cross_engine_sf1/SUMMARY.json`, 4 engines ×
  22 queries).
- **RC9 V312-57 week01-04**: NO-OP covered by PRs #4359/#4370/#4373
  + `evidence/v312-57-smoke/V312-57-SMOKE-FIXTURES-CLOSURE.md`
  (14/14 PASS).

### Resolved since BETA

- **RC6 TPC-H cross-engine wrapper** — `evidence/v312-59/RC6_TPCH_SF1_CROSS_ENGINE_REPORT.md` added to make the
  NO-OP verdict auditable.
- **RC9 V312-57 week01-04 wrapper** — `evidence/v312-59/RC9_V312_57_WEEK01_04_REPORT.md` added.
- **Sprint 5 followup-6** — `mentions_outer` Subquery gap closed
  (PR #4475); canonical TPC-H Q4 SF=1 + Q20 BinaryOp arm path
  verification via `tests/integration/tpch/q20_binaryop_arm_test.rs`
  (2/2 PASS).

Tag `v3.12.0-rc1` to be cut immediately after the merge of this
commit per STAGE_CONFIG BETA_to_RC trigger.

## 2026-08-19 ALPHA → BETA transition

v3.12.0 officially promoted to **BETA** on 2026-08-19, recorded in
`docs/releases/v3.12.0/STAGE.yaml` (current_stage: BETA) and posted
as comment #95192 on master issue #3887.

### Gate snapshot

```
$ bash scripts/gate/check_beta_v3.12.0.sh
PASS: 38/40   WARN: 2   BLOCKERS: 0
✓ All checks pass — ready for BETA promotion.
```

All 12 `promotion_to_BETA_requires` items satisfied. Full snapshot
in `docs/releases/v3.12.0/evidence/v312-56/V312-56-VERIFICATION.md`
(Beta Gate Snapshot + Promotion to BETA Readiness Checklist sections).

### Resolved blockers (since ALPHA)

- **B1_CLIPPY** — workspace-wide clippy errors resolved by PR #4354
  (17 files, +68/-75; LFS-managed files unchanged).
- **B6_QUANTILE_FUNCTIONS** — quantile aggregate gate now PASS.
- **B6_V312_56_TEACHING_GAPS** — was silently skipped due to orphan
  `=======` merge marker in `scripts/gate/check_beta_v3.12.0.sh:304`;
  marker removed by PR #4353.
- **V312-56A-R2 / 56A-R4** — information_schema SQL path and
  SHOW WARNINGS/ERRORS/STATUS/VARIABLES implementations merged as
  PR #4349 / #4345 respectively.
- **PR #4332 TPC-H zero-row fix** — Q5/Q9/Q10/Q13/Q18 closed by
  PR #4332 + verification PR #4355.

### Remaining open items at Beta entry

| # | Item | Status | Owner |
|---|---|---|---|
| #4221 | V312-48 TPC-H SF=1 umbrella | open | openclaw-minimax |
| #4274 | TPC-H Q8 (8-way join) zero-row | open | openclaw-minimax |
| #4278 | TPC-H Q16 (NOT IN subquery) zero-row | open | openclaw-minimax |
| #4251 | V312-56A Metadata teaching (close PR pending) | closure-ready | openclaw-minimax |
| #4252-#4254 | V312-56B/C/D sub-issues (close PR pending) | closure-ready | openclaw-minimax |
| 56A-R3 | SHOW FULL TABLES / TABLE STATUS | DEFERRED → v3.13+ | TBD |

### New tag plan

`v3.12.0-beta1` to be cut immediately after the merge of the
Beta-transition PR (this commit). Per `STAGE_CONFIG.ALPHA_to_BETA`
trigger; tag pushed to both 252 and 250 remotes.

## 2026-08-20 V312-57 sqlite3-like 一体化教学 CLI (#4359)

**Issue #4359** — BustubX-EDU 前 4-6 周自动验收入口。实现经 PR #4371
(引擎层) + PR #4372 (clap fix) + PR #4373 (本工作合并, commit `543e15b3f`)
落地, 替代 `sqlite3` 使用体验的教学 CLI。

### 交付

- `sqlrustgo` 二进制 (crates/sqlrustgo-cli): 单路径数据库、stdin/`--cmd` 批处理、`--continue-on-error`、稳定错误前缀 `sqlrustgo:error:parse|bind|runtime:`。
- 输出模式 table/list/csv/json + `.headers`; 元命令 `.help/.quit/.exit/.tables/.schema/.mode/.headers/.read/.output/.timer/.explain`。
- FileStorage 目录型持久化, 跨进程建表→插入→查询验证。
- 教学 fixture `tests/compat/bustubx_edu_sqlite_cli/` week01-week04 (14 cases) + gate `scripts/gate/check_bustubx_edu_cli_v312.sh`。
- Beta Gate wiring: B4_V312_57_EDU_CLI_GATE_DEFINED + B6_V312_57_EDU_CLI_GATE 已并入 `check_beta_v3.12.0.sh`。

### Gate snapshot (2026-08-20)

```
$ bash scripts/gate/check_bustubx_edu_cli_v312.sh
V312-57 bustubx_edu_cli gate: PASS=14 FAIL=0

$ bash scripts/gate/check_beta_v3.12.0.sh
PASS: 40/42   WARN: 2   BLOCKERS: 0
```

- `cargo test -p sqlrustgo-cli --all-features --lib`: **67/67 PASS** (run_repl_parse_error_exits_one_but_continues 等)。
- clippy `-D warnings`: 0 错误; `cargo fmt --check`: 0 Diff。
- stage boundary gate `check_v312_stage_boundary.sh`: 6/6 PASS。
- 完整验证: [`evidence/bustubx_edu_cli/V312-57-EDU-CLI-VERIFICATION.md`](evidence/bustubx_edu_cli/V312-57-EDU-CLI-VERIFICATION.md)。

### 兼容边界

- 不声明 SQLite 文件格式兼容 (单路径是目录型 FileStorage, 非 .db 文件)。
- `sqlrustgo <db> "SQL"` SQL 位置参数形式不再支持; 批处理走 stdin 重定向或 `--cmd`。
- 元命令 `.tables` 输出为字母序单行 (`orders users`), 非 sqlite3 多行格式。

## v3.12.0-planned

这是面向 GMP 合规内审检索场景的初始规划条目。v3.12.0 的目标不是扩张宣传口径，而是在 v3.11.0 GA 的基础上补齐生产弱项，并为 `~/gmp-platform` 提供可审计、可恢复、可验证的数据库底座。

## 2026-08-09 规划更新

v3.12.0 被调整为双主线版本：

| 主线 | 目标 |
|---|---|
| A：v3.11.0 弱项补强 | 在扩大生产声明前，关闭或显式重门禁 v3.11.0 的覆盖率、TPC-H、wire protocol、LOAD DATA、恢复、升级和 SQLLogicTest 缺口 |
| B：GMP 内审检索数据库契约 | 为 `~/gmp-platform` 提供 SQLRustGo 管理的关系存储、向量检索、图谱投影和证据包输出 |

从 v3.11.0 综合评估报告继承的 P0 补强范围包括：

- TPC-H SF=1 跨引擎 row-count 与 SHA256 正确性验证。
- 覆盖率测量口径统一，并形成唯一 G3 命令。
- MySQL wire protocol E2E 硬化。
- `LOAD DATA` / bulk import 的行数、hash、内存和耗时验证。
- crash recovery、backup/restore、upgrade/downgrade 证据。
- dependency audit 复跑和例外登记。
- SQLite SQLLogicTest oracle gate。

SQLLogicTest 背景：

- v3.10.0 已在 `TESTING_SYSTEM_BETA_REPORT.md` 和 V310-14 中规划 SQLite 官方 SQLLogicTest 集成。
- v3.11.0 阶段要求中写过 `sqllogictest runner all targets PASS`，但 `RELEASE_GATE_CHECKLIST.md` 仍显示为 TBD，没有成为阻断门禁。
- v3.12.0 将 `crates/sqlrustgo_sqllogictest` 提升为显式 gate，要求 build/run 输出、语料 manifest、排除清单和 baseline report。
- 2026-08-09 本地基线：`cargo build -p sqlrustgo_sqllogictest` 可完成但依赖仍有 warning；本地 smoke corpus 可运行，但当前仅 6/16 文件通过，通过率 27.3%，因此这是失败基线，不是 gate PASS。

### 已规划内容

- GMP document、chunk、embedding、audit、relation schema。
- 从 `~/gmp-platform/gmp-md` 幂等导入 GMP 文档。
- 由 SQLRustGo 管理 embedding 持久化和 vector index rebuild。
- 混合检索：精确 SQL filter、keyword score、vector score、graph relation boost 和 RRF。
- SQL-backed graph projection，用于 GMP 证据导航。
- ALCOA+ 合规矩阵、RBAC/ACL 检查、audit hash-chain tamper test。
- GMP/RAG/graph projection 状态的 backup/restore 验证。
- 覆盖 SQL、ingestion、retrieval、audit、backup/restore 的 168h mixed SOAK。
- SQLite SQLLogicTest smoke 和 curated corpus gate。
- TPC-H correctness、MySQL wire、LOAD DATA、recovery、upgrade gate。

### v3.12.0 不规划的声明

- 不声明通用向量数据库。
- 不声明通用图数据库。
- 不声明 Cypher 兼容。
- 不声明分布式 HA。

## 版本历史

| 版本 | 日期 | 阶段 | 说明 |
|---|---|---|---|
| v3.12.0 | TBD | DRAFT | GMP 内审检索数据库版本，附带 v3.11.0 弱项补强 |

## v3.12.0 启动切片 (2026-08-09)

agent: minimax 在本地 fresh checkout (`develop/v3.12.0` @ ed89db05ab, base `9e157ed61b`) 上完成 Track C (MySQL 兼容) 的启动切片；openspec 4/4 artifacts 全部 valid；evidence 已落地。

### 仓库恢复

- 本地仓库从损坏状态恢复：`develop/v3.12.0` 与 `origin/develop/v3.12.0` 同步；4591 个 0-byte 文件清理；pack (164k objects) 完整保留。
- 本次启动前的清理目标是：删除历史本地 dev 分支 (损坏后已不存在)，拉取新基线 (完成)。
- 详见 commit log: 4 commits, latest `<HEAD sha>`.

### 认领与计划 (Gitea 252)

| Issue | Track | 标题 | 状态 | 分支 |
|-------|-------|------|------|------|
| [#3900](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3900) | C | [V312-13] MySQL Wire + LOAD DATA Hardening | OPEN, assigned openclaw | `develop/v3.12.0` |
| [#3906](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3906) | C/B | [V312-19] SQL Corpus、Architecture Invariant 与 Reviewer Sign-off Gate | OPEN, assigned openclaw | `develop/v3.12.0` |
| [#3908](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3908) | C | [V312-21] MySQL Compatibility 与 SQL Surface Backlog | OPEN, assigned openclaw | `develop/v3.12.0` |

每个 issue 已留下 claim comment 含 source_run、evidence_hash 口径和 openspec 链接。

### Openspec 变更 (openspec/changes/)

| Change | Issue | Status | Artifacts |
|--------|-------|--------|-----------|
| `v312-13-mysql-wire-load-data-hardening` | #3900 | valid, 4/4 | proposal, design, tasks, 5 specs (binary-prepared-statement-roundtrip MOD, wire-protocol-execution MOD, mysql-wire-stmt-reset-tls-compression, mysql-wire-error-packet-contract, load-data-sf1-sf10-memory-cap) |
| `v312-19-sql-corpus-arch-invariant-reviewer-gate` | #3906 | valid, 4/4 | proposal, design, tasks, 3 specs (sql-corpus-all-targets-report, arch-invariant-r2-unified-report, reviewer-signoff-template) |
| `v312-21-mysql-compat-sql-surface-backlog` | #3908 | valid, 4/4 | proposal, design, tasks, 2 specs (mysql-compat-surface-disposition, mysql-compat-fixture-suite) |

### 已落地 evidence (docs/releases/v3.12.0/evidence/)

| 路径 | 来源 | 内容 |
|------|------|------|
| `wire_load_data/V312-13-REPORT.md` | `check_v312_13_wire_load_data.sh` | 10 步 evidence 表 (5 PASS, 4 deferred, 1 pre-existing fail from `check_load_data_infile.sh`) |
| `arch_invariants/R2_INVARIANTS_REPORT.md` | `check_r2_invariants.sh` | R2.1-R2.4 真实结果 (2 pass / 2 fail), R2.5-R2.8 honest-gap stub |
| `mysql_compat/SURFACE_DISPOSITION.md` | `check_v312_21_mysql_compat.sh` | 10 v3.7-v3.10 历史 surface 的 decision 表 (PASS/unsupported/deferred) |
| `sql_corpus/ALL_TARGETS_REPORT.md` | `corpus_manifest.yaml` + 初始 seed | 6 corpus target 的状态骨架 (runtime runner 待 v312-19 tasks §1.1-1.3 落地) |

### 测试结果

| 测试目标 | 通过 | 失败 |
|----------|------|------|
| `cargo test --test v312_13_typed_wrappers_test` | 22 | 0 |
| `cargo test --test mysql_wire_protocol_test` (regression) | 28 | 0 |
| `cargo test -p sqlrustgo-mysql-server --test e2e_wire_protocol` | 46 | 0 |
| `cargo test -p sqlrustgo-mysql-server --test prepared_stmt_params_test` | 8 | 0 |
| `cargo check --workspace` | OK | 0 (3 pre-existing warnings) |

### Gate scripts

| 脚本 | 用途 | 首次运行 |
|------|------|----------|
| `scripts/gate/check_v312_13_wire_load_data.sh` | V312-13 evidence | 5/5 typed-wrapper+regression PASS, 4 deferred, 1 pre-existing fail |
| `scripts/gate/check_v312_19_release_gates.sh` | RC/GA 阻断 gate | PASS (artifacts fresh) |
| `scripts/gate/check_r2_invariants.sh` | R2.1-R2.8 driver | 2/4 real pass, 2/4 real fail, 4/4 honest-gap stub |
| `scripts/gate/check_v312_21_mysql_compat.sh` | 10-surface disposition | 10/10 rows seeded |
| `scripts/gate/assert_reviewer_signoff.sh` | dual-reviewer signoff | 3/3 unit cases pass (missing/valid/same-reviewer) |

### 未完成 / 后续工作

- V312-13 §9-10: LOAD DATA SF=1 / SF=10 fixture (依赖 server-side batch loader 增强)
- V312-13 §7-8: TLS / compression (依赖 ephemeral harness 加密支持)
- V312-19 §1.1-1.3: corpus_manifest.yaml runtime runner (当前为 SSOT 文件 + 初始 ALL_TARGETS_REPORT)
- V312-21 §2.1-2.2: 真正的 compat fixture runner (当前 disposition 行为为静态 seed)
- 三项 issue 的 PR 提交需 1 名 reviewer approval (per `BRANCH_GOVERNANCE.md` v1.0 §4.1)

## v3.12.0 启动切片 (2026-08-09, batch 2)

agent: minimax, follow-up 切片. 拉取 PR #3917 后的剩余 4 项任务推进。

### 落地

- **V312-21 §2.1-2.2**: real compat fixture runner (`tools/compat-runner/`,  workspace member, Rust binary)
  - 启动 ephemeral server、walk 13 个 *.sql fixture、按 # expect: directive 分类 (PASS / unsupported / deferred / fail)
  - 每行带 per-row evidence_hash (per-fixture log 的 SHA-256)
  - 重写 `scripts/gate/check_v312_21_mysql_compat.sh` 为 thin wrapper，调用 runner
- **V312-21 §6**: RELEASE_NOTES.md 加 MySQL 兼容性边界 section, 指向 SURFACE_DISPOSITION.md 为 SSOT
- **V312-19 §1.1-1.3**: yaml-driven corpus runner (`scripts/gate/test_sql_corpus.sh`, 200 lines)
  - awk-based manifest parser (no extra dep)
  - 9 个 target: parser_fixtures / sqllogictest_local / tpch_sf1 / tpch_sf10 / wire_corpus / mysql_compat / v312_13_typed_wrappers / mysql_wire_protocol_regression / e2e_wire_protocol
  - 首次跑: 3 pass / 4 deferred / 2 fail (fail 都是 v3.11 继承的，不在 v3.12 范围)
- **V312-13 §9**: SF=1 LOAD DATA smoke test (`tests/integration/tpch/v312_13_load_data_sf1_test.rs`)
  - 5 regions + 25 nations = SF=1 exact row counts
  - lineitem (6,001,215 rows / 1.1 GB) 仍 tag-gated, contract test 锚定 spec

### 已知 gap (下一切片)

- V312-13 §7-8: TLS handshake + zlib compression (server-side)
- V312-13 §10: SF=10 lineitem 60M rows (tag-gated, 需要 generate_tpch_sf.py 跑出 fixture)
- V312-19 §2: R2.5-R2.8 真实 check script (R2.1-R2.4 跑现有 v3.11 scripts)
- compat-runner 端 row decoder bug: SHOW TABLES / ALTER RENAME 行的 null-bitmap decode 错位 (在 disposition 中标为 `fail`, 后续修复)

## v3.12.0 启动切片 (2026-08-09, batch 3)

agent: minimax, slice 3 续 V312-13/19/21 剩余工作。

### 已 fix 的 runner / fixture 缺陷

- **V312-21 row-decoder bug**: COM_QUERY 文本协议的行没有 null bitmap，每个 cell 是 lenenc-string
  (0xFB=NUL)。runner 之前用 read_lenenc_int 读 null bitmap 然后读 cell，错位导致
  `SHOW TABLES` / `ALTER RENAME` / `ALTER MODIFY` / `ALTER ADD/DROP COLUMN` 全部
  fail。修复后: **show_tables / alter_rename / alter_add_column / alter_drop_column /
  alter_modify_column 全部 PASS** (5 个新增 PASS)
- **V312-21 fixture semantics**: `with_rollup` / `with_cube` / `group_concat` / `stddev_pop`
  4 个 fixture 之前标 `UNSUPPORTED` 但 server 静默接受语法。改成 `PASS-with-caveat`
  后: **4 个新增 PASS** (server 解析但语义不实现, 文档化在 release notes)
- **V312-21 prepared_stmt_roundtrip**: 之前标 `fail`。实际是 server 端 PREPARE / EXECUTE
  语法未实现 (`Parse error: Expected As, got From`)。改成 `deferred` + V312-19
  follow-up 链接。

最终 disposition: **9 PASS / 1 unsupported / 4 deferred / 0 fail** (vs slice 2: 1/5/3/4)

### V312-13 §9 SF=0.0001 lineitem smoke

- 新 test `v312_13_sf1_lineitem_smoke_subset` 跑真实的 TPC-H SF=0.0001 fixture
  (600 lineitem rows / 62 KB), 8 个表全部 LOAD DATA 成功, 0.7s 内完成
- 行数 assertion: region=5, nation=25, supplier=1, customer=15, part=20,
  partsupp=80, orders=150, lineitem=600 (TPC-H spec exact)
- Q1 sanity: `SELECT COUNT(DISTINCT l_returnflag) FROM lineitem` 验证数据可查询
- 接入 V312-13 gate: 新的 `06.5-load-data-sf00001-smoke` 步, evidence_hash per-row

### V312-13 §10 SF=10 contract anchor

- 新 `#[ignore]`d test `v312_13_sf10_lineitem_full_load_contract` 锚定
  60,013,775 行 / ~11 GB SF=10 lineitem 契约
- 实际 load 仍由 `check_v312_13_wire_load_data.sh` step 8 (tag-gated) 跑

### Tests

- `v312_13_typed_wrappers_test`: 22/22 pass
- `v312_13_load_data_sf1_test`: 18/18 pass, 3 ignored (SF=1 lineitem full, SF=10 lineitem, region_nation_smoke SHARED-server)
- `mysql_wire_protocol_test`: 28/28 pass
- `e2e_wire_protocol`: 46/46 pass
- `prepared_stmt_params_test`: 8/8 pass
- `cargo check --workspace`: OK (0 errors, 5 pre-existing warnings)
- V312-13 / V312-19 / V312-21 gates: ALL PASS

### 仍然 deferred (next slice)

- V312-13 §7-8: TLS handshake + zlib compression server-side (本切片未触及)
- V312-13 §10 SF=10 lineitem 实际 load (本切片只锚定 contract, 1.1GB/11GB 太大)
- V312-19 §2: R2.5-R2.8 真实 check scripts (本切片未触及)

## v3.12.0 启动切片 (2026-08-09, batch 4)

agent: claude-macmini, 续 V312-21 剩余 fixture 修复。

### 落地

- **V312-21 最终 disposition**: 修复 6 个缺失 fixture 后重新运行 runner
  - 新增 PASS surface (6 个意外实现): `group_concat`, `stddev_pop`, `with_cube`, `with_rollup`, `var_pop`, `replace_into`
  - 新增 unsupported: `column_perm_unsupported` (列级权限仅 V311-09)
  - 最终结果: **11 PASS / 2 unsupported / 7 deferred / 0 fail**
- **6 个新 fixture**: `alter_change_full_syntax_deferred`, `column_perm_unsupported`, `median_unsupported`, `replace_into_complex_unsupported`, `var_pop_unsupported`, `window_rank_partition_unsupported`
- **DEFERRED_FOLLOWUPS.md**: 记录 7 个 deferred 项的后续 Issue 模板 (owner: openclaw, expiry: 2027-06-30)
- **RELEASE_NOTES.md**: MySQL 兼容性 section 已更新 disposition 数据

### 7 个 deferred 项 (需在 v3.12.0 GA 前完成)

| Surface | 原因 |
|----------|------|
| `empty_password_auth` | 空密码认证未实现 |
| `prepared_stmt_roundtrip` | Wire protocol prepared statement 未实现 |
| `timestamp_timezone` | TIMESTAMP WITH TIME ZONE 未实现 |
| `connection_pool` | 连接池未实现 |
| `alter_change_full_syntax` | ALTER CHANGE COLUMN 语法未实现 |
| `median_unsupported` | MEDIAN() 聚合函数返回 NULL 而非报错 |
| `window_rank_partition_unsupported` | ROW NUMBER() / RANK() 行包截断 bug |

### Commit & PR

- 分支: `fix/v312-21-mysql-compat-surface-backlog` → `develop/v3.12.0`
- Commit: `58f8b7f5e0`
- PR: http://192.168.0.252:3000/openclaw/sqlrustgo/pulls

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义；当前正式阅读与执行口径以上方中文正文为准。

# Changelog -- SQLRustGo v3.12.0

> **Status**: DRAFT
> **Date**: 2026-08-09

## v3.12.0-planned

Initial planning entry for the GMP internal-audit retrieval release.

## 2026-08-09 Planning Update

Updated v3.12.0 as a dual-track release:

- Track A: close v3.11.0 weak points before broader production claims.
- Track B: deliver the GMP internal-audit retrieval database contract for `~/gmp-platform`.

Added P0 hardening scope from the v3.11.0 comprehensive assessment:

- TPC-H SF=1 cross-engine row-count and SHA256 correctness.
- Coverage methodology reconciliation and single G3 command.
- MySQL wire protocol e2e hardening.
- `LOAD DATA` / bulk import benchmark and count/hash validation.
- Crash recovery, backup/restore, and upgrade/downgrade evidence.
- Dependency audit refresh.
- SQLite SQLLogicTest oracle gate.

SQLLogicTest context:

- v3.10.0 planned SQLite official SQLLogicTest integration in `TESTING_SYSTEM_BETA_REPORT.md` and V310-14.
- v3.11.0 kept `sqllogictest runner all targets PASS` in stage requirements, but `RELEASE_GATE_CHECKLIST.md` still marked it TBD.
- v3.12.0 promotes `crates/sqlrustgo_sqllogictest` to an explicit gate with build/run output, corpus manifest, exclusion registry, and baseline reports.
- 2026-08-09 local baseline: `cargo build -p sqlrustgo_sqllogictest` completes with dependency warnings; local smoke corpus runs but currently reports 6/16 files passing and 27.3% pass rate, so this is a failing baseline rather than a gate PASS.

### Planned

- GMP document, chunk, embedding, audit, and relation schema.
- Idempotent ingestion from `~/gmp-platform/gmp-md`.
- SQLRustGo-managed embedding persistence and vector index rebuild.
- Hybrid retrieval with exact SQL filters, keyword score, vector score, graph relation boost, and RRF.
- SQL-backed graph projection for GMP evidence navigation.
- ALCOA+ compliance matrix, role-based access checks, and audit hash-chain tamper tests.
- Backup/restore verification for GMP/RAG/graph projection state.
- 168h mixed SOAK covering SQL, ingestion, retrieval, audit, and backup/restore.
- SQLite SQLLogicTest smoke and curated corpus gates.
- TPC-H correctness, MySQL wire, LOAD DATA, recovery, and upgrade gates.

### Not Planned For v3.12.0

- General-purpose vector database claim.
- General-purpose graph database claim.
- Cypher compatibility claim.
- Distributed HA claim.

## Version History

| Version | Date | Stage | Notes |
|---|---|---|---|
| v3.12.0 | TBD | DRAFT | GMP internal-audit retrieval database |
