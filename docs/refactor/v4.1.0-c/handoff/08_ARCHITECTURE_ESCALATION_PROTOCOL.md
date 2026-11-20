# 架构升级与裁决协议

> source_agent: Codex GPT-6.1 Sol · source_run: C-ALPHA-01-HANDOFF-20261120 · timestamp: 2026-11-20 (Asia/Shanghai)
> evidence_hash: sha256:4649fccc566613f04568ef3564e9d585739c4e92c73ad5972714b7d4f65b992f（HARA 输入 manifest）；本轮来源及产物 hash 见 evidence/SOURCE_BINDINGS.json 和 handoff/10_HANDOFF_MANIFEST.json。
> conflict_resolution: Owner 裁决优先；本文 PROPOSED，计划交付不等于 ADR 批准、任务授权或产品认证。

| 情况 | DSH处理 | 升级方 |
|---|---|---|
| 已批范围内普通测试/代码错误 | 保留最小SQL/seed、raw日志/hash；最多两次尝试；无新证据不重试 | OpenCode整改、OMP复核 |
| 工具/资源不可用 | BLOCKED_INFRA/WAITING_RESOURCE，保留lease/fencing证据；不云升级 | DSH与资源属主 |
| 未知schema字段、sidecar绑定丢失、ADR digest变 | 停止导入/执行，保留preimage | DSH负责人+Codex技术评审 |
| 事务/身份/隔离/commit/WAL/checkpoint/MVCC核心契约冲突 | 不自行改oracle/协议；提交受影响ADR、调用链、反例、备选与回退 | Codex关键技术意见，Owner批准 |
| 格式/产品功能/公开API/预算/服务配置/阶段 | human gate，禁止自动派发/合并 | Owner |
| 设计者自己的最终合格证 | 不接受自签；实施/设计/最终验收分离 | 独立Reviewer+Owner |

升级材料必须有Task→Run→base/head SHA→artifact fullSHA256→receipt，以及expected/actual、MOCK/LIVE/NOT_RUN、影响文件、审批缺口。Codex可以给设计建议，不能接管DSH常规dispatch loop；KOS只登记知识/证据，Hub仅跨项目治理观测。
