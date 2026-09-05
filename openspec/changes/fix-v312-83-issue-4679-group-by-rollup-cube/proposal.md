# Proposal — Issue #4679: GROUP BY ROLLUP / CUBE 实现

## 问题

`GROUP BY ROLLUP(col)` 和 `GROUP BY CUBE(col1, col2)` 只返回 1 行而非预期的多行。

```sql
CREATE TABLE t(grp int, val int);
INSERT INTO t VALUES (1, 10), (1, 20), (2, 10), (2, 30), (3, 30);

-- ROLLUP 应返回每组的聚合 + 总和
SELECT grp, SUM(val) FROM t GROUP BY ROLLUP(grp);
-- 期望: 1|30, 2|40, 3|30, |90  (4行，含总和)
-- 实际: 只返回 1 行

-- CUBE 应返回所有组合 + 总和
SELECT grp, SUM(val) FROM t GROUP BY CUBE(grp);
-- 期望: 1|30, 2|40, 3|30, |90  (4行)
-- 实际: 只返回 1 行

-- GROUPING SETS 报 Parse error
SELECT grp, SUM(val) FROM t GROUP BY GROUPING SETS((grp), ());
-- 错误: Parse error: Expected identifier, got LParen
```

## 根因

Parser 在解析 GROUP BY 时，未识别 ROLLUP、CUBE、GROUPING SETS 关键字。

ROLLUP(a,b) 展开为: GROUP BY a,b + GROUP BY a + GROUP BY ()
CUBE(a,b) 展开为: GROUP BY a,b + GROUP BY a + GROUP BY b + GROUP BY ()

## 方案

1. **Parser 扩展**:
   - 添加 ROLLUP 语法解析（在 GROUP BY 后面识别 ROLLUP 关键字）
   - 添加 CUBE 语法解析
   - 添加 GROUPING SETS 语法解析

2. **Executor 扩展**:
   - 实现分组扩展逻辑
   - 对每个分组键组合执行聚合
   - UNION ALL 累积所有结果

3. **语义**:
   - ROLLUP 生成层次聚合（从最细到总计）
   - CUBE 生成所有组合的聚合
   - GROUPING SETS 显式指定分组组合

## 范围与限制

- 支持简单列的 ROLLUP/CUBE
- 支持嵌套 ROLLUP/CUBE
- 不支持复杂表达式

## 验证

- 新增 ROLLUP 测试用例
- 新增 CUBE 测试用例
- 新增 GROUPING SETS 测试用例
