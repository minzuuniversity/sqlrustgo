# Design: EXCEPT ALL / INTERSECT ALL 多重集合语义实现

## 1. 问题分析

### 1.1 当前实现（占位符）

`crates/executor/src/stored_proc.rs` 中 `execute_cte_subquery` 的 `INTERSECT ALL` 分支：

```rust
if intersect_stmt.intersect_all {
    // For ALL variant, we need multiset intersection.
    // Fall back to chain (placeholder — real impl in PR3).
    let mut combined: Vec<Vec<Value>> =
        left_records.into_iter().chain(right_records).collect();
    combined.sort();
    combined.dedup();  // ← BUG: 完全丢失多重性，结果退化为 DISTINCT
    Ok(combined)
}
```

`EXCEPT ALL` 分支：

```rust
if except_stmt.except_all {
    // Multiset difference (placeholder — real impl in PR3).
    let mut result = left_records;
    for row in right_records {
        if let Some(pos) = result.iter().position(|r| r == &row) {
            result.remove(pos);  // ← BUG: 仅移除第一个匹配项，未考虑出现次数
        }
    }
    Ok(result)
}
```

### 1.2 正确语义（SQL-92）

**INTERSECT ALL**：多重集合交集
- 每行结果保留 `min(count_in_left, count_in_right)` 次
- 示例：`[1,1,1,2] INTERSECT ALL [1,1,2,3] = [1,1,2]`

**EXCEPT ALL**：多重集合差集
- 每行结果保留 `max(0, count_in_left - count_in_right)` 次
- 示例：`[1,1,1,2] EXCEPT ALL [1,1,2,3] = [1]`

### 1.3 失败 Fixture

`setops__test_setops.test` 第 116-124 行：

```sql
-- EXCEPT ALL / INTERSECT ALL
query II
select x, count(*) as c
from ((select * from (values(1),(2),(2),(3),(3),(3),(4),(4),(4),(4)) s(x) except all select * from (values(1),(3),(3)) t(x)) intersect all select * from (values(2),(2),(2),(4),(3),(3)) u(x)) s
group by x order by x
----
2  2
3  1
4  1
```

手动验证：
1. `s except all t`：`[1,2,2,3,3,3,4,4,4,4] - [1,3,3] = [2,2,3,4,4,4,4]`（1出现1次被抵消，3出现2次被抵消，2和4不变）
2. 结果再 `intersect all u`：u = `[2,2,2,4,3,3]`
   - 交集：2在中间结果出现2次，在u出现3次，保留2次
   - 3在中间结果出现1次，在u出现2次，保留1次
   - 4在中间结果出现4次，在u出现1次，保留1次
3. 最终 `[2,2,3,4]`，GROUP BY + COUNT 得到 `2→2, 3→1, 4→1` ✓

## 2. 修复方案

### 2.1 新增 multiset.rs 模块

在 `crates/executor/src/multiset.rs` 中实现多重集合运算：

```rust
/// 多重集合交集：返回 min(count_left, count_right) 次的条目
pub fn multiset_intersect(left: &[Vec<Value>], right: &[Vec<Value>]) -> Vec<Vec<Value>>

/// 多重集合差集：返回 max(0, count_left - count_right) 次的条目
pub fn multiset_except(left: &[Vec<Value>], right: &[Vec<Value>>) -> Vec<Vec<Value>>
```

### 2.2 算法实现

**多重集合交集（INTERSECT ALL）**：

```rust
pub fn multiset_intersect(left: &[Vec<Value>], right: &[Vec<Value>]) -> Vec<Vec<Value>> {
    // 1. 统计 right 中每行的出现次数
    use std::collections::HashMap;
    let mut right_counts: HashMap<&Vec<Value>, usize> = HashMap::new();
    for row in right {
        *right_counts.entry(row).or_insert(0) += 1;
    }

    // 2. 遍历 left，按 min(count_left_sofar, right_count) 保留
    let mut result = Vec::new();
    for row in left {
        if let Some(counter) = right_counts.get_mut(row) {
            if *counter > 0 {
                result.push(row.clone());
                *counter -= 1;
            }
        }
    }
    result
}
```

**多重集合差集（EXCEPT ALL）**：

```rust
pub fn multiset_except(left: &[Vec<Value>], right: &[Vec<Value>]) -> Vec<Vec<Value>> {
    // 1. 统计 right 中每行的出现次数
    use std::collections::HashMap;
    let mut right_counts: HashMap<&Vec<Value>, usize> = HashMap::new();
    for row in right {
        *right_counts.entry(row).or_insert(0) += 1;
    }

    // 2. 遍历 left，按 max(0, left_count - right_count) 保留
    let mut result = Vec::new();
    for row in left {
        if let Some(counter) = right_counts.get_mut(row) {
            if *counter > 0 {
                *counter -= 1;
                continue;  // 被 right 抵消
            }
        }
        result.push(row.clone());
    }
    result
}
```

### 2.3 更新 stored_proc.rs

将占位符替换为对 `multiset.rs` 的调用：

```rust
sqlrustgo_parser::Statement::Intersect(intersect_stmt) => {
    let left_records = self.execute_cte_subquery(&intersect_stmt.left, ctx)?;
    let right_records = self.execute_cte_subquery(&intersect_stmt.right, ctx)?;
    if intersect_stmt.intersect_all {
        Ok(multiset::multiset_intersect(&left_records, &right_records))
    } else {
        // Set intersection: keep rows present in BOTH (dedup first).
        let mut result: Vec<Vec<Value>> = left_records
            .into_iter()
            .filter(|row| right_records.contains(row))
            .collect();
        result.sort();
        result.dedup();
        Ok(result)
    }
}
sqlrustgo_parser::Statement::Except(except_stmt) => {
    let left_records = self.execute_cte_subquery(&except_stmt.left, ctx)?;
    let right_records = self.execute_cte_subquery(&except_stmt.right, ctx)?;
    if except_stmt.except_all {
        Ok(multiset::multiset_except(&left_records, &right_records))
    } else {
        // Set difference: keep rows in left but NOT in right.
        let mut result: Vec<Vec<Value>> = left_records
            .into_iter()
            .filter(|row| !right_records.contains(row))
            .collect();
        result.sort();
        result.dedup();
        Ok(result)
    }
}
```

### 2.4 单元测试边界条件

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multiset_intersect_basic() {
        let left = vec![vals!(1), vals!(2), vals!(2), vals!(3)];
        let right = vec![vals!(2), vals!(2), vals!(3), vals!(4)];
        let result = multiset_intersect(&left, &right);
        assert_eq!(result.len(), 3); // 2, 2, 3
    }

    #[test]
    fn test_multiset_except_basic() {
        let left = vec![vals!(1), vals!(1), vals!(1), vals!(2)];
        let right = vec![vals!(1), vals!(2), vals!(3)];
        let result = multiset_except(&left, &right);
        assert_eq!(result.len(), 2); // 1, 1
    }

    #[test]
    fn test_multiset_intersect_no_overlap() {
        let left = vec![vals!(1), vals!(2)];
        let right = vec![vals!(3), vals!(4)];
        let result = multiset_intersect(&left, &right);
        assert!(result.is_empty());
    }

    #[test]
    fn test_multiset_except_all_removed() {
        let left = vec![vals!(1), vals!(2)];
        let right = vec![vals!(1), vals!(2)];
        let result = multiset_except(&left, &right);
        assert!(result.is_empty());
    }
}
```

## 3. Fixture 验证

修复后 `setops__test_setops.test` 中的 `EXCEPT ALL / INTERSECT ALL` 查询应返回：

```
2  2
3  1
4  1
```

## 4. 关键决策

| 决策点 | 选项 | 选择 | 理由 |
|--------|------|------|------|
| 算法复杂度 | O(n²) / O(n log n) / O(n) | O(n) HashMap | 简洁且高效 |
| NULL 值 | 视为普通值 / 特殊处理 | 视为普通值 | HashMap 自动处理相等性 |
| 空集合 | 应返回空 | 返回空 | 符合 SQL 语义 |
| 子查询嵌套 | 递归处理 / 展平 | 递归 | executor 已支持 CTE 子查询 |

## 5. 验证方式

```bash
# 1. 构建
cargo build -p sqlrustgo-executor

# 2. 运行新增单元测试
cargo test -p sqlrustgo-executor multiset

# 3. 运行 sqllogictest
cargo test -p sqlrustgo_sqllogictest setops__test_setops

# 4. 运行完整测试
cargo test --all-features
```

## 6. 失败模式

- 若结果行数偏少：可能是 counter 递减时机错误（应在遍历 left 时递减，而非预计算）
- 若 NULL 值处理异常：检查 `Value::Null` 的 Hash 相等性（应使用 `PartialEq` 而非 `==` 的指针比较）
- 若嵌套 setop 失败（如 `(a EXCEPT ALL b) INTERSECT ALL c`）：确认 `execute_cte_subquery` 递归处理 `Statement::Intersect` 变体时正确传递 `intersect_all` 标志
