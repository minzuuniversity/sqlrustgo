# Proposal — Issue #4644: WITH RECURSIVE 递归 CTE 支持

## 问题

`WITH RECURSIVE ... UNION ALL ...` 递归 CTE 报 Parse error: Expected Select, got Eof。

```sql
CREATE TABLE t(id int, parent int, name varchar(20));
INSERT INTO t VALUES (1, NULL, 'root'), (2, 1, 'a'), (3, 1, 'b'), (4, 2, 'a1'), (5, 2, 'a2');

WITH RECURSIVE cte AS (
  SELECT id, parent, name, 1 as depth FROM t WHERE parent IS NULL
  UNION ALL
  SELECT t.id, t.parent, t.name, cte.depth + 1 
  FROM t JOIN cte ON t.parent = cte.id
)
SELECT * FROM cte;
```

## 根因

Parser 在解析 WITH RECURSIVE 时，遇到 RECURSIVE 关键字后未正确处理后续的 CTE 定义。

递归 CTE 的语法比普通 CTE 更复杂，需要支持：
- 递归锚点（初始查询）
- 递归成员（引用自身）
- UNION ALL 连接

## 方案

1. 修改 `parse_with_clause` 函数以支持 RECURSIVE 关键字
2. 区分递归 CTE 和普通 CTE 的解析逻辑
3. 实现递归 CTE 的执行逻辑：
   - 执行锚点查询
   - 循环执行递归成员直到无新行产生
   - UNION ALL 累积所有结果

## 范围与限制

- 支持 `WITH RECURSIVE cte AS (anchor UNION ALL SELECT ... FROM cte WHERE ...)`
- 不支持多层递归、互递归 CTE
- 不支持递归终止条件（需要用户确保有界递归）

## 验证

- 新增递归 CTE 测试用例
- 验证普通 WITH 不受影响（回归测试）
