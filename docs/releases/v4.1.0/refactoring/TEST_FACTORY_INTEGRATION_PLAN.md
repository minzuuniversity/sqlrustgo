# Test Factory接入计划

> source_agent: Codex GPT-6.1 Sol · source_run: C-ALPHA-01-HANDOFF-20261120 · timestamp: 2026-11-20 (Asia/Shanghai)
> evidence_hash: sha256:4649fccc566613f04568ef3564e9d585739c4e92c73ad5972714b7d4f65b992f（HARA 输入 manifest）；本轮来源及产物 hash 见 evidence/SOURCE_BINDINGS.json 和 handoff/10_HANDOFF_MANIFEST.json。
> conflict_resolution: Owner 裁决优先；本文 PROPOSED，计划交付不等于 ADR 批准、任务授权或产品认证。

复用DSH已有queue/lease/adapter/receipt、SQLRustGo已有tools/test_factory.py试点、现有sqlrustgo_sqllogictest crate和TF75manifest；不建立新Controller、Scheduler或通用Runner。本轮只新增离线交付校验器，不执行任务派发。

先处理pilot的空测试PASS、raw日志丢失、ignored被统一归UNSUPPORTED、KNOWN_FAIL与预期匹配、缺run_id/完整hash；T1“日志不丢重复条目”不能替代实际RowStore identity oracle。既有试点未进DSH，五项申报经hold报告降为2CONFIRMED+3PARTIAL；本轮只验证源码与契约反例，不引用其919/4/33为新实跑。

TEST_COVERAGE_MATRIX.json保留75条来源场景，运行状态全标本轮NOT_RUN，历史claims单列；HARA F01–F11作为11个证据绑定家族，不能把一个probe输出行当多个独立成功用例。TFv0.1的数量目标不是“75条发现就认证75条”。

SQLLogicTest已有crate使用sqllogictest0.29、MemoryExecutionEngine和broadcast hub，不代表默认file/parallel。C01-15仅接入自写最小fixture并复用runner；外部SQLite公共测试/corpus导入前需冻结版本、文件hash及license，TH3禁止；SQLite共同SQL子集与MySQL方言分开，不从SQLite事务语义推导产品承诺。

七态：PASS、FAIL、KNOWN_FAIL、UNSUPPORTED、NOT_RUN、DIALECT_DIFFERENCE、INFRA_ERROR。JSONschema外层DSH result只能success/failure/skipped/INCONCLUSIVE：仅真正PASS映射success；FAIL/KNOWN_FAIL映射failure，NOT_RUN/INFRA_ERROR映射INCONCLUSIVE，其他映射skipped并保留原因及Owner批准引用。必需case任何非PASS不得认证成功；Gatekeeper PASS_TO_REVIEW只代表送审。

每条case必须含ID/feature/invariant/source/seed/backend/assembly/baseSHA/expected/actual/execution status/full artifact hash。command存在不等于执行，基础设施阻断必须单列；保留完整stdout/stderr、exit、实际发现/执行数量。独立OMP审测试和oracle，实施者不得自行改期望刷绿。
