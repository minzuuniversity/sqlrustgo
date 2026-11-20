# 提交范围与文档工作计划

> source_agent: Codex GPT-6.1 Sol · source_run: C-ALPHA-01-HANDOFF-20261120 · timestamp: 2026-11-20 (Asia/Shanghai)
> evidence_hash: sha256:4649fccc566613f04568ef3564e9d585739c4e92c73ad5972714b7d4f65b992f（HARA 输入 manifest）；本轮来源及产物 hash 见 evidence/SOURCE_BINDINGS.json 和 handoff/10_HANDOFF_MANIFEST.json。
> conflict_resolution: Owner 裁决优先；本文 PROPOSED，计划交付不等于 ADR 批准、任务授权或产品认证。

SPEC：只新增release重构资料目录和docs/refactor交接入口，逐字节归档HARA九报告+全部证据，新增13规划交付物（含JSON）、十ADR、12交接文件。无正式核心接口、依赖、Cargo、stage/tag/门禁/权限/生产服务改动。

用户明确要求提交报告到Gitea252，是本次独立文档分支commit/push/PR的授权；不扩展为后续重构或DSH派发授权。正式develop不直接写，不自动合并。

文档流程：核验原件/当前基线→列新增目录/来源/审批状态→apply_patch撰写、原件cp复制→schema/hash/DAG/link/diff检查→工作记录/PR五类栏目→仅推文档分支。既有他方worktree及未提交文件不改、不回退。
