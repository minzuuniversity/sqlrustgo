# Proposal — Issue #4639: RIGHT JOIN / FULL OUTER JOIN 实现

## 问题

`SELECT * FROM a RIGHT JOIN b ON a.id = b.id` 输出等同于 INNER JOIN，RIGHT JOIN 和 FULL OUTER JOIN 的语义未实现。

```sql
-- 表 a (id 1,2,3), 表 b (id 1,3)
SELECT * FROM a LEFT JOIN b ON a.id=b.id;
-- 期望: 1|...|NY, 2|bob||, 3|...|LA
SELECT * FROM a RIGHT JOIN b ON a.id=b.id;
-- 期望: 1|alice|NY, 3|bob|LA (RIGHT JOIN 应保留 b 的所有行)
```

## 根因

在 `src/engine_select.rs` 的 `execute_joins` 函数中，只实现了 NestedLoopJoin 和 HashJoin 的 INNER JOIN 逻辑：
- RIGHT JOIN 需要在外表（右表）无匹配时输出 NULL-padded 行
- FULL OUTER JOIN 需要在任一表无匹配时输出 NULL-padded 行

当前实现中，LEFT/RIGHT/FULL 的 JoinType 被检测但未正确处理。

## 方案

1. 在 `execute_joins` 中新增 RIGHT/FULL OUTER JOIN 处理分支：
   - 对 RIGHT JOIN：遍历右表，对于右表中无匹配的行，补齐左表列为 NULL 后输出
   - 对 FULL OUTER JOIN：综合 LEFT 和 RIGHT JOIN 的处理

2. 修改 HashJoin 执行器以支持 outer joins：
   - 记录左表和右表中未匹配的行
   - 对 FULL OUTER JOIN，输出未匹配行的 NULL-padded 版本

## 范围与限制

- 仅处理单表 JOIN（不含子查询、USING 等复杂形式）
- 不涉及 CROSS JOIN、NATURAL JOIN
- SQLite 不支持 FULL OUTER JOIN（保持与 SQLite 兼容，返回错误）

## 验证

- 新增测试用例验证 RIGHT JOIN 和 FULL OUTER JOIN 输出正确
- 回归测试验证 LEFT JOIN 和 INNER JOIN 不受影响

## 验证命令
```bash
cargo test --all-features -- right_join
cargo test --all-features -- full_outer_join
```
