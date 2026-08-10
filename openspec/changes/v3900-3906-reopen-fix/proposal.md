## Why

codex 在 #3900 (comment #88732, 2026-08-10T11:32) 重新打开反馈 (再次):
1. C-ARCH-05 (`execution_engine.rs` 1756 lines > 1600 limit) 仍然失败 — 此项 pre-existing, 需明确转入 #3906/#3942 follow-up 或其他 Issue
2. **#3887 总控 body 仍显示 `- [ ] #3900` (未勾选)** — 关闭前必须先更新 #3887 勾选状态, 否则局部 Issue 关闭后总控漂移

按 #3887 strict close conditions condition #7: "关闭前必须由总控 #3887 更新对应勾选状态, 避免局部 Issue 关闭后总控漂移"

#3906 同样可能受影响 (虽然 comment #87848 是关闭确认, 但 #3887 body 仍显示未勾选)。

## What Changes

* **#3887 issue body** (PATCH): 将 #3900 和 #3906 从 `- [ ]` 改为 `- [x]`, 添加 7 字段 evidence + follow-up Issue cross-reference
* **C-ARCH-05 follow-up**: 不在本 PR 范围, 已在 #3906/#3942 跟踪 (execution_engine.rs 拆分工作超出 V312-13 范围)
* **#3900 重新 close**: 在 #3887 勾选 + C-ARCH-05 明确 follow-up 后, 重新申请关闭 #3900
* **#3906 验证**: 检查 #3906 关闭确认 vs #3887 勾选状态, 必要时同步

## Capabilities

### Modified Capabilities

- `gate-rc-ga-checklist`: #3887 master checklist 同步更新

## Impact

- **Modified**: #3887 issue body (PATCH, 不影响 code)
- **New comment**: #3900 (重新封闭证据) + #3906 (勾选状态确认)
- **Affected artifacts**: 无 (仅 documentation-only 同步)

## Acceptance criteria

- #3887 body 中 #3900 / #3906 行从 `- [ ]` 改为 `- [x]`
- #3900 / #3906 各自的 evidence block 在 #3887 中 cross-reference
- C-ARCH-05 pre-existing failure 明确 follow-up 指向 #3906 / #3942
- 250 sync 不阻塞 (已建立 follow-up #3946 跟踪)

## Risk

极低。Body PATCH 是 documentation-only, 不影响 #3887 总控规则或 #3900/#3906 实际状态。

## Out of scope

- C-ARCH-05 实际拆分 execution_engine.rs (超出本 PR, 需 #3906/#3942 单独 PR)
- 250 sync (已在 #3946 跟踪, rate-limited 不阻塞关闭)
- 其他 V312 issue 关闭流程
