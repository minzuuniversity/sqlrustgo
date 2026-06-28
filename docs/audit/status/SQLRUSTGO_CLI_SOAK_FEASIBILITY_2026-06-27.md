# sqlrustgo-cli 作为 SOAK 测试前端的可行性评估

> **任务**: 评估 `sqlrustgo-cli` 是否可用作 SOAK 测试前端
> **日期**: 2026-06-27
> **评估者**: Claude AI (reviewing PR #3347 上下文)

## 一、TL;DR

**结论**: 当前 `sqlrustgo-cli` **不可直接**作为 SOAK 测试前端，缺少 PR #3347 `tpch_soak_driver.py` 所依赖的两个核心能力：

1. ❌ **长连接支持** — 当前每次 `cli` 调用都建立新 TCP 连接
2. ❌ **多线程支持** — 当前 `sqlrustgo-cli` 是单进程单连接 CLI

**但** `sqlrustgo-cli` 具备**战略优势**：它直接使用 `sqlrustgo-mysql-client` 库（Rust 原生实现），已支持 COM_QUERY / COM_STMT_PREPARE / COM_STMT_EXECUTE / 多语句。扩展为 SOAK 前端**比 PR #3347 的 Python mysql CLI 路线更高效**（无 subprocess spawn 开销）。

---

## 二、PR #3347 现状

PR #3347 实现了 **3 层 SOAK 测试框架** + **多线程 server**：

### 关键 commits

| Commit | 描述 |
|--------|------|
| `b6c1db7e1` test(p1-3): 3-layer soak test framework | 3 层 SOAK 框架：Rust harness + Python TPC-H driver + Reporting |
| `d2f2a9d65` refactor(p2-1): multi-thread server runtime analysis | 多线程 server OpenSpec (WorkerPool 设计) |
| `9aa9e881b` feat(server): configurable multi-thread worker pool | 实现 WorkerPool + `--worker-threads` CLI |

### PR #3347 关键性能数据

- **持久连接批处理** (16 线程, batch=22): **27,000 QPS**
- **subprocess-per-query** (16 线程): **93 QPS**
- **in-process direct** (无网络): **10,000+ QPS**
- **结论**: 瓶颈是 `mysql CLI` subprocess spawn 开销，不是 server 端并发能力

### PR #3347 TPC-H driver (Python) 核心逻辑

```python
def worker(tid, host, port, user, queries, batch_size, stop, stats):
    batch = make_batch(queries[:batch_size])  # 22 个 TPC-H 查询合并为 1 个 COM_QUERY
    while time.time() < stop:
        r = subprocess.run(["mysql", ...], input=batch, capture_output=True, text=True, timeout=30)
        # 收集 latency, 写入 stats
```

**核心需求**:
- 持久连接 (每个 thread 1 个)
- 批处理 (多个语句合并为 1 个 COM_QUERY)
- 多线程并发 (16 thread pool)

---

## 三、当前 `sqlrustgo-cli` 能力评估

### 3.1 文件结构

```
crates/sqlrustgo-cli/
├── Cargo.toml        (仅 clap, tokio 依赖)
├── src/
│   ├── main.rs       (10 行: 调用 lib::run())
│   └── lib.rs        (204 行: CLI + 完整 MySQL 客户端)
```

### 3.2 当前 `cli` 子命令

```rust
SubCmd::Cli {
    host: String,         // 默认 127.0.0.1
    port: u16,            // 默认 3307
    user: Option<String>,
    password: Option<String>,
    query: String,        // 单次查询
}
```

### 3.3 当前 `run_cli()` 流程

```rust
fn run_cli(query: &str, host: &str, port: u16, user: &str, password: &str) -> i32 {
    let addr = format!("{host}:{port}").parse()?;
    let mut conn = MySqlConnection::connect(&addr, user, password, "")?;  // 每次新建连接
    println!("Connected to {}:{} (server: {})", host, port, conn.server_version);
    let result = conn.execute(query)?;  // 单次执行
    // 输出结果
    // conn 在函数结束时 drop
}
```

**关键问题**:
- `MySqlConnection` 在 `run_cli` 结束时被 drop
- TCP 连接被关闭
- **不支持** 跨多次调用的连接复用

### 3.4 当前支持 vs PR #3347 需要

| 能力 | sqlrustgo-cli 现状 | PR #3347 SOAK driver 需求 | 评估 |
|------|-------------------|--------------------------|------|
| MySQL wire protocol | ✅ 通过 `sqlrustgo-mysql-client` | ✅ | 优势 |
| COM_QUERY | ✅ `conn.execute()` | ✅ | OK |
| 多语句 COM_QUERY | ✅ `execute_multi()` | ✅ `make_batch()` | OK |
| COM_STMT_PREPARE | ✅ `conn.prepare()` | ❌ 未用 (用 batch 而非 prepared) | 优势未用 |
| COM_STMT_EXECUTE | ✅ `conn.execute_prepared()` | ❌ | 优势未用 |
| **长连接** | ❌ 每次新建 | ✅ thread 持有 1 个 | **缺失** |
| **多线程** | ❌ 单连接单进程 | ✅ 16 线程 | **缺失** |
| 批处理 | ❌ | ✅ batch_size 参数 | **缺失** |
| 资源监控 (RSS/FD) | ❌ | ✅ 单独 sampler thread | **缺失** |

---

## 四、扩展方案：将 `sqlrustgo-cli` 改造为 SOAK 前端

### 4.1 方案 A: 子进程长连接 REPL 模式（最简单）

**思路**: `sqlrustgo cli --persistent` 启动一个长连接 REPL 进程，Python driver 通过 stdin/stdout 通信（类似 mysql CLI 行为，但更快）。

```rust
// crates/sqlrustgo-cli/src/lib.rs 新增子命令:
SubCmd::Soak {
    host: String,
    port: u16,
    user: Option<String>,
    password: Option<String>,
    // REPL 模式: 读 stdin 上的 "QUERY\n" 行，写 stdout 上的结果
}
```

```rust
fn run_soak_repl(host, port, user, password) -> i32 {
    let conn = MySqlConnection::connect(&addr, user, password, "")?;
    // 读 stdin 行循环
    for line in std::io::stdin().lines() {
        let query = line?;
        let result = conn.execute(&query)?;
        // 写 stdout: status + columns + rows
    }
}
```

**优点**:
- 单进程实现，不修改 driver
- 连接复用，零 subprocess 开销
- 复用现有 Rust MySQL 客户端
- Python driver 简单改 1 行 (`subprocess.run(['sqlrustgo', 'soak', ...])`)

**缺点**:
- 仍然有 stdin/stdout pipe 开销（但比 mysql CLI 的完整 subprocess 启动快 10x+）
- 单进程单连接（线程 = 进程）

### 4.2 方案 B: 持久连接多线程模式（推荐）

**思路**: sqlrustgo-cli 启动后保持 N 个长连接线程，每个线程独立接受查询。

```rust
// 新增子命令:
SubCmd::SoakMt {
    host: String,
    port: u16,
    user: Option<String>,
    password: Option<String>,
    threads: u16,         // 连接数（默认 16）
    max_connections: u32, // 接受的最大并发数
}
```

**实现**:
- 启动时创建 N 个 `MySqlConnection` 池
- `TcpListener` 接受本地驱动连接
- 每个驱动连接分配一个 backend connection（轮询）
- 请求转发: 驱动 → stdin → backend MySQLConnection

**优点**:
- 多线程支持 (16+ connections in pool)
- 持久连接（无 spawn 开销）
- 复用 PR #3347 多线程 server 优化

**缺点**:
- 实现复杂（需要连接池、负载均衡）
- 与 mysql CLI 行为差异大（driver 需要修改）

### 4.3 方案 C: 配合 PR #3347 既有方式 (过渡)

**思路**: 不改造 sqlrustgo-cli，直接用 PR #3347 的 `tpch_soak_driver.py`。优势是零代码改动，劣势是仍受 subprocess 开销影响。

---

## 五、推荐路线

### 短期 (1-2 天)
**实施方案 A**: `sqlrustgo cli --persistent` REPL 模式
- 加一个 `Soak` 子命令
- 单连接 stdin/stdout 通信
- 修改 PR #3347 的 `tpch_soak_driver.py` 用 `subprocess.run(['sqlrustgo', 'soak', ...])` 替换 `mysql`
- **预期 QPS 提升**: 93 → 数千 (无 subprocess 启动开销)

### 中期 (1 周)
**实施方案 B**: 多线程连接池
- 在 sqlrustgo-cli 加 `SoakMt` 子命令
- 实现 Rust 端连接池
- 修改 Python driver 用 socket 直接通信（绕过 stdin/stdout pipe）
- **预期 QPS 提升**: 数千 → 接近 27,000 (PR #3347 持久连接水平)

### 长期
- sqlrustgo-cli 完全替代 `mysql CLI` 作为 SOAK driver
- 配合 PR #3347 多线程 server (--worker-threads) 实现端到端高性能 SOAK

---

## 六、具体实现工作（短期，方案 A）

### 6.1 `sqlrustgo-cli/src/lib.rs` 改动

```rust
#[derive(Subcommand)]
enum SubCmd {
    // ... 现有子命令 ...
    /// Soak mode: 保持持久连接，循环从 stdin 读取查询，输出结果到 stdout
    /// (PR #3347 替代 mysql CLI 方案)
    Soak {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(short, long, default_value = "3307")]
        port: u16,
        #[arg(short, long)]
        user: Option<String>,
        #[arg(short, long)]
        password: Option<String>,
    },
}

fn run_soak_repl(host: &str, port: u16, user: &str, password: &str) -> i32 {
    use sqlrustgo_mysql_client::MySqlConnection;
    use std::io::{self, BufRead, Write};

    let addr: std::net::SocketAddr = match format!("{host}:{port}").parse() {
        Ok(a) => a,
        Err(e) => { eprintln!("Invalid address: {e}"); return 1; }
    };

    let mut conn = match MySqlConnection::connect(&addr, user, password, "") {
        Ok(c) => c,
        Err(e) => { eprintln!("Connection failed: {e}"); return 1; }
    };

    eprintln!("# Soak REPL ready: server={}", conn.server_version);
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line { Ok(l) => l, Err(_) => break };
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        if line == "QUIT" { break; }

        match conn.execute(line) {
            Ok(result) => {
                use sqlrustgo_mysql_client::ResultSet;
                match result {
                    ResultSet::Ok { affected_rows, .. } => {
                        writeln!(out, "OK\t{}", affected_rows).ok();
                    }
                    ResultSet::Select { columns, rows } => {
                        writeln!(out, "ROWS\t{}", columns.len()).ok();
                        for c in &columns { writeln!(out, "COL\t{}\t{}", c.name, c.column_type).ok(); }
                        writeln!(out, "DATA\t{}", rows.len()).ok();
                        for r in &rows { writeln!(out, "ROW\t{}", r.join("\t")).ok(); }
                    }
                    ResultSet::Error { error_code, error_message, .. } => {
                        writeln!(out, "ERR\t{}\t{}", error_code, error_message).ok();
                    }
                }
            }
            Err(e) => { writeln!(out, "ERR\t-1\t{}", e).ok(); }
        }
        out.flush().ok();
    }
    0
}
```

### 6.2 `tpch_soak_driver.py` 改动 (5 行)

```python
# Before:
r = subprocess.run(["mysql", "-h", host, "-P", str(port), ...], input=batch, ...)

# After:
r = subprocess.run(["sqlrustgo", "soak", "-h", host, "-P", str(port), ...],
                   input=batch, ...)
```

但需解析新的 tab-separated 输出格式（OK\tn / ROW\t...），而不是 mysql 的默认格式。

### 6.3 性能预期

- 当前 mysql CLI 路线: 93 QPS (subprocess spawn 开销)
- sqlrustgo-cli REPL 路线: 数千 QPS (无 spawn 开销，仅 stdin/stdout pipe)
- 预期提升: 10-50x

---

## 七、总结

| 维度 | 现状 | 扩展后 |
|------|------|--------|
| **长连接** | ❌ | ✅ REPL 模式 |
| **多线程** | ❌ | ⚠️ 短期单连接; 中期多线程池 |
| **QPS** | N/A (cli 只能单查询) | 短期数千, 中期接近 27,000 |
| **代码复杂度** | 低 (cli 子命令) | 中 (REPL 解析) - 高 (多线程池) |
| **向后兼容** | - | ✅ 不破坏现有 cli 用法 |

**推荐**: 先实施方案 A (1-2 天)，立即获得 10-50x QPS 提升。然后评估是否需要进入中期多线程实现。
