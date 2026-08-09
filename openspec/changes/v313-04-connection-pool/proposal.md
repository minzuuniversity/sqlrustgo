# V313-04: 连接池生产实现

## Why

V312-21 的 `connection_pool_deferred` fixture 揭示了当前实现的真实状态：`SELECT @@max_connections` 返回空值，说明连接池尚未接入生产路径。

`crates/server/src/connection_pool.rs` 中已存在骨架代码（`ConnectionPool`、`PooledSession`、`PoolConfig`、`LoadBalanceStrategy`），但关键方法 `acquire()`、`try_acquire()` 尚未实现，连接池无法在网络层作为多连接调度器使用。这导致：

1. 每个客户端连接独享一个 `LocalExecutor`，无法复用
2. 并发连接数受限于单线程模型
3. `@@max_connections` 系统变量无意义（返回空）

本变更将连接池从骨架实现提升为生产可用，支撑 v3.13 高并发场景。

## What Changes

- **`crates/server/src/connection_pool.rs`**：实现 `acquire()`、`try_acquire()`、`release()` 方法，完成 `PooledConnection` 获取/释放链路
- **`crates/server/src/server.rs`** 或等价网络入口：将单 `LocalExecutor` 替换为 `ConnectionPool`，在每个入站连接上调用 `pool.acquire()`
- **`crates/server/src/`** 或 **`crates/common/src/`**：实现 `@@max_connections` 系统变量（返回 `PoolConfig::size`）
- **`crates/network/src/lib.rs`**：若尚未支持多路复用，在网络层支持多个并发连接
- **`tests/compat/mysql_v3_13/connection_pool_multi.sql`** + `.out`：多连接并发 fixture，验证池化正确性
- **`tests/compat/mysql_v3_12/connection_pool_deferred.sql`**：更新 `expect: PASS`，正式解除 deferred 状态
- **`docs/tutorials/connection-pool-guide.md`**：更新 API 文档，对齐实际实现

## Capabilities

### 新增能力

- `connection-pool-acquire`：通过 `ConnectionPool::acquire()` 获取池化连接，支持阻塞等待和超时
- `connection-pool-multiplex`：网络层支持多客户端并发连接，共享连接池
- `system-variable-max_connections`：`SELECT @@max_connections` 返回连接池配置大小

### 修改能力

- `server-connection-lifecycle`：从单 executor 切换为从池中获取/释放 session，支持连接复用
- `deferred-connection-pool`：解除 V312-21 的 deferred 状态，fixture 从 `DEFERRED: follow-up TBD` 升级为 `PASS`

## Impact

- **修改文件**：`crates/server/src/connection_pool.rs`（方法实现）、`crates/server/src/server.rs`（集成）、`crates/common/src/connection_pool.rs`（`@@max_connections`）
- **新增文件**：`tests/compat/mysql_v3_13/connection_pool_multi.sql` + `.out`
- **风险**：连接池初始化在 server 启动阶段，若 `PoolConfig::size` 配置过大可能导致内存占用过高；需设置合理默认值（`num_cpus::get()`）
- **无新增外部 crate 依赖**（`crossbeam_channel` 已存在）
