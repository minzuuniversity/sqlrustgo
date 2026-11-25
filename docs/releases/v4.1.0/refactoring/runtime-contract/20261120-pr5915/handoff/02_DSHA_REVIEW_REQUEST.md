# DSH-A材料接收请求

source_agent=Codex GPT-6.1 Sol；source_run=RUNTIME-5915-20261120；timestamp=2026-11-20 Asia/Shanghai；evidence_hash=sha256:57fe60174b4642317a534de402027e42603ba976601a8ed1ff4916cb9fb7df2a；conflict_resolution=正式交付待审材料，不授予核心实施或Worker派发权限。

请登记ADR-410-011统一架构裁决材料及本包manifest，提交Owner Decision Queue，当前只作材料接收/哈希和协议核验。不要将本消息当Task enqueue、Permit或Worker派发请求，不修改任何核心实现/资源/阶段。

必须绑定产品e9ff21fca3、测试PR #5915@56cfa222f1、历史设计包45da6968ae和本包已发布文档commit，不能用latest分支替代。只传14字段最小元数据；内容审查与Owner/本地授权边界分开，不能把摘要格式验证当批准。

Owner待批：稳定持久RowId/格式方向、TxStore唯一Undo+私有workspace、提交/durable frontier/恢复、隔离/功能范围、迁移风险与独立验收安排。请仅回传材料签收及未决项，不从本请求扩展任何执行权限。
