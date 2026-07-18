## Why

当前 SOAK 测试仅验证正常负载下的稳定性，无法验证系统在极端条件下的恢复能力。GA 发布前必须确认 Double-Write Buffer (V311-04) 和 WAL 在磁盘 I/O 延迟、内存压力、进程异常退出等混沌场景下的数据完整性和自动恢复能力。

## What Changes

- 新增 `scripts/soak/chaos_inject.py` — 混沌注入控制脚本
- 新增 `tests/soak/chaos_soak_test.rs` — Rust 混沌 SOAK 测试用例
- 新增 `scripts/gate/check_chaos_soak.sh` — GA-P0 门禁脚本
- 修改 `scripts/soak/tpch_mixed_soak_driver.py` — 集成混沌注入阶段

### Chaos Experiments

1. **I/O 延迟注入**: 使用 `tc qdisc` 模拟磁盘延迟，验证 5s 内自动恢复
2. **内存压力**: 使用 `stress-ng --vm` 模拟内存压力，验证无 OOM panic
3. **进程 kill -9**: 随机终止从库进程，验证数据完整性
4. **网络分区**: 模拟网络延迟/丢包，验证重连机制

## Capabilities

### New Capabilities

- `chaos-soak`: 混沌工程 SOAK 测试能力，集成到现有 SOAK 测试流程
- `chaos-recovery`: 故障恢复验证能力，验证 DWB 和 WAL 在极端条件下的恢复

### Modified Capabilities

- `soak-test`: 扩展现有 SOAK 测试流程，增加混沌注入阶段
