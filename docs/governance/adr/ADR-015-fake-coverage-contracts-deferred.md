# ADR-015: 12 条 ACL/审计链契约测试的显式延后（假覆盖 → 登记在案的未实现）
> **Status**: ACCEPTED
> **Deciders**: omp (macmini) agent + User (李哥) 的裁定待补（双专家席位恢复后需复核）
> **Date**: 2026-11-06
> **Branch scope**: `develop/v4.1.0`
> **Related**: [ADR-001 真实性框架](ADR-001-truthfulness-framework.md),
> [ANTI_FABRICATION_POLICY.md](../../ANTI_FABRICATION_POLICY.md),
> [看板行 SEC-1](../../../releases/v4.1.0/KANBAN_v4.1.0_ALPHA.md),
> [ADR-008 ignore 理由规范](ADR-008-ignore-reason-requirement.md)

---
## Context

### 问题

`crates/security/tests/v400_acl_audit.rs` 里有 **12 条测试**长这样：

```rust
#[test]
fn acl_baseline_fail_closed_on_unknown_user() {
    let contract = "unknown user => deny (fail closed)";
    assert!(!contract.is_empty());
}
```

⇒ 它们只断言**一句描述文字非空**，完全不触及 ACL 判定、审计链校验或任何产品行为。
这些测试**一直是绿的**，因此给「ACL/审计链有覆盖」这个印象提供了**零实质**的支撑。

更糟的是它们**既不占 `#[ignore]` 额度、也不登记进 ignore 台账** ——
即：它们是一笔**未被登记的隐性欠账**，既不受 `active_max` / `total_allowed_max` 的约束，
也不出现在任何「已延后」的清单里。

发现路径：归档 run `20261106-ga5594-004` 的 `cargo clippy --workspace --all-features
--all-targets -- -D warnings` 报出 12 处 `this expression always evaluates to false`
（`assert!(!contract.is_empty())` 在常量上求值 ⇒ clippy 证明它恒真）。

### 根因

- 写这 12 条测试的意图是**把契约（contract）写进代码**，但缺少「契约 → 断言产品行为」这一步，
  于是只留下了一句字符串。
- 仓里对 `#[ignore]` 有完整治理（ADR-008 要求理由、台账登记、额度上限），但对
  **「测试根本没测东西」**没有对应的登记口径 ⇒ 这类测试成了台账外的盲区。

---
## Decision

### D-01: 12 条测试显式标为 `#[ignore]`，带明确理由

```rust
#[test]
#[ignore = "契约未实现：本测试原本只断言一句描述文字非空（没有触及任何产品行为），属于假覆盖；真实实现落地前不得计入覆盖"]
```

理由文本说明**为什么被跳过**（ADR-008 要求），且明确「在真实实现落地前不得计入覆盖」。

### D-02: 全部登记进 `tests/baseline/ignore_registry.json`

- `total_allowed`：123 → **135**
- `limits.active_max`：80 → **92**，并在台账里写明理由（额度随真债显性化而 +12，
  **正好等于新增条目数**，没有放宽对未登记欠账的约束）。
- 12 条新条目的 `status` 为 `ACTIVE`（理由指向未实现行为 ⇒ 按台账口径属「活动欠账」），
  `category` 为 `GAP_DOCUMENTED`。

### D-03: 缺口进看板，不靠「标注」了事

看板新增行 **SEC-1**：这 12 条契约（unknown user/table fail-closed、audit chain 修改/重放检测、
sequence 单调、ACL 大小写敏感、role-based 授权、SQL+vector+graph+audit 原子性、
audit event 关联 tx_id、ROLLBACK 丢弃待追加的审计链、两条延迟目标）**要么落地真实实现并断言
产品行为，要么由属主裁定该契约作废并从文件里删掉**。
`assert!(!contract.is_empty())` 这种形态一个都不许留。

### D-04: 基线按 ADR 流程更新

`tests/baseline/test_count.json` 的 `ignored_tests` 123 → 135（`active_tests` 同步为现值），
`note` 指向本 ADR —— 这就是 `check_test_count_monotonic.sh` 所说的「ADR process」。

---
## Consequences

**好的**：
- 「有 ACL 覆盖」这个印象被拆掉：12 条不再贡献绿色。
- 隐性欠账变成**登记在案的欠账**：受额度约束、可被看板追踪、有明确的关闭条件。
- ignore 台账与现场树重新一致（`check_anti_ignore_gate.sh` 绿：`total_allowed=135`、`active=81 ≤ 92`、
  tree recount 135）。

**代价 / 风险**：
- `active_max` 被抬高 12（80→92）—— 这是**显性化**的代价，不是放宽：额度只跟着
  **已登记**的欠账走，未登记的欠账仍然一条都不许进。
- 这 12 条契约**仍然是没实现的**；本 ADR 不实现它们，只是不再假装它们实现了。
  真正的关闭条件在看板 SEC-1。
- `ignored_tests` 增加触发了 `check_test_count_monotonic.sh` 的 P12 ⇒ 按仓规必须有 ADR
  ⇒ 本文件即该 ADR（并据此更新基线）。

## References

- 归档 run `20261106-ga5594-004`（`cargo clippy --all-targets` 的完整日志；20 条命令，15 绿 5 红）
- 归档 run `20261106-ga5594-005`（本 ADR 落地后的复跑证据）
- 看板行 `SEC-1`（缺口与关闭条件）、`LINT-1`（`--all-targets` 的 279 处欠账分类）
