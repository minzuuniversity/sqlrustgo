# SQLRustGo v3.12.0 Release Notes

> **provenance:** generated_by=v3.12.0-remediation-round-3, generated_at=2026-08-10T10:49:33Z, commit=1903545df6d036f7f6d5035a0503b5fa932aac51, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0

> **状态**: **RC / GA candidate preparation** (2026-09-02 文档刷新; 2026-08-26 转入 RC; 2026-08-19 转入 BETA)
> **日期**: 2026-09-02 (GA candidate docs); 2026-08-26 (RC); 2026-08-20 (BETA); 2026-08-09 (initial)
> **说明**: 本文是 v3.12.0 开发与阶段发布说明；当前阶段仍为 RC，尚未 promoted to GA。
> **commit**: 1903545df6d036f7f6d5035a0503b5fa932aac51
> **current_HEAD**: b14ad8df03 (`origin/develop/v3.12.0`, post PR #4609 merge)

## 版本定位

v3.12.0 是面向 GMP 内审检索系统的开发版本。目标是在 SQLRustGo 中形成受控的 GMP 数据内核，支持关系存储、内部向量检索、SQL-backed 图谱投影、RAG evidence bundle、审计链和恢复验证。同时承担 v3.11.0 GA 弱项补强职责。

## 当前状态

- 分支：`develop/v3.12.0`
- 阶段：**RC / GA candidate preparation** (2026-08-26 从 BETA 转入; STAGE.yaml current_stage: RC)
- Gitea 总控 Issue：`#3887` (master), `#4386` (V312-59-C RC umbrella)
- 任务范围：V312-01 ~ V312-57 (ISSUES_PLAN.md)
- RC Gate：`bash scripts/gate/check_v312_promotion_to_rc.sh` → PASS 9/11, NO-OP 2/11, BLOCKERS 0
- RC Gate Report：[`RC_GATE_REPORT.md`](RC_GATE_REPORT.md)

## 2026-09-02 GA candidate 推进状态

v3.12.0 milestone 在 Gitea 上已达到 `open_issues=0 / closed_issues=79`，最新
`origin/develop/v3.12.0` HEAD 为 `b14ad8df03`。这表示 milestone 内任务已清零，
但不等于 GA gate 已经通过。

当前 GA 推进边界：

- GA-2：已有 1h demo 与 8h local V5 SOAK 证据；Linux/Docker SOAK 5691 复验仍需完成或正式 reclassification。
- GA-1：最终 GA cut 必须生成 `mode: full` 的 aggregate JSON；现有 fast-path JSON 不可作为最终 GA PASS。
- GA-3：安全报告有 prior PASS-with-caveat，最终 cut 需要 fresh scan。
- GA-7：本文档刷新后必须重跑 docs links / consistency。
- Release claims：新开 #4607、#4608、#4610-#4613 不在 v3.12 milestone，但限制广义 SQLite teaching / SQL function correctness 声明。

新增 GA candidate 文档入口：

- [`GA_RELEASE_REPORT.md`](GA_RELEASE_REPORT.md)
- [`PERFORMANCE_REPORT.md`](PERFORMANCE_REPORT.md)
- [`SECURITY_AUDIT.md`](SECURITY_AUDIT.md)
- [`RELEASE_CHECKLIST.md`](RELEASE_CHECKLIST.md)
## V312-57 sqlite3-like 一体化教学 CLI (2026-08-20)

**Issue #4359** — 支撑 BustubX-EDU 前 4-6 周自动验收, 合并为 PR #4373 (commit `543e15b3f`, 基于 PR #4371/#4372)。

交付内容:

- `sqlrustgo` 二进制 (crates/sqlrustgo-cli): 单路径数据库 `sqlrustgo edu.db`, 无需 MySQL server/端口/账号。
- 批处理: stdin 脚本、`--cmd` 单条、`--continue-on-error`; 解析/绑定/执行错误输出稳定前缀 `sqlrustgo:error:parse|bind|runtime:` 并返回非零退出码。
- 输出模式: 默认 table + `.mode list/csv/json` + `.headers on/off`。
- 元命令: `.help` `.quit` `.exit` `.tables` `.schema [table]` `.mode` `.headers` `.read` `.output` `.timer` `.explain`。
- 持久化: FileStorage 目录型数据库, 跨进程建表→插入→查询验证通过。
- 教学 fixture: `tests/compat/bustubx_edu_sqlite_cli/` week01-week04, 14 cases; gate `scripts/gate/check_bustubx_edu_cli_v312.sh` **14/14 PASS**。
- 单元测试: `cargo test -p sqlrustgo-cli --lib` **67/67 PASS**。
- 兼容边界: 不声明 SQLite 文件格式兼容; `sqlrustgo <db> "SQL"` 位置参数形式不再支持(经 implicit alias 走 stdin/`--cmd`)。

本阶段新增教学入口说明见 [`README.md`](README.md) 2026-08-19/2026-08-20 条目; 完整验证报告: [`evidence/bustubx_edu_cli/V312-57-EDU-CLI-VERIFICATION.md`](evidence/bustubx_edu_cli/V312-57-EDU-CLI-VERIFICATION.md)。

## 本阶段允许声明

- v3.12.0 处于 **RC**: 全部 12 项 `promotion_to_RC_requires` 已满足, RC Gate 0 BLOCKERS。
- 允许按 RC 口径声明: 受控 GMP 内审检索工作负载 + BustubX-EDU 教学 CLI 脚本化验收 + V312-58 Sprint 5 followup-6 (Q20 BinaryOp arm 路径已验证)。
- 允许引用 [`RC_GATE_REPORT.md`](RC_GATE_REPORT.md) 与 [`evidence/v312-59/RC{1..11}_*_REPORT.md`](evidence/v312-59/) 作为本阶段执行证据。
- 允许把 `tests/integration/tpch/q20_binaryop_arm_test.rs` (2/2 PASS) 作为 Q20 BinaryOp 右端子查询路径的路径验证证据。

## 本阶段禁止声明

- 禁止声明 v3.12.0 已通过 GA；GA 仍需 GA-2 SOAK 证据闭环或正式 reclassification，并需要 final full-mode aggregate。
- 禁止声明通用向量数据库 / 通用图数据库 / 完整 MySQL 5.7 替代。
- 禁止把本文档当作 GA 执行证据；GA 由 [`promotion_to_GA_requires`](STAGE.yaml) 单独门禁。

## 后续入口

进入 Alpha 前必须执行或补齐：

- `scripts/gate/check_alpha_v3.12.0.sh`
- `scripts/gate/check_sqllogictest_v312.sh`
- `bash scripts/gate/check_stage.sh --version v3.12.0 --stage ALPHA`

## MySQL 兼容性边界 (V312-21 / ISSUE #3908)

> **本节是 SSOT**: 任何关于 v3.12.0 MySQL 兼容性的声明都必须从
> [`docs/releases/v3.12.0/evidence/mysql_compat/SURFACE_DISPOSITION.md`](evidence/mysql_compat/SURFACE_DISPOSITION.md) 读取；
> 不得在 release note 中复制未经 disposition 表验证的兼容性声明。


当前 disposition（2026-08-09 minimax V312-21 runner，`source_run: minimax-v312-21-runner-71fc33a9d2`，commit `71fc33a9d2`）：

- 数据行数：20（PASS: 11 / unsupported: 2 / deferred: 7 / fail: 0）
- **PASS surface**: `alter_add_column`, `alter_drop_column`, `alter_modify_column`, `alter_rename`, `show_tables`, `group_concat` (意外实现), `stddev_pop` (意外实现), `with_cube` (意外实现), `with_rollup` (意外实现), `var_pop` (意外实现), `replace_into` (意外实现)
- **unsupported surface**: `create_procedure_unsupported` (存储过程未实现), `column_perm_unsupported` (列级权限仅 V311-09)
- **deferred surface** (owner: openclaw, expiry: 2027-06-30): `empty_password_auth`, `prepared_stmt_roundtrip`, `timestamp_timezone`, `connection_pool`, `alter_change_full_syntax`, `median_unsupported`, `window_rank_partition_unsupported`
- **fail surface**: 0 (全部修复)

v3.12.0 GA 之前必须完成：

- 所有 deferred 项必须创建 follow-up issue 并填入 SURFACE_DISPOSITION.md
- deferred 项见: `evidence/mysql_compat/DEFERRED_FOLLOWUPS.md`

每个 row 的 `evidence_hash` 字段指向 `evidence/mysql_compat/logs/<surface>.log` 的 SHA-256 摘要。
