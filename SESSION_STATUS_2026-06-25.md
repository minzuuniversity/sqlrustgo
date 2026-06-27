# Issue #3225 — Soak Test Status Report

**Date**: 2026-06-25
**Author**: claude-macmini agent
**Branch**: `develop/v3.9.0` (commit `d0269fab5`)

---

## 进展总结

### 已完成 ✅

**1. TPC-H 数据加载链路验证**
- `start_sf001()` 加载 SF=0.001：lineitem=501行, orders=150, customer=15, part=20, partsupp=80 ✅
- `start_sf01()` 加载 SF=0.1：lineitem=60,000行 ✅
- `LOAD DATA LOCAL INFILE` 正常工作 ✅

**2. 新增 Soak 测试文件 (commit `d0269fab5`)**
- `tests/tpch_soak_test.rs` — 4个 `#[ignore]` 测试: 5m / 10m / 20m / 30m
- `tests/tpch_soak_qps.rs` — 30s QPS benchmark
- `SESSION_STATUS_2026-06-25.md` — 完整 session 报告

**3. 已验证结果**
| 测试 | 结果 | 耗时 |
|------|------|------|
| 5m TPC-H soak | ✅ PASS | 301s |
| 10m TPC-H soak | ✅ PASS | 601s |
| 20m TPC-H soak | ✅ PASS | 1201s |
| 30m TPC-H soak | ⏳ 进行中 | — |
| 30s QPS benchmark | QPS=14,954 | 30s |

**4. 基础设施确认**
- `CREATE DATABASE` 不支持 — 所以用 TPC-H 表数据做真实查询
- sysbench 需要 `CREATE DATABASE sbtest` — 不适用，改用 in-process harness
- MariaDB 占用 port 3306 — `brew services stop mariadb` 可解决

---

## 本机测试方法

```bash
# 构建
cargo build --release --bin sqlrustgo-mysql-server

# 跑 5m soak (最快验证)
cargo test --release --test tpch_soak_test -- --ignored test_soak_5m

# 跑 30s QPS benchmark
cargo test --release --test tpch_soak_qps -- --nocapture

# 完整 ladder (需要 ~65 分钟)
cargo test --release --test tpch_soak_test -- --ignored
```

---

## 建议后续 (Z6G4 / Z440)

**Z6G4 (192.168.0.252)** 继续时：
1. SSH 连上后先 `brew services stop mariadb`
2. 推荐用本机的 `tests/tpch_soak_test.rs` 方法 — 编译后直接跑
3. 更长 soak (30m→1h→2h→4h→24h→72h) 可直接增加 duration 参数

**关键发现**：
- TPC-H SF=0.001 QPS=14,954，极快
- 长时间 soak 主要验证：内存增长(RSS)、FD泄漏、WAL增长、进程稳定性

---

## 阻塞项
- Z6G4 网络不可达 (SSH timeout 5s)
- 更长 soak 需要有人在本机或 Z6G4 机器上跑

---

## 相关文件
- `tests/tpch_soak_test.rs` — TPC-H soak ladder
- `tests/tpch_soak_qps.rs` — QPS benchmark
- `SESSION_STATUS_2026-06-25.md` — 完整 session 报告
