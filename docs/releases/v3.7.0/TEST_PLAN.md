# SQLRustGo v3.7.0 测试计划 — GA Final

> **版本**: v3.7.0 GA
> **分支**: `origin/develop/v3.7.0` (commit `83d70e7c`)
> **日期**: 2026-05-30
> **状态**: GA ✅

---

## 1. 测试策略（GA 验证结果）

| 层级 | 工具 | 目标 | GA 结果 |
|------|------|------|---------|
| 单元测试 | `cargo test -p sqlrustgo-mysql-server --lib` | 0 failures | ✅ 93/93 PASS |
| Clippy | `cargo clippy --all-features` | 0 errors | ✅ 0 errors |
| Format | `cargo fmt -- --check` | 0 failures | ✅ 0 failures |
| Integration | sql-corpus | ≥85% | ✅ 93 tests pass |
| E2E | integration tests | 28 files | ✅ 28/28 PASS |
| Performance | TPC-H SF=1 | 22/22 | ✅ 22/22 PASS |

---

## 2. GA 测试结果（最终）

### Alpha Gate — 代码质量

| ID | 测试 | 命令 | 结果 |
|----|------|------|------|
| A1 | cargo build --release | `cargo build --release -p sqlrustgo-mysql-server` | ✅ PASS |
| A2 | cargo test | `cargo test -p sqlrustgo-mysql-server --lib` | ✅ 93/93 PASS |
| A3 | clippy | `cargo clippy --all-features -- -D warnings` | ✅ 0 errors |
| A4 | cargo fmt | `cargo fmt -- --check` | ✅ 0 failures |

### Beta Gate — 集成测试

| ID | 测试 | 命令 | 结果 |
|----|------|------|------|
| B1 | E2E integration | `scripts/test/e2e_integration.sh` | ✅ 28/28 PASS |
| B2 | TPC-H SF=1 | `./target/release/sqlrustgo-bench-cli tpch-bench` | ✅ 22/22 PASS |
| B3 | Auth flow | mysql/mysql auth | ✅ PASS |
| B4 | Transaction | BEGIN/INSERT/COMMIT | ✅ PASS |

### GA Gate — 发布检查

| ID | 测试 | 结果 |
|----|------|------|
| G1 | GA_GAP_REPORT | ✅ 存在，65/100 |
| G2 | P0 blockers | ✅ 0 (2/2 fixed) |
| G3 | Documentation | ✅ 15/15 存在 |
| G4 | LEGACY_ISSUES | ✅ v3.8.0 已归档 |

---

## 3. 已知遗留问题（不影响 GA）

| Issue | 说明 | 计划 |
|-------|------|------|
| #2583 | SHOW TABLES 未实现 | v3.7.x |
| #2584 | 空密码认证 edge case | v3.7.x |
| Coverage 32.59% | 低于 50% 目标 | v3.8.0 PR-900 |

---

## 4. v3.8.0 测试计划

参见 [../v3.8.0/TEST_PLAN.md](../v3.8.0/TEST_PLAN.md)

重点新增：
- Layer 3 ACID verification (Isolation Suite + Crash Simulation)
- Execution consistency harness (mysql-server vs bench-cli vs direct)
- VTU performance regression