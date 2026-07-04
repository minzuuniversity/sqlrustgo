# SQLRustGo v3.10.0 版本计划

> **本文件** 是 v3.10.0 的版本入口. 详细内容见 `plans/V310_VERSION_PLAN.md`.

## 快速链接

- 战略定位: `plans/V310_VERSION_PLAN.md` § 1
- 任务清单: `plans/V310_DEVELOPMENT_PLAN.md` (26 任务 / 500h / 4 阶段)
- CLI binary 计划: `plans/V310_CLI_BINARY_PLAN.md` (C-6 + C-7)
- 当前阶段: `STAGE.yaml` (DRAFT, 待 ALPHA)
- 变更历史: `CHANGELOG.md`
- 架构: `ARCHITECTURE.md`

## 1. 战略定位 (简版)

**v3.10.0 = MySQL 5.7 替代**

- 常用 DML/DDL/DQL 完整, 不含破坏性 bug
- ACID 四项完整, MVCC/ROLLBACK 正确
- TPC-H SF=0.01 完整正确; QPS 不显著退化
- 24h+ soak 无错误; Crash recovery 正确

**不做**: 新语法 (Cypher, SIMD, Vector SQL), 高级 MySQL 函数 (GIS, FEOLE), 新索引类型

详细见 `plans/V310_VERSION_PLAN.md`.

---

*Created: 2026-07-01 (DRAFT stage init)*
*Replaces: 直接访问 `plans/V310_VERSION_PLAN.md`*
