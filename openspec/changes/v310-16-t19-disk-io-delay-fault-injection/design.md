## Context

当前 `crates/storage/src/io_delay.rs` 仅有 ~40 行，包含两个基础函数：
- `io_delay_ms()` — 从 `SQLRUSTGO_IO_DELAY_MS` 环境变量读取延迟值
- `maybe_delay()` — 如果有延迟值则 sleep

缺少：
- 结构化配置（corruption_rate, dropout_rate）
- 可组合的故障注入包装器
- 数据损坏模拟（位翻转）
- IO 操作丢弃模拟

本设计在现有基础上扩展，保持完全向后兼容。

## Goals / Non-Goals

**Goals:**
- 将 `io_delay.rs` 从 ~40 行基础函数扩展为 ~250 行完整故障注入框架
- 支持 `IoDelayConfig` 结构化配置 + 环境变量覆盖
- 提供 `IoFaultInjector` 包装器，通过闭包组合方式包装 IO 操作
- 实现三种故障模式：延迟、数据损坏、操作丢弃
- 保持 `io_delay_ms()` 和 `maybe_delay()` 签名完全不变
- 新增 `tests/disk_io_fault_injection.rs` 集成测试

**Non-Goals:**
- 不修改 `FileStorage` 内部的 IO 路径（集成通过 feature gate 留给后续任务）
- 不引入新的外部依赖（使用自制 LCG 伪随机数生成器）
- 不做跨进程故障注入（仅本进程内模拟）

## Decisions

### D1: 使用自制 LCG 而非引入 rand crate

**选择**: 自制 LCG (Linear Congruential Generator) 伪随机数生成器

**理由**: `rand` crate 不在 `sqlrustgo-storage` 的依赖中，为单一故障注入功能引入依赖不值得。LCG 实现简单（约 10 行），测试中确定性足够。

**Alternatives considered**: `oorandom`（轻量但需加 dep）、`rand`（功能过多）

### D2: 闭包组合而非 trait 包装

**选择**: `IoFaultInjector` 提供 `apply_read<F, T>(buf: &mut [u8], f: F)` 和 `apply_write<F, T>(data: &[u8], f: F)` 方法，接受闭包作为实际 IO 操作。

**理由**: 
- 不需要修改 `FileStorage` 的 trait 签名
- 调用方决定何时/如何应用故障注入
- 测试中可以传入 mock 闭包

**Alternatives considered**:
- Trait 方式：需要修改 `StorageEngine` trait，影响面过大
- 直接包装 `File` 结构体：复杂度过高

### D3: 环境变量覆盖设计

**选择**: 在 `IoFaultInjector::from_env()` 中使用与现有 `io_delay_ms()` 相同模式，新增 `SQLRUSTGO_IO_CORRUPTION_RATE` 和 `SQLRUSTGO_IO_DROPOUT_RATE` 环境变量。

**理由**: 与现有 `SQLRUSTGO_IO_DELAY_MS` 模式一致，运维人员可以通过环境变量快速启用故障注入。

### D4: 数据损坏方式

**选择**: 单字节位翻转 — 随机选择数据中的一个字节，翻转该字节中的随机一位。

**理由**: 模拟存储介质位翻转（bit flip）的真实场景。实现简单，效果好。

### D5: IoDelayConfig 结构体位置

**选择**: 与现有代码在同一文件 `io_delay.rs` 中。

**理由**: 
- 所有故障注入逻辑集中在一处
- 文件大小约 250 行，不至于过大
- 导入路径不变，调用方只需 `use crate::io_delay::IoFaultInjector`

## API 设计

```rust
/// 故障注入配置
#[derive(Debug, Clone)]
pub struct IoDelayConfig {
    /// 每次 IO 操作的延迟毫秒数（0 = 无延迟）
    pub delay_ms: u64,
    /// 数据损坏概率 0.0 ~ 1.0
    pub corruption_rate: f64,
    /// IO 操作丢弃概率 0.0 ~ 1.0
    pub dropout_rate: f64,
}

impl Default for IoDelayConfig {
    fn default() -> Self {
        Self { delay_ms: 0, corruption_rate: 0.0, dropout_rate: 0.0 }
    }
}

/// 故障注入器
#[derive(Debug)]
pub struct IoFaultInjector {
    config: IoDelayConfig,
    rng: LcgRng,
}

impl IoFaultInjector {
    pub fn new(config: IoDelayConfig) -> Self;
    pub fn from_env() -> Self;  // 从环境变量读取配置
    pub fn apply_read<F, T>(&mut self, buf: &mut [u8], f: F) -> std::io::Result<T>
        where F: FnOnce(&mut [u8]) -> std::io::Result<T>;
    pub fn apply_write<F, T>(&mut self, data: &[u8], f: F) -> std::io::Result<T>
        where F: FnOnce(&[u8]) -> std::io::Result<T>;
}
```

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| LCG 随机数质量不足导致测试不可重复 | LCG 使用固定种子，测试可完全复现；仅在故障注入场景使用，不涉及密码学 |
| `IoFaultInjector` 需要 `&mut self` 因为 RNG 状态变更 | 文档明确标注，使用时需要可变引用 |
| `from_env()` 解析 `f64` 可能 panic | 使用 `f64::parse` + `ok()` + 默认值处理，不会 panic |
| 与现有 `io_delay_ms()`/`maybe_delay()` 功能重叠 | 保留旧函数做薄封装，新代码建议用 `IoFaultInjector` |
