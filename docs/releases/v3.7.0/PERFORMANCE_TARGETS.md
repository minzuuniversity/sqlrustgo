# SQLRustGo v3.7.0 性能目标

> **版本**: v3.7.0
> **分支**: develop/v3.7.0
> **日期**: 2026-05-30

---

## 性能基线

| 测试 | 指标 | v3.6.0 基线 | v3.7.0 目标 | 状态 |
|------|------|------------|------------|------|
| TPC-H Q1 | elapsed_ms | ~4449ms | < 4671ms | ✅ |
| TPC-H Q3 | elapsed_ms | ~4449ms | < 4671ms | ✅ |
| TPC-H Q6 | elapsed_ms | ~5000ms | < 基线+5% | ✅ |
| TPC-H Q11 | elapsed_ms | ~5041ms | < 5293ms | ✅ |

---

## 吞吐量目标

| 场景 | 目标 | 说明 |
|------|------|------|
| 并发连接数 | 100+ | MySQL protocol |
| QPS (simple) | > 10000 | 单表简单查询 |
| 延迟 P99 | < 100ms | 端到端查询延迟 |

---

## Execution Telemetry 开销

| 指标 | 目标 | 说明 |
|------|------|------|
| Trace 开销 | < 1ms/query | span 创建传播 |
| Memory | < 5% CPU | 指标采集 |
| Log volume | < 2x | 增加 trace_id |

---

## 回归策略

- TPC-H SF=1 每次 PR 必须运行
- 回归阈值: +5% 触发审查
- 性能测试在 CI 中独立 job