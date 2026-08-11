# V312-11-Fix: VALUES Constructor in Derived Table

## 问题
Parser不识别`VALUES(...)`作为FROM子句中的派生表源。

### 错误
```
Parse error: Expected table name in derived table, got LParen
```

### 失败测试
- setops__test_setops.test

### 涉及SQL
```sql
SELECT * FROM (VALUES(1),(2),(3)) AS t(x)
```

## 解决方案
在parser的derived table解析逻辑中添加VALUES支持。

## 验收标准
- setops__test_setops.test PASS

## 影响范围
- parser: 添加VALUES作为合法表源
