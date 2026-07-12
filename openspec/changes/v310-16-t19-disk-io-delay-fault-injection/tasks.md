## 1. 增强 io_delay.rs 故障注入框架

- [x] 1.1 添加 `IoDelayConfig` 结构体（delay_ms, corruption_rate, dropout_rate）及 Default 实现
- [x] 1.2 实现自制 LCG 伪随机数生成器（用于 corruption/dropout 判定）
- [x] 1.3 实现 `IoFaultInjector::new(config)` 和 `from_env()`
- [x] 1.4 实现 `apply_read<F, T>(buf, f)` — 延迟 + 损坏 + 丢弃组合逻辑
- [x] 1.5 实现 `apply_write<F, T>(data, f)` — 延迟 + 丢弃组合逻辑
- [x] 1.6 更新 `crates/storage/src/lib.rs` 导出新类型（`IoFaultInjector`, `IoDelayConfig`）

## 2. 单元测试（io_delay.rs 内 mod tests）

- [x] 2.1 测试 IoDelayConfig 默认值和 from_env 解析
- [x] 2.2 测试 apply_read 延迟注入（时间测量）
- [x] 2.3 测试 apply_write 延迟注入（时间测量）
- [x] 2.4 测试 corruption_rate=0.0 不损坏，corruption_rate=1.0 每次损坏
- [x] 2.5 测试 dropout_rate=0.0 不丢弃，dropout_rate=1.0 总是 Err
- [x] 2.6 测试旧函数 `io_delay_ms()` / `maybe_delay()` 向后兼容

## 3. 集成测试

- [x] 3.1 创建 `tests/disk_io_fault_injection.rs` 集成测试文件
- [x] 3.2 测试 IoFaultInjector 在真实文件 IO 场景下的行为
- [x] 3.3 验证 clippy + fmt 通过

## 4. 更新文档与 Issue 关闭

- [x] 4.1 更新 `docs/releases/v3.10.0/ARCHITECTURE.md` 中 io_delay.rs 状态
- [x] 4.2 关闭 Issue #3772（验证测试通过后）
