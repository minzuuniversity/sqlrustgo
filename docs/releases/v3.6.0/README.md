# v3.6.0 (2026-05-29) — 协议栈整合

> **状态**: Alpha FAIL ❌ — 双链路执行缺陷未通过
>
> **注意**: v3.6.0 Alpha Gate 在 Z440 实测覆盖率 32.59% (要求 ≥75%)，未通过。
> 声称 GA 但 GA Gate 未实际执行，状态声明不成立。
>
> **后续**: v3.7.0 起为重构改进版本，旨在统一双链路执行路径。

## 一、版本定位

v3.6.0 是 SQLRustGo 的**协议栈整合**版本，聚焦于 MySQL 协议完整处理、SIMD 向量化、WAL 验证工作区。

## 二、关键文档

| 文档 | 说明 |
|------|------|
| [ALPHA_GATE_REPORT_v3.6.0.md](ALPHA_GATE_REPORT_v3.6.0.md) | Alpha 门禁报告 |
| [INTEGRATION_DEBT_REPORT.md](INTEGRATION_DEBT_REPORT.md) | 集成债务分析 |
| [TEST_REPORT.md](TEST_REPORT.md) | 测试报告 |
| [BENCHMARK.md](BENCHMARK.md) | TPC-H 性能基准 |

## 三、门禁状态

| 门禁 | 状态 | 报告 |
|------|------|------|
| Alpha Gate | ❌ **FAIL** (32.59% Z440) | [ALPHA_GATE_REPORT_v3.6.0.md](ALPHA_GATE_REPORT_v3.6.0.md) |
| GA Gate | ❌ **未执行** | GA_GATE_REPORT.md 不存在 |

### Alpha Gate 实测数据

- Z6G4 覆盖率: 81.97% (≥75%) ✅
- Z440 覆盖率: 32.59% (<75%) ❌
- **结论**: Alpha FAIL ❌

## 四、已知缺陷

| 缺陷 | 描述 | 优先级 |
|------|------|--------|
| **双链路执行** | Path A (ExecutionEngine) 与 Path B/C (MySQL Protocol/StoredProc) 行为不一致，trigger executor 绕过 | P0 |
| **WAL 未集成** | WalStorage 实现存在但未接入 mysql-server 生产路径 (IMPL-002) | P0 |
| INT-3 | expr crate 孤岛 | P1 |

> **说明**: v3.6.0 存在双链路执行架构缺陷，WAL/并行/CBO 等关键性能模块未集成至生产 mysql-server 路径，属于"虚假演进"。v3.7.0 起为重构改进版本，聚焦架构统一。

## 五、快速开始

```bash
git clone ssh://git@192.168.0.252:222/openclaw/sqlrustgo.git
cd sqlrustgo
git checkout v3.6.0
cargo build --release
cargo test --all-features
```