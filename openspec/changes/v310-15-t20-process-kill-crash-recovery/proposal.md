## Why

T-20 (Process kill -9 mid-transaction) 是 INT5+ 债务清单中最后一个未实现的故障注入场景。现有 crash 测试框架 (`crash_test_harness.rs`) 使用 `SQLRUSTGO_CRASH_AT` env var 模拟进程崩溃 (exit 134)，但无法覆盖真正的 kill -9 (SIGKILL) 场景——SIGKILL 不可捕获、无法执行任何清理逻辑，是唯一能验证 WAL recovery 完整性的真实崩溃手段。

已有 `scripts/crash/run_real_crash_test.sh` 实现了 shell 级别的 kill -9 测试，但没有 Rust 测试级别的等价实现，无法纳入 `cargo test` 门禁。

## What Changes

- 新增 `tests/stress/process_kill_crash_test.rs` — Rust 集成测试，使用 `std::process::Command` 启动 sqlrustgo server 子进程，通过 MySQL wire protocol 客户端执行 SQL，发送 SIGKILL，重启 server，验证数据一致性
- 所需场景:
  - BEGIN → INSERT → UPDATE → (kill -9) → 重启 → SELECT 验证 insert/update 提交或回滚
  - BEGIN → DELETE → (kill -9) → 重启 → SELECT 验证删除数据是否回滚
  - 空事务 kill -9 → 重启 → 无数据损坏
- 利用 `BinaryTableStorage` 直连模式重启验证（不依赖 server 进程），或使用相同 data-dir 重启 server 验证

## Capabilities

### New Capabilities
- `process-kill-crash-recovery`: 真实 kill -9 进程崩溃恢复测试框架，验证 WAL recovery 在不可捕获信号下的正确性

### Modified Capabilities
- (none)

## Impact

- `tests/stress/process_kill_crash_test.rs` — 新增 Rust 集成测试 (~350 行)
- 无新增依赖（使用 `std::process::Command` + MySQL wire 或 `BinaryTableStorage` 直接验证）
- 测试标记为 `#[ignore]` 默认跳过（因需要编译 release binary + 真正的子进程管理），可通过 `cargo test --test process_kill_crash_test -- --ignored` 运行
- 与现有 `crash_test_framework.rs` / `crash_test_harness.rs` 互补
