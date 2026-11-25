# SQLRUSTGO-TEST-ARCHITECTURE（六层测试体系 · DSH-B）

- source_run: dshb-first-round-20261120-4
- 定位：属主指令 T0-T5 六层；与既有资产映射，不另建重复目录。

## 层级 ↔ 现有资产映射（实查）

| 层 | 定义 | 现有资产（已核实存在） | 缺口 |
|---|---|---|---|
| T0 | 静态检查/构建 | scripts/gate/ D1-D16、check_arch_sem_debt（D8 PASS 实测）、cargo build/clippy/fmt | 门禁有效性审计（见 GATE-EFFECTIVENESS-AUDIT） |
| T1 | Parser/Planner/Executor/Storage 单元 | crates/* 内联 #[test]、tests/unit/、试点 T1 族 30 用例 | 试点 30 用例全部 NOT_RUN（矩阵复跑=0） |
| T2 | SQLLogicTest/DDL/DML/MySQL 语义 | crates/sqlrustgo_sqllogictest、scripts/gate/check_sqllogictest_v312.sh、tests/integration/{compat,dml,mysql_server_e2e} | SQLLogicTest 最小适配未做（C01-15） |
| T3 | 事务/WAL/持久化/崩溃恢复 | tests/integration/fault_injection/、wal 测试族、试点 T6 崩溃恢复/撕裂写 | kill-9 与撕裂写场景未复跑 |
| T4 | 并发/性能/资源回归 | benches/、z6g4-tpch-sf1-cell-diff workflow、168h SOAK 框架 | 大型 SOAK 本轮不启用（属主令） |
| T5 | 门禁/反向变异/Issue 治理 | NEG_FIXTURES_v4.1.0、mutation 夹具、Issue 台账 | 反向变异独立副本机制未接 |

## 每测试 12 要素契约（属主令，强制）

test_id / 对应 Issue / 输入·操作·初始状态 / 独立正确性判据 / 预期 vs 实际 / 运行环境 / 固定代码 SHA / 执行退出码 / 原始证据 / 对应门禁。DSH-B 的运行包装器（接线后）在 Receipt 中逐项落字段；Codex 的 dsh-receipt.schema.json（哈希已核）为对齐基准。

## 关键纪律

- 不同 Storage Backend 分别标注：**不用 MemoryStorage PASS 证明 FileStorage/BinaryStorage 同语义**（试点矩阵 backend 字段已按此设计）。
- AI 生成测试，但 Oracle 独立：模型输出不得充当唯一正确性判据；判据来自契约/参考实现/确定性断言。
- 测试七态分类沿用试点工具（PASS/FAIL/KNOWN_FAIL/UNSUPPORTED/DIALECT_DIFFERENCE/NOT_RUN/INFRA_ERROR），KNOWN_FAIL≠PASS。

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)
