# Test Design

> source_agent: Codex GPT-6.1 Sol · source_run: C-ALPHA-01-HANDOFF-20261120 · timestamp: 2026-11-20 (Asia/Shanghai)
> evidence_hash: sha256:4649fccc566613f04568ef3564e9d585739c4e92c73ad5972714b7d4f65b992f（HARA 输入 manifest）；本轮来源及产物 hash 见 evidence/SOURCE_BINDINGS.json 和 handoff/10_HANDOFF_MANIFEST.json。
> conflict_resolution: Owner 裁决优先；本文 PROPOSED，计划交付不等于 ADR 批准、任务授权或产品认证。

离线verify_handoff使用python3.14已有jsonschema Draft202012Validator，读取复制的DSH真实Schema；DAG以拓扑检查、显式external human gates判缺失/环；hash拒绝任一字节漂移。Mutation checks必须覆盖依赖环、丢依赖、将dispatch打开、批准未批ADR、native额外字段、hash损坏；验证器必须拒绝这些输入，不能仅正向PASS。

MOCK观察原pilot空测试exit0→PASS和DSH failure→PASS_TO_REVIEW只证明边界，未修改他方程序，也不宣称绕过最终Review成功。新的架构反例设计应交DSH执行，本轮不启动Scheduler。
