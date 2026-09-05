# Proposal — Issue #4679: GROUP BY ROLLUP / CUBE

## 状态: 已实现 ✅

`GROUP BY ROLLUP(col)` 和 `GROUP BY CUBE(col1, col2)` 均正常工作。

```sql
-- ROLLUP 测试
CREATE TABLE t(grp int, val int);
INSERT INTO t VALUES (1, 10), (1, 20), (2, 10), (2, 30), (3, 30);
SELECT grp, SUM(val) FROM t GROUP BY ROLLUP(grp);
-- 输出: 1|30, 2|40, 3|30, Null|100 (4行，含总和)

-- CUBE 测试
CREATE TABLE t2(a int, b int, val int);
INSERT INTO t2 VALUES (1, 1, 10), (1, 2, 20), (2, 1, 30), (2, 2, 40);
SELECT a, b, SUM(val) FROM t2 GROUP BY CUBE(a, b);
-- 输出: 9行，包含所有组合 + 各维度总计
```

## 历史问题描述

`GROUP BY ROLLUP(col)` 和 `GROUP BY CUBE(col1, col2)` 只返回 1 行而非预期的多行。
