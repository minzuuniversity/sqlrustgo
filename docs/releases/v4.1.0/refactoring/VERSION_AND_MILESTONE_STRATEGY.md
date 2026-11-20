# 版本与内部里程碑

> source_agent: Codex GPT-6.1 Sol · source_run: C-ALPHA-01-HANDOFF-20261120 · timestamp: 2026-11-20 (Asia/Shanghai)
> evidence_hash: sha256:4649fccc566613f04568ef3564e9d585739c4e92c73ad5972714b7d4f65b992f（HARA 输入 manifest）；本轮来源及产物 hash 见 evidence/SOURCE_BINDINGS.json 和 handoff/10_HANDOFF_MANIFEST.json。
> conflict_resolution: Owner 裁决优先；本文 PROPOSED，计划交付不等于 ADR 批准、任务授权或产品认证。

使用C-ALPHA-01..05作工程标识，M-Pre/M0..M6是依赖阶段，不是Git tag或STAGE状态。暂不创建refactor集成分支、alpha.1..5、beta.1或rc.1标签。当前文档分支codex/v410-calpha01-docs不是核心集成线。

现有真实v4.1.0-alpha1和建议的v4.1.0-alpha.1不同，前者有撤销注释。命名空间不同不能当可安全重用编号的证据。tags实时快照见evidence/gitea-tags.json；正式命名由Owner按VERSION_PROMOTION_SOP与STAGE_CONFIG决定，禁止改写旧tag。

STAGE.yaml:current_stage=ALPHA是权威阶段；Cargo workspace version=4.1.0，入口文档仍称3.12.0-fix-zombie的漂移不由本轮修正。ALPHA/BETA/RC/GA及human_gate只读既有SSOT；预算/交接JSON不是另一份阶段主账。

所有中间里程碑禁止生产使用。公开API、磁盘兼容或功能承诺若破坏，提交独立版本策略ADR；不能自动推断v4.2即可容纳不兼容公开API。无本轮阶段晋级。
