# Spec — P2-1: Multi-Thread Server Runtime

> **Issue**: #3175 follow-up
> **日期**: 2026-06-27

## R1: CLI 参数

**Given** a `sqlrustgo-mysql-server` binary  
**When** started with `--worker-threads=N`  
**Then** the server accepts `N` worker threads for query processing

| 参数 | 类型 | 默认值 | 范围 |
|------|------|--------|------|
| `--worker-threads` | usize | 1 | 1–16 |

**Acceptance**：
- R1.1：`--worker-threads=1` 行为与当前完全一致（向后兼容）
- R1.2：`--worker-threads` 超出范围时报错并退出
- R1.3：`--worker-threads` 未指定时打印日志：`worker_threads = 1 (default)`

## R2: Worker Pool 行为

**Given** `N > 1` worker threads configured  
**When** a client connection is accepted  
**Then** the connection task is submitted to the worker pool, not directly spawned as a new OS thread

**Acceptance**：
- R2.1：Worker 线程数 = `N`（通过 `ps -o nlwp` 验证）
- R2.2：接受循环在主线程继续运行（不受 worker 阻塞影响）
- R2.3：Worker 线程在服务器关闭时正确 join

## R3: 向后兼容

**Given** `N = 1` worker threads  
**When** a client connects  
**Then** the server spawns one OS thread per connection (current behavior, unchanged)

**Acceptance**：
- R3.1：单个连接的 QPS 与改造前完全一致
- R3.2：NLWP = 2（1 accept 线程 + 1 连接线程）

## R4: 性能

**Given** `N = 8` worker threads  
**When** 16 concurrent client connections execute TPC-H Q1-Q6  
**Then** QPS ≥ 2x single-thread baseline

**Acceptance**：
- R4.1：`--worker-threads=8` QPS ≥ 2x `--worker-threads=1` QPS
- R4.2：P99 latency ≤ 2x single-thread P99（无退化）

## R5: 线程安全

**Given** multiple concurrent connections  
**When** queries execute in parallel  
**Then** no data races or panics occur

**Acceptance**：
- R5.1：`cargo test --test mysql_server_integration_test` — 全部 PASS
- R5.2：`RUST_MIN_STACK=16777216 cargo test` — 无 panic
- R5.3：Server 可平稳运行 30min SOAK（`tpch_soak_driver.py --level=30m`）零错误

## R6: 日志

**Acceptance**：
- R6.1：启动日志打印：`Multi-thread mode: N workers`
- R6.2：关闭日志打印：`Shutting down N worker threads`

## 测试用例

### TC-1: 默认行为（向后兼容）
```
$ sqlrustgo-mysql-server serve --port 3396
$ mysql -h 127.0.0.1 -P 3396 -u ai -e "SELECT 1;"
→ col_1 = 1
$ ps -o nlwp $(pgrep sqlrustgo)
→ NLWP ≤ 3
```

### TC-2: 4 Worker 模式
```
$ sqlrustgo-mysql-server serve --port 3396 --worker-threads 4
$ mysql -h 127.0.0.1 -P 3396 -u ai -e "SELECT 1;"
→ col_1 = 1
$ ps -o nlwp $(pgrep sqlrustgo)
→ NLWP ≥ 5
```

### TC-3: 性能提升验证
```
$ # Worker=1 baseline: ~20 qps
$ # Worker=4: ≥ 40 qps
```

### TC-4: 异常参数
```
$ sqlrustgo-mysql-server serve --port 3396 --worker-threads 0
→ ERROR: worker-threads must be 1-16

$ sqlrustgo-mysql-server serve --port 3396 --worker-threads 100
→ ERROR: worker-threads must be 1-16
```

### TC-5: 并发安全
```
$ python3 scripts/soak/tpch_soak_driver.py --level=5m --concurrency 8
→ errors = 0, alert_triggered = false
```
