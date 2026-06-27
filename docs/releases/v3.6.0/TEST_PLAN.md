# v3.6.0 Alpha Test Plan

## 1. 测试概述
v3.6.0 Alpha 阶段测试覆盖编译器、执行引擎、存储引擎和网络协议。

## 2. 测试范围
- 单元测试: 所有 crate --lib 测试
- 集成测试: workspace 级测试 (--exclude sqlrustgo-mysql-server)

## 3. 测试环境
- Z6G4: 409GB RAM, cargo test --workspace
- CARGO_TARGET_DIR=/tmp/sqlrustgo-target

## 4. 功能测试矩阵
| 模块 | 测试数 | 状态 |
|------|--------|------|
| parser | 98 | ✅ PASS |
| executor | 242 | ✅ PASS |
| storage | 180 | ✅ PASS |
| transaction | 87 | ✅ PASS |
| catalog | 56 | ✅ PASS |
| types | 81 | ✅ PASS |
| planner | 45 | ✅ PASS |
| optimizer | 34 | ✅ PASS |
| mysql-server | (tests2) | ❌ 43 errors |

## 5. 性能测试
Alpha 阶段不要求性能基线 (Beta 阶段 B6 TPC-H)

## 6. 门禁映射
| 门禁 | 覆盖 |
|------|------|
| A1 Build | cargo build --release --workspace |
| A2 Test | cargo test --lib --exclude mysql-server |
| A3 Clippy | cargo clippy --all-features -- -D warnings |
| A4 Format | cargo fmt --all -- --check |
| A5 Coverage | L1 8 crates avg ≥75% |
