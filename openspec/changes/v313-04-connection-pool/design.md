# Design: 连接池生产实现

## 1. 架构概览

```
TCP Client(s)                    ConnectionPool                    Storage
     |                               |                              |
     |--- TCP connect -------------->|                              |
     |                               |-- acquire() ---------------->|
     |                               |   (PooledSession)            |
     |                               |<-- session ------------------|
     |<-- OK: connected -------------|                              |
     |                               |                              |
     |--- SQL query ---------------->|                              |
     |                               |-- session.execute(query) --->|
     |                               |                              |
     |                               |<-- result -------------------|
     |<-- resultset -----------------|                              |
     |                               |                              |
     |--- TCP disconnect ----------->|                              |
     |                               |-- release(session) --------->|
     |                               |   (return to channel)        |
```

关键组件：

- `ConnectionPool`：基于 `crossbeam_channel` 的有界信道实现，容量 = `PoolConfig::size`
- `PooledSession`：包含 `LocalExecutor`、`MemoryStorage`、`transaction_id`，不可跨连接共享
- `PooledConnection`：`PooledSession` 的 RAII 封装，`drop` 时自动调用 `release()`

## 2. 当前骨架分析

`crates/server/src/connection_pool.rs` 已实现：

```rust
pub struct ConnectionPool {
    sessions: Arc<Vec<PooledSession>>,     // 预创建的 session 列表
    available: Sender<PooledSession>,       // 可用 session 信道
    received: Receiver<PooledSession>,      // 归还 session 信道
    config: PoolConfig,
    strategy: LoadBalanceStrategy,
    round_robin_index: Arc<AtomicUsize>,
}

pub struct PooledSession {
    pub executor: LocalExecutor<'static>,
    pub storage: Arc<MemoryStorage>,
    pub transaction_id: Option<u64>,
    in_use: bool,
}
```

**缺失部分**：

- `ConnectionPool::acquire()` — 阻塞获取可用 session
- `ConnectionPool::try_acquire()` — 非阻塞获取
- `PooledConnection` 的 `acquire()` 入口及 `Drop` 实现
- `@@max_connections` 系统变量

## 3. 实现方案

### 3.1 ConnectionPool 方法

```rust
impl ConnectionPool {
    /// 阻塞获取可用连接，超时由 PoolConfig::timeout_ms 控制
    pub fn acquire(&self) -> Result<PooledConnection, PoolError> {
        let session = self.available
            .recv_timeout(Duration::from_millis(self.config.timeout_ms))
            .map_err(|_| PoolError::Timeout)?;
        Ok(PooledConnection::new(session, self.available.clone()))
    }

    /// 非阻塞获取，None 表示无可用连接
    pub fn try_acquire(&self) -> Option<PooledConnection> {
        self.available.try_recv().ok()
            .map(|session| PooledConnection::new(session, self.available.clone()))
    }

    /// 获取当前池统计信息
    pub fn stats(&self) -> PoolStats { ... }
}
```

### 3.2 PooledConnection RAII

```rust
pub struct PooledConnection {
    session: Option<PooledSession>,  // None after take()
    sender: Sender<PooledSession>,
}

impl PooledConnection {
    pub fn executor(&mut self) -> &mut LocalExecutor<'static> {
        &mut self.session.as_mut().unwrap().executor
    }

    pub fn session(&mut self) -> &mut PooledSession {
        self.session.as_mut().unwrap()
    }
}

impl Drop for PooledConnection {
    fn drop(&mut self) {
        if let Some(session) = self.session.take() {
            let _ = self.sender.send(session);  // 归还池中
        }
    }
}
```

### 3.3 @@max_connections 系统变量

在 `crates/common/src/connection_pool.rs` 的 `PoolConfig` 基础上，在 server 变量注册表中注册：

```rust
// 伪代码
fn get_max_connections() -> i64 {
    POOL_CONFIG.load().size as i64
}

register_system_variable("max_connections", VariableGetter(get_max_connections));
```

### 3.4 网络层集成

`crates/server/src/server.rs`（或等价入口）修改：

```rust
// Before: 单 executor
let executor = LocalExecutor::new(storage.clone());
execute(&executor, &plan);

// After: 从池获取
let mut conn = connection_pool.acquire()?;
let result = conn.executor().execute(&plan)?;
// conn.drop() 自动归还
```

### 3.5 负载均衡策略

当前支持三种策略（骨架已实现）：

| 策略 | 实现 | 说明 |
|------|------|------|
| `RoundRobin` | `round_robin_index` 原子计数器 | 轮询分配 |
| `LeastConnections` | 无实现（待补充） | 选择空闲连接数最少的 session |
| `HealthCheck` | 无实现（待补充） | 检测 session 健康状态 |

本变更优先实现 `RoundRobin`，其他策略为后续迭代预留。

## 4. 关键决策

| 决策点 | 选项 | 选择 | 理由 |
|--------|------|------|------|
| 信道实现 | `crossbeam_channel` / `std::mpsc` | `crossbeam_channel` | 支持超时、多接收者、性能更好 |
| 获取语义 | 阻塞 / 非阻塞 | 两者都支持 | 覆盖同步和异步场景 |
| 错误处理 | 超时 error / panic | 超时返回 `PoolError::Timeout` | 避免级联失败 |
| session 复用 | 每次新建 / 池化复用 | 池化复用 | 减少 executor 创建开销 |
| 默认池大小 | 固定值 / `num_cpus::get()` | `num_cpus::get()` | 合理匹配 CPU 核数 |

## 5. Fixture 设计

### `tests/compat/mysql_v3_13/connection_pool_multi.sql`

```sql
# name: connection_pool_multi
# expect: PASS

-- 验证 @@max_connections 返回池大小
SELECT @@max_connections;

-- 验证并发连接可以获取不同 session
CREATE TABLE t1 (id INT PRIMARY KEY, v INT);
INSERT INTO t1 VALUES (1, 100), (2, 200);

-- 并发执行（多连接场景）
SELECT * FROM t1 WHERE id = 1;
SELECT * FROM t1 WHERE id = 2;

DROP TABLE t1;
```

### 更新 `tests/compat/mysql_v3_12/connection_pool_deferred.sql`

将 `# expect: DEFERRED: follow-up TBD` 改为 `# expect: PASS`，解除 deferred 状态。

## 6. 验证方式

```bash
# 运行连接池单元测试
cargo test -p sqlrustgo-server connection_pool

# 运行兼容测试
./scripts/gate/run_compat_tests.sh

# 验证 @@max_connections
mysql -h 127.0.0.1 -P 3306 -u root -e "SELECT @@max_connections"

# 验证多连接并发
cargo test -p sqlrustgo-server --test connection_pool_concurrent
```

## 7. 失败模式

- 若 `available.recv_timeout()` 超时：返回 `PoolError::Timeout`，客户端收到连接超时错误
- 若 `session.take()` 被多次调用（逻辑 bug）：第二次 `drop` 时 `sender.send()` 失败（session 已归还），日志警告但不 panic
- 若 `PoolConfig::size == 0`：初始化时 `bounded(0)` 会 panic，明确在 `new()` 中检查并返回 `PoolError::InvalidConfig`
