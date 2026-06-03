# v3.8.0 API 文档 (API Documentation)

> **Date**: 2026-06-03
> **Author**: Hermes Agent
> **Baseline**: `origin/develop/v3.8.0`
> **Internal API**: UNSTABLE (v3.8.0 内不保证向后兼容)
> **Wire Protocol API**: STABLE (MySQL 5.7 协议子集)

---

## 0. TL;DR

v3.8.0 提供 2 个稳定 API:
1. **Wire Protocol API** (MySQL 5.7 协议子集) - 通过 `sqlrustgo-mysql-server` 暴露
2. **Canonical Binary CLI API** (subcommands) - 通过 `sqlrustgo-mysql-server <subcommand>` 暴露

内部 Rust crate API 不稳定, 仅供扩展使用。

---

## 1. Canonical Binary CLI API

### 1.1 启动
```bash
# 默认启动 (serve 模式)
sqlrustgo-mysql-server

# 显式指定 subcommand
sqlrustgo-mysql-server serve
```

### 1.2 Subcommands

#### `serve` - MySQL wire-protocol server
```
sqlrustgo-mysql-server serve [OPTIONS]

OPTIONS:
    --host <HOST>    Bind address [default: 127.0.0.1]
    --port <PORT>    Port number [default: 3306]
    --log-level <LVL> Log level (debug/info/warn/error) [default: info]
```

**Examples**:
```bash
# 启动监听所有接口 3306
sqlrustgo-mysql-server serve --host 0.0.0.0 --port 3306

# Debug 模式
sqlrustgo-mysql-server serve --log-level debug
```

#### `exec "<sql>"` - Execute single SQL
```
sqlrustgo-mysql-server exec "<SQL_STATEMENT>"
```

**Examples**:
```bash
sqlrustgo-mysql-server exec "SELECT 1+1"
sqlrustgo-mysql-server exec "CREATE TABLE t (id INT, name VARCHAR(50))"
sqlrustgo-mysql-server exec "INSERT INTO t VALUES (1, 'alice')"
```

#### `repl` - Interactive REPL
```
sqlrustgo-mysql-server repl
```

**交互模式**:
```
sqlrustgo> CREATE TABLE t (id INT);
OK
sqlrustgo> INSERT INTO t VALUES (1);
1 row affected
sqlrustgo> SELECT * FROM t;
| 1 |
sqlrustgo> .exit
```

#### `bench` - Performance benchmark
```
sqlrustgo-mysql-server bench [BENCHMARK_NAME]
```

**状态**: 部分实现 (placeholder for full feature migration)

#### `gmp` - GMP (AI Native) workflow
**状态**: placeholder

#### `diag` - Diagnostics
**状态**: placeholder

---

## 2. Wire Protocol API (MySQL 5.7 Subset)

### 2.1 连接示例
```bash
mysql -h 127.0.0.1 -P 3306 -u root
```

### 2.2 支持的 SQL 子集
参见 `docs/SQL92_COMPLIANCE.md` 和 `docs/releases/v3.8.0/test-design/FEATURE_MATRIX.md`

#### DDL
- CREATE TABLE / DROP TABLE
- CREATE INDEX / DROP INDEX
- ALTER TABLE (ADD/DROP COLUMN) — **部分支持**

#### DML
- INSERT (VALUES, SET)
- UPDATE
- DELETE
- SELECT (含 JOIN, GROUP BY, ORDER BY, LIMIT, OFFSET)

#### 事务
- BEGIN / START TRANSACTION
- COMMIT
- ROLLBACK
- AUTOCOMMIT

### 2.3 错误码
遵循 MySQL 5.7 错误码 (1064 syntax error, 1146 table not found, etc.)

---

## 3. Rust Crate API (Internal - UNSTABLE)

### 3.1 Crate 入口
```rust
// 公开 API (在 sqlrustgo 根 crate)
use sqlrustgo::*;  // 整个 engine + types
use sqlrustgo::executor::ExecutionEngine;
use sqlrustgo::storage::Storage;
use sqlrustgo::parser::Parser;
```

### 3.2 关键 Traits

#### `Storage` trait
```rust
pub trait Storage: Send + Sync {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;
    fn put(&mut self, key: &[u8], value: &[u8]) -> Result<()>;
    fn delete(&mut self, key: &[u8]) -> Result<()>;
    fn scan(&self, range: Range) -> Result<Box<dyn Iterator<...>>>;
}
```

实现:
- `MemoryStorage` (内存)
- `FileStorage` (磁盘)
- `WalStorage` (WAL 包装)

#### `Executor` trait
```rust
pub trait Executor {
    fn execute(&self, plan: &Plan) -> Result<Rows>;
    fn execute_dml(&self, plan: &DmlPlan) -> Result<u64>;
}
```

#### `Transaction` trait
```rust
pub trait Transaction {
    fn commit(self: Box<Self>) -> Result<()>;
    fn rollback(self: Box<Self>) -> Result<()>;
}
```

### 3.3 关键 Structs
- `ExecutionEngine<S: Storage>` - 主执行引擎
- `Parser` - SQL 解析
- `AstNode` - AST 节点 (SUM, SELECT, JOIN, etc.)
- `Value` - 数据值 (Int, Float, String, Null, etc.)
- `Row` - 数据行
- `Schema` - 表结构

### 3.4 Extension API

#### 自定义 Storage
```rust
use sqlrustgo::storage::Storage;

pub struct MyStorage { /* ... */ }

impl Storage for MyStorage {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        // 实现
    }
    // ... 其他方法
}

// 使用
let mut engine = ExecutionEngine::new(MyStorage::new());
```

#### 自定义 Executor
```rust
use sqlrustgo::executor::Executor;

pub struct MyExecutor { /* ... */ }

impl Executor for MyExecutor {
    fn execute(&self, plan: &Plan) -> Result<Rows> {
        // 实现
    }
    // ... 其他方法
}
```

---

## 4. Embedded Test Harness API

### 4.1 start_ephemeral
```rust
use sqlrustgo_mysql_server::testing::start_ephemeral;

let server = start_ephemeral().await?;
// 使用 server 跑测试
server.shutdown().await?;
```

### 4.2 MySqlTestClient
```rust
use tests::common::MySqlTestClient;

let client = MySqlTestClient::connect("127.0.0.1:3306").await?;
let result = client.query("SELECT 1+1").await?;
```

---

## 5. Vector Store API (Experimental)

### 5.1 启用
```toml
[dependencies]
sqlrustgo = { version = "0.1", features = ["vector"] }
```

### 5.2 Usage
```rust
use sqlrustgo::vector::{VectorStore, Distance};

// 创建向量存储
let mut store = VectorStore::new(Distance::Cosine);

// 插入
store.insert(1, vec![0.1, 0.2, 0.3])?;

// 查询
let results = store.query(&vec![0.1, 0.2, 0.3], 10)?; // top-10
```

### 5.3 SIMD 加速
- 内部使用 `crates/vector/src/simd_explicit.rs`
- 13 AVX2 intrinsics
- 自动检测 CPU 支持 (SSE/AVX/AVX2/AVX-512)

---

## 6. Benchmark Suite API

### 6.1 TPC-H
```rust
use sqlrustgo_bench::tpch;

let mut bench = tpch::Benchmark::new(sf = 0.1);
bench.run_query(1)?;  // Q1 pricing summary
```

### 6.2 Sysbench-equivalent
```rust
use sqlrustgo_bench::sysbench;

let mut bench = sysbench::Benchmark::new();
bench.run_oltp_read_write(threads = 4, duration = 60)?;
```

---

## 7. 错误处理

### 7.1 SqlError enum
```rust
pub enum SqlError {
    SyntaxError(String),
    TableNotFound(String),
    ColumnNotFound { table: String, column: String },
    TypeMismatch { expected: String, found: String },
    IoError(io::Error),
    WalError(WalError),
    // ... 50+ variants
}
```

### 7.2 Result<T> 包装
```rust
pub type Result<T> = std::result::Result<T, SqlError>;
```

---

## 8. 版本兼容性 (Version Compatibility)

### 8.1 Wire Protocol (STABLE)
- v3.8.0 兼容 MySQL 5.7 协议子集
- 客户端应用可使用 mysql-client 8.x 或 5.7.x
- 协议版本: 10

### 8.2 Rust Crate API (UNSTABLE)
- v3.8.0 内部 API 可能变化
- 升级时需重新编译
- 升级时需 re-test

### 8.3 升级路径
- 旧 binary → canonical binary: 通过 subcommand 兼容
- 数据格式: v3.7.0 → v3.8.0 兼容 (storage format 稳定)

---

## 9. 链接 (Links)

- **Source code**: `crates/` (workspace)
- **Specs**: `docs/releases/v3.8.0/specs/`
- **Test plan**: `docs/releases/v3.8.0/test-design/TEST_PLAN_INTEGRATED.md`
- **CHANGELOG**: `CHANGELOG.md`

---

## 10. 结论

v3.8.0 提供:
- ✅ **稳定**: Wire Protocol API (MySQL 5.7 协议子集)
- ✅ **稳定**: Canonical Binary CLI (6 subcommands)
- ⚠️ **UNSTABLE**: Rust Crate API (内部扩展用)
- ✅ **Experimental**: Vector Store API (带 SIMD 加速)

**建议**: 外部集成用 Wire Protocol; 内部扩展用 Rust API (升级时注意测试)。
