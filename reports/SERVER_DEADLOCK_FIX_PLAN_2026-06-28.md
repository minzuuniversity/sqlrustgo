# Server 死锁修复方案 (Issue: 8-worker busy-loop panic storm)

## Status: APPROVED by user 2026-06-28

## Provenance
- generated_by: ai
- generated_at: 2026-06-28
- input_refs:
  - type: commit, value: c347d8171 (develop/v3.9.0 HEAD)
  - type: evidence, value: SOAK log /tmp/soak_sf1_progress.txt
  - type: evidence, value: server log /tmp/srv_sf1.log (panic storm)
- evidence:
  - source_run: SOAK #4 SF=1.0
  - observed: 0 QPS, 800% CPU, RSS 47GB, 8 worker all in panic loop
  - panic_location: crates/executor/src/trigger.rs:104

## Problem Statement
SQLRustGo server 在 `--storage binary` 模式下，8 worker 全部进入 panic
busy-loop，每个 query 触发 trigger.rs:104 panic（"Storage MUST be WalStorage
in production"），worker_loop 的 catch_unwind 捕获后立即重新 recv → 无止境
panic → CPU 800% + 0 业务输出。

## Root Cause Analysis

1. **TriggerExecutor::new()** (crates/executor/src/trigger.rs:101-110)
   在 non-test 模式下 assert! `is_wal_enabled() = true`
2. **BinaryTableStorage::is_wal_enabled() = false** (binary_storage.rs:554-556)
3. **MemoryStorage::is_wal_enabled() = true** (engine.rs:1112)
4. 所以 `--storage binary` 模式下每次 query → 创建 TriggerExecutor → panic

## Fix Plan (B 方案 from user 2026-06-28)

### 1. trigger.rs:104 - 不 panic，返回错误
- `assert!()` → `tracing::error!()` + 早期 return Ok(()) 跳过 trigger 执行
- 影响：binary storage 不支持 trigger，但 binary mode 也不需要 trigger

### 2. server 端日志增强
- worker_loop: 已有的 panic 日志保留
- do_command_loop: 加 query start/end 计时
- trigger.rs: 加详细的 trigger 创建/执行日志

### 3. 数据加载独立线程
- BinLoader 已在 server 启动时一次性加载（不影响）
- 确认启动后没有后台加载任务

## Out of Scope
- WAL 强制策略本身（不在本任务范围）
- Trigger 完整功能（binary mode 不需要）

## Verification Plan
1. `cargo build --all-features` 通过
2. `cargo clippy --all-features -- -D warnings` 通过
3. 启动 server + 8 worker 并发 SOAK 1 分钟
4. 验证：CPU < 50%（无 busy-loop），有 QPS 数据
5. 跑全套 SOAK 24h

## Risk
- trigger.rs 修改可能影响其他用 trigger 的测试
- 缓解：保留 cfg!(test) 分支，测试用相同路径
