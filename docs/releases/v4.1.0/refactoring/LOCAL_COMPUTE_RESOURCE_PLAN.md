# 本地算力与执行预算

> source_agent: Codex GPT-6.1 Sol · source_run: C-ALPHA-01-HANDOFF-20261120 · timestamp: 2026-11-20 (Asia/Shanghai)
> evidence_hash: sha256:4649fccc566613f04568ef3564e9d585739c4e92c73ad5972714b7d4f65b992f（HARA 输入 manifest）；本轮来源及产物 hash 见 evidence/SOURCE_BINDINGS.json 和 handoff/10_HANDOFF_MANIFEST.json。
> conflict_resolution: Owner 裁决优先；本文 PROPOSED，计划交付不等于 ADR 批准、任务授权或产品认证。

DSH唯一执行控制器，Codex本会话属Owner主动架构例外，不计为Worker本地零云成绩。SQLRustGo全部自动执行语义model_policy=LOCAL_ONLY，覆盖planner/coder/reviewer/摘要/重试/bridge/子agent/检索；Gitea252访问不是推理出口。

当前DSH routing.json active_strategy=LOCAL_FIRST；LOCAL_ONLY仅全局过滤cloud-前缀。OMP/OpenCode适配器Popen未传隔离env，OpenCode模型可未指定，子进程继承环境；端点/模型/credential/egress/子agent需端到端负向验证。旧报告模型DOWN不视为当前健康事实；本轮不调用模型、不改服务/端口/配置。

候选本地27B/35B只是任务粒度目标，实际模型ID、上下文、工具能力、吞吐由DSH在独占资源窗口验证。无法核验则BLOCKED_MODEL_IDENTITY/RESOURCE；不升级云端。普通任务OpenCode，测试/独立review OMP；这是未来职责，不是本轮调度选择。

首批18包token上限合计见DAG，属于PROPOSED总尝试预算，低于首批5M建议帽，非已批额度。任务最多两次尝试，无新证据不得重试；资源失败不升级模型。全路线用户建议17–34/15–30/45–100/30–69/15–37M合计122–270M仅规划假设，不承诺工期或一次授权，按各阶段成本重估。

记录prompt/completion/cache、prefill、吞吐、编译/测试、排队、重试和人工审查，未知记null不写0。GMP数据库/CI/备份/演练/联调优先；无采购、无GPU抢占、无GMP真实数据，crash/fuzz/SOAK必须有独占资源准入。当前扩派=禁止。
