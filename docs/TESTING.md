# 测试体系文档

> 版本：v3.4.0 GA | 日期：2026-05-25
> 目标：打造具有完整测试体系的示范版本

---

## 测试命令

### 统一 Makefile 命令

| 命令 | 说明 | 耗时 |
|------|------|------|
| `make unit` | 快速单元测试 | ~5min |
| `make allunit` | 完整测试 | ~30min |
| `make coverage` | 覆盖率报告 | ~10min |
| `make benchmark` | TPC-H SF=1 性能基准 | ~3min |
| `make fmt` | 代码格式化检查 | <1min |
| `make clippy` | Lint 检查 | ~5min |
| `make chaos` | 混沌测试 | ~10min |

### 详细说明

#### make unit

快速测试，验证基本功能。使用 L1 CRATES 列表：

- sqlrustgo-core
- sql-parser
- executor
- storage
- network
- sqlrustgo

```bash
make unit
# 等价于
cargo test -p sqlrustgo --lib
```

#### make allunit

完整测试，包含所有集成测试和稳定性测试：

```bash
make allunit
# 等价于
cargo test --all-features
bash scripts/gate/check_stability.sh
```

#### make coverage

生成覆盖率报告，要求 >= 85%：

```bash
make coverage
# 等价于
cargo llvm-cov test --all-features --codecov
```

覆盖率目标：L1 CRATES >= 85%

#### make benchmark

TPC-H SF=1 性能基准测试：

```bash
make benchmark
# 等价于
./target/release/sqlrustgo-bench-cli tpch-bench --ddl scripts/tpch/tpch_schema.sql --data ~/sqlrustgo-tpch-sf1/data --queries all
```

---

## 门禁体系

v3.4.0 使用四阶段门禁：Alpha → Beta → RC → GA

### Alpha 门禁（G1-G4）

```bash
bash scripts/gate/check_alpha_v340.sh
```

检查项：A1-A5
- A1: Build
- A2: Test
- A3: Clippy
- A4: Format
- A5: Coverage（>= 75%）

### Beta 门禁

```bash
bash scripts/gate/check_beta_v340.sh
```

检查项：B1-B14
- B1-B5: 同 Alpha
- B6: Security（cargo audit）
- B7: SQL Compatibility
- B8-B14: 核心功能验证

### RC 门禁

```bash
bash scripts/gate/check_rc_v340.sh
```

检查项：R1-R22
- 全部功能验证
- GMP 功能
- Trust Infrastructure
- 稳定性初测

### GA 门禁（最终发布）

```bash
bash scripts/gate/check_ga_v340.sh
```

检查项：G1-G13 + S1-S4
- G1-G13: 代码质量/功能/安全/GMP/Trust
- S1-S4: 稳定性测试（TPC-H SF=1）

---

## 混沌测试

v3.4.0 引入了 Chaos Testing，用于验证系统在异常条件下的行为。

### OOM 测试

模拟内存耗尽，验证系统优雅降级：

```bash
# 方法 1: make 命令
make chaos-oom

# 方法 2: 直接运行
MALLOC_LIMIT=1MB ./target/release/sqlrustgo "SELECT * FROM huge_table"
```

预期行为：
- 不 panic
- 返回有意义的错误信息
- 不泄漏内存

### I/O Error 测试

模拟磁盘故障：

```bash
# 使用 fault injection
./scripts/chaos/io_error_test.sh
```

预期行为：
- WAL recovery 正常
- 数据完整性保持

### Crash 测试

模拟进程异常退出：

```bash
# 模拟崩溃
./scripts/chaos/crash_test.sh
```

预期行为：
- WAL 日志恢复
- 无数据丢失
- 可重新启动

---

## 覆盖率

### L1 CRATES（必须 >= 85%）

| Crate | 目标 | 当前 |
|------|------|------|
| sqlrustgo-core | 85% | 82.88% |
| sql-parser | 85% | - |
| executor | 85% | - |
| storage | 85% | - |

### L2 CRATES（建议 >= 60%）

| Crate | 目标 | 当前 |
|------|------|------|
| network | 60% | - |
| gmp | 60% | - |
| trust | 60% | - |

### 覆盖率报告

```bash
# HTML 覆盖率报告
cargo llvm-cov html --open

# LCov 格式
cargo llvm-cov lcov --output-path lcov.info
```

---

## TPC-H 性能基准

### 环境

- TPC-H SF=1（~1GB 数据）
- 22 个查询
- 60 秒超时

### 参考性能（v3.4.0 GA）

| 指标 | 目标 | 实际 |
|------|------|------|
| 总耗时 | < 180s | 150s |
| Q1 响应时间 | < 5s | 3.2s |
| 内存峰值 | < 1GB | 512MB |

---

## Truthfulness Score

门禁报告使用 Truthfulness Score 标注数据来源：

| Score | 含义 |
|-------|------|
| 100% | 实际执行，有命令输出 |
| 65-85% | 历史重建，有部分证据 |
| 40-60% | 推断，无实际执行 |
| < 40% | 未验证 |

---

## 相关文档

- [CONTRIBUTING.md](../CONTRIBUTING.md) — 贡献指南
- [BUILD.md](../BUILD.md) — 构建指南
- [docs/governance/gate_spec_v350.md](../governance/gate_spec_v350.md) — 门禁规范
- [docs/releases/v3.4.0/](../releases/v3.4.0/) — v3.4.0 版本文档
