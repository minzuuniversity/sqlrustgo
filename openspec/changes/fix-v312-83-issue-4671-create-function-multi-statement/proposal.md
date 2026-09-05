# Proposal — Issue #4671: CREATE FUNCTION 多语句函数体支持

## 状态: 部分实现

### 解析 ✅ 已实现

Parser (`crates/parser/src/parser.rs`) 现在支持 `AS BEGIN ... END` 语法：

```sql
-- 解析成功
CREATE FUNCTION f2(x int) RETURNS int AS 
BEGIN 
  DECLARE r int; 
  SET r = x * 2; 
  RETURN r; 
END;
```

- 添加了 `body_block: Option<String>` 字段到 `CreateFunctionStatement`
- 添加了 `read_until_end_block()` 辅助函数处理嵌套 BEGIN/END

### 执行 ❌ 未实现

多语句函数体存储在 `body_block` 中，但执行器尚未实现多语句逻辑。

## 待实现

1. 实现多语句函数执行：
   - 解析 DECLARE 变量声明
   - 实现 SET 赋值
   - 按顺序执行语句
   - RETURN 终止执行并返回值

## 验证命令

```sql
-- 简单函数（已工作）
CREATE FUNCTION f1(x int) RETURNS int RETURN x * 2;
SELECT f1(10);  -- 20

-- 多语句函数（解析成功，执行返回 NULL）
CREATE FUNCTION f2(x int) RETURNS int AS 
BEGIN DECLARE r int; SET r = x * 2; RETURN r; END;
SELECT f2(10);  -- Null (执行未实现)
```
