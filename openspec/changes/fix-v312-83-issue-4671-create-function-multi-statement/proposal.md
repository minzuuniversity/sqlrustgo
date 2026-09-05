# Proposal — Issue #4671: CREATE FUNCTION 多语句函数体支持

## 状态: 部分实现

### 简单函数 ✅ 已实现

`CREATE FUNCTION f1(x int) RETURNS int RETURN x * 2;` 正常工作。

### 多语句函数 ❌ 未实现

```sql
CREATE FUNCTION f2(x int) RETURNS int AS 
BEGIN
  DECLARE r int;
  SET r = x * 2;
  RETURN r;
END;
-- 错误: Parse error: Expected Return, got As
```

Parser (`crates/parser/src/parser.rs:3498-3606`) 只支持 `RETURN expression` 形式。

## 根因

`parse_create_function` 不支持 `AS BEGIN ... END` 块和多语句语法。

## 待实现

1. Parser 支持 `AS` 关键字和 `BEGIN ... END` 块
2. 支持 `DECLARE variable_name data_type` 局部变量声明
3. 支持 `SET variable = expression` 赋值
4. 支持多语句执行

## 验证命令

```sql
-- 简单函数（已工作）
CREATE FUNCTION f1(x int) RETURNS int RETURN x * 2;
SELECT f1(10);  -- 20

-- 多语句函数（未工作）
CREATE FUNCTION f2(x int) RETURNS int AS 
BEGIN DECLARE r int; SET r = x * 2; RETURN r; END;
```
