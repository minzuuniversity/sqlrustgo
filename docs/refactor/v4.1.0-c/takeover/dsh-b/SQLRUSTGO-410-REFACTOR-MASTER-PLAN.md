# SQLRUSTGO-410-REFACTOR-MASTER-PLAN（DSH-B 执行版）

- source_agent: DSH lead / DSH-B (opencode-go/glm-5.3-flash)
- source_run: dshb-first-round-20261120-4
- timestamp: 2026-11-20T16:21:03+08:00 起本轮
- 定位：属主 DSH-B 指令的首轮交付物 #1。**不复制 Codex 主计划正文**——以哈希绑定引用其权威版本，DSH-B 只补充执行序列与责任映射。

## 一、权威来源绑定（不重复建设）

| 权威文档 | 位置（Codex 交接包） | DSH-B 使用方式 |
|---|---|---|
| C_REFACTOR_MASTER_PLAN.md | docs/releases/v4.1.0/refactoring/ | 引用+按其阶段执行 |
| 10 个 ADR（ADR-410-001~010） | 同上 adrs/ | 作为冻结契约，修改需属主→Codex |
| REFACTOR_TASK_DAG.json（18 包） | 同上 | DSH-B 执行池（⚠️ manifest 漂移待重签） |
| TEST_COVERAGE_MATRIX.json（75 场景） | 同上 | 复跑验收对象（reexecution=0，声明≠验收） |
| HANDOFF_VALIDATION_V2.json | takeover/control-plane/ | 导入门禁状态 |

## 二、执行序列（C-ALPHA-01，全部为准备/测试/只读，无核心编码）

```
S1 反例清除（本轮已完成一半）
  S1.1 gatekeeper FAIL 检查修复（deepseek-schedule）✅ 已完成+4回归测试（11 passed, 全量 143+1s）
  S1.2 pilot test_factory 零测试 exit0→PASS 修复 ⬜ 待做（试点工具属主确认归属后）
  S1.3 Codex 重签 manifest（3 文件漂移）⬜ 等待 Codex
S2 任务导入接线（依赖 S1.1✅ + S1.3）
  S2.1 deepseek-schedule Dispatcher 接线 18 包（租约/幂等已备）⬜
  S2.2 G1-G10 复跑（G3 通过后）⬜
S3 能力评测 Task B-E（S1.1 完成即可启动）
  S3.1 Task B 固定反例写测试（252 实现/8011 审）⬜
  S3.2 Task C 局部修复、D 互审、E 恢复 ⬜
S4 Test Factory 复跑（75 场景全部实跑分类，含 4 KNOWN_FAIL 复现）⬜ 依赖 S2/S3
S5 C-ALPHA-01 出口 → 提交属主审批（不自动晋级）
```

## 三、里程碑映射（沿用 Codex 五阶段，保持锁定）

C-ALPHA-02…05：**HUMAN_GATE_LOCKED**。每阶段出口提交 Codex 独立审核 + 属主验收。

## 四、本轮决策记录

- 角色拆分：DSH-A=全局调度/GMP 优先；DSH-B=本专项技术总控（本会话）；不建第三调度器。
- ChatGPT-web 咨询通道：属主授权（16:40 指令），仅限**最小任务元数据**（不含受保护源码/完整 diff/敏感日志），CONTROL_BOOTSTRAP 域记账，不冒充 LOCAL_ONLY。

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)
