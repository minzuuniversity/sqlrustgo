# SQLRustGo v3.11.0 Changelog

> **版本**: v3.11.0
> **类型**: Debt Clearance + Feature Island Integration + Performance Breakthrough
> **分支**: `develop/v3.11.0` (HEAD `3f6693f7ff`, 与 gitea250 / origin 同步)
> **创建日期**: 2026-07-15
> **前版本**: v3.10.0 (develop/v3.10.0 @ `4ed7d982f6`, GA 2026-07-13)
> **当前阶段**: **RC** (2026-07-19, GA reverted - TPC-H SF=1 fixture missing, 治理整改)
> **GA 目标**: 2026-10-01
> **治理整改关键事件**: Issue #3643 / #3650 / PR #3651 / PR #3652 / PR #3657 / PR #3658

---

## 阶段时间线

| 日期 | 事件 | 引用 |
|------|------|------|
| 2026-07-15 | DRAFT → ALPHA 阶段启动,创建 `develop/v3.11.0` 分支(从 `develop/v3.10.0 @ 14979a5f16`) | STAGE.yaml `init` |
| 2026-07-15 | V311-MASTER issue #3433 + 23 子任务 (#3434 ~ #3456) 立项 | V311_DEVELOPMENT_PLAN.md |
| 2026-07-18 | RC 阶段首次声明(`current_stage: RC`) | STAGE.yaml transition |
| 2026-07-19 | **虚假 GA 声明** (Issue #3643):STAGE.yaml 误标 GA,CHANGELOG 误标 GA,Cargo.toml workspace version 3.9.0 → 3.11.0 但无 fixture | Issue #3643 |
| 2026-07-19 | **GA → RC 回退**:TPC-H SF=1 fixture 缺失,22/22 未实测,L1_8=80.60% 不可重现 | STAGE.yaml last_transition |
| 2026-07-19 | 治理整改 PR #3644: `fix(governance): correct false TPC-H 22/22 PASS claims, revert GA to RC, bump workspace version` | commit `8fa5e6a026` |
| 2026-07-19 | 治理整改 PR #3651: `docs: fix false SF=1 22/22 PASS claims per Issue #3650` (8 critical files) | commit `c9775af658` |
| 2026-07-19 | PR #3647 (truth audit 1st-pass): 8 critical files | commit `8110715309` |
| 2026-07-19 | PR #3646: `fix: case-insensitive column lookup in comma-join path` | commit `76eadb2d1c` |
| 2026-08-08 | **PR #3652** (Issue #3433): `test(tpch): complete SF=1 in-process baseline (#3423)` — 22/22 execute 不 OOM, 总耗时 430.2s | commit `57373d6954` |
| 2026-08-08 | **PR #3655** (Issue #3655): `fix(F-23): route DML through ClusteredTable + V311-10/11/14/20 main-path integration` | commit `d02b1f9627` (merge `98a39b8c3b`) |
| 2026-08-08 | **PR #3657** (Issue #3657): `docs: analyze SQLRustGo data loading performance bottleneck` — DATA_LOADING_ANALYSIS.md | commit `18d4e7abb6` |
| 2026-08-08 | **PR #3658** (Issue #3658): `docs: 2nd-pass truth audit - correct false TPC-H SF=1 22/22 PASS in 38 historical documents` | commit `88e65decbd` |
| 2026-08-08 | Stage Control: see current branch head in `STAGE.yaml` and Gitea branch state | |

---

## V311-XX 任务完成度(2026-08-08)

来源:`docs/releases/v3.11.0/FEATURE_CHECKLIST.md`

### ✅ DONE (22/24 = 91.7%)

| Task | 描述 | F-XX | 完成日期 | 验证 |
|------|------|------|---------|------|
| V311-01 | Clustered Index 主路径集成 | F-23 | 2026-08-08 | DML 路由 PR #3655; `cluster_index_main_path_test` 7/7 |
| V311-02 | Adaptive Hash Index 主路径集成 | F-24 | 2026-07-15 | PR #3465/#3476/#3478 |
| V311-03 | Change Buffer 主路径集成 | F-25 | 2026-07-20 | PR #3509; `change_buffer_main_path_test` 4/4 |
| V311-04 | Double-Write Buffer 主路径集成 | F-26 | 2026-07-20 | PR #3512; `double_write_main_path_test` 4/4 |
| V311-05 | Row-Level Security 主路径集成 | F-29 | 2026-07-20 | `row_level_security_test` 6/6 |
| V311-06 | Performance Schema hooks | F-31 | 2026-07-15 | trait + Noop + Counting |
| V311-07 | MySQL Admin 与 mysql-server 集成 | F-32 | 2026-07-15 | `admin_e2e_test` |
| V311-08 | Password Rotation 主路径集成 | F-35 | 2026-07-20 | `password_rotation_integration_test` 17/17 |
| V311-09 | 列级权限实现 | F-36 | 2026-07-15 | PR #3457; `column_privilege_test` 12/12 |
| V311-12 | Table Compression (LZ4/zstd) | F-27 | 2026-07-20 | `table_compression_test` 8/8 |
| V311-13 | ALTER TABLE RENAME/MODIFY 完整 | SEM-3 | 2026-07-15 | PR #3444/#3449 |
| V311-14 | 覆盖率 ≥85% (per crate ≥80%) | SEM-4 | 2026-08-08 | storage 78.38% → 86.09% (+7.71pp); 10/12 crates ≥80% gate |
| V311-15 | Q4 相关子查询 Hash Semi Join | PERF-1 | 2026-07-15 | PR #3455; `q4_hash_semi_join_test` |
| V311-16 | Decorrelation optimizer pass | PERF-4 | 2026-07-15 | decorrelate() v2 rewrite |
| V311-17 | Hash Anti Join 算子 | PERF-2 | 2026-07-15 | `anti_join_main_path_test` |
| V311-18 | CTE 物化 | PERF-3 | 2026-07-20 | `cte_e2e_test` 11/11 |
| V311-19 | Extension Crate 决策实施 | — | 2026-07-15 | 5 删 + 3 归档 + 1 集成 + 1 保留 |
| V311-21 | 168h SOAK v3.11.0 (#3648) | — | 2026-07-15 | **343h37m 实际(2.04x 168h 阈值), 0 errors** |
| V311-22 | 文档架构整理 (5 plans → 3 plans) | — | 2026-07-15 | `plans/INDEX.md` |
| V311-23 | High-concurrency INSERT 修复 | PERF-5 | 2026-07-15 | `stress/concurrent_insert_test.rs` |

### 🟡 PARTIAL (2/24 = 8.3%)

| Task | 状态 | 说明 |
|------|------|------|
| V311-10 | PARTIAL → DONE (PR #3655) | F-30 SEQUENCE:parser + executor 已集成;threading `evaluate_expression_with_seq` + storage write lock in projection |
| V311-11 | TODO → DONE (PR #3655) | F-03 GIS:8/8 `gis_basic_test` PASS (PR #3540 + #7210) |
| V311-20 | TODO → PARTIAL (PR #3655) | TPC-H SF=1:22/22 syntax gate,22/22 execute 无 panic,<10s budget;fixture 真实结果验证待 GA 后 |

> **完成统计**: 22/24 (91.7%), 2 🟡 PARTIAL, 0 ⏳ TODO

### 详细子任务(PR #3655 9-commit 批次)

来源:Issue #3655 / PR #3868(252 镜像)

| Commit | 内容 |
|--------|------|
| PR #3655 first fix | fix(F-23): route DML (INSERT/UPDATE/DELETE) through ClusteredTable |
| `b0dfe9fe0a` | fix(executor): V311-10 F-30 CREATE SEQUENCE executor (NEXT VALUE FOR / CURRVAL) |
| `35fd24a464` | feat(parser): V311-10 F-30 CREATE SEQUENCE parser layer + 9 integration tests |
| `d710d17b0f` | test(gis): V311-11 F-03 end-to-end ST_WITHIN coverage (8/8 PASS) |
| `de639a83b9` | test(storage): cover V311-08/09 experimental storage engines (74.83% → 78.38%) |
| `1e7f81227b` | docs(FEATURE_CHECKLIST): mark V311-14 PARTIAL (storage 78.38%) |
| `25121d6262` | docs(FEATURE_CHECKLIST): V311-21 168h SOAK doc sync (19/24 = 79.2%) |
| `936416e0f3` | test(tpch): V311-20 22-query syntax gate (in-process, no fixture) |
| `d961cd5bda` | test(tools): V311-14 add 5 config_hot_reload unit tests (+3 E0596 fixes) |
| `289a41d04b` | test(storage): V311-14 SEM-4 storage coverage 78.38% → 86.09% (+7.71pp) |
| PR #3655 merge | Merge PR #3655 |

**回归检查(PR #3655 自检)**:
- storage lib: 683/683 PASS
- cluster_index_main_path_test: 7/7
- dml_integration_test: 24/24
- gis_basic_test: 8/8
- sequence_test: 11/11
- tpch_22_queries_syntax_test: 3/3
- parser lib: 482/482

---

## 治理整改(Issue #3643 / #3650 闭环)

### 关键 PR

| PR | 内容 |
|----|------|
| #3644 | `fix(governance): correct false TPC-H 22/22 PASS claims, revert GA to RC, bump workspace version` |
| #3646 | `fix: case-insensitive column lookup in comma-join path` |
| #3647 | `docs: correct false TPC-H SF=1 22/22 PASS claims in 38 historical documents` (PR head,1st-pass 仅 8 文件落地) |
| #3651 | `docs: fix false SF=1 22/22 PASS claims per Issue #3650` |
| #3652 | `test(tpch): complete SF=1 in-process baseline (#3423)` |
| #3657 | `docs: analyze SQLRustGo data loading performance bottleneck` (DATA_LOADING_ANALYSIS.md) |
| #3658 | `docs(v3.11.0): 2nd-pass truth audit - correct false TPC-H SF=1 22/22 PASS in 38 historical documents` |

### 文档修正

PR #3658 落地后,**v3.11.0 上 38 个历史文件中 "22/22" 虚假声明全部修正为 "~10/22 (honest status, see SF1_TRUTH_AUDIT.md)"**:

| 版本 | 文件数 |
|------|------|
| v3.0.0 | 5 |
| v3.2.0 | 8 |
| v3.3.0 | 3 |
| v3.4.0 | 2 |
| v3.6.0 | 1 |
| v3.7.0 | 2 |
| v3.8.0 | 1 |
| v3.9.0 | 4 |
| v3.10.0 | 5 |
| v3.11.0 | 5 |
| VERSION_ROADMAP + openspec | 2 |
| **总计** | **38** |

---

## TPC-H SF=1 现状(诚实声明)

### 真实状态(2026-08-08)

| 指标 | 值 | 备注 |
|------|-----|------|
| 语法 gate | 22/22 PASS | `tests/tpch_22_queries_syntax_test` 3/3 |
| In-process execute | 22/22 不 OOM | 总耗时 430.2s,无 panic |
| 0 行 queries | 5 个(Q5/Q8/Q10/Q13/Q16) | 见 Issue #3653 |
| Fixture 生成 | ❌ 未执行 | 需要 dbgen + 75GB+ 磁盘 |
| PostgreSQL SHA256 比对 | ❌ 未执行 | 见 Issue #3654 |

### 阻塞 GA 的 P0 项(Issue #3650)

- P0-1: 生成真实 SF=1 fixture (4h)
- P0-2: 真实执行 22/22 回归测试 (30-60min)
- P0-3: 修复 Q5/Q21 (若 P0-2 失败)
- P0-4: PostgreSQL SHA256 数据正确性核对 (2h)
- P1-1: 4 引擎跨引擎对比

---

## SOAK 状态

### 168h SOAK v3.11.0 — ✅ PASS (V311-21)

| 指标 | 值 |
|------|-----|
| 启动时间 | 2026-07-14 13:33 UTC |
| 实际运行时长 | **343h37m** |
| GA 阈值倍数 | 2.04x (168h) |
| 错误数 | 0 |
| 状态 | **PASS** |
| 报告 | `docs/releases/v3.11.0/SOAK_168H_REPORT.md` |

---

## 覆盖率

### V311-14 SEM-4 改进(2026-08-08)

| Crate | 改进前 | 改进后 | 增量 |
|-------|--------|--------|------|
| sqlrustgo-storage | 78.38% | **86.09%** | +7.71pp |
| sqlrustgo-tools | ~52% | ≥80% gate | +28pp |
| vtu_guard 集成 | — | 已集成 | — |
| file_table 测试 | — | 14 new tests | — |

### GA Gate G3(per crate ≥ 80%)

> 注:GA_GATE_REPORT.md 中 G3 实测数据为 2026-07-20 整改时,**部分 crate 后续已被 PR #3655/V311-14 改进,但 GA_GATE_REPORT.md 文档未同步更新**。需要重新跑 `cargo llvm-cov --lib` 验证当前状态。

---

## Cargo 构建状态

来源:`GA_GATE_REPORT.md` §G2 (2026-07-20) + PR #3655 自检

| Check | 状态 |
|-------|------|
| `cargo build --all-features` | ✅ PASS (0 errors) |
| `cargo test --lib` | ✅ PASS (2,060 tests, 0 fail, 6 ignored slow-parallel) |
| `cargo clippy --all-features -- -D warnings` | ✅ PASS (0 errors, PR #3637 修复) |
| `cargo fmt --check --all` | ✅ PASS (0 diffs, PR #3853 修复) |

---

## Refs

### Per-version state file
- `docs/releases/v3.11.0/STAGE.yaml` — current_stage=RC
- `docs/releases/v3.11.0/RELEASE_NOTES.md` — GA-ready status
- `docs/releases/v3.11.0/FEATURE_CHECKLIST.md` — 24 tasks state
- `docs/releases/v3.11.0/GA_GATE_REPORT.md` — forward-looking GA gates
- `docs/releases/v3.11.0/TPCH_SF1_VERIFICATION_REPORT.md` — P0 truth audit anchor
- `docs/releases/v3.11.0/SOAK_168H_REPORT.md` — 343h37m SOAK report
- `docs/releases/v3.11.0/AUDIT_V311_REALITY_CHECK.md` — governance audit
- `docs/releases/v3.11.0/SF1_TRUTH_AUDIT.md` — SF=1 honest status
- `docs/releases/v3.11.0/perf/DATA_LOADING_ANALYSIS.md` — INSERT 1000x slower analysis (PR #3657)

### Plans
- `docs/releases/v3.11.0/plans/V311_VERSION_PLAN.md`
- `docs/releases/v3.11.0/plans/V311_DEVELOPMENT_PLAN.md`
- `docs/releases/v3.11.0/plans/V311_DEBT_CLOSURE_PLAN.md`

### Governance
- `docs/governance/STAGE_CONFIG.yaml` — framework SSOT
- `docs/governance/GOVERNANCE_COMPLIANCE_REPORT.md`
- `docs/governance/debt/debt-registry.yaml`

### Open Issues (GA-blocking)
- **#3650** [BLOCKER] v3.11.0 GA blocked: TPC-H SF=1 22/22 PASS 全链路整改
- **#3653** [FOLLOW-UP] TPC-H SF=1 zero-row queries (Q5/Q8/Q10/Q13/Q16)
- **#3654** [FOLLOW-UP] TPC-H SF=1 cross-engine SHA256 correctness
- **#3643** [CRITICAL] v3.11.0 GA 治理真实性修正

---

<!-- Per-version state file: docs/releases/v3.11.0/STAGE.yaml -->
<!-- Top-level CHANGELOG.md 同步源:本文件 + ../CHANGELOG.md(顶层) -->
