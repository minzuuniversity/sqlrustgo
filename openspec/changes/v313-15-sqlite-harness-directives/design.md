# Design: SQLite Harness 指令支持

## 1. 架构概览

SQLLogicTest 格式支持特殊的 harness 指令行，以 `set`/`reset`/`statement`/`query` 等关键字开头，区别于普通 SQL 语句。当前 runner 在解析这些行时遇到未注册的 directive 直接抛出 `parse error`。

```
# 典型 harness directive（在 quantile_fun.test 第 5 行）
set variable sf 0.001
```

本方案在 `sqllogictest::Runner` 层面注册自定义 directive 处理器，将 `set variable <name> <value>` 指令的解析与存储接驳到 `SltDb` 实例。

## 2. 当前状态分析

### 2.1 Runner 初始化

当前 `main.rs` 的 runner 创建：
```rust
let mut tester = Runner::new(|| async { Ok(SltDb::new()) });
tester.with_normalizer(strip_debug_format);
tester.with_validator(...);
```

`Runner::new()` 返回一个支持 `register_directive` 的实例（从 `sqllogictest = "0.29"` API 可知）。

### 2.2 SltDb 结构

当前 `SltDb` 仅持有 `engine: MemoryExecutionEngine`，没有 variables 字段：
```rust
pub struct SltDb {
    engine: MemoryExecutionEngine,
}
```

### 2.3 错误链路

当 `set variable sf 0.001` 被解析时，`sqllogictest` parser 遇到未注册的 directive，抛出：
```
parse error: invalid line: "set variable sf 0.001"
```

该错误来自 `sqllogictest` crate 内部，确切地说在 `Parse` 阶段，不是 `sqllogictest::Runner` 的 `run` 方法层面。

### 2.4 sqllogictest 0.29 Directive API

从 `sqllogictest` crate API 文档（risinglightdb/sqllogictest-rs）可知：
- `Runner` 提供了 `register_directive` 方法用于注册自定义 directive 处理器
- 签名：`fn register_directive<F>(&mut self, handler: F) where F: FnMut(&str) + Send + Sync`
- 当 parser 遇到无法识别的指令行时，调用注册的 handler

## 3. 实现方案

### 3.1 SltDb 扩展

```rust
use std::collections::HashMap;

pub struct SltDb {
    engine: MemoryExecutionEngine,
    variables: HashMap<String, String>,  // 新增
}

impl SltDb {
    pub fn new() -> Self {
        Self {
            engine: MemoryExecutionEngine::new(MemoryStorage::new()),
            variables: HashMap::new(),
        }
    }
}
```

### 3.2 Directive 注册

```rust
async fn async_main() {
    // ... 参数解析 ...

    let mut tester = Runner::new(|| async { Ok(SltDb::new()) });

    // 注册 set variable 指令处理器
    tester.register_directive(move |line: &str| {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("set variable ") {
            let mut parts = rest.splitn(2, char::is_whitespace);
            if let (Some(name), Some(value)) = (parts.next(), parts.next()) {
                // 通过 Arc<RwLock<HashMap>> 共享 variables
                // 注意：这里需要线程安全，因为 sqllogictest 可能并发调用
                return;
            }
        }
        // 无法解析，返回原样让 sqllogictest 报 parse error
    });

    tester.with_normalizer(strip_debug_format);
    tester.with_validator(...);

    // 运行 ...
}
```

### 3.3 Arc<RwLock<HashMap>> 共享方案

由于 `register_directive` 的闭包需要 `Send + Sync`，而 `SltDb` 实例通过 `Runner::new()` 的 factory 闭包创建，每次创建 fresh instance，直接捕获 `&mut SltDb` 不可行。

方案：使用 `Arc<RwLock<HashMap<String, String>>>` 在 `SltDb` 外部存储所有变量，runner-level 共享：

```rust
async fn async_main() {
    let shared_vars: Arc<RwLock<HashMap<String, String>>> = Arc::new(RwLock::new(HashMap::new()));

    let shared_vars_clone = Arc::clone(&shared_vars);
    let mut tester = Runner::new(|| async {
        Ok(SltDb::new_with_vars(Arc::clone(&shared_vars_clone)))
    });

    let sv = Arc::clone(&shared_vars);
    tester.register_directive(move |line: &str| {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("set variable ") {
            let mut parts = rest.splitn(2, char::is_whitespace);
            if let (Some(name), Some(value)) = (parts.next(), parts.next()) {
                let vars = sv.read();
                vars.insert(name.to_string(), value.to_string());
                return;
            }
        }
        // unrecognized directive — let sqllogictest handle it
    });
}
```

对应 SltDb：
```rust
pub struct SltDb {
    engine: MemoryExecutionEngine,
    variables: Arc<RwLock<HashMap<String, String>>>,
}

impl SltDb {
    fn new_with_vars(vars: Arc<RwLock<HashMap<String, String>>>) -> Self {
        Self {
            engine: MemoryExecutionEngine::new(MemoryStorage::new()),
            variables: vars,
        }
    }
}
```

### 3.4 支持多行 set variable

部分 harness 指令格式为：
```
set variable sf 0.001
```
（当前失败案例均为单行，后续扩展方向）

### 3.5 其他 harness directive 扩展点

| Directive | 示例 | 处理策略 |
|-----------|------|---------|
| `set variable <name> <value>` | `set variable sf 0.001` | 解析并存储（本次实现） |
| `set threads <n>` | `set threads 4` | 解析，跳过（未来并行优化） |
| `reset <name>` | `reset sf` | 解析，删除变量 |
| `reset` | `reset` | 解析，清空所有变量 |
| `hash threshold <n>` | `hash threshold 1000` | 解析，跳过（暂不支持） |

## 4. 变量替换（后续任务，非本变更范围）

`set variable` 指令存储变量值后，测试文件可能使用 `${name}` 或 `$name` 语法引用变量。当前 runner 不处理变量替换，这是本变更的明确非目标。

Exclusions 消除后，文件会报 SQL 执行结果不匹配（如果 quantile 函数未实现），而不是 parse error。

## 5. Exclusion 移除

### 5.1 exclusions.yml

从 `docs/releases/v3.12.0/evidence/sqllogictest/exclusions.yml` 中删除以下条目：
```yaml
  - file: aggregate__quantile_fun.test
    reason: "Parse error: 'set variable sf 0.001' — test harness directive not supported"
    category: harness
    owner: openclaw
    expiry: "2027-06-30"
    follow_up: "v313-15-sqlite-harness-directives"

  - file: sql__quantile_fun.test
    reason: "Parse error: 'set variable sf 0.001' — test harness directive not supported"
    category: harness
    owner: openclaw
    expiry: "2027-06-30"
    follow_up: "v313-15-sqlite-harness-directives"

  - file: quantile_fun.test
    reason: "Parse error: 'set variable sf 0.001' — test harness directive not supported"
    category: harness
    owner: openclaw
    expiry: "2027-06-30"
    follow_up: "v313-15-sqlite-harness-directives"
```

### 5.2 sqlite-corpus-manifest.json

更新以下 3 个文件的状态：
```json
{"name": "aggregate__quantile_fun.test", "status": "excluded", "category": "harness"}
{"name": "sql__quantile_fun.test", "status": "excluded", "category": "harness"}
{"name": "quantile_fun.test", "status": "excluded", "category": "harness"}
```
改为：
```json
{"name": "aggregate__quantile_fun.test", "status": "unknown", "category": "harness"}
{"name": "sql__quantile_fun.test", "status": "unknown", "category": "harness"}
{"name": "quantile_fun.test", "status": "unknown", "category": "harness"}
```

`category` 字段保留为 `harness`（因为变量替换尚未实现），`status` 改为 `unknown`（等待首次运行结果）。

## 6. 验证方式

```bash
# 验证 set variable 指令不再报 parse error
cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata --filter quantile_fun

# 预期输出不再是 parse error，而是 SQL 执行结果
# （quantile 函数可能尚未实现，结果为 fail 而非 parse error）

# 运行全量 runner
cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata

# 确认 3 个 quantile 文件不再报 "parse error: invalid line"
```

## 7. 失败模式

- **Parse error 仍然出现**：说明 `register_directive` 未正确拦截，或 directive 格式不匹配
- **SQL 执行结果 fail**：这是预期行为，因为 quantile 函数本身（`quantile_disc`、`percentile_cont`）尚未实现；排除 parse error 后，这些 fixture 会进入 SQL 执行阶段失败，不影响本变更目标
- **变量替换场景**：当前实现的 `set variable` 仅存储值，不做替换；引用变量的测试（如 `${sf}`）会返回 NULL 或错误，但这是后续任务范围
