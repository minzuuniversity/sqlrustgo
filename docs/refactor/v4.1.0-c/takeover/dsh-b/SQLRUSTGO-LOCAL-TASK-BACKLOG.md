# SQLRUSTGO-LOCAL-TASK-BACKLOG（本地任务池 · DSH-B）

- source_run: dshb-first-round-20261120-4
- 来源：Codex DAG 18 包（C01-01..18）+ DSH-B 补充评测任务；经 DSH-A 结构化任务提交派发，DSH-B 不自建调度器。

## 主池（Codex 18 包，全部 C-ALPHA-01，base e9ff21fca3）

| 包 | 类型 | 命令类 | 风险 | 依赖 |
|---|---|---|---|---|
| C01-01 | manifest/来源校验 | verify_handoff.py --check | L0 | — |
| C01-02 | Schema 导入负向 | pytest test_dsh_contract_binding | L1 | C01-01 |
| C01-03 | LOCAL_ONLY 策略观察 | pytest test_local_only_policy_observation | L1 | — |
| C01-04 | 结果分类负向 | pytest test_result_classification | L1 | S1 反例修复 |
| C01-05~08 | 事务/undo/snapshot/frontier/failed_state 契约测试 | cargo test ca01_* | L1-L2 | C01-04 |
| C01-09 | backend 能力契约 | cargo test ca01_backend_caps | L1 | C01-05 |
| C01-10/11/12 | 复杂 DML/BufferPool/wire 装配 | cargo test ca01_* | L1-L2 | C01-09 |
| C01-13 | checkpoint 中断 | cargo test ca01_checkpoint_interrupt | L2 | C01-07 |
| C01-14 | 可见性 oracle | cargo test ca01_visibility_oracle | L2 | C01-06 |
| C01-15 | SQLLogicTest 最小适配 | cargo run -p sqlrustgo_sqllogictest | L1 | — |
| C01-16/17 | Receipt 映射/history fixture | pytest test_receipt_mapping 等 | L1 | C01-02 |
| C01-18 | 终验 | verify_handoff.py --check | L0 | 全部 |

（⚠️ manifest 漂移 3 文件重签前，C01-01/C01-18 的哈希断言必失败——重签为硬前置）

## DSH-B 补充任务（能力评测，先于主池大量派发）

| 任务 | 执行器 | 状态 |
|---|---|---|
| Task A 调用链分析 | 252/8021/8011 | ✅ 完成见 MATRIX_V2 |
| Task B 固定反例写测试 | 252 实现 + 8011 审 | ⬜ 待批 |
| Task C 局部修复 | 252 | ⬜ 待批 |
| Task D 互审 | 8021 审 252 的 diff | ⬜ 待批 |
| Task E 恢复演练 | DSH 控制面 | ⬜ 待批 |

## 派发纪律

- 经 DSH-A 提交结构化任务（租约/幂等/预算走控制面）；写入并发 1；整改 ≤2 轮。
- 本地模型分工：252=Developer/Planner、8021=过程独立 Reviewer/Tester、8011=失败分析/反例深审（慢、费 token，仅用于深度分析）。
- 云端只允许：最小元数据咨询（ChatGPT-web，属主已授权）；**受保护源码/完整 diff/敏感日志不离开本机**。

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)
