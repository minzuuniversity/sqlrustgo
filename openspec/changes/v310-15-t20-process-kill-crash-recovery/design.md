## Context

现有 crash 测试体系：

| 层 | 方式 | 覆盖场景 |
|---|---|---|
| `crash_test_harness.rs` | `SQLRUSTGO_CRASH_AT` + `exit(134)` 模拟崩溃 | 9 个注入点 (INSERT/page/DELETE/COMMIT/CHECKPOINT) |
| `scripts/crash/run_real_crash_test.sh` | shell 脚本 + `kill -9` 真实进程 | 8 类 (sigkill/power_loss/disk_full/OOM/WAL corruption/hang) |
| `crash_recovery_test.rs` | WALManager 模拟 (drop manager) | WAL entry 级别恢复 |

**缺口**: 无 Rust 测试级别的 kill -9 真实进程测试。

## Goals / Non-Goals

**Goals:**
- `tests/stress/process_kill_crash_test.rs` — Rust 集成测试
- 子进程管理: `ProcessKillTestHarness` (start/kill/restart/verify)
- 3 个测试场景: INSERT/UPDATE/DELETE mid-transaction kill -9
- WAL RecoveryReport 验证

**Non-Goals:**
- 不修改 WAL recovery 引擎 (已有实现)
- 不修改 storage 层代码
- 不新增外部依赖
- 不做跨机器崩溃测试

## Decisions

### D1: 验证策略 — 直连 storage 而非通过 server 重启

**选择**: kill -9 后使用 `BinaryTableStorage::new(data_dir)` 直接打开数据目录验证数据，而非启动第二个 server 进程。

**理由**:
- 避免二次子进程管理的复杂性
- 可以直接获取 `RecoveryReport`
- 测试更快速、更可靠

**权衡**: 不测试 server 重启路径的完整 TCP 握手。

### D2: MySQL wire client vs 直连 ExecutionEngine

**选择**: 通过 MySQL wire protocol client (已有 `mysql` crate 或 `sqlrustgo` client) 操作 server 执行 SQL，kill 后用 storage 直连验证。

**理由**:
- 真实模拟用户操作路径
- 验证 server 的完整 SQL→WAL→storage 路径

### D3: 测试标记为 `#[ignore]`

**选择**: 默认 `#[ignore]`，需要显式 `--ignored` 运行。

**理由**:
- 需要先 `cargo build --release --bin sqlrustgo` 编译 binary
- 测试依赖端口可用性
- 执行时间较长 (~5s 每个测试)
- 适合 CI nightly 或手动触发

## API 设计

```rust
pub struct ProcessKillTestHarness;

impl ProcessKillTestHarness {
    /// Build the server binary (cargo build --release)
    pub fn build_server() -> PathBuf;
    
    /// Start server and return child process
    pub fn start_server(data_dir: &Path, port: u16) -> std::process::Child;
    
    /// Send SIGKILL to server
    pub fn kill_server(child: &mut std::process::Child);
    
    /// Verify data by directly opening storage
    pub fn verify_storage(data_dir: &Path, expected: &[Record]) -> bool;
}
```

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| 端口冲突 | 用随机端口 (`port: 0` 或固定偏移) |
| 编译耗时 | 测试用 `#[ignore]` 标记，只按需运行 |
| 子进程残留 | `Drop` 实现中 kill + wait |
| 测试环境无 rustc | 需要 pre-built binary 或 CI 跳过 |
