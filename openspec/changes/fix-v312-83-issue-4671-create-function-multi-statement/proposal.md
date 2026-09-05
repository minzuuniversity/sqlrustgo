# Proposal — Issue #4671: CREATE FUNCTION 多语句函数体支持

## 问题

CREATE FUNCTION 仅支持最简单的单 RETURN 形式，不支持多语句函数体。

```sql
-- 简单函数（当前支持）
CREATE FUNCTION f1(x int) RETURNS int RETURN x * 2;
SELECT f1(10);  -- 20

-- 多语句函数（当前不支持）
CREATE FUNCTION f2(x int) 
RETURNS int AS 
BEGIN
  DECLARE @result int;
  SET @result = x * 2;
  RETURN @result;
END;

-- RETURNS TABLE（当前不支持）
CREATE FUNCTION get_users_by_dept(dept_id int)
RETURNS TABLE(id int, name varchar(50))
AS
BEGIN
  RETURN QUERY SELECT id, name FROM users WHERE department_id = dept_id;
END;
```

## 根因

Parser 在解析 CREATE FUNCTION 时，只支持 `RETURN expression` 形式的简单函数体，未解析 `BEGIN ... END` 块。

## 方案

1. **扩展 Parser**:
   - 修改 `parse_create_function` 以支持 `BEGIN ... END` 块
   - 支持 `RETURNS TABLE(column_name column_type, ...)` 类型声明
   - 支持 DECLARE 语句

2. **实现多语句执行**:
   - 解析语句序列（赋值、IF、RETURN 等）
   - 按顺序执行语句
   - RETURN 终止执行并返回值

3. **实现表返回函数**:
   - 解析表结构
   - 支持 RETURN QUERY

## 范围与限制

- 支持单函数多语句
- 支持 RETURNS TABLE 返回表
- 支持 DECLARE 局部变量
- 不支持复杂控制流（IF、LOOP 等）
- 不支持存储过程

## 验证

- 新增 CREATE FUNCTION 测试用例验证多语句函数
- 新增 RETURNS TABLE 测试用例
- 回归测试验证简单函数不受影响
