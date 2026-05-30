# v3.6.0 Release — Knowledge Enhancement + Storage Reliability

> **状态**: GA（正式发布）
> **起点**: v3.5.0 GA (`5720805b`)
> **分支**: `develop/v3.6.0`
> **发布日期**: 2026-05-30
> **Tag**: `v3.6.0`

---

## 一、版本定位

v3.6.0 是 SQLRustGo 的**知识增强 + 存储可靠性**版本，聚焦于 WALVerifier、SIMD 向量化、Knowledge OS 桥接。

---

## 二、关键文档

| 文档 | 说明 |
|------|------|
| [CHANGELOG.md](CHANGELOG.md) | v3.6.0 变更日志 |
| [RELEASE_NOTES.md](RELEASE_NOTES.md) | v3.6.0 发布说明 |
| [ALPHA_GATE_REPORT_v3.6.0.md](ALPHA_GATE_REPORT_v3.6.0.md) | Alpha 门禁报告 |
| [INTEGRATION_DEBT_REPORT.md](INTEGRATION_DEBT_REPORT.md) | 集成债务分析 |
| [TEST_REPORT.md](TEST_REPORT.md) | 测试报告 |
| [BENCHMARK.md](BENCHMARK.md) | TPC-H 性能基准 |
| [QUICK_START.md](QUICK_START.md) | 快速开始 |

---

## 三、门禁状态

> v3.6.0 已于 2026-05-30 正式 GA。

| 门禁 | 状态 | 报告 |
|------|------|------|
| Alpha Gate | ✅ CONDITIONAL PASS | [ALPHA_GATE_REPORT_v3.6.0.md](ALPHA_GATE_REPORT_v3.6.0.md) |
| GA Gate | ✅ PASS | — |

---

## 四、快速开始

```bash
git clone ssh://git@192.168.0.252:222/openclaw/sqlrustgo.git
cd sqlrustgo
git checkout v3.6.0
cargo build --release
cargo test --all-features
```