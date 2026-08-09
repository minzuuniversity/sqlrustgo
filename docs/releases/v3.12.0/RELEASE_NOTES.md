# SQLRustGo v3.12.0 Release Notes

> **状态**: DRAFT
> **日期**: 2026-08-09
> **说明**: 本文是 v3.12.0 开发入口说明，不是发布完成说明。

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

当前 disposition（2026-08-09 V312-21 启动切片后）：

- 数据行数：13（包含 1 PASS / 5 unsupported / 3 deferred / 4 fail 的混合状态）
- PASS 表面：`alter_add_column`
- unsupported 表面：`create_procedure_unsupported`、`group_concat_unsupported`、`stddev_pop_unsupported`、`with_rollup_unsupported`（部分）
- deferred 表面：`empty_password_auth`、`timestamp_timezone_deferred`、`connection_pool_deferred`
- fail 表面（runner 端 bug 或 fixture 不匹配，见日志）：剩余项

v3.12.0 GA 之前必须完成：

- 把所有 `fail` 行的根因归类为 (a) runner 端 bug (修复) / (b) fixture 端 bug (修复) / (c) 服务端真实缺陷 (转 issue)
- 把所有 `deferred` 行升级为 PASS / unsupported / issue
- 把"with_rollup"等 server 静默接受的语义从 `unsupported` 重新分类为 `PASS-with-caveat`（这是 fixtures/服务器契约问题，不是 v3.12 范围）

每个 row 的 `evidence_hash` 字段指向 `docs/releases/v3.12.0/evidence/mysql_compat/logs/<surface>.log`
的 SHA-256 摘要；任何"我在 v3.12.0 跑了 X"的声明必须引用该 hash 而非
其他来源。
