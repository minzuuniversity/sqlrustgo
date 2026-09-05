# Proposal — Issue #4671: CREATE FUNCTION 多语句函数体支持

## 状态: ✅ 已实现

### 解析 ✅

Parser 支持 `AS BEGIN ... END` 语法：
- `body_block: Option<String>` 字段在 `CreateFunctionStatement`
- `read_until_end_block()` 辅助函数处理嵌套 BEGIN/END

### 执行 ✅

实现内容 (`crates/executor/src/expr/mod.rs`)：

```rust
pub struct UdfDefinition {
    pub params: Vec<String>,
    pub return_type: String,
    pub body_expr: String,
    pub body_block: Option<String>,  // Issue #4671
}

pub fn register_udf_with_body(...) { ... }

fn invoke_udf_multi(def: &UdfDefinition, args: &[Value], body_block: &str) -> Value {
    // 1. 检查参数数量
    // 2. 查找 RETURN 语句并提取表达式
    // 3. 使用 substitute_udf_params 替换参数
    // 4. 求值返回
}
```

执行器 (`src/execution_engine.rs`)：
```rust
fn execute_create_function(&self, stmt: &CreateFunctionStatement) {
    if let Some(ref body_block) = stmt.body_block {
        expr_mod::register_udf_with_body(&stmt.name, param_names, ...);
    } else {
        expr_mod::register_udf(&stmt.name, param_names, ...);
    }
}
```

## 验证

```sql
-- 单表达式 UDF
CREATE FUNCTION f0(x int) RETURNS int RETURN x * 2;
SELECT f0(10);  -- 20

-- 多语句 UDF
CREATE FUNCTION f1(x int) RETURNS int AS BEGIN RETURN x * 2; END;
SELECT f1(10);  -- 通过 invoke_udf_multi 求值
```

## 限制

- CLI 有 OR-downgrade 拒绝 CREATE FUNCTION (Issue #4652)
- SET 变量赋值和 DECLARE 声明尚未支持
- 需通过 MySQL wire protocol 测试完整功能
