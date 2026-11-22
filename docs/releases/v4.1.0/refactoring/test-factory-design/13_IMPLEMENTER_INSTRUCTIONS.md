# 实现器指令

1. 只能按 `09_IMPLEMENTATION_TASK_DAG.jsonl` 的依赖顺序工作；一次只领取一个包。
2. 每包从固定 base 创建独立 worktree；先验证 allowed/forbidden paths。
3. 先写会失败的正向契约与反例测试，再写最小实现；不得先改断言适配现状。
4. 不得把 Gap Probe、SKIP、零测试、编译失败、超时或基础设施错误记为 PASS/EXPECTED_RED。
5. 不得修改产品核心事务实现；若测试暴露需要产品改动，停止并提交 HUMAN_GATE 材料。
6. 不得修改正式 DAG、`approved_adr_refs`、STAGE、release/tag、受保护分支或 Issue 状态。
7. 每个 claim 提供 source_agent、source_run、timestamp、evidence_hash、conflict_resolution。
8. DSH-B 是协调者而非独立验收者；最终验收必须换 Reviewer 和干净 worktree。
9. `token_budget=OWNER_TO_AUTHORIZE` 表示未授权，不得自行填数或派发。
10. 任何开放式设计缺口立即停止，不由 MiniMax 自行补决策。

提交前必须运行适用的目标测试、格式检查、manifest honesty、目标执行完整性和 GitNexus change detection；工具不可用时状态为 INCOMPLETE，不得替代性宣称通过。
