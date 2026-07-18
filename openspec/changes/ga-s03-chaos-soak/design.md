## Overview

混沌注入 SOAK 测试通过在正常 SOAK 测试流程中嵌入故障注入阶段，验证系统在极端条件下的数据完整性和自动恢复能力。

## Architecture

### Chaos Injection Points

```
SOAK Test Lifecycle
├── Setup Phase
│   ├── Prepare data (SF=0.1)
│   └── Start sqlrustgo server
├── Normal Phase (30min)
│   └── Run TPC-H mixed workload
├── Chaos Phase (每种故障各10min)
│   ├── I/O 延迟注入 (tc qdisc)
│   ├── 内存压力 (stress-ng)
│   └── kill -9 进程
└── Recovery Phase (5min)
    └── Verify data integrity
```

### Chaos Controller (`chaos_inject.py`)

```python
class ChaosController:
    def inject_io_latency(self, delay_ms: int = 100):
        """使用 tc qdisc 注入 I/O 延迟"""

    def inject_memory_pressure(self, workers: int = 4, megs: int = 512):
        """使用 stress-ng 制造内存压力"""

    def kill_server_process(self):
        """随机终止服务器进程"""

    def verify_recovery(self, timeout: int = 5) -> bool:
        """验证系统在 N 秒内恢复正常"""
```

### Integration with SOAK Driver

修改 `tpch_mixed_soak_driver.py`，在正常 phase 后增加 `--chaos` 选项：

```bash
python3 scripts/soak/tpch_mixed_soak_driver.py \
    --level=30m \
    --chaos \
    --chaos-types=io_latency,memory_pressure,kill9
```

## Implementation Details

### 1. chaos_inject.py

- 平台检测：Linux (tc, stress-ng) vs macOS (不适用)
- 权限要求：需要 sudo 权限执行 tc qdisc 和 stress-ng
- 清理机制：所有故障注入后自动清理（`tc qdisc del`）

### 2. chaos_soak_test.rs

```rust
#[test]
fn test_chaos_io_latency_recovery() {
    // 注入 100ms I/O 延迟
    // 运行查询
    // 验证 5s 内恢复
}

#[test]
fn test_chaos_memory_pressure_no_oom() {
    // 注入内存压力
    // 验证无 OOM panic
}

#[test]
fn test_chaos_kill9_data_integrity() {
    // kill -9 服务器进程
    // 重启
    // 验证数据完整性
}
```

### 3. Gate Script

`check_chaos_soak.sh` 验证：
- chaos_inject.py 存在且语法正确
- chaos_soak_test.rs 存在且通过
- 混沌注入后恢复时间 ≤ 5s

## Platform Support

| 平台 | 支持的混沌实验 | 原因 |
|------|--------------|------|
| Linux | 全部 | tc, stress-ng 可用 |
| macOS | kill9 only | tc 不支持，stress-ng 需 brew |

## Dependencies

- `tc` (Linux iproute2)
- `stress-ng` (需安装)
- Python 3.8+
