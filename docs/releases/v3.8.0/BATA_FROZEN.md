# bata/v3.8.0 分支冻结声明

> **Date**: 2026-06-04
> **Status**: 🧊 **FROZEN — 历史版本，不再修改**
> **Last commit**: `34cd64d72` (Bata 完成公告)
> **Tag**: `v3.8.0-beta` @ `052524890` (合并基线)

## 冻结原因

bata/v3.8.0 是 v3.8.0 Beta 阶段的最终快照（与 PR #3031 Bata 完成公告同时发布）。
该分支不再接受任何代码改动，仅作为历史参考保留。

## 后续开发路径

| 阶段 | 分支 | 状态 |
|------|------|------|
| Bata | bata/v3.8.0 | 🧊 **FROZEN** |
| RC1 | develop/v3.8.0 | 🟢 active (包含 Bata 之后的 RC1 工作) |
| RC2 | develop/v3.8.0 | 🟡 计划 |
| GA | develop/v3.8.0 → main | 🟡 计划 (需 PM 授权) |

## 如何查看 Bata 状态

```bash
git checkout bata/v3.8.0
git log --oneline -5
git show v3.8.0-beta
```

## 关键交付 (Bata 阶段)

- 18 个 milestone issue 全部关闭 (PR #2995~#3022)
- 4 个跟踪 issue 已更新 (#2702 评审 closed, #2763, #2743, #2937 评论)
- 治理资产：EXECUTION_SEMANTICS contract, check_execution_semantics.sh,
  update_changelog.sh
- 22 个 v3.8.0 Milestone Issue 状态: 18 closed / 2 → RC1 / 4 → v3.9.0+ 冻结

## References

- Bata PR: #3031 (Bata 完成公告)
- 路线决策: 文档 `docs/releases/v3.8.0/RELEASE_NOTES.md` (v4: Route B)
- 冻结清单: `docs/releases/v3.8.0/V380_FROZEN_TO_V390.md`
