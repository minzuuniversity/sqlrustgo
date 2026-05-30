# SQLRustGo v3.7.0 测试计划

> **版本**: v3.7.0
> **分支**: develop/v3.7.0
> **日期**: 2026-05-30

---

## 测试策略

| 层级 | 工具 | 目标覆盖率 |
|------|------|-----------|
| 单元测试 | cargo test | 50%+ |
| Integration | sql-corpus | >=85% pass |
| E2E | integration tests | 28 files |
| Performance | TPC-H | SF=1 基线 |

---

## Alpha 阶段测试

### 必须通过的测试

| 测试 | 命令 | 通过标准 |
|------|------|--------|
| cargo test --all-features | ✅ | 0 failures |
| cargo clippy --all-features | ✅ | 0 errors |
| cargo fmt --check | ✅ | 无格式错误 |
| sql-corpus | ✅ | >=85% |
| 28 integration files | ✅ | 0 failures |

### 回归测试

| 测试套件 | 上次基线 | 当前状态 |
|---------|---------|---------|
| WAL integration | 16/16 PASS | - |
| SQL parser | 32/32 PASS | - |
| Executor unit | - | - |

---

## Beta 入口条件

- Alpha Gate: 14/14 PASS
- Coverage >= 50% (tarpaulin)
- 所有 integration tests PASS
- TPC-H SF=1 Q1-Q3 性能回归 < 5%