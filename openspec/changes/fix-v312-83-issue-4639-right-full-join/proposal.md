# Proposal — Issue #4639: RIGHT JOIN / FULL OUTER JOIN 实现

## 状态: 已实现 ✅

经分析，`src/engine_select.rs:4463-4471` 已实现 RIGHT JOIN 和 FULL OUTER JOIN 逻辑：

```rust
if matches!(join_type, JoinType::Right | JoinType::Full) {
    for (ri, right_row) in right_rows.iter().enumerate() {
        if !right_matched.contains(&ri) {
            let mut combined = vec![Value::Null; left_col_count];
            combined.extend(right_row.clone());
            matched.push(combined);
        }
    }
}
```

## 历史问题描述

`SELECT * FROM a RIGHT JOIN b ON a.id = b.id` 输出等同于 INNER JOIN，RIGHT JOIN 和 FULL OUTER JOIN 的语义未实现。

## 验证

- 测试 `test_right_join_preserves_right_rows` 已添加
- Lib tests: 131 passed

## 验证命令

```bash
cargo test --all-features --lib  # 131 passed
```
