# V313-04 Tasks

## 1. 分析当前单连接架构

- [ ] 1.1 读取 `crates/server/src/server.rs`，定位当前单 `LocalExecutor` 实例化位置，确认连接生命周期
- [ ] 1.2 读取 `crates/server/src/connection_pool.rs:88-231`，分析骨架代码的 `ConnectionPool` 结构与现有方法签名
- [ ] 1.3 确认 `crates/common/src/connection_pool.rs` 中 `PoolConfig` 是否已导出并被 server crate 使用
- [ ] 1.4 读取 `tests/compat/mysql_v3_12/connection_pool_deferred.sql` 和 `.log`，确认 deferred fixture 的期望行为

## 2. 实现 ConnectionPool::acquire() 与 try_acquire()

- [ ] 2.1 在 `crates/server/src/connection_pool.rs` 的 `impl ConnectionPool` 中实现 `acquire()`：调用 `self.available.recv_timeout(Duration::from_millis(self.config.timeout_ms))`，超时返回 `PoolError::Timeout`
- [ ] 2.2 实现 `try_acquire()`：`self.available.try_recv().ok()` 映射为 `Option<PooledConnection>`
- [ ] 2.3 添加 `PoolError` 枚举（`Timeout`、`PoolExhausted`、`InvalidConfig`）到 `crates/server/src/connection_pool.rs`
- [ ] 2.4 运行 `cargo test -p sqlrustgo-server connection_pool` 确认基础 acquire/try_acquire 测试通过

## 3. 实现 PooledConnection RAII 封装

- [ ] 3.1 实现 `PooledConnection::new(session, sender)` 构造方法，将 session 封装为 `Option<PooledSession>`
- [ ] 3.2 实现 `PooledConnection::executor(&mut self) -> &mut LocalExecutor`，委托给内部 session
- [ ] 3.3 实现 `PooledConnection::session(&mut self) -> &mut PooledSession`
- [ ] 3.4 实现 `impl Drop for PooledConnection`：在 `drop` 时将 session 归还 `sender`
- [ ] 3.5 添加 `PooledConnection` 的单元测试：验证 `drop` 后 session 可被重新 `acquire()`

## 4. 实现 @@max_connections 系统变量

- [ ] 4.1 在 `crates/server/src/` 查找系统变量注册机制（如 `SqlError` 或 `VariableRegistry`）
- [ ] 4.2 若变量注册机制存在：注册 `max_connections` 变量，`get` 返回 `PoolConfig::size as i64`
- [ ] 4.3 若变量注册机制不存在：在 `crates/server/src/server.rs` 中为 `SELECT @@max_connections` 添加专用处理分支
- [ ] 4.4 运行 `cargo test -p sqlrustgo-server` 确认无回归

## 5. 网络层集成：单连接 → 连接池

- [ ] 5.1 修改 `crates/server/src/server.rs`（或等价入口）：将全局单 `LocalExecutor` 替换为 `ConnectionPool` 实例
- [ ] 5.2 在每个入站连接的处理循环中，调用 `pool.acquire()` 获取 `PooledConnection`，通过 `conn.executor()` 执行查询
- [ ] 5.3 确认 `PooledConnection` 的作用域正确：在连接处理函数结束时 `drop(conn)`，触发归还逻辑
- [ ] 5.4 添加 `cargo test -p sqlrustgo-server --test server_concurrent` 验证多并发连接场景

## 6. 更新 connection_pool_deferred fixture

- [ ] 6.1 将 `tests/compat/mysql_v3_12/connection_pool_deferred.sql` 的 `# expect: DEFERRED: follow-up TBD` 改为 `# expect: PASS`
- [ ] 6.2 更新对应的 `connection_pool_deferred.out`，记录 `SELECT @@max_connections` 的实际输出
- [ ] 6.3 删除 `docs/releases/v3.12.0/evidence/mysql_compat/logs/connection_pool_deferred.log`（旧的 deferred 证据）

## 7. 创建多连接并发 fixture

- [ ] 7.1 新建 `tests/compat/mysql_v3_13/connection_pool_multi.sql`：
  - 验证 `@@max_connections` 返回正数
  - 验证多连接场景（创建表、插入、并发查询）
- [ ] 7.2 新建 `tests/compat/mysql_v3_13/connection_pool_multi.out`，记录期望输出
- [ ] 7.3 若 `scripts/gate/run_compat_tests.sh` 不存在，创建兼容测试运行脚本
- [ ] 7.4 将新 fixture 路径加入 compat-runner 扫描范围

## 8. 更新文档

- [ ] 8.1 更新 `docs/tutorials/connection-pool-guide.md`：补充 `acquire()` / `try_acquire()` / `PooledConnection` 的 API 文档，对齐实际实现
- [ ] 8.2 确认 `@@max_connections` 行为在文档中有说明

## 9. 运行 Runner 验证

- [ ] 9.1 运行 `cargo test -p sqlrustgo-server connection_pool` 确认所有连接池测试通过
- [ ] 9.2 运行 `cargo test --all-features` 确认全量测试通过（无回归）
- [ ] 9.3 运行 `cargo clippy --all-features -- -D warnings` 确认 lint 通过
- [ ] 9.4 运行 `cargo fmt --check` 确认代码格式正确
- [ ] 9.5 运行 `./scripts/gate/run_compat_tests.sh`（或等价命令），确认 `connection_pool_deferred` 和 `connection_pool_multi` fixture PASS

## 10. PR 与合并

- [ ] 10.1 提交所有变更到特性分支
- [ ] 10.2 打开 PR 指向 `develop/v3.13.0`
- [ ] 10.3 获得至少 1 个 reviewer 批准
- [ ] 10.4 合并到 develop/v3.13.0
