# DSH-A 互通验证报告（C01 任务接入就绪）

- source_run: goal-R7
- timestamp: 2026-11-20T17:31:03+08:00 起
- 依据：DSH-A `docs/multi-project/SQLRUSTGO-TASK-HANDOFF-SPEC.md`（14 字段 digest 契约）+ `scheduler/handoff.py`（strict 校验）

## 互通实测

| 项 | 结果 |
|---|---|
| Codex DAG 18 包 → DSH-A 14 字段载荷转换 | ✅ 全部转换成功 |
| `scheduler.handoff.validate(strict=True)` | ✅ **18/18 PASS，0 HandoffViolation** |
| 数据隔离 | ✅ 外发载荷仅含 digest/路径 glob/风险/预算——**零源码、零 diff、零日志** |
| LOCAL_ONLY 标记 | ✅ execution_mode=LOCAL_ONLY 全部显式 |

## 提交就绪文件

- `dsh-b/C01-DSHA-SUBMISSION-READY.jsonl`：18 条载荷（task_id/objective_digest/acceptance_digest/allowed+forbidden globs/risk/budget/base_sha）
- 原文（objective/acceptance 全文）保留在 Codex DAG 与本地任务 JSON 中，双方以 digest 校验一致性

## ChatGPT-web 架构咨询采纳记录（consult-002，2026-11-20 17:44 返回，807.6s）

| 建议 | DSH-B 处置 |
|---|---|
| Task B-E 立即推进，不等 manifest 重签 | ✅ **已照做**（B/C/D/E 四项评测本轮前已完成） |
| 252=开发/反例、8011=深度审核、8021=常规审核；250 双端点错峰 | ✅ 采纳为执行器路由规则 |
| manifest 验真后 18 包按 **2+4+12** 分批导入；导入≠执行授权 | ✅ 采纳（第一批= C01-01 + C01-03） |
| 75 复跑三轮：15（阻断/假PASS/证据失真）→30→30，同冻结版本全量验收 | ✅ **已超额完成**（75/75 全量一次完成：70 PASS/5 RED，base 冻结 e9ff21fca3） |
| Q3 Reviewer 独立性：252 不得自审放行；8021 过程独立（非血统独立）；8011 异血统审 P0/架构契约 | ✅ 与现有独立性标注一致，固化 |
| Q4 哈希漂移：**必须 Codex 重签**（附 diff/原因/绑定 commit），DSH 独立复算 111/111 后解除阻断；Codex 无法重签则维持 HANDOFF_BLOCKED | ✅ 采纳——与既有立场一致（不代改 Codex 产物） |

咨询原文：`dsh-b/artifacts/consult-002-reply.md`；账本 ARCH-ADVISOR-001 attempt=3（remote 记账）。

## 派发前置条件（不变，冻结中）

1. **G3**：Codex 重签 manifest（3 文件哈希漂移）→ 否则 C01-01/C01-18 的哈希断言必失败
2. **属主批准**首批派发（授权数当前=0，包自述 CANDIDATE_NOT_AUTHORIZED）
3. file_count/line_stats 当前为 0 占位——派发前由实际任务包补充
4. 派发顺序建议：C01-01（L0 校验）→ C01-03/04（LOCAL_ONLY 观察 + 结果分类）→ 并行只读族 → 写入族串行（并发 1）

## DSH-B / DSH-A 边界（现行有效）

- DSH-B：产出并维护任务元数据、本地模型任务执行、测试验收、证据账本
- DSH-A：公共调度、租约执行、handoff 校验、资源仲裁（GMP 优先）
- 双方不做对方的事；本报告的载荷文件即 DSH-B → DSH-A 的正式输入格式

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)
