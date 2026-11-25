# DSH正式交接包

> source_agent: Codex GPT-6.1 Sol · source_run: C-ALPHA-01-HANDOFF-20261120 · timestamp: 2026-11-20 (Asia/Shanghai)
> evidence_hash: sha256:4649fccc566613f04568ef3564e9d585739c4e92c73ad5972714b7d4f65b992f（HARA 输入 manifest）；本轮来源及产物 hash 见 evidence/SOURCE_BINDINGS.json 和 handoff/10_HANDOFF_MANIFEST.json。
> conflict_resolution: Owner 裁决优先；本文 PROPOSED，计划交付不等于 ADR 批准、任务授权或产品认证。

本包供Owner批准后交给DSH（deepseek-schedule）接管。**当前授权运行数=0，未enqueue、未启动任何执行Agent。** C路线原则/职责已裁决，具体十ADR、LOCAL_ONLY、资源和执行仍待批准。

实际内容目录：[release/refactoring](../../../releases/v4.1.0/refactoring/README.md)，HARA原件在其audit/HARA-410。交接03是现有AgentTask原生JSONL，补充安全/依赖/token约束唯一保存在release的REFACTOR_TASK_DAG.json；04只哈希引用它。不修改DSH schema、数据库或队列，不创建第二个调度器。

接管顺序：1 Owner/DSH离线核验10 manifest及01来源；2核对DSH/KOS live schema是否仍与绑定一致；3复核18任务/75来源场景与全部批准项；4补齐LOCAL_ONLY与导入强制绑定，未证则HOLD；5 Owner另发正式接管批准，DSH才可选出已批准的小批任务。当前不得运行hub enqueue、hub run --real或修改EXECUTE_ENABLED。

源码任务base固定e9ff，计划包在另一个文档提交；二者不能混淆。Owner批准时须同时固定已发布文档commit及其manifest，DSH以只读checkout提供任务上下文；C01-01/18的交付校验在该只读包运行，其余命令在任务资产实现后的task worktree运行。禁止从未固定的latest分支默默读取计划，也不假定旧base里已有新交接文件。

交付验证（无模型/调度/服务副作用）：在本PR独立checkout执行 `python3.14 docs/releases/v4.1.0/refactoring/tools/verify_handoff.py --check`。sha256和本地schema只证明包一致性，不证明运行认证。不存在一个通过schema后自动执行的入口。

18个首批包只有C-ALPHA-01准备/测试/只读分析，M1–M6只有锁定epic。未来普通任务可候选自动执行，但现在没有已授权任务。DSH负责日常轮询、资源/租约、工作树、失败整改和状态；OpenCode实施、OMP测试/独立审查，Codex仅处理架构升级。
