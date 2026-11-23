# RERUN-PROGRESS — 75 场景复跑台账（滚动更新）

- source_run: goal-R4 (dshb-first-round-20261120-4 续)
- 工具：`srg-test-factory-pilot/tools/test_factory.py`（反例#1 修复后版本：零测试 NOT_RUN、summary 交叉校验、should_panic 解析、--test 模式、原始输出留档）
- base：e9ff21fca3（develop tip，pilot worktree）

## 批次记录（全部真实 cargo 执行）

| 批次 | 套件 | 结果 | 证据 |
|---|---|---|---|
| 1 | RERUN-T1（undo/rollback/savepoint/transaction） | **22 PASS**，exit 0 | evidence/RERUN-T1-20261120T170800.json |
| 2 | RERUN-T1B2（vtu_guard/semisync/rws/pitr/parallel_wal/append_only/file_storage/table_level） | **218 PASS**（含 4 条 should_panic，交叉校验 218/218 一致） | evidence/RERUN-T1B2-20261120T171028.json |
| 3 | RERUN-T1C（3 条 tx_id 定向） | **3 PASS** | evidence/RERUN-T1C-20261120T172251.json |
| 4 | RERUN-T2T3（bplus_tree/mvcc/binary/buffer_pool/adaptive_hash_index/checkpoint） | **222 PASS** | evidence/RERUN-T2T3-20261120T172252.json |
| 5 | tf_t1_undo_invariants（根 crate 集成） | **4 PASS** | evidence/rerun-tf_t1_undo_invariants-*.log |
| 6 | torn_write_recovery（D-004 领域） | **1 PASS**（矩阵声明 KNOWN_FAIL 未复现） | evidence/rerun-torn_write_recovery-*.log |
| 7 | e2e_crash_recovery_proof | **4 PASS** | evidence/rerun-e2e_crash_recovery_proof-*.log |
| 8 | multi_db_crash_recovery_5192 | **4 PASS** | evidence/rerun-multi_db_crash_recovery_5192-*.log |
| 9 | wal_recovery_dedup_p0_08 | **5 PASS** | evidence/rerun-wal_recovery_dedup_p0_08-*.log |
| 10 | crash_commit_fsync_b1（D-003 领域，先遇 INFRA_ERROR → 构建 fault-injection 服务器二进制后） | **2 PASS**（SIGKILL 后已提交事务存活 2.91s） | evidence/d003-red-run-*.log |

## 中间插曲（诚实记录）

- RERUN-T1B 首跑判 INFRA_ERROR（218 vs 214 解析差）→ 根因 = 4 条 `should panic` 测试行未被 TEST_LINE 匹配 → 修复解析器 → 218/218 一致。交叉校验按设计工作。
- crash_commit_fsync_b1 首跑 FAIL = INFRA_ERROR（缺 `sqlrustgo-mysql-server` 二进制，需 `--all-features` fault-injection）→ 构建后实跑 2/2 PASS。**FAIL≠缺陷：先查 INFRA。**
- record() 原始留档缺陷修复：.log 现在存真实 cargo 输出（此前存 JSON，证据质量差）。

## 与矩阵 75 场景的对照状态（R5 官方汇总）

**官方数字（evidence/rerun-summary-20261120.json，逐条映射 evidence/case-mapping-20261120.json）：**

| 状态 | 数量 |
|---|---|
| **实测 PASS** | **70**（68 逐一实跑 + 2 特殊场景实证绑定） |
| **RED 确认** | **5**（D-001 桩静默成功 ×4 + ODKU 回滚 ×1） |
| NOT_RERUN | **0**（75/75 全覆盖） |

- 矩阵 case_id 存在重复（T6-001 两个来源批次），以 matrix_index 区分。
- 75 外新增 RED：脏读（D-002b，隔离违反 RC）。
- 声明 KNOWN_FAIL=4 vs 实测 RED=5：映射关系与属主确认的 D-001~004 编号待对齐。

## Task C 执行器闭环实测（R5）

语义整改 1 轮 + 编译反馈 2 轮收敛；终验 AI 测试 4/4 RED 与人工同效。结论与接线要求见 EXECUTOR_CAPABILITY_MATRIX_V2.md Task C 节。（模型调用 3 轮全入账，本地累计 ~12.8k tokens）

## 咨询通道状态

- consult-002 两次尝试均超时（600s/900s，RemoteDisconnected/Timeout）——桥多消费者竞争严重。建议：ChatGPT-web 咨询降频至里程碑级，或属主独占时段。待属主定。

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)
