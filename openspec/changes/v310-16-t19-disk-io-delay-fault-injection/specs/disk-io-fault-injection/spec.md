## ADDED Requirements

### Requirement: IoFaultInjector 可配置故障参数
系统 SHALL 提供 `IoFaultInjector` 结构体，通过 `IoDelayConfig` 配置以下故障参数：
- `delay_ms`: 每次 IO 操作注入的延迟毫秒数（0 = 无延迟）
- `corruption_rate`: 数据损坏概率，范围 0.0 ~ 1.0（0 = 从不损坏）
- `dropout_rate`: IO 操作丢弃/失败概率，范围 0.0 ~ 1.0（0 = 从不丢弃）

配置 SHALL 支持通过环境变量覆盖（`SQLRUSTGO_IO_DELAY_MS`、`SQLRUSTGO_IO_CORRUPTION_RATE`、`SQLRUSTGO_IO_DROPOUT_RATE`）。

#### Scenario: IoFaultInjector 默认无故障
- **WHEN** 创建一个新的 `IoFaultInjector` 且未设置任何故障参数
- **THEN** `delay_ms` 默认值为 0，`corruption_rate` 默认值为 0.0，`dropout_rate` 默认值为 0.0
- **THEN** `apply_read` / `apply_write` 行为与直接 IO 完全一致

#### Scenario: IoFaultInjector 从环境变量读取配置
- **WHEN** 设置环境变量 `SQLRUSTGO_IO_DELAY_MS=100` 和 `SQLRUSTGO_IO_CORRUPTION_RATE=0.5`
- **THEN** `IoFaultInjector::from_env()` 返回配置为 delay_ms=100, corruption_rate=0.5, dropout_rate=0.0 的注入器

### Requirement: 写入延迟注入
系统 SHALL 在写操作路径中支持注入延迟，延迟值由 `IoDelayConfig.delay_ms` 控制。
当 `delay_ms > 0` 时，每次 `apply_write` 调用 SHALL 在写入前 sleep 指定毫秒数。

#### Scenario: 写操作注入延迟
- **WHEN** `IoFaultInjector` 配置 `delay_ms = 50`
- **THEN** 调用 `apply_write(data)` SHALL 耗时至少 50ms
- **THEN** 返回的写入结果 SHALL 与无延迟时一致（数据正确写入）

#### Scenario: 延迟为 0 时不 sleep
- **WHEN** `IoFaultInjector` 配置 `delay_ms = 0`
- **THEN** 调用 `apply_write(data)` SHALL 不引入额外延迟

### Requirement: 读取延迟注入
系统 SHALL 在读操作路径中支持注入延迟，延迟值由 `IoDelayConfig.delay_ms` 控制。
当 `delay_ms > 0` 时，每次 `apply_read` 调用 SHALL 在读取前 sleep 指定毫秒数。

#### Scenario: 读操作注入延迟
- **WHEN** `IoFaultInjector` 配置 `delay_ms = 100`
- **THEN** 调用 `apply_read(buf)` SHALL 耗时至少 100ms
- **THEN** 返回的读取结果 SHALL 与无延迟时一致（数据正确读取）

### Requirement: 数据损坏注入
系统 SHALL 支持模拟数据损坏（位翻转），损坏概率由 `IoDelayConfig.corruption_rate` 控制。
当 corruption_rate > 0 时，每个 `apply_read` 返回的数据有 corruption_rate 概率被损坏。
损坏方式 SHALL 是翻转数据中的随机一个字节的随机一位。

#### Scenario: corruption_rate=0 时不损坏数据
- **WHEN** `IoFaultInjector` 配置 `corruption_rate = 0.0`
- **THEN** 连续调用 `apply_read(buf)` 1000 次 SHALL 返回的数据与原始数据完全一致

#### Scenario: corruption_rate=1.0 时每次读取都损坏
- **WHEN** `IoFaultInjector` 配置 `corruption_rate = 1.0`
- **THEN** 调用 `apply_read(buf)` SHALL 返回的数据至少有一位的值与原始数据不同

#### Scenario: corruption_rate=0.5 时约一半读取损坏
- **WHEN** `IoFaultInjector` 配置 `corruption_rate = 0.5`，且重复读取相同数据 100 次
- **THEN** 损坏次数 SHOULD 在 20 到 80 之间（非严格，取决于随机种子）

### Requirement: IO 丢弃注入
系统 SHALL 支持模拟 IO 操作失败（丢弃），丢弃概率由 `IoDelayConfig.dropout_rate` 控制。
当 dropout_rate > 0 时，每个 `apply_write` / `apply_read` 调用有 dropout_rate 概率返回 `Err(ioError)`。

#### Scenario: dropout_rate=0 时不丢弃操作
- **WHEN** `IoFaultInjector` 配置 `dropout_rate = 0.0`
- **THEN** 连续调用 `apply_write(data)` 100 次 SHALL 全部返回成功

#### Scenario: dropout_rate=1.0 时所有操作都失败
- **WHEN** `IoFaultInjector` 配置 `dropout_rate = 1.0`
- **THEN** 调用 `apply_write(data)` SHALL 返回 `Err`

### Requirement: 向后兼容
系统 SHALL 保持现有 `io_delay_ms()` 和 `maybe_delay()` 函数的签名不变，确保已调用这两个函数的代码无需修改。

#### Scenario: 旧函数行为不变
- **WHEN** 设置环境变量 `SQLRUSTGO_IO_DELAY_MS=50` 后调用 `io_delay_ms()`
- **THEN** 返回 `Some(50)`
- **WHEN** 调用 `maybe_delay()`
- **THEN** 线程 sleep 50ms

### Requirement: 与现有 FileStorage 集成
系统 SHALL 提供可选的 `IoFaultInjector` 到 `FileStorage` 的集成点，通过 feature gate 控制。
当启用 `fault-injection` feature 时，`FileStorage` 的 read/write 操作 SHALL 可以被 IoFaultInjector 包装。

#### Scenario: 无 feature gate 时行为不变
- **WHEN** 编译时不启用 `fault-injection` feature
- **THEN** `FileStorage` 的 read/write SHALL 与修改前行为一致
