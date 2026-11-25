# SRG-D004-01 门禁收口 — G3 语义复核 + G6 模型身份协议（DSH-B 侧闭环）

- source_run: autonomy-r1
- 目标：消除 SRG-D004-01 剩余 3 个 PARTIAL 门禁中 DSH-B 可闭环的两项（G3、G6）；G5（出口控制）留 DSH-A/属主。

## 一、G3 验收条件确定性 — 语义复核（响应"4 条但语义待复核"）

四条判据每条的定义、执行命令、退出码语义与"为何是确定性的"：

| # | 判据 | 精确语义 | 执行 | 确定性来源 |
|---|---|---|---|---|
| 1 | **RED 成立（前置）** | `cargo test -p sqlrustgo-storage --test tf_t4_stub_silent_success` 中 tf_t4_04 状态=FAILED 且进程 exit=101 | 已实测（2026-11-20 16:54 与 17:21 两次独立运行一致） | 断言字符串 "UNSUPPORTED_NOT_REJECTED" 由测试固定；cargo 退出码二值 |
| 2 | **GREEN** | 修复后同命令 tf_t4_04=ok（exit 0），且 tf_t4_01/02/03 **仍 FAILED** | `tools/verify_srg_d004_acceptance.py --phase post` | 三态输出固定；"仍 FAILED" 是防扩大修复面的硬判据 |
| 3 | **REGRESSION** | storage lib 全量（undo/rollback/savepoint/transaction 过滤族 + 75 场景基线族）无新增 FAILED；75 基线 70 PASS 不回退 | test_factory（七态分类+summary 交叉校验，反例#1 修复后版本） | 分类确定性 + 交叉校验拒绝盲信 cargo 声明 |
| 4 | **MUTATION_RED** | 在独立 git worktree 中将修复回退为 Ok(0)/Ok(1) → tf_t4_04 再次 FAILED | 同脚本 R4 步骤（临时 worktree，用后即删） | 变异是机械替换（字节级模式匹配）；失败即报 pattern mismatch，不静默 |

判定器：`tools/verify_srg_d004_acceptance.py`（exit 0=判据全成立；exit 1=fail-closed）。**四条判据中无任何"模型主观判断"**；Review 的职责是审 patch 最小性与是否有越权改动，不是替代测试判据。

## 二、G6 模型身份 — 执行前后二次校验协议（DSH-B 实现并即刻生效）

问题：门禁要求模型身份在任务执行前后一致（防执行中途换模型/换端点）。

协议（对所有 LOCAL_ONLY 模型任务生效）：

1. **PRE**：`GET /v1/models` 记录 model id + n_ctx → 写入任务 evidence（`model_identity.pre`）。
2. **EXEC**：每次 chat 请求响应中的 `resp.model` 字段与 PRE 记录比对，不一致立即中止任务（BLOCKED，证据记录）。
3. **POST**：再次 `GET /v1/models` → `model_identity.post`；PRE==POST 且 ==EXEC 才算门禁过。
4. 证据三段全部入账本 note 与 Receipt。

脚本化实现：`dsh-b/c01-prep/model_identity_guard.py`（本轮交付，后续派发直接复用）。

## 三、门禁矩阵更新（DSH-B 视角）

| 门禁 | 状态 | DSH-B 动作 |
|---|---|---|
| G1/G2/G4/G7 | ✅ PASS | — |
| G3 | ✅ 语义复核完成（本文件 §一） | 待 DSH-A 复核确认 |
| G6 | ✅ 协议已实现（本文件 §二） | 首次执行时实测 |
| **G5 出口控制** | ⚠️ PARTIAL（凭据✅ 出口❌） | **DSH-A/属主**：pf 白名单（OD-002） |

**结论：DSH-B 侧可闭环的门禁已全部闭环；SRG-D004-01 只剩 G5（出口控制）一个外部依赖。**

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)
