# DSH-A / DSH-B · 运行时架构裁决正式交接

source_agent=Codex GPT-6.1 Sol；source_run=RUNTIME-5915-20261120；timestamp=2026-11-20 Asia/Shanghai；evidence_hash=sha256:57fe60174b4642317a534de402027e42603ba976601a8ed1ff4916cb9fb7df2a；conflict_resolution=正式交付待审材料，不授予核心实施或Worker派发权限。

本包是ARCHITECTURE_REVIEW_NOTICE，**不是Task enqueue、Permit、M0/M1准入或执行指令**。它不撤销DSH已有其他授权，也不因为送达就授予本统一方案实施权；本轮不派发Worker。Owner批准引用为空。

- DSH-A：接收元数据/绑定摘要并登记Owner待批，维护调度边界，不自行批准此核心协议。
- DSH-B：保留#5915原断言、原装配、raw日志和失败状态；将INV-07b描述限定为正常对象释放/flush/reopen，真正crash/recovery仍NOT_RUN；不能把改描述当修复。
- Codex：交付统一契约/证据分级/迁移与反例矩阵；不成为日常调度器、不自签最终独立合格证。

沿用两层既有协议：本地AgentHandoff schema（native_handoff.jsonl，双接收方）与DSH-A的14字段strict元数据（dsh_metadata.jsonl）。二者都是材料描述，没有live队列/lease/run写入。raw源码/日志留本地或受控Gitea资料目录；外发元数据只有hash、路径、风险和预算，不把源码/日志塞进allowed字段，也不以云端Controller解释这些完整内容。

01_LOCAL_BINDING.json含完整本地objective/acceptance及hash算法；MANIFEST.json绑定本包所有内容、输入源、ADR与原45da包。14字段file_count/line_stats按MANIFEST内容文件真实计数，不用占位零；其budget_seconds/attempts仅元数据预检预算，不是Worker/Token额度。

校验：`python3.14 tools/verify_packet.py --check`（cwd为本包），不会启动Scheduler、模型、Worker或服务。首项校验实际调用冻结的DSH handoff.validate(strict=True)；Schema通过不等于Owner批准或LOCAL_ONLY端到端已证。

回执要求：receiver、received artifact commit、manifest SHA256、ADR SHA256、schema/module SHA256、验证结果、Owner approval ref（当前null）、core_implementation_authorized=false、worker_dispatch_requested=false、unresolved项。没有真实接收回执前不得伪造DSH签收/批准。发送聊天通知只传材料定位和摘要，既有授权与Owner审批机制保持原样。
