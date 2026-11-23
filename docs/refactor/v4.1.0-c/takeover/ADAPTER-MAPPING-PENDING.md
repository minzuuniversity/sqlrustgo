# ADR→task 映射待决清单（approved_adr_refs 为空的包）

> source_agent: Task 4 执行者（Codex CLI）· source_run: V410-READINESS-TASK-4-20261123 · timestamp: 2026-11-23 (Asia/Shanghai)
> 依据：属主集中裁决账本 [ADR-DECISION-RECORD.md](dsh-b/ADR-DECISION-RECORD.md) 第 35-47 行；映射规则 A2.2（设计文档 §A2.2，已由属主审阅）；[REFACTOR_TASK_DAG.json](../../../releases/v4.1.0/refactoring/REFACTOR_TASK_DAG.json) 的 `tasks[].approved_adr_refs`。
> conflict_resolution: 本清单只登记「未映射」这一残余状态，不代属主决定映射，也不构成实施授权。

## 规则回顾（A2.2）

| 规则 | 判定 |
|---|---|
| R1 | 每个 task 的 `approved_adr_refs` 至少 1 项 |
| R2 | 引用项必须是该 task 的 `invariant_ids` / `target_files` 实际关联的 ADR |
| R3 | 仅批准方向/实施另审档位（003/004/007/008/009/010）不得被 `authorized_for_execution=true` 的 task 引用 |
| R4 | 条件批准档位（006/011）在前置冻结物记录之前不得被任何 task 引用 |
| R5 | 只能引用 `adrs/` 中真实存在的 id |

R1 与 R2 冲突时（该包与任何已批准 ADR 都无实际关联）以 R2 为准：该包保持 `[]`，
并登记在本清单里，**不得**为凑数填入无关 ADR。

## 待决包

| task_id | epic | invariant_ids | target_files | 说明 |
|---|---|---|---|---|
| C01-15 | E01-INTEGRATION | INV-17 | `tests/test_factory/ca01_common_subset.slt` | 复用既有 sqllogictest runner 与自写共同 SQL 小夹具；外部 corpus 授权未核前不导入，不替换 MySQL oracle |

### 为什么留空

1. **不变量侧无归属**：C01-15 的唯一不变量是 `INV-17`（"必需 case 的 FAIL/KNOWN_FAIL/NOT_RUN/未批准 UNSUPPORTED 均阻断认证"，见
   [TRANSACTION_INVARIANTS.md](../../../releases/v4.1.0/refactoring/TRANSACTION_INVARIANTS.md)）。11 份 ADR 的「目标不变量」字段里
   **没有任何一份声明 INV-17**——INV-17/INV-18 是测试工厂的认证口径不变量，不是架构不变量。
2. **ADR 依赖图侧无入边**：C01-15 不落在 `001/002→003/004`、`008←002/006`、`009←all`、`010←005/007` 任何一条边上，
   依赖图无法为它推出关联 ADR。
3. **target_files 侧无关联**：`tests/test_factory/ca01_common_subset.slt` 是自写的 SQL 逻辑测试子集，
   不在任何一份 ADR 的「预期影响」文件面里；该包只验证"既有 SQL 语义可被外部 runner 复跑"，不实现任何 ADR 的契约。

### 考虑过但拒绝的候选（避免事后被当成"漏填"）

| 候选 | 为什么不填 |
|---|---|
| [ADR-410-009](../../../releases/v4.1.0/refactoring/adrs/ADR-410-009.md)（StorageCapabilities 与支持矩阵） | 它的「目标不变量」是 INV-14/INV-15/INV-18，与 C01-15 的 INV-17 无交集；把它当成"共同子集 = 支持矩阵"是一种读感，不是语料里声明的关联 |
| [ADR-410-010](../../../releases/v4.1.0/refactoring/adrs/ADR-410-010.md)（格式、迁移与外部消费者） | 它的「目标不变量」是 INV-11/INV-16（迁移与 redo 幂等），与 C01-15 无交集；"外部 corpus 授权"是测试素材的授权问题，不是磁盘/sidecar 格式迁移 |

## 关闭它需要什么

由属主在放行裁决时拍板（spec O1/O10），二者择一即可：

1. **裁定 C01-15 与某份 ADR 无需绑定**（例如把它归为"测试工厂治理包"，不属于 ADR 覆盖范围）——
   则本行从待决清单移除，`approved_adr_refs` 保持 `[]`，并在放行裁决记录里写明该口径。
2. **裁定 C01-15 应绑定某份 ADR** —— 则按 A2.2 R2 给出该 ADR 与 C01-15 的实际关联依据
   （不变量、target_file 或依赖图边），执行者据此回填并重签 manifest。

在属主拍板前，`verify_handoff.py --check` 会把 C01-15 计入 `tasks_pending_adr_approval`，
交接包的 `adr_approval_state` 保持 `PENDING`——这是如实状态，不是门禁失败。
