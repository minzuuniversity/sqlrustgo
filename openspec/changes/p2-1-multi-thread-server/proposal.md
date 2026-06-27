# Proposal — P2-1: Multi-Thread Server Runtime

> **Issue**: #3175 follow-up（发现于 SOAK 测试）
> **作者**: Claude
> **日期**: 2026-06-27
> **Phase**: 2.1
> **状态**: Draft

## 一、问题

### 1.1 现象

SOAK 测试（16 并发 client 线程）实测数据：

| 并发 | QPS | P50 | P99 | Server CPU |
|------|-----|-----|-----|-----------|
| 16 | 265.2 | 53ms | 205ms | ~15% |
| 32 | 385.8 | 70ms | 295ms | ~15% |
| 64 | 378.5 | 158ms | 381ms | ~15% |

Server CPU 仅 **15%**，**严重未饱和**。增加 client 并发不能提升 QPS。

### 1.2 根因

`crates/mysql-server/src/lib.rs:2510`：

```rust
while !shutdown.load(Ordering::SeqCst) {
    match listener.accept() {
        Ok((stream, addr)) => {
            thread::spawn(move || handle_connection(stream, addr, st, tc, us));
        }
        ...
    }
}
```

- **每连接一个 OS 线程**（`std::thread::spawn`），非复用
- 瓶颈：client 共享**单一 TCP 连接**，查询在 server 端串行执行
- `handle_connection` + `do_command_loop` 是同步阻塞的

### 1.3 影响

- SOAK 测试 QPS 上限 ~400（实测），Production 性能受限
- Server 无法利用多核（XEON E5-2680 v4 = 28 物理核）
- 并发连接数受限于 OS 线程数

## 二、目标

**配置化多线程 accept**：可配置 4-16 个 worker 线程并行处理查询。

```
--worker-threads=8   (默认 4)
```

| 模式 | 行为 |
|------|------|
| `worker-threads=1` | 当前行为（向后兼容）|
| `worker-threads=N` | N 个 worker 线程处理查询 |

## 三、约束

1. **向后兼容**：`--worker-threads=1` 时行为与当前完全一致
2. **最小侵入**：不改 `handle_connection` / `do_command_loop` 内部逻辑
3. **配置简单**：新增一个 CLI 参数，不改变现有 API
4. **Storage 线程安全**：已满足（`Arc<RwLock<WalStorage>>` 是 `Send + Sync`）

## 四、非目标

- 不引入 tokio 异步 runtime（当前 `std::thread`，最小化改动）
- 不改变查询执行层（planner、executor、storage）
- 不做连接池（连接复用由 MySQL 协议处理）

## 五、验收标准

1. `--worker-threads=1` QPS 与当前 baseline 一致
2. `--worker-threads=8` 时 QPS 提升 ≥ 2x（4 核并行）
3. Server NLWP 增长符合预期（1 主线程 + N worker）
4. 16 并发 client 零错误
5. G7 Gate PASS

## 六、风险

| 风险 | 影响 | 缓解 |
|------|------|------|
| `PreparedStatementManager` 非线程安全 | 并发竞争 | 改 per-connection 内部使用 `Mutex` |
| `do_command_loop` 有内部状态 | 数据竞争 | 需要审计（初步看是 per-connection，无共享状态）|
| WAL 日志并发写入 | 竞争 | WAL manager 已有内部锁 |
