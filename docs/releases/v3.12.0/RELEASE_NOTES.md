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
