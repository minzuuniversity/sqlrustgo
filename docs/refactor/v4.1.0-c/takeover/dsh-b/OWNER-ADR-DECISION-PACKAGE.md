# OWNER-ADR-DECISION-PACKAGE — 11 份 ADR 集中决策包（v7.8-B r6）

- source_agent: DSH-B (opencode-go/glm-5.3-flash)
- source_run: v78-r6-owner-decision-pack
- timestamp: 2026-11-20T23:30+08:00
- 依据：交接包 ADR-410-001~010（45da6968ae，manifest 绑定）+ ADR-410-011（PR #5917 b681b74ca3，PROPOSED/HUMAN_GATE_REQUIRED）+ DSH-B 自治 RED 实证
- 性质：决策材料汇总；**属主已于 2026-11-21 01:28 依据本包作出集中裁决**（绑定式，见 ADR-DECISION-RECORD.md）
- 裁决结果：011/006 条件批准；001/002/005 批准；003/004/007/008/009/010 批准方向；010 迁移实施另审；RowId 跨重启持久化=批准

## 〇、裁决结构建议（一次会话可完成）

1. **总纲先行**：批准或否决 **ADR-410-011（统一运行时契约）**——它是十稿的收敛伞；总纲裁决后，其余按依赖生效。
2. **必须立即裁决**（M0/M1 前置，阻塞首批工作包）：ADR-011、001、002、005、006。
3. **可延后裁决**（对应后批工作包，先批方向亦可）：ADR-003、004、008。
4. **可延后更多**（后阶段）：ADR-007、009、010。
5. **无需 Owner 决策（实现细节，授权技术负责人后由 DSH-B/Codex 细化）**：RowId 具体位宽与分配算法、spool 目录布局与 magic/版本格式、GroupCommit 批大小、typed error 枚举完整清单、reclaim floor 计算细节。
   ⚠️ 例外：ADR-011 §2.2 第 5 点 **"已提交 RowId 跨重启持久化与否"** 属磁盘格式层裁决，**必须 Owner 决**（选持久化=推荐；不持久化=须另交等价 redo 幂等方案）。

## 一、必须立即裁决（5 项）

### ADR-410-011 · 稳定身份、唯一恢复与提交/恢复统一契约（伞）
- **待批决策**：采纳"TxStore 唯一数据主账/物理 mutation+undo 所有者 + 事务私有 workspace + committed-only 持久化基线 + 显式事务 WAL/commit/frontier 协议；TM 唯一协调、不再二次恢复行"。
- **推荐**：批准（十稿收敛版，六项备选已显式否决并给出理由）。
- **已验证 RED 证据**：INV-01 双重撤销（回滚 3→6 行，PR #5915）、INV-07b flush 泄漏未提交数据进 recovery 基线、inv02 相同行 2→4 行（#5908）、脏读双引擎固化——全部指向"多恢复者/共享 pending 主账"结构缺陷。
- **关联工作包**：C01-05/06/07/08/14（几乎全部核心族）。
- **替代选项**：否决则回到分散十稿或保持现状（双层 undo）——现状已被实证违反 INV-01。
- **迁移风险**：大（换事务主账结构）；验证方法：M0 契约测试族 + 75 场景回归 + crash 注入（真实崩溃/WAL RecoveryEngine/断电验证在批准后按计划补齐，当前 NOT_RUN——如实声明）。
- **证据边界更正**（响应 ARCHITECTURE_REVIEW_NOTICE）：inv07b 为受控 flush 观察；**真实进程崩溃、WAL RecoveryEngine、断电验证均 NOT_RUN**。

### ADR-410-001 · 唯一协调器与 TxStore
- **待批决策**：方案 C（TM 管协议与冲突验证，TxStore 唯一物理变更/恢复所有者，引擎只解释 SQL/savepoint 名）。
- **推荐**：C；否决 A（保留双层——已被 INV-01 实证违反）与 B（仅 leaf 补定位——#5910 覆盖不足）。
- **关联**：C01-05/06/08；替代=维持现状。风险与验证同 011 伞。

### ADR-410-002 · 身份与命名空间
- **待批决策**：方案 C（TableId+RowId+OperationId+epoch/generation；UPDATE 主键不改 RowId；DELETE undo 恢复原 ID）。
- **推荐**：C；**含必决子项**：已提交 RowId 是否跨重启持久化（推荐持久化；否决则须另交等价方案）。
- **已验证 RED**：inv02 相同行 2→4 行、#5908 原件。
- **关联**：C01-05/14。风险：身份模型若持久化涉格式；验证=身份契约测试 + 迁移用例。

### ADR-410-005 · NO-STEAL 与大事务
- **待批决策**：方案 B（committed-only snapshot + 私有 workspace；大事务用隔离证明的临时 spool）。
- **推荐**：B。
- **已验证 RED**：**INV-07b flush 泄漏未提交数据进 recovery 基线**（DSH-B 实测，受控 flush 观察）+ 共享 insert_buffer 不可控 flush 的结构性风险。
- **关联**：C01-06。风险：大事务资源策略需配额/AbortOnly 设计；验证=大事务 spill/恢复用例。

### ADR-410-006 · Commit、Durable Frontier 与 ACK
- **待批决策**：方案 C（统一 prepare→validate→WAL durable decision→publish→ACK；同步模式 ACK 前 durable_lsn≥commit_lsn）。
- **推荐**：C。
- **关联**：C01-07。风险：性能（GroupCommit 引入延迟）；验证=两 commit 入口一致性 + durable frontier 测试（当前 NOT_RUN，批准后执行）。

## 二、可延后裁决（3 项，建议同批表态方向）

### ADR-410-003 · 语句原子性与 Savepoint —— 推荐 C（MutationBatch+语句 cursor 唯一 undo；RELEASE 只退名字）。关联 C01-08/14。
### ADR-410-004 · 事务状态与 Typed Error —— 推荐 C（typed 结果+可恢复状态机）。关联 C01-07/08。
### ADR-410-008 · MVCC、隔离与索引发布 —— 推荐 B（统一 RowId+read token/commit publish）。已验证 RED：脏读双引擎固化。关联 C01-12/14。

## 三、可延后更多（3 项，M2+ 前裁决即可）

- **ADR-410-007**（Checkpoint/reclaim）：推荐 B（版本化 generation/manifest+原子 segment 切换）。关联 C01-13。
- **ADR-410-009**（StorageCapabilities）：推荐 C（正交能力集+组合态认证）。关联 C01-09。
- **ADR-410-010**（格式/迁移）：推荐 B（保留 payload+版本化 sidecar+显式迁移；新格式旧程序拒开）。关联 C-ALPHA-04/05；**迁移风险最大**，裁决时须附回滚演练计划。

## 四、预算与释放计划（申请属主一次性授权）

| 批 | 工作包 | 金额（Token） | 释放条件 |
|---|---|---|---|
| 1 | C01-01、C01-03、C01-04 | 400,000 | 命名错位处置 + authorized_dispatch>0 |
| 2 | C01-02、C01-05、C01-06（测试已备，复跑验收） | 300,000 | 批 1 通过 |
| 3 | C01-07~C01-14（含 2 个人工门包） | 1,200,000 | ADR-011/001/002/005/006 批准 |
| 4 | C01-15~C01-17 + C01-18 | 670,000 | 批 3 通过 |
| **合计** | | **2,570,000** | 本地 Token，云端 0 |

## 五、继承与冲突声明

- ADR-011 收敛原十稿：批准 011 即视为对 001~010 的方向性确认；010/007/009 仍按各自阶段单独落地。
- 若属主否决 011 的任何子项 → 对应 ADR 回到 PROPOSED 并暂停关联工作包，不自动降级执行。
- 全部 RED 原始断言与关联 Issue 保留不变（含 #5915 独立复跑 4 FAIL / 2 PASS 结果）。

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)
