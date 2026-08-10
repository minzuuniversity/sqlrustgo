# SQLRustGo v3.12.0 Release Notes

> **状态**: DRAFT
> **日期**: 2026-08-09
> **说明**: 本文是 v3.12.0 开发入口说明，不是发布完成说明。
**commit**: 1903545df6d036f7f6d5035a0503b5fa932aac51

## 版本定位

v3.12.0 是面向 GMP 内审检索系统的开发版本。目标是在 SQLRustGo 中形成受控的 GMP 数据内核，支持关系存储、内部向量检索、SQL-backed 图谱投影、RAG evidence bundle、审计链和恢复验证。

## 当前状态

- 分支：`develop/v3.12.0`
- 规划提交：`9e157ed61b68a2a7c61ebae18cd0a6c15ceb79b1`
- Gitea 总控 Issue：`#3887`
- 任务范围：`#3888-#3911`
- 阶段：DRAFT，准备进入 Alpha 开发。

## 本阶段允许声明

- v3.12.0 的开发计划、测试计划、Issue 拆分和初始门禁入口已建立。
- Hermes/OMP 可以按 V312 Issue 开始实现。

## 本阶段禁止声明

- 禁止声明 v3.12.0 已通过 Alpha/Beta/RC/GA。
- 禁止声明 SQLLogicTest、TPC-H correctness、wire、LOAD DATA、recovery 或 GMP compliance 已通过。
- 禁止把本文档当作执行证据。

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
