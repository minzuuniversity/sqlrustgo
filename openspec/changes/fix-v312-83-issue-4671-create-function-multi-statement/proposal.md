# Proposal — Issue #4671: CREATE FUNCTION 多语句函数体支持

## 状态: 部分实现

### 解析 ✅ 已实现

Parser 支持 `AS BEGIN ... END` 语法：
- 添加了 `body_block: Option<String>` 字段到 `CreateFunctionStatement`
- 添加了 `read_until_end_block()` 辅助函数

### 执行 ⚠️ 部分实现

- 添加了 `body_block` 支持到 `UdfDefinition`
- 简单的 RETURN 表达式求值
- SET/DECLARE 变量暂不支持

## 实现内容

```rust
// executor/src/expr/mod.rs
pub struct UdfDefinition {
    pub params: Vec<String>,
    pub return_type: String,
    pub body_expr: String,
    pub body_block: Option<String>,  // 新增
}

// invoke_udf_multi: 解析 RETURN 后的表达式并求值
fn invoke_udf_multi(def: &UdfDefinition, args: &[Value], body_block: &str) -> Value {
    // 查找 RETURN 关键字后的表达式
    // 解析并求值（参数替换）
}
```

## 待完成

1. SET 变量赋值支持
2. DECLARE 变量声明支持
3. 变量在 RETURN 表达式中的引用

## 验证

```sql
-- 简单 RETURN（已工作）
CREATE FUNCTION f1(x int) RETURNS int AS BEGIN RETURN x * 2; END;
SELECT f1(10);  -- 20

-- 带变量（解析成功，执行返回 NULL）
CREATE FUNCTION f2(x int) RETURNS int AS 
BEGIN DECLARE r int; SET r = x * 2; RETURN r; END;
SELECT f2(10);  -- Null
```
