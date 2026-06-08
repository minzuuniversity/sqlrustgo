# v3.9.0 快速收敛跟踪

> **状态**: Active
> **基线**: develop/v3.9.0 @ 3afe482f6
> **目标**: 真实运行验证 → RC3 → RC4 → GA

## 当前进度 (2026-06-08)

### Q3 root cause 定位 + 部分修复
**根因**: `ORDER BY revenue DESC` (其中 `revenue` 是 SUM alias) — code 直接用 `select.columns` 索引去取 row，但 row layout 是 `[group_by_cols..., aggregate_value...]`。`revenue` 在 select.columns[1] 但实际 row[3] (group_schema_len=3 + agg_idx=0)。

**修复** (src/engine_select.rs, +55/-6):
- 计算 `group_schema_len = group_exprs.len()`
- 检测 ORDER BY col 是否 aggregate-typed (Expression::Aggregate 或 BinaryOp(.., Aggregate) 或 idx >= group_schema_len)
- 如果是 aggregate: 实际 row idx = `group_schema_len + (select_idx 之前的 aggregate count)`
- 否则: 用 select_idx
- 排序也支持 per-column ASC/DESC

### 端到端测试结果

| 测试 | 修复前 | 修复后 |
|------|--------|--------|
| 总 PASS | 16/22 | **17/22** |
| FAIL | 5 (Q3/Q8/Q10/Q17/Q18) | 4 (Q3/Q8/Q17/Q18) |
| TIMEOUT | 1 (Q21) | 1 (Q21) |

**Q10 ✅ 已修复**! Q3 部分修复 (revenue 列正确，但 o_shippriority 仍然 0 — 还有 column re-projection bug).

### 剩余 Bug

| Query | 状态 | 下一步 |
|-------|------|--------|
| Q3 | 80% 修复 | reproject 阶段 o_shippriority 仍错位 (row[3] 重映射) |
| Q8 | cell_diff | 类似 Q3 需查 trace |
| Q17 | value_mismatch | Float 算术 + Subquery |
| Q18 | cell_diff | 类似 Q3 |
| Q21 | TIMEOUT | N² EXISTS 性能优化 |

### 优先级
1. **Q3 完成 reproject fix** — 50% 可能同样修好 Q8/Q18
2. Q8/Q18/Q17 一并处理
3. Q21 性能优化
4. PR + merge + 4 remote 同步

### 待办
- [ ] 修 Q3 reproject 阶段 o_shippriority 错位
- [ ] Q8/Q18 用同样方法修
- [ ] 移除所有 DBG Q3 trace（已干净）
- [ ] PR + 4 remote 同步
- [ ] 启动 TPCH_FORCE=1 真实 G1
- [ ] 启动 G8 真实 Crash
- [ ] 启动 G13 24h Soak
