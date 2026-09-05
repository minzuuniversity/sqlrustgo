# Proposal — Issue #4669: DROP INDEX / 函数索引 / 部分索引 实现

## 问题

多个 DDL 命令实现错误：

1. `DROP INDEX idx_name` 报错 "DROP INDEX not fully supported yet"
2. `CREATE INDEX ON t(UPPER(a))` 函数索引报 "Column not found"
3. `CREATE INDEX ... WHERE val > 200` 部分索引静默接受但不生效

```sql
-- DROP INDEX 报错
DROP INDEX idx_name;
-- 错误: DROP INDEX not fully supported yet

-- 函数索引报错
CREATE INDEX idx ON t(UPPER(a));
-- 错误: Column not found: UPPER(a)

-- 部分索引静默失败
CREATE INDEX idx_partial ON t(val) WHERE val > 200;
-- 成功创建但查询不使用该索引
```

## 根因

1. DROP INDEX 在 storage 层未实现（仅在 FileStorage 框架存在）
2. 函数索引的列表达式未在解析时处理，索引创建时将 `UPPER(a)` 当作列名
3. 部分索引的 WHERE 子句未存储和执行

## 方案

1. **DROP INDEX**: 实现 `FileStorage.drop_index` 和 `MemoryStorage.drop_index`
   - 从 catalog 中移除索引元数据
   - 从 `self.indexes` 中移除 B+Tree
   - 删除磁盘上的索引文件

2. **函数索引**: 修改 `create_index` 逻辑
   - 检测列表达式（如 `UPPER(a)`）
   - 解析并求值表达式以获取实际列
   - 或者实现表达式索引（更复杂）

3. **部分索引**: 
   - 在 IndexInfo 中存储 WHERE 条件
   - 在 scan 时应用部分索引过滤

## 范围与限制

- DROP INDEX 需要更新 catalog 和 storage
- 函数索引仅支持确定性函数（如 UPPER、LOWER）
- 部分索引仅支持简单 WHERE 条件

## 验证

- 新增 DROP INDEX 测试
- 新增函数索引测试
- 新增部分索引测试
