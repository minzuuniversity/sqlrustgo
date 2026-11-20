# 里程碑出口与human gate

> source_agent: Codex GPT-6.1 Sol · source_run: C-ALPHA-01-HANDOFF-20261120 · timestamp: 2026-11-20 (Asia/Shanghai)
> evidence_hash: sha256:4649fccc566613f04568ef3564e9d585739c4e92c73ad5972714b7d4f65b992f（HARA 输入 manifest）；本轮来源及产物 hash 见 evidence/SOURCE_BINDINGS.json 和 handoff/10_HANDOFF_MANIFEST.json。
> conflict_resolution: Owner 裁决优先；本文 PROPOSED，计划交付不等于 ADR 批准、任务授权或产品认证。

| 阶段 | 确定性证据 | 独立复核 | human gate |
|---|---|---|---|
| C-ALPHA-01 | 原件hash、native schema、DAG、反例/资产对账、LOCAL_ONLY负向试验 | OMP/独立Reviewer；Codex非自签 | 10ADR、支持/隔离/格式/资源、DSH接管授权 |
| C-ALPHA-02 | RowId/undo/workspace/savepoint全入口；已有旧红的新绿/变异红 | 非实施harness审查 | M1实现和范围批准；不认证durable |
| C-ALPHA-03 | file+parallel组合、ACK/崩溃/checkpoint/恢复/隔离 | 存储恢复专审 | 持久化/隔离承诺与故障模型 |
| C-ALPHA-04 | 全DML/后端/历史迁移/外部消费者 | 兼容性专审 | 功能删减/不兼容格式单列批准 |
| C-ALPHA-05 | 正式CI/回归/SOAK/SSOT阶段门禁实际执行 | Independent Judge | Owner阶段批准；保持原STAGE机制 |

此表是计划依赖，不修改任何scripts/gate、GATE_CATALOG、STAGE_CONFIG或数值。必需项NOT_RUN/KNOWN_FAIL/未批准UNSUPPORTED不计PASS。没有降低既有M-Pre出口；新内核尚不存在，不要求它在准备阶段全绿，但原调查缺口必须保留并裁决。
