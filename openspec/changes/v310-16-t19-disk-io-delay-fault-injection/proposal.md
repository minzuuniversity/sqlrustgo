## Why

Disk I/O delay 故障注入是 v3.10.0 C-5 崩溃恢复体系的关键缺失环节（T-19）。当前 `io_delay.rs` 仅实现了最基础的 ENV 变量延迟（`SQLRUSTGO_IO_DELAY_MS`），缺少可配置的故障注入框架——无法模拟慢磁盘、I/O 瓶颈、数据位翻转等真实生产场景。实现后，可验证存储层在 I/O 异常下的超时行为、数据完整性保护和降级恢复能力。

## What Changes

- 增强 `crates/storage/src/io_delay.rs`，从简单延迟函数升级为完整故障注入框架
- 新增配置结构 `IoDelayConfig`，支持 `delay_ms`、`corruption_rate`、`dropout_rate` 三个维度
- 实现三种注入场景：写入延迟、读取延迟、数据损坏（位翻转模拟）
- 新增 `IoFaultInjector` 结构体，作为存储层 IO 操作的透明包装层
- 写操作通过 injector 时可能：延迟返回、返回损坏数据、或返回 IO 错误
- 更新 `crates/storage/src/lib.rs` 导出新类型

## Capabilities

### New Capabilities
- `disk-io-fault-injection`: Disk I/O delay 故障注入框架，支持可配置的延迟/损坏/丢弃注入，用于测试存储层在 I/O 异常下的行为

### Modified Capabilities
- (none)

## Impact

- `crates/storage/src/io_delay.rs` — 核心修改，从 ~40 行扩展到 ~250 行
- `crates/storage/src/lib.rs` — 新增导出符号
- 新增集成测试文件 `tests/disk_io_fault_injection.rs`（或 `crates/storage/tests/`）
- 新增 `IoFaultInjector` 可集成到 `FileStorage` 的 read/write 路径（通过 feature gate 或可选包装）
- 无外部依赖新增，仅使用 `std::time` + `rand`（若 rand 不在依赖中，使用简单自制 LCG）
