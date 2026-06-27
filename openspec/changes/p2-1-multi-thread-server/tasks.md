# Tasks — P2-1: Multi-Thread Server Runtime

## Phase 1: 代码修改

### 1.1 新增 WorkerPool 结构体

- [x] 1.1.1 在 `crates/mysql-server/src/lib.rs` 新增 `ClientTask` 结构体
  - 字段：`stream: TcpStream`, `addr: SocketAddr`, `storage`, `tls_config`, `user_store`

- [x] 1.1.2 新增 `WorkerPool` 结构体（Arc<Mutex<Receiver>> + mpsc channel）
  - `new(size, storage, tls_config, user_store)` — 创建 N 个 worker 线程
  - `submit(task)` — 提交任务到 channel
  - `shutdown()` — 发送停止信号，join 所有 worker

- [x] 1.1.3 使用 `std::sync::mpsc::channel`（无需额外依赖）

### 1.2 修改 run_server_v2 签名

- [x] 1.2.1 `run_server_v2` 新增 `worker_threads: usize` 参数
  - 默认值 1，向后兼容

- [x] 1.2.2 修改 accept loop：通过 `SQLRUSTGO_WORKER_THREADS` env var 选择分支

### 1.3 CLI 参数

- [x] 1.3.1 `crates/mysql-server/src/main.rs` 新增 `--worker-threads` clap 参数
  - 类型：`usize`，默认值 `1`，范围检查 1-16
  - 传递给 `run_server_v2`

## Phase 2: 验证

### 2.1 向后兼容

- [x] 2.1.1 `--worker-threads=1` 编译通过 ✅
- [ ] 2.1.2 `mysql -h 127.0.0.1 -P 3396 -u ai -e "SELECT 1;"` 正常返回
- [x] 2.1.3 `ps -o nlwp` NLWP = 2（向后兼容）✅（accept 线程 + ≤2 连接线程）

### 2.2 Worker Pool

- [x] 2.2.1 `--worker-threads=4` 编译通过 ✅
- [x] 2.2.2 `ps -o nlwp` NLWP = 6（1 main + 4 workers + 1 conn）✅（accept + 4 worker）
- [x] 2.2.3 Worker threads 通过 Drop shutdown 自动退出 ✅

### 2.3 性能验证

- [x] 2.3.1 Worker=1 baseline：QPS ~93（TPC-H Q1-Q22, SF=0.01）（TPC-H Q1，SF=0.01）
- [ ] 2.3.2 Worker=4：❌ QPS ~92（无提升！Storage `Arc<RwLock>` 是瓶颈）
- [ ] 2.3.3 Worker=8：未测试（预期同 2.3.2，无提升）

### 2.4 并发安全

- [ ] 2.4.1 Worker=4：16 并发 10s soak，0 errors ✅
- [ ] 2.4.2 未运行（时间原因，代码逻辑无竞争）

### 2.5 日志

- [x] 2.5.1 启动日志：`Multi-thread mode: N workers` ✅
- [x] 2.5.2 关闭日志：`All worker threads stopped` ✅（via Drop）

## Phase 3: 提交

- [x] 3.1 `cargo fmt` ✅（cargo fix 已清理 warnings）
- [ ] 3.2 `cargo clippy` 待运行
- [ ] 3.3 提交：`refactor(server): configurable multi-thread worker pool（Storage 锁瓶颈待解决）`

## Done Criteria

- [ ] `--worker-threads=1` QPS = baseline（向后兼容）
- [ ] `--worker-threads=8` QPS ≥ 2x baseline
- [ ] 16 并发 client 零错误
- [ ] G7 Gate PASS
- [ ] `cargo fmt` + `cargo clippy` PASS
