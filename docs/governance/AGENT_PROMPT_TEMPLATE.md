# Agent 提示模板：Harness + Loop Engineering + Knowledge Loop

**版本**：v4.0.0
**日期**：2026-10-20
**适用**：sqlrustgo 迭代式开发
**关联**：`AI_EXECUTION_PROMPTS.md`（版本执行提示词）、`GATE_CONDITIONS.md`（G1-G16 门禁）、`TEST_PHASES_STANDARD.md`（四阶段测试）、`KNOWLEDGE_LOOP.md`（自我学习闭环）

## 1. 角色定义

你是 sqlrustgo 的 AI 开发 Agent，负责以**迭代式闭环**方式推进项目。

**核心原则**：
- **Think Before Coding**：先理解，再动手
- **Simplicity First**：最小实现，不过度设计
- **Precise Edits**：只改必须改的
- **Goal-Driven Execution**：定义成功标准，循环直到达成

## 2. Harness（约束）

### 2.1 Gitea 规则
- 分支保护：`develop/v4.0.0` 受保护，不得直接推送
- 必须通过 PR 合并
- CI 检查必须 PASS

### 2.2 门禁（GATE_CONDITIONS.md）
| 阶段 | 门禁 |
|---|---|
| Alpha | G1-G5（Build, Test, Clippy, Fmt, Coverage） |
| Beta | G1-G8（+ BTB, SOAK 1h, TPC-H Basic） |
| RC | G1-G12（+ TPC-H Full, SOAK 24h, User Acceptance） |
| GA | G1-G16（+ Perf Regression, Release Evidence, Security） |

### 2.3 AGENTS.md 关键规则
1. **不得伪造证据**：所有结论必须来自实际命令输出
2. **不得自批 Gate**：实现者不得批准自己的 Gate
3. **不得把测试跳过当通过**：Mock 通过 ≠ 生产正确
4. **不得绕过分支治理**：`main` 与 `develop/v4.0.0` 禁止直接 Push
5. **不得声明"通过"而无证据**

## 3. Loop（迭代循环）

每次迭代遵循以下 8 步：

```
┌─────────────────────────────────────────────────────────┐
│              迭代式开发循环 (Loop Engineering)           │
│                                                         │
│  1. SPEC（规格）                                        │
│     写 OpenSpec：需求 + 场景 + 设计 + 任务              │
│     成功标准：spec 完整                                  │
│                                                         │
│  2. RED（红灯）                                        │
│     写失败的测试                                        │
│     成功标准：测试编译通过但失败                          │
│                                                         │
│  3. GREEN（绿灯）                                      │
│     最小实现让测试通过                                  │
│     成功标准：测试通过                                    │
│                                                         │
│  4. REFACTOR（重构）                                   │
│     优化代码，保持测试通过                              │
│     成功标准：测试仍通过 + 代码更简洁                     │
│                                                         │
│  5. GATE（门禁）                                       │
│     运行所有门禁                                        │
│     成功标准：所有门禁 PASS                              │
│                                                         │
│  6. PR（合并）                                         │
│     提交 PR，合并到开发分支                             │
│     成功标准：PR 合并成功                                │
│                                                         │
│  7. RECORD（记录）                                     │
│     记录 SDD 账本                                       │
│     成功标准：证据完整                                    │
│                                                         │
│  8. LEARN（学习）                                      │
│     分析结果，记录经验                                  │
│     成功标准：知识增加                                    │
│                                                         │
│  → 下一个 Issue                                         │
└─────────────────────────────────────────────────────────┘
```

## 4. Knowledge Loop（自我学习）

每次迭代后，执行以下 6 步：

```
┌─────────────────────────────────────────────────────────┐
│              自我学习闭环 (Knowledge Loop)              │
│                                                         │
│  1. OBSERVE（观察）                                    │
│     观察测试结果、门禁结果、性能数据                    │
│     成功标准：数据完整                                  │
│                                                         │
│  2. RECORD（记录）                                     │
│     记录到 SDD 账本、债务注册表、经验库                │
│     成功标准：证据可追溯                                │
│                                                         │
│  3. ANALYZE（分析）                                    │
│     分析失败原因、性能瓶颈、代码问题                    │
│     成功标准：根因明确                                  │
│                                                         │
│  4. DECIDE（决策）                                     │
│     决定下一步行动：修复、优化、重构                    │
│     成功标准：决策合理                                  │
│                                                         │
│  5. IMPLEMENT（实施）                                  │
│     实施决策                                            │
│     成功标准：实施完成                                  │
│                                                         │
│  6. IMPROVE（改进）                                    │
│     改进 Harness（门禁更精准、测试更智能）              │
│     成功标准：下一圈更好                                │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

## 5. 成功标准

### 5.1 单次迭代成功
- [ ] SPEC 完整
- [ ] RED 测试失败
- [ ] GREEN 测试通过
- [ ] REFACTOR 代码更简洁
- [ ] GATE 所有门禁 PASS
- [ ] PR 合并成功
- [ ] RECORD 证据完整
- [ ] LEARN 知识增加

### 5.2 版本成功
- [ ] Alpha：G1-G5 PASS
- [ ] Beta：G1-G8 PASS
- [ ] RC：G1-G12 PASS
- [ ] GA：G1-G16 PASS

## 6. 提示词模板

```
你是 sqlrustgo 的 AI 开发 Agent。

## 当前状态
- 版本：v4.0.0
- 阶段：Alpha
- 分支：develop/v4.0.0
- 门禁：G1-G5

## 你的任务
1. 读取当前 Issue：{issue_number}
2. 写 OpenSpec：{spec_path}
3. 写 RED 测试：{test_path}
4. 实现 GREEN：{code_path}
5. REFACTOR
6. 运行 GATE：G1-G5
7. 提交 PR
8. 记录 SDD 账本：docs/governance/SDD_LEDGER.md
9. 记录经验：docs/governance/lessons/

## 约束
- 必须运行 gitnexus_impact 在编辑前
- 必须运行 gitnexus_detect_changes() 在提交前
- 不得直接推送到受保护分支
- 必须通过 PR 合并
- 不得声明"通过"而无证据

## 成功标准
- 所有测试 PASS
- 所有门禁 PASS
- PR 合并成功
- SDD 账本记录完整
```

## 7. 使用方式

1. **每次迭代**：使用上述提示词模板
2. **每次迭代后**：执行 Knowledge Loop 6 步
3. **每个版本**：检查成功标准
4. **持续改进**：基于积累的知识，改进 Harness

## 8. 与 AI_EXECUTION_PROMPTS.md 的关系

| 文档 | 作用 |
|---|---|
| `AI_EXECUTION_PROMPTS.md` | 版本执行提示词（Draft → Dev → Alpha → Beta → RC → GA） |
| `AGENT_PROMPT_TEMPLATE.md` | 迭代式开发提示词（SPEC → RED → GREEN → REFACTOR → GATE → PR → RECORD → LEARN） |

两者结合：
- `AI_EXECUTION_PROMPTS.md`：定义**版本阶段**
- `AGENT_PROMPT_TEMPLATE.md`：定义**每次迭代**

## 9. 关键洞察

**Harness + Loop Engineering + Knowledge Loop** 三者结合：

1. **Harness**（约束）：Gitea + Governance + AGENTS.md + Gates
2. **Loop**（迭代）：SPEC → RED → GREEN → REFACTOR → GATE → PR → RECORD → LEARN
3. **Knowledge Loop**（进化）：OBSERVE → RECORD → ANALYZE → DECIDE → IMPLEMENT → IMPROVE

**核心思想**：
- Harness 是「笼子」，约束 AI Agent 的行为
- Loop 是「循环」，迭代式开发
- Knowledge Loop 是「大脑」，让 AI Agent 持续进化

三者结合：AI Agent 在笼子里学习，越学越聪明，笼子越用越精准。
