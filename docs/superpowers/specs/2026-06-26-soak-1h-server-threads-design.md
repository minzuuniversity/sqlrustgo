# SOAK 1h 调优：服务端 --server-threads 客户端 16 线程 — 设计规范

> **目标:** 把 `scripts/stability/run_wired_soak.sh` (当前 30 分钟级 / 8 线程) 升级到
> **1 小时级 / 16 客户端线程 / 16 服务端 worker 线程 (上限 80)**, 作为 Issue
> [#3265](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3265) 72h Z6G4 跑前的
> 并发参数基线验证。
>
> **范围:**
> 1. `sqlrustgo-mysql-server serve` 新增 `--server-threads N` CLI 参数 (0..=80)
> 2. accept loop 改为 mpsc channel + 固定 worker pool (N>0 模式), N=0 保留旧行为
> 3. `scripts/stability/run_wired_soak.sh` 默认值改 HOURS=1 / THREADS=16 / SERVER_THREADS=16
> 4. 10 个新测试 (4 单元 + 3 CLI + 3 e2e) + 3 分钟冒烟 + 1h 正式跑
>
> **不在范围:** 改 async/tokio、改 epoll-based accept、改 run_24h/72h_soak_v2.sh、
> 把 `--max-connections` 真正生效、引入 crossbeam-channel。

## 1. 架构与数据流

```
┌────────────────────────────────────────────────────────────────┐
│ sqlrustgo-mysql-server serve [flags]                          │
│                                                                │
│   --host              (existing)                              │
│   --port              (existing)                              │
│   --data-dir          (existing)                              │
│   --max-connections   (existing, default 100, no-op currently) │
│   --server-threads N  (NEW, default 16, range 0..=80,         │
│                        0 = unbounded = current behavior)       │
│   --auth-mode         (existing)                              │
│   --log-level         (existing)                              │
└────────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌────────────────────────────────────────────────────────────────┐
│ run_server_v2(..., server_threads: usize)                      │
│   ├── publish to ACTIVE_CONFIG (EphemeralConfig{               │
│   │     server_threads, data_dir, ...                         │
│   │   })                                                       │
│   └── call run_server_with_listener_and_shutdown_with_         │
│         bootstrap_tables_and_sql(                             │
│           ..., ServerThreadPool { workers, channel }           │
│         )                                                      │
└────────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌────────────────────────────────────────────────────────────────┐
│ accept loop (lib.rs:2689-2708)                                 │
│   while !shutdown {                                            │
│     (stream, addr) = listener.accept()                         │
│     match pool {                                              │
│       None => thread::spawn(handle_connection(...))           │
│       Some(p) => p.tx.send(ServerJob {...})                   │
│     }                                                          │
│   }                                                            │
└────────────────────────────────────────────────────────────────┘
                          │
                          ▼ (N worker threads, N = server_threads)
┌────────────────────────────────────────────────────────────────┐
│ for _ in 0..N {                                               │
│     thread::spawn(move || {                                   │
│         loop {                                                 │
│             let job = rx.lock().recv()?;                       │
│             catch_unwind(|| handle_connection(job));          │
│             if recv() == Err { break; }                       │
│         }                                                      │
│     });                                                        │
│ }                                                              │
└────────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌────────────────────────────────────────────────────────────────┐
│ EphemeralHandle / shutdown path:                               │
│   - drop tx → rx.recv() returns Err → worker exits             │
│   - workers join via ServerThreadPool::join() on Drop          │
└────────────────────────────────────────────────────────────────┘
```

**关键决策:**
- `server_threads=0` (默认行为) 保留旧语义: accept loop 直接 `thread::spawn`,
  无 channel/worker pool, 完全向后兼容现有用户
- `server_threads=N>0`: channel + N workers 模式
- channel buffer size = `N * 2` (容纳 2 个 round-trip 的突发)
- 数据通过 `EphemeralConfig::server_threads` 字段传到 lib 内部 (与现有 `data_dir`
  字段并列)

## 2. CLI 表面、默认值与校验

### 2.1 新增 CLI 参数 (`crates/mysql-server/src/main.rs`)

```rust
#[derive(Subcommand, Debug)]
enum Command {
    Serve {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value = "3306")]
        port: u16,
        #[arg(long, default_value = "/tmp/sqlrustgo-data")]
        data_dir: String,
        #[arg(long, default_value_t = 100)]
        max_connections: usize,
        #[arg(long, default_value_t = 16,
              value_parser = validate_server_threads)]
        server_threads: usize,                    // ← NEW
        #[arg(long, default_value = "none")]
        auth_mode: String,
        #[arg(long, default_value_t = false)]
        verbose: bool,
    },
    // ... 其他子命令不变
}

fn validate_server_threads(s: &str) -> Result<usize, String> {
    let n: usize = s.parse()
        .map_err(|e| format!("not an integer: {e}"))?;
    if n > 80 {
        return Err(format!("must be ≤ 80 (got {n})"));
    }
    Ok(n)
}
```

### 2.2 行为矩阵

| `--server-threads` 值 | 行为 | 用途 |
|---|---|---|
| `0` | 不启 worker pool; accept loop 直接 `thread::spawn` | 现有用户无需改 |
| `1..=80` | 启 N 个 worker + `sync_channel(N*2)` | 生产/压测 (推荐) |
| `>80` 或非整数 | CLI 启动失败, `ExitCode::from(64)` (EX_USAGE) | 显式拒绝 |
| 未指定 | `main.rs` 默认 16 | 安全默认值 |

### 2.3 `--max-connections` 现状

| 参数 | 当前实现 | 决定 |
|---|---|---|
| `--max-connections` (default 100) | 仅 `set_var("SQLRUSTGO_MAX_CONN", ...)`, 代码无读 | **保留**, 不删 (避免破坏外部脚本); 未来若需联动再补逻辑 |

### 2.4 `EphemeralConfig` 字段新增 (`crates/mysql-server/src/lib.rs`)

```rust
pub struct EphemeralConfig {
    pub host: String,
    pub bootstrap_tables: bool,
    pub bootstrap_users: bool,
    pub data_dir: Option<std::path::PathBuf>,
    pub bootstrap_sql: Vec<String>,
    pub bulk_insert_buffer_size: usize,
    pub server_threads: usize,            // ← NEW (concrete, 与 CLI 默认对齐)
                                            // 0 = 不限流 (legacy, 仅显式传入)
                                            // 1..=80 = worker pool size
                                            // 默认 16 (与 main.rs CLI 默认一致)
}

impl Default for EphemeralConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            bootstrap_tables: true,
            bootstrap_users: true,
            data_dir: None,
            bootstrap_sql: Vec::new(),
            bulk_insert_buffer_size: 1_048_576,
            server_threads: 16,              // ← NEW (与 CLI 默认一致)
        }
    }
}
```

> **注意:** 与 `data_dir: Option<PathBuf>` (None = auto-create temp dir) 不同,
> `server_threads` 是**单一数字**而非 Option — 因为没有"未指定"的语义需求,
> 测试 harness 与 CLI 共享同一默认值 16。`run_server_v2` 接到 CLI 解析的
> `usize` 后, 直接 `EphemeralConfig { server_threads: cli_value, .. }` 覆盖。

## 3. 并发模型 (mpsc + worker pool)

### 3.1 Channel 选择与拓扑

```rust
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};

pub struct ServerThreadPool {
    tx: SyncSender<ServerJob>,
    workers: Vec<std::thread::JoinHandle<()>>,
    rx: Arc<Mutex<Receiver<ServerJob>>>,     // 多 worker 共享
}

const CHANNEL_BUFFER_MULTIPLIER: usize = 2;   // 总 buffer = N * 2
```

**为什么选 `std::sync::mpsc::sync_channel`:**
- 零新依赖 (已有 std)
- `SyncSender::send` 在 buffer 满时**阻塞** → accept loop 自然背压
- `Receiver` 不可 `Clone` (天然单消费者) → 多个 worker 通过 `Arc<Mutex<Receiver>>` 共享
  (避免引入 crossbeam)

### 3.2 Job 结构体

```rust
pub struct ServerJob {
    pub stream: TcpStream,
    pub addr: SocketAddr,
    pub storage: Arc<Storage>,         // 已是 Arc, clone cheap
    pub tls_config: Option<Arc<TlsConfig>>,
    pub user_store: UserStore,         // Clone cheap (derive Clone)
}
```

> `UserStore` 已 derive `Clone, Default` (`crates/mysql-server/src/lib.rs:144`),
> 内部 `HashMap<String, UserPassword>` clone 廉价, **直接 move 进 ServerJob 即可**, 不需 Arc 包装。

### 3.3 Worker 主循环 (含 panic 隔离)

```rust
fn worker_loop(rx: Arc<Mutex<Receiver<ServerJob>>>, worker_id: usize) {
    loop {
        let job = {
            let rx = rx.lock().expect("worker mutex poisoned");
            match rx.recv() {
                Ok(job) => job,
                Err(_) => {
                    tracing::debug!("worker {worker_id}: channel closed, exiting");
                    return;
                }
            }
        };  // 锁在 recv 后立即释放

        let result = std::panic::catch_unwind(
            std::panic::AssertUnwindSafe(|| {
                handle_connection(job.stream, job.addr, job.storage,
                                  job.tls_config, job.user_store)
            })
        );
        if let Err(e) = result {
            tracing::error!(
                "worker {worker_id}: connection handler panicked: {:?}",
                e.downcast_ref::<&str>().unwrap_or(&"unknown")
            );
        }
    }
}
```

### 3.4 accept loop 改造 (关键 diff)

```rust
// before (lib.rs:2689-2708) — 无 pool
while !shutdown.load(Ordering::SeqCst) {
    match listener.accept() {
        Ok((stream, addr)) => {
            thread::spawn(move || handle_connection(stream, addr, st, tc, us));
        }
        Err(e) if e.kind() == ErrorKind::WouldBlock => { sleep_50ms() }
        Err(e) => { log + sleep_50ms() }
    }
}

// after — 有 pool 分支
// 注: server_threads 是 concrete usize (来自 CLI 默认 16 或用户指定 0..=80)
let pool = if server_threads == 0 {
    None    // 0 = 不限流 (legacy)
} else {
    Some(ServerThreadPool::start(server_threads, st.clone(), tc.clone(), us.clone()))
};
while !shutdown.load(Ordering::SeqCst) {
    match listener.accept() {
        Ok((stream, addr)) => {
            match &pool {
                None => {
                    thread::spawn(move || handle_connection(stream, addr, st, tc, us));
                }
                Some(p) => {
                    let job = ServerJob { stream, addr,
                                          storage: st.clone(),
                                          tls_config: tc.clone(),
                                          user_store: us.clone() };
                    if p.tx.send(job).is_err() {
                        tracing::warn!("worker pool shut down; dropping connection");
                    }
                }
            }
        }
        Err(e) if e.kind() == ErrorKind::WouldBlock => { sleep_50ms() }
        Err(e) => { log + sleep_50ms() }
    }
}
// 退出时 drop pool → 所有 worker 退出循环
drop(pool);
```

### 3.5 Worker 优雅退出

```rust
impl ServerThreadPool {
    fn start(n: usize, st: Arc<Storage>, tc: Option<Arc<TlsConfig>>,
             us: UserStore) -> Self {
        let (tx, rx) = sync_channel(n * CHANNEL_BUFFER_MULTIPLIER);
        let rx = Arc::new(Mutex::new(rx));
        let mut workers = Vec::with_capacity(n);
        for worker_id in 0..n {
            let rx = rx.clone();
            workers.push(thread::spawn(move || worker_loop(rx, worker_id)));
        }
        Self { tx, workers, rx }
    }

    fn join(self) {
        drop(self.tx);
        for h in self.workers {
            let _ = h.join();
        }
    }
}
```

### 3.6 风险点与缓解

| 风险 | 缓解 |
|---|---|
| 所有 worker 崩溃 → `tx.send` 永远阻塞 | worker panic 已用 `catch_unwind` 隔离; shutdown 路径 drop tx 解阻 |
| `Mutex<Receiver>` 锁竞争 (多 worker 等同一锁) | N=16 时竞争不激烈; profile 显示瓶颈再换 `crossbeam-channel` |
| Channel buffer = N*2 太小导致 accept 频繁阻塞 | 16 worker × 2 = 32 in-flight 任务足够吸收突发 |
| `ServerJob` 大小 (拷贝) | 所有字段都是 Arc/原生句柄, move 即可无拷贝 |
| Drop `tx` 后 in-flight 连接被截断 | join() 在 shutdown flag 之后调用, listener 已停接新连接 |

## 4. `run_wired_soak.sh` 集成

### 4.1 默认值变更

| 位置 | 旧默认 | 新默认 |
|---|---|---|
| `HOURS` | 24 | **1** |
| `THREADS` (sysbench 客户端) | 8 | **16** |
| `SERVER_THREADS` (新增) | n/a | **16** |

### 4.2 新增/修改环境变量 (`scripts/stability/run_wired_soak.sh`)

```bash
HOURS="${HOURS:-1}"                # 旧默认 24 → 新默认 1
THREADS="${THREADS:-16}"           # 旧默认 8  → 新默认 16
TABLE_SIZE="${TABLE_SIZE:-10000}"
TABLES="${TABLES:-1}"
PORT="${PORT:-3396}"
HOST="${HOST:-127.0.0.1}"
FIXTURE="${FIXTURE:-tpch-sf001}"
TPCH_ROTATE="${TPCH_ROTATE:-1}"
TPCH_ROTATE_MAX_ROUNDS="${TPCH_ROTATE_MAX_ROUNDS:-0}"
SQLRUSTGO_BIN="${SQLRUSTGO_BIN:-./target/release/sqlrustgo-mysql-server}"

# 新增
SERVER_THREADS="${SERVER_THREADS:-16}"
SERVER_THREADS_MAX=80

# 校验
if ! [[ "$SERVER_THREADS" =~ ^[0-9]+$ ]]; then
    echo "FAIL: SERVER_THREADS='$SERVER_THREADS' is not an integer" >&2; exit 1
fi
if [ "$SERVER_THREADS" -gt "$SERVER_THREADS_MAX" ]; then
    echo "FAIL: SERVER_THREADS=$SERVER_THREADS > $SERVER_THREADS_MAX (binary rejects)" >&2; exit 1
fi
```

### 4.3 启动命令改动 (`run_wired_soak.sh:195-199`)

```bash
# 旧
nohup bash -c "$LIMIT_PREFIX exec '$SQLRUSTGO_BIN' serve \
    --host '$HOST' --port '$PORT' \
    --data-dir '$DATA_DIR' \
    --log-level info" \
    > "$LOG_FILE" 2>&1 &

# 新
nohup bash -c "$LIMIT_PREFIX exec '$SQLRUSTGO_BIN' serve \
    --host '$HOST' --port '$PORT' \
    --data-dir '$DATA_DIR' \
    --log-level info \
    --server-threads '$SERVER_THREADS'" \
    > "$LOG_FILE" 2>&1 &
```

### 4.4 启动横幅更新

```bash
echo "Hours=$HOURS_DISPLAY  Interval=${INTERVAL}s  Threads=$THREADS"
echo "ServerThreads=$SERVER_THREADS  Port=$PORT  Host=$HOST  Data=$DATA_DIR"
```

### 4.5 `STABILITY_REPORT.md` 新增行

```markdown
| sysbench threads | $THREADS |
| **server worker threads** | **$SERVER_THREADS (max $SERVER_THREADS_MAX)** |
```

### 4.6 变更范围

| 文件 | 是否改 | 说明 |
|---|---|---|
| `scripts/stability/run_wired_soak.sh` | ✅ 是 | 改默认值 + 加 SERVER_THREADS + 启动命令 + 报告 |
| `scripts/stability/launch_parallel_soak.sh` | ❌ 否 | 通过环境透传 (不修改内部默认) |
| `scripts/stability/run_24h_soak_v2.sh` | ❌ 否 | 用户未要求 |
| `scripts/stability/run_72h_soak_v2.sh` | ❌ 否 | 用户未要求 |
| `scripts/stability/run_tpch_30min.sh` | ❌ 否 | 用户未要求 |

## 5. 修改文件清单

| # | 文件 | 改动 |
|---|------|------|
| 1 | `crates/mysql-server/src/main.rs` | Serve subcommand 新增 `--server-threads` 参数 + `validate_server_threads()` |
| 2 | `crates/mysql-server/src/lib.rs` | `EphemeralConfig` 新增 `server_threads: Option<usize>` |
| 3 | `crates/mysql-server/src/lib.rs` | 新增 `ServerJob` struct + `ServerThreadPool::start/join` + `worker_loop` |
| 4 | `crates/mysql-server/src/lib.rs` | accept loop (lib.rs:2689-2708) 改为 match pool 模式 |
| 5 | `crates/mysql-server/src/lib.rs` | `run_server_v2` / `run_server_with_listener_and_shutdown_with_bootstrap_tables_and_sql` 新增 `server_threads: usize` 参数 (concrete, 不为 Option) |
| 6 | `crates/mysql-server/src/lib.rs` `#[cfg(test)] mod` | 新增 4 个 ServerThreadPool 单元测试 + 4 个 CLI 校验测试 |
| 7 | `tests/server_thread_pool_e2e_test.rs` (新文件) | 3 个 e2e 集成测试 |
| 8 | `scripts/stability/run_wired_soak.sh` | 默认值改 HOURS=1 / THREADS=16 / SERVER_THREADS=16 + 校验 + 启动命令 + 报告 |
| 9 | `docs/superpowers/specs/2026-06-26-soak-1h-server-threads-design.md` | 本设计文档 |
| 10 | `docs/releases/v3.9.0/SOAK_72H_LIVE_STATUS_2026-06-19.md` | 追加 1h 跑结果章节 |

## 6. 验收准则

### 6.1 单元测试 (`crates/mysql-server/src/lib.rs` 内 `#[cfg(test)] mod`)

```rust
#[test]
fn server_thread_pool_processes_all_jobs() {
    // n=4 workers, 发 100 个 job, 每个 job 简单加 counter
}

#[test]
fn server_thread_pool_panic_isolation() {
    // n=2 worker, 其中一个 job panic; 另一个 job 必须正常完成
}

#[test]
fn server_thread_pool_graceful_shutdown() {
    // 启 4 worker, drop tx, 所有 worker 在 join() 中干净退出
}

#[test]
fn server_thread_pool_unbounded_mode_works() {
    // server_threads=0 模式: 直接 thread::spawn (验证旧路径不退化)
}
```

### 6.2 CLI 校验测试

```rust
#[test] fn cli_server_threads_default_16() { ... }       // 默认 16
#[test] fn cli_server_threads_rejects_81() { ... }       // >80 拒绝
#[test] fn cli_server_threads_rejects_non_integer() { ... } // 非整数拒绝
#[test] fn cli_server_threads_accepts_80() { ... }       // 边界值 80 OK
```

### 6.3 e2e 集成测试 (`tests/server_thread_pool_e2e_test.rs`)

```rust
#[test] fn e2e_server_threads_16_handles_burst() { ... }   // 32 并发 client, 320 请求全成功
#[test] fn e2e_server_threads_0_legacy_behavior() { ... }  // 等价旧行为
#[test] fn e2e_server_threads_backpressure() { ... }       // server_threads=1 + 32 client → 不死锁
```

### 6.4 回归测试

| 已有测试 | 期望 |
|---|---|
| `concurrency_stress_test::*` | 不退化 (in-process 不走 worker pool) |
| `tests/long_run_stability_*` | 不退化 (受 server_threads=0 默认保护) |
| WAL P0 回归 (PR #3533) | 不退化 (WAL 仍 bounded) |
| G15 wire oracle 22/22 | 不退化 (Q8/Q9 不再 hit 300s) |
| 6/6 meta-gates (P11-P16) | 全 PASS |

### 6.5 端到端验收

**3 分钟冒烟 (开发机/CI):**

```bash
HOURS=0.05 THREADS=16 SERVER_THREADS=16 \
    bash scripts/stability/run_wired_soak.sh
# 期望: ~36 行样本 (5s interval × 3 min); STABILITY_REPORT 全 PASS
```

**1 小时正式跑 (开发机/夜跑):**

```bash
HOURS=1 THREADS=16 SERVER_THREADS=16 \
    bash scripts/stability/run_wired_soak.sh
# 期望指标 (24h 标准 1/24 缩放):
#   - Crashes: 0
#   - RSS growth: < 2 MB
#   - FD growth: < 5
#   - Final RSS: < 4096 MB
#   - Final WAL: < 10240 MB (PR #3533 后预期 0 MB)
#   - sysbench errors: 0
#   - TPC-H rounds: ≥ 1
#   - Q8/Q9 timeout: 不出现
```

### 6.6 完成定义 (Definition of Done)

1. ✅ `--server-threads 0..=80` CLI flag 实现并通过 clap 校验
2. ✅ `ServerThreadPool` 在 `lib.rs` 实现 + 4 个单元测试全过
3. ✅ `ServerJob` 在 accept loop 替换 `thread::spawn` (N>0 模式)
4. ✅ `run_wired_soak.sh` 默认值改为 HOURS=1 / THREADS=16 / SERVER_THREADS=16
5. ✅ `cargo clippy --all-features -- -D warnings` 通过
6. ✅ `cargo fmt --check --all` 通过
7. ✅ `cargo test --all-features` 全部通过 (含新增 4+4+3 = 11 个测试)
8. ✅ 3 分钟短冒烟 PASS (STABILITY_REPORT.md)
9. ✅ 1 小时跑通且报告归档到 `test_results/wired_soak_1h_*/`
10. ✅ 更新 Issue #3265 评论: 附 1h 跑结果链接 + STABILITY_REPORT 摘要

## 7. 上线计划 (Rollout)

### 阶段 1: 本地开发与单测 (开发机) — 估时 1.5h

| 步骤 | 内容 | 预期时间 |
|---|---|---|
| 1.1 | 在 `main.rs` / `lib.rs` 实现 `--server-threads` + worker pool | 30 min |
| 1.2 | 新增 4 单元 + 4 CLI 测试 | 20 min |
| 1.3 | `cargo build --all-features` | 3 min |
| 1.4 | `cargo clippy --all-features -- -D warnings` | 2 min |
| 1.5 | `cargo fmt --check --all` | 1 min |
| 1.6 | `cargo test -p sqlrustgo-mysql-server --all-features` | 5 min |
| 1.7 | 改 `run_wired_soak.sh` 默认值 + 新增 SERVER_THREADS | 10 min |
| 1.8 | `bash scripts/gate/check_docs_links.sh` | 1 min |

**阶段 1 gate**: `cargo test --all-features` + `cargo clippy -D warnings` 全过; 新增 8 个测试全 PASS。

### 阶段 2: 3 分钟冒烟 — 估时 5 min

```bash
HOURS=0.05 THREADS=16 SERVER_THREADS=16 \
    bash scripts/stability/run_wired_soak.sh
```

| 指标 | 期望 |
|---|---|
| `metrics.csv` 样本 | ~36 行 (5s interval × 3 min) |
| RSS delta | < 10 MB |
| FD delta | < 5 |
| WAL | 0 MB |
| sysbench errors | 0 |
| TPC-H rounds | ≥ 1 |
| STABILITY_REPORT.md | 完整 + PASS |

**阶段 2 gate**: STABILITY_REPORT 全部 PASS; 若 FAIL → 调查后回退或调小 SERVER_THREADS。

### 阶段 3: 1 小时正式跑 — 估时 1h+

```bash
HOURS=1 THREADS=16 SERVER_THREADS=16 \
    bash scripts/stability/run_wired_soak.sh
```

(见 § 6.5 期望指标)

### 阶段 4: 回归全测 — 估时 30 min

```bash
cargo test --all-features
bash scripts/gate/check_docs_links.sh
bash scripts/gate/check_gate_self_verification.sh
bash scripts/gate/check_ignore_count.sh
bash scripts/gate/check_test_count_monotonic.sh
bash scripts/gate/check_drift_not_pass.sh
bash scripts/gate/check_oracle_present.sh
bash scripts/gate/check_gate_test_integrity.sh
```

**阶段 4 gate**: 6/6 meta-gates PASS; 无新增 `#[ignore]`; 测试数单调不减。

### 阶段 5: 合并与回 Issue

```bash
git add -A
git commit -m "feat(mysql-server): --server-threads CLI flag + worker pool

- Add --server-threads N (0..=80) to sqlrustgo-mysql-server serve
- 0 = legacy unbounded thread::spawn (backwards compatible)
- N>0 = N workers + mpsc sync_channel(N*2) with panic isolation
- Update run_wired_soak.sh: HOURS=1, THREADS=16, SERVER_THREADS=16
- 11 new tests (4 unit + 4 CLI + 3 e2e)

Refs: #3265, #3533, #3465"
git push origin feature/soak-3265-test
# 提 PR: feature/soak-3265-test → develop/v3.9.0
# 在 Issue #3265 评论附 1h 跑结果 + STABILITY_REPORT 链接
```

### 阶段 6: 72h / 168h Z6G4 复用

由于本 PR 改的是 binary 默认值 (`server_threads=16`), Z6G4 后续 72h / 168h 跑
自动获得 worker pool 保护 (避免单连接阻塞拖死线程)。无需额外配置。

## 8. 风险与回退

| 风险 | 触发条件 | 回退步骤 |
|---|---|---|
| 1h 跑发现回归 (崩溃/泄漏) | STABILITY_REPORT 有 WARN/FAIL | `git revert` 本 PR; server_threads=0 默认路径不受影响 |
| `Mutex<Receiver>` 锁竞争成为瓶颈 | profile 显示 worker 等锁 > 30% | 切到 `crossbeam-channel` (新增 dep, API 等价) |
| `--server-threads` CLI flag 与下游脚本冲突 | 用户报告 | 加 deprecated alias `--max-threads N` 转发到 server_threads |
| Channel buffer 太小导致 accept 频繁阻塞 | sysbench QPS 显著低于 server_threads=0 模式 | 把 `CHANNEL_BUFFER_MULTIPLIER` 2 → 4 |

## 9. 引用

- **分支**: `feature/soak-3265-test` (基于 `develop/v3.9.0`)
- **Worktree**: `.worktrees/soak-3265/`
- **相关 Issue**: [#3265](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3265) (本任务),
  [#3225](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3225), 
  [#3266](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3266) (168h),
  [#3531](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3531) (WAL P0 关闭),
  [#3575](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3575) (wired-soak DDL 修复)
- **相关 PR**: #3465 (soak infra), #3533 (WAL checkpoint fix),
  #3546 (run_72h_soak.sh), #3549 (run_72h_soak_v2.sh),
  #3522 (Q8/Q9 fix), #3526 (G15 split), #3550 (72h v2 watchdog)
- **Issue #3265 进度声明**: [comment 68551](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3265#issuecomment-68551) (2026-06-25 23:14 UTC)
- **AGENTS.md**: 强制约束 (中文沟通 / 不改 main 分支 / `cargo clippy -D warnings` / `cargo fmt --check`)
- **ADR-008** (Test Claim Transparency): 所有 "PASS" 声明必须注明实际跑的测试数
- **doc 5-step 流程** (`docs/governance/DOC_CHECK_CORRECTION_RULES.md`): 改 `docs/` 前必走

## 10. 实施检查清单 (给后续 plan 使用)

- [ ] 步骤 1: `main.rs` 新增 `--server-threads` + validator
- [ ] 步骤 2: `lib.rs` 新增 `ServerJob` + `ServerThreadPool` + `worker_loop`
- [ ] 步骤 3: `lib.rs` `EphemeralConfig` 新增 `server_threads` 字段
- [ ] 步骤 4: `lib.rs` accept loop 改造 (match pool)
- [ ] 步骤 5: `lib.rs` `run_server_v2` 等函数签名扩展 `server_threads` 参数
- [ ] 步骤 6: `lib.rs` `#[cfg(test)]` 新增 4 单元 + 4 CLI 测试
- [ ] 步骤 7: 新增 `tests/server_thread_pool_e2e_test.rs` (3 测试)
- [ ] 步骤 8: `run_wired_soak.sh` 默认值 + 校验 + 启动命令 + 报告
- [ ] 步骤 9: `cargo build --all-features`
- [ ] 步骤 10: `cargo clippy --all-features -- -D warnings`
- [ ] 步骤 11: `cargo fmt --check --all`
- [ ] 步骤 12: `cargo test --all-features` 全过
- [ ] 步骤 13: 3 分钟冒烟 → STABILITY_REPORT.md 全 PASS
- [ ] 步骤 14: 1 小时正式跑 → 报告归档
- [ ] 步骤 15: 6/6 meta-gates 全 PASS
- [ ] 步骤 16: commit + push + 提 PR
- [ ] 步骤 17: 更新 Issue #3265 评论附 1h 跑结果
