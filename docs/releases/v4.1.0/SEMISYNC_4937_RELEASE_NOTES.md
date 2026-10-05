# 半同步复制（#4937）— v4.1.0 发布说明

## 交付内容

半同步复制从 `archive/v3.11/deleted-crates/distributed/src/semisync.rs` 移植落地
（20358 B，35 个 pub 项，**26 个测试**）。该文件自 v3.11 起被移入 archive，
**既不编译、其测试也从未运行过** —— 本 issue 原记录的「全仓零实现」是准确的。

| 产物 | 位置 | commit |
|---|---|---|
| `SemiSyncMaster` / `SemiSyncSlave` / `SemiSyncMode` | `crates/storage/src/semisync.rs` | `312e28fd7b` |
| 同步化改造（见下） | 同上 | 同上 |
| PR 合并 | — | `f900f4fb6` |
| 硬前置：主库 ACK 记账 | `crates/storage/src/binary_server.rs` | `bd48b1651`（#4936 PR-B） |

## 同步化改造

移植后只有一处不兼容：`wait_for_acks` 是 `async fn`，用 `tokio::time::sleep`
做 10ms 轮询间隔。

`sqlrustgo-storage` 是**同步 crate，无 tokio 依赖**。为一个 10ms 轮询间隔引入
tokio 是错误的取舍——调用方是 MySQL worker 线程，不是 reactor task。改为：

- `wait_for_acks` 改为同步 `fn`，`std::thread::sleep`
- 3 个 `#[tokio::test]` → `#[test]`，去掉 `.await`

## 前置条件已就位

#4936 PR-B（`bd48b1651`）给主库加了 ACK 记账：`acked_lsn_of` /
`min_acked_lsn` / `acking_slave_count`。半同步等的就是 `min_acked_lsn()` ——
没有它 `SemiSyncMaster` 只能数本地计数，无法判断从库是否真的追上了。这条硬依赖已解除。

## 验证

```
cargo test -p sqlrustgo-storage --lib -- semisync   26 passed; 0 failed
cargo test -p sqlrustgo-storage --lib              847 passed; 0 failed（原 821）
cargo fmt -p sqlrustgo-storage --check              clean
```

26 个测试**此前从未运行过**，本次全部通过。

## 未完成：与 BinlogServer 的端到端接线

**这是本节存在的唯一理由，请勿按「已实现」理解。**

`SemiSyncMaster` 目前**零生产调用点** —— `wait_for_acks` 在 `crates/` 与
`src/` 下均无调用。这 26 个测试只覆盖 `SemiSyncMaster` / `SemiSyncSlave` 自身的
计数与超时逻辑，与实际复制路径无连接。

这正是本轮反复验证的教训：#5027 处置 `backup_scheduler.rs` 时，「已 `mod` 声明
+ 测试全过」并不等于功能可用——那份代码的 `start()` 只置一个 bool，连全量备份
的执行函数都不存在。

### 剩余工作

1. `BinlogServer` 提交源库写时，调 `SemiSyncMaster::wait_for_acks(从库数)`
2. 超时后按 `SemiSyncMode` 降级（`Async` 则直接返回成功不等；`Semisync` 则报错）
3. `mysql-server` 侧加配置项（启用开关、超时、等待副本数）

这三步需要改 `mysql-server`，故未并入 PR #5014。

### 建议

如需在 release note 中表述，建议写作「半同步复制的核心状态机已移植并通过其
单元测试」，而非「半同步复制已实现」。功能可用的判据应是第 1 步完成后、
从库 ACK 能实际阻塞主库提交的那种。

## 关联

- #4936 PR-B（`bd48b1651`）— 硬前置，已合入
- #5014 / `f900f4fb6` — 本次移植
- #5027 — 同批 archive 文件的处置教训（删除而非接通）
