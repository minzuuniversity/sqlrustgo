# SQLRustGo v4.1.0 C 路线重构资料入口

> source_agent: Codex GPT-6.1 Sol · source_run: C-ALPHA-01-HANDOFF-20261120 · timestamp: 2026-11-20 (Asia/Shanghai)
> evidence_hash: sha256:4649fccc566613f04568ef3564e9d585739c4e92c73ad5972714b7d4f65b992f（HARA 输入 manifest）；本轮来源及产物 hash 见 evidence/SOURCE_BINDINGS.json 和 handoff/10_HANDOFF_MANIFEST.json。
> conflict_resolution: Owner 裁决优先；本文 PROPOSED，计划交付不等于 ADR 批准、任务授权或产品认证。

本目录是 release 4.1.0 下独立的架构/计划归档。HARA 原件在 [audit/HARA-410](audit/HARA-410/01_EXECUTIVE_ARCHITECTURE_DECISION.md)，逐字节保存原 manifest；后续计划在本目录，十份 ADR 在 adrs/，DSH 正式交接包在 [docs/refactor/v4.1.0-c/handoff](../../../refactor/v4.1.0-c/handoff/00_HANDOFF_README.md)。只有这套内容主目录，不复制维护第二套计划。

阅读顺序：[主计划](C_REFACTOR_MASTER_PLAN.md) → [证据基线](HARA_EVIDENCE_BASELINE.md) → [ADR](ADR_INDEX.md) → [准入报告](ALPHA01_READINESS_REPORT.md) → DSH交接。

状态：C路线原则接受；组织职责按属主最终指令；十ADR待审；DSH自动接管未授权；M-Pre未关闭；M0正式接口实现/M1–M6禁止启动；ALPHA不变。历史报告中“未push”等描述保留为原审计时点事实，不改写为本轮发布状态。

HARA执行脚本保留原目录/相对路径假设，原样归档不意味着新位置可直接运行。TF资产适配任务必须在独立worktree重建原布局或显式传root，不能改写旧证据。当前75场景试点是输入，不把其声明的68个PASS带入新认证。
