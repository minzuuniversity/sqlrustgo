# RED-LEDGER — 75 场景 5 项 RED 台账（活文档，禁止以删/跳/弱断言消 RED）

- source_run: autonomy-r2（r7 更新：属主已集中裁决 ADR；RED 全部保留；INV-07b 证据边界已更正）
- 裁决绑定：ADR-DECISION-RECORD.md（commit 45da6968ae + manifest c722885f + ADR-011@b681b74ca3）
- 基线冻结：e9ff21fca3（70 PASS / 5 RED / 0 NOT_RUN，evidence/rerun-summary-20261120.json）
- **测试语义分类（Owner 裁决 2026-11-21 §一）**：Gap Probe / Contract Test / Regression Test / Mutation Test / Acceptance Test 五类
  - **Gap Probe（记录当前能力缺口，其 PASS 仅表示缺口仍存在，不得解释为目标契约满足）**：
    `c01_07c_paired_frontier_observation_must_exist`（d0b1157b62）、`c01_09a_capability_descriptor_must_exist_for_write_before_refuse`、`c01_09c_wrapper_capability_attestation_must_be_observable`（334ccf423a）
    —— Owner 裁决：**三例仍属 INEFFECTIVE_TEST**，保持可追溯，**DSH-B 不再自行修改**，交 Codex 独立裁决
  - **Contract Test（验证正确产品行为）**：下表 17 项 RED 主体（断言方向=目标契约，当前实现不满足故 RED）
  - **Regression Test（保护已有正确行为）**：六包中的 PASS 项（inv01b / inv07c / inv15a / inv15b / inv14_bp_update_undo_applied / c01_07b）
  - **Mutation Test（验证判别能力）**：`tools/verify_srg_d004_acceptance.py` R4 独立 worktree 反向验证
  - **Acceptance Test（工作包出口）**：DAG 各包 `required_tests`（含 negative_control）
  - 依据：`S2-DESIGN-INPUT.md` §2.2；**Gap Probe 的 PASS 不得计入契约满足或发布门禁通过**

| # | RED 场景 | 缺陷对象 | Issue | 反例稳定性 | 架构依赖 | 自治整改条件 | 状态 |
|---|---|---|---|---|---|---|---|
| 1 | tf_t4_01 scan 恒空 | BinaryTableStorageV2 桩 | **#5913** | ✅ 3+2 次复现 | 修复方向属主裁决（补齐 vs UNSUPPORTED） | ❌ 等裁决 | OPEN |
| 2 | tf_t4_02 delete 恒零 | 同上 | **#5913** | ✅ 同上 | 同上 | ❌ 等裁决 | OPEN |
| 3 | tf_t4_03 update 恒零 | 同上 | **#5913** | ✅ 同上 | 同上 | ❌ 等裁决 | OPEN |
| 4 | tf_t4_04 begin_transaction 未拒绝 | BinaryTableStorageV2 事务接口 | **#5914** | ✅ 稳定 | 无（显式拒绝为既定方向，DSH-A/Codex 一致） | ✅ **已入队**（SRG-D004-01, PLANNED, delegation DSH-B-SRG-001） | IN_FLIGHT |
| 5 | PILOT-T5-101 odku-rollback-noop | FileStorage.update 静默无效撤销 | #5905/#5906/#5908/#5909 族 | ✅ 基线 RED 实证 | p2b proto 分支已局部修复（未合并）；与 #5909 单层恢复方案联动 | ⚠️ 合并决策属 PR 流程（属主） | OPEN(修复已存) |

| 6 | ca01_file_undo inv01a 多重集违反（回滚后 3→6 行） | FileStorage 双重撤销（delete-undo/update-undo 叠加） | #5905/#5909 族（PR #5915） | ✅ c0105-run4 实证 | 架构级（双层 undo 单一恢复责任，#5909） | ❌ HUMAN_GATE | OPEN(新实证) |
| 7 | ca01_file_undo inv02 相同行 2→4 行 | 同上（INV-02 口径，#5908 强化） | #5908 + PR #5915 | ✅ 同上 | 同上 | ❌ HUMAN_GATE | OPEN(新实证) |

| 8 | ca01_snapshot_reopen inv07a 脏读（双引擎固化版） | 引擎会话隔离缺失 | p2c1iso 同源（PR #5915） | ✅ c0106-run2 实证 | 架构级 | ❌ HUMAN_GATE | OPEN(固化) |
| 9 | ca01_snapshot_reopen inv07b flush 泄漏未提交数据 | 未提交状态随盘持久化（INV-07/F03 违反） | 拟并入 #5909 裁决材料（PR #5915） | ✅ c0106-run2 实证（**边界更正 per ARCHITECTURE_REVIEW_NOTICE：真实进程崩溃 / WAL RecoveryEngine / 断电验证均 NOT_RUN**） | 架构级（recovery 基线语义） | ❌ HUMAN_GATE | OPEN(新实证) |

| 10 | ca01_commit_frontier c01_07a：COMMIT ACK 不隐含持久化 | 当前实现 vs ADR-006 目标契约差距（M-Pre 证据，非缺陷判定） | PR #5916 域（本单） | ✅ c0107-run 实证 | 契约差距（durable frontier 缺失） | ⏳ 等 ADR-006 补充契约冻结后实施闭合 | OPEN(基线差距) |

| 11 | ca01_backend_caps c01_09b：写前拒绝缺失（INV-14 能力侧） | 非事务后端接受 begin_transaction（D-004 同源） | #5914（PR #5919，INV-14 口径互证） | ✅ c0109-run3 实证 | 能力描述符缺失（ADR-009 输入） | ⏳ 等 ADR-009 冻结补充契约 | OPEN(口径互证) |
| 12 | ca01_complex_dml inv05a：多行 UPDATE 回滚 4→8 行 | 双重撤销（多行语句） | PR #5920（#5909 族） | ✅ c0110-run 实证 | 架构级（单一 undo） | ❌ HUMAN_GATE | OPEN(新实证) |
| 13 | ca01_complex_dml inv05b：跨表 DML 回滚行重复 | 跨表共同撤销被违反 | PR #5920（#5909 族） | ✅ c0110-run 实证 | 架构级 | ❌ HUMAN_GATE | OPEN(新实证) |
| 14 | ca01_bufferpool inv14_bp_delete_insert：buffer_pool 装配 INSERT 撤销未生效（**绕过 Undo**） | buffer_pool 装配路径 undo 不安全 | PR #5921（业主点名问题实证） | ✅ c0111-final 实证（PLAIN 对照正常） | 装配层（test-only capability 提升为生产前必须闭合） | ✅ 属可自治整改候选（能力/装配层，非核心事务协议） | OPEN(新实证) |
| 15 | ca01_bufferpool inv15_bp_and_plain：两装配可见状态不一致 | buffer_pool 装配语义等价性被违反 | PR #5921 | ✅ c0111-final 实证 | 装配层 | ✅ 可自治整改候选 | OPEN(新实证) |
| 16 | ca01_bufferpool inv14_bp_adds_no_extra：buffer_pool 装配 WHERE/PK 查询返回空 | buffer_pool 读路径丢行 | PR #5921 | ✅ 同上 | 装配层 | ✅ 可自治整改候选 | OPEN(新实证) |
| 17 | D-002b 脏读（75 外新增，来源 wt-p2b 分支探针，非六包） | | 引擎隔离（read-committed 不满足） | 拟并入 #5909 族 | ✅ 基线 RED 实证 | 架构级（C 路线核心） | ❌ HUMAN_GATE | OPEN |
| 18 | ca01_visibility_oracle inv13：未提交状态对 peer 引擎可见 | 隔离语义（read-committed 下限）违反 | PR #5922（与 inv07a 同源） | ✅ c0114-run 实证（2 vs 1） | 架构级 | ❌ HUMAN_GATE | OPEN(新实证) |

## 纪律

- RED 是资产：每项必须保持可复现；修复落地后由 verify/复跑判据判定 NEW_GREEN，MUTATION_RED 反向验证后才能销账。
- 任何"通过删除/跳过/弱化断言使 RED 消失"的行为 = 反伪造违规（Type C）。

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)
