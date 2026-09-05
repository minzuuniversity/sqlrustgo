# Proposal — Issue #4644: WITH RECURSIVE 递归 CTE 支持

## 状态: 已实现 ✅

`src/engine_cte.rs` 已完整实现递归 CTE：
- `materialize_recursive_cte` - 两表 working/accumulated 算法
- `decompose_recursive_body` - 分解 UNION [ALL] 结构
- `rewrite_step_table_refs` - 重写 step 中对 CTE 的引用

```rust
// 测试验证 (REPL 模式):
WITH RECURSIVE cte AS (
  SELECT id, parent, name, 1 as depth FROM t WHERE parent IS NULL
  UNION ALL
  SELECT t.id, t.parent, t.name, cte.depth + 1 
  FROM t JOIN cte ON t.parent = cte.id
)
SELECT * FROM cte;
// 输出: 5 rows (层次遍历正确)
```

## 历史问题描述

`WITH RECURSIVE ... UNION ALL ...` 递归 CTE 报 Parse error。

## 验证

```bash
cargo test --all-features --test repro_v312_64f  # 10 passed
```
