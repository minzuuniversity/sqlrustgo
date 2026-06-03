# SQLRustGo v3.8.0 GA Gate Checklist
<!-- env:blocked:no-ci -->

> **版本**: v3.8.0
> **类型**: Architecture Unification Release
> **分支**: `origin/develop/v3.8.0`
> **Auditor**: Hermes Agent
> **标准**: v3.8.0 必须通过以下全部门禁方可发布 GA

---

## 0. 门禁执行原则（强制）

### 0.1 Truthfulness 原则

门禁检查和文档治理必须遵循以下原则，**违反即问责**：

| 原则 | 要求 | 禁止行为 |
|------|------|----------|
| **计划不可伪造** | 开发/测试计划是历史记录，禁止重写原始计划以通过门禁 | 将 VERSION_PLAN.md 从"Alpha 阶段"重写为"GA Final" |
| **原始记录保留** | 计划文档只追加、不改写 | 修改原始计划状态掩盖真实进度 |
| **执行结果优先** | 以命令输出、测试结果为依据，禁止编造 | 在 TEST_PLAN.md 写入"PASS"但未实际执行 |
| **禁止形式主义** | 门禁是质量关卡，不是文档美化 | 改文档状态而不执行实际验证 |

### 0.2 合规做法

```
✅ 正确做法：
1. VERSION_PLAN.md 显示 "Alpha 阶段" → 实际完成 Alpha → 创建 VERSION_PLAN_ALPHA_REPORT.md
2. TEST_PLAN.md 显示 "测试进行中" → 执行测试 → 创建 TEST_RESULT_ALPHA.md 记录实际结果
3. 状态变更是执行结果 → 在门禁检查后记录 → 不预先修改文档

❌ 禁止做法：
1. 将 VERSION_PLAN.md 重写为 "GA Final" 企图让文档看起来一致
2. 修改 TEST_PLAN.md 状态而不执行测试
3. 在 TEST_PLAN.md 写入 "93 tests PASS" 但未执行 cargo test
```

### 0.3 复核机制

每次门禁检查前必须复核：
1. 原始计划文档是否被改写（检查 git log 是否有大量 rewrite commit）
2. 测试结果是否有对应命令输出证据
3. 状态变更是否有实际执行记录

---

## 1. L1 — Unit Correctness Gate

| ID | 检查项 | 命令 | 标准 |
|----|--------|------|------|
| L1-1 | Parser unit tests | `cargo test -p sqlrustgo-parser --lib` | 0 failures |
| L1-2 | Executor unit tests | `cargo test -p sqlrustgo-executor --lib` | 0 failures |
| L1-3 | Storage unit tests | `cargo test -p sqlrustgo-storage --lib` | 0 failures |
| L1-4 | Transaction unit tests | `cargo test -p sqlrustgo-transaction --lib` | 0 failures |
| L1-5 | WAL unit tests | `cargo test -p wal-verification --lib` | 0 failures |
| L1-6 | Clippy (all features) | `cargo clippy --all-features -- -D warnings` | 0 errors |
| L1-7 | Format check | `cargo fmt -- --check` | 0 failures |

---

## 2. L2 — Execution Consistency Gate

| ID | 检查项 | 命令 | 标准 |
|----|--------|------|------|
| L2-1 | Execution consistency harness | `python3 scripts/test/execution_consistency_harness.py --corpus data/sql_corpus.json --paths mysql-server,bench-cli,direct` | PASS (all paths same hash) |
| L2-2 | E2E integration tests | `cargo test -p sqlrustgo-integration-tests` | 28/28 PASS |
| L2-3 | TPC-H SF=1 regression | `./target/release/sqlrustgo-bench-cli tpch-bench --queries all` | 22/22 PASS |
| L2-4 | Direct storage call grep | `grep -r "storage\.insert\|storage\.update\|storage\.delete" --include="*.rs" \| grep -v "crates/storage/\|crates/executor/"` | 0 matches |
| L2-5 | mysql-server vs bench-cli DDL parity | `scripts/test/ddl_parity_check.sh` | hash 一致 |

---

## 3. L3 — ACID Verification Gate

### 3.1 Transaction Isolation Suite

| ID | 检查项 | 命令 | 标准 |
|----|--------|------|------|
| L3-01 | Dirty Read Prevention | `python3 scripts/test/isolation_test_suite.py --test dirty_read` | PASS (uncommitted data NOT visible) |
| L3-02 | Non-repeatable Read | `python3 scripts/test/isolation_test_suite.py --test non_repeatable_read` | PASS |
| L3-03 | Phantom Read | `python3 scripts/test/isolation_test_suite.py --test phantom_read` | PASS |
| L3-04 | Write-Write Conflict | `python3 scripts/test/isolation_test_suite.py --test write_conflict` | PASS (one blocks/fails) |
| L3-05 | Lost Update | `python3 scripts/test/isolation_test_suite.py --test lost_update` | PASS |

### 3.2 Crash Simulation Suite

| ID | 检查项 | 命令 | 标准 |
|----|--------|------|------|
| L3-06 | Commit crash recovery | `python3 scripts/test/crash_sim.py --scenario commit_kill9` | 数据存在 after restart |
| L3-07 | Rollback crash | `python3 scripts/test/crash_sim.py --scenario rollback_kill9` | 数据未改变 |
| L3-08 | Partial write recovery | `python3 scripts/test/crash_sim.py --scenario partial_write` | 数据一致或 empty |
| L3-09 | WAL replay ordering | `python3 scripts/test/crash_sim.py --scenario replay_ordering` | 数据正确 |
| L3-10 | Double commit prevention | `python3 scripts/test/crash_sim.py --scenario double_commit` | 仅一次生效 |

### 3.3 Execution Divergence

| ID | 检查项 | 命令 | 标准 |
|----|--------|------|------|
| L3-11 | Same SQL all paths | `python3 scripts/test/execution_divergence.py` | PASS |
| L3-12 | NULL handling | `python3 scripts/test/execution_divergence.py --test null_handling` | PASS |
| L3-13 | Type coercion | `python3 scripts/test/execution_divergence.py --test type_coercion` | PASS |
| L3-14 | Error handling | `python3 scripts/test/execution_divergence.py --test error_handling` | PASS |

---

## 4. L4 — Architecture Gate

| ID | 检查项 | 命令 | 标准 |
|----|--------|------|------|
| L4-1 | AST routing only | `grep "eng.execute.*raw_sql" --include="*.rs"` | 0 matches |
| L4-2 | Execution path single | `grep -r "ExecutionEngine::execute_write\|eng.execute_write" --include="*.rs" \| grep -v "crates/executor/"` | 0 matches |
| L4-3 | VTU not fallback | `scripts/test/vtu_path_check.sh` | VTU is primary (not fallback) |
| L4-4 | execution_engine.rs size | `wc -l src/execution_engine.rs` | < 1500 lines |
| L4-5 | ParallelVolcanoExecutor integrated | `scripts/test/vtu_integration_check.sh` | PASS (not stub) |

---

## 5. L5 — Performance Gate

| ID | 检查项 | 命令 | 标准 |
|----|--------|------|------|
| L5-1 | TPC-H SF=1 | `./target/release/sqlrustgo-bench-cli tpch-bench --queries all` | 22/22 PASS |
| L5-2 | QPS regression | `scripts/bench/qps_regression.sh` | < 5% degradation vs v3.7.0 |
| L5-3 | VTU performance | `scripts/bench/vtu_perf.sh` | VTU enabled improves or equal |
| L5-4 | Stress 24h | `scripts/stress/stress_24h.sh` | 0 panic, 0 hang |
| L5-5 | Coverage delta | `scripts/coverage/delta_check.sh` | Z6G4 vs Z440 delta < 10pp |

---

## 6. L6 — Documentation Gate

| ID | 检查项 | 命令 | 标准 |
|----|--------|------|------|
| L6-1 | GA Gate Report exists | `ls docs/releases/v3.8.0/GA_GATE_REPORT.md` | 文件存在 |
| L6-2 | Changelog complete | `scripts/docs/changelog_check.sh` | 所有 PR 已记录 |
| L6-3 | Migration guide | `ls docs/releases/v3.8.0/MIGRATION_GUIDE.md` | 文件存在 |
| L6-4 | API reference | `ls docs/releases/v3.8.0/API_REFERENCE.md` | 文件存在 |
| L6-5 | SSOT cross-check | `bash scripts/docs/ssot_cross_check.sh` | PASS |

---

## 7. GA Score 计算

| 类别 | 最高分 | 门禁 |
|------|--------|------|
| Execution Core | 10 | L1-1~L1-7 |
| Transaction System | 15 | L3-01~L3-14 |
| Execution Consistency | 15 | L2-1~L2-5 |
| Architecture | 15 | L4-1~L4-5 |
| Performance | 15 | L5-1~L5-5 |
| Documentation | 10 | L6-1~L6-5 |
| **TOTAL** | **80** | **必须 ≥ 56 (70%)** |

---

## 7.5 Cross-Version Debt Gate

| ID | 检查项 | 命令/文件 | 标准 |
|----|--------|------------|------|
| CV-01 | INT-1~INT-4 状态已追踪 | `bash scripts/gate/check_cross_version_debt.sh` | PASS |
| CV-02 | Ghost PR 已正式 Defer | `docs/governance/adr/ADR-010-ghost-pr-resolution.md` | 存在 |
| CV-03 | v3.8.0+1 Post-GA Plan 存在 | `docs/releases/v3.8.0/POST_GA_PLAN.md` | 存在 |

**Cross-Version Debt Summary**:
| Debt ID | 问题 | 首次出现 | 状态 |
|---------|------|----------|------|
| INT-1 | DML 不经过 WAL/TransactionManager | v1.2.0 | ACTIVE (→ v3.9.0) |
| INT-2 | ParallelVolcanoExecutor 孤岛 | v2.6.0 | ACTIVE (→ v3.9.0) |
| INT-3 | expr crate 功能孤岛 | v3.0.0 | ACTIVE (→ v3.9.0) |
| INT-4 | mysql-server 未与主 server 集成 | v2.6.0 | ACTIVE (→ v3.9.0) |

---

## 8. 门禁执行脚本

**SSOT 入口**：`scripts/gate/check_rc_ga_gate.sh`

GA Gate 的真实执行入口是 [`check_rc_ga_gate.sh`](../../../../scripts/gate/check_rc_ga_gate.sh)，
支持五阶段分级调用：

```bash
# v3.8.0 GA Gate 入口（SSOT 真实脚本）
bash scripts/gate/check_rc_ga_gate.sh ga           # 完整 GA gate (D1+D2+D3+D4+D5+RC-to-GA checklist)
bash scripts/gate/check_rc_ga_gate.sh rc           # RC gate
bash scripts/gate/check_rc_ga_gate.sh beta         # Beta gate
bash scripts/gate/check_rc_ga_gate.sh alpha        # Alpha gate (D1 only)
bash scripts/gate/check_rc_ga_gate.sh all          # 五维度全开
bash scripts/gate/check_rc_ga_gate.sh --help       # 帮助
```

### 为什么 §8 不再是"fake script"

**历史**：v3.7.0 的 §8 是一段伪 `#!/bin/bash` 文档块，混合了
真实命令（`cargo test`）和**不存在的脚本**（`scripts/test/arch_check.sh`、
`scripts/test/vtu_path_check.sh`、`scripts/test/isolation_test_suite.py` 等），
违反 P5（未通过的必须有记录）原则。

**v3.8.0 修复（Issue #2878）**：
- ✅ §8 改为指向真实 SSOT 脚本 `check_rc_ga_gate.sh`
- ✅ 该脚本内部已实现五维度门禁（D1-D5）+ RC-to-GA checklist
- ✅ 文档中的命令路径与实际存在的脚本 1:1 对应

### 实际执行的 7 类检查

`check_rc_ga_gate.sh ga` 内部实际运行的检查（按维度）：

| 维度 | 检查项 | 性质 |
|------|--------|------|
| D1-Alpha | L1 + Clippy + Format + Coverage + ADR | 真实 cargo/脚本 |
| D2-Beta | Build + WAL Contract + Integration Gate | 真实 cargo/脚本 |
| D3-SGL | SGL-001~005 (Layer-3 语义检查) | 真实脚本 |
| D4-WAL | INV-1, INV-2, INV-3 (WAL 不变量) | 真实脚本 |
| D5-DeepSeek | 10 Principles for RC/GA gate | 真实脚本 |
| C-ARCH-05 | execution_engine.rs 行数限制 (SSOT: 1800) | 真实 wc + check |

**结论**：v3.8.0 §8 不再是 fake script。GA 通过的真实路径唯一：
`bash scripts/gate/check_rc_ga_gate.sh ga`，所有子检查由该脚本分发。

---

## 9. 快速失败检查（5分钟内）

在完整 GA Gate 之前，先运行快速检查（发现快速失败）：

```bash
# 5分钟快速检查
cargo test --lib --quiet && echo "L1 PASS" || echo "L1 FAIL"
python3 scripts/test/execution_consistency_harness.py --quick && echo "L2 PASS" || echo "L2 FAIL"
grep "eng.execute.*raw_sql" src/ --include="*.rs" && echo "L4 FAIL" || echo "L4 PASS"
wc -l src/execution_engine.rs | awk '{if($1<1500) print "L4 PASS"; else print "L4 FAIL"}'
```

---

## 10. 一句话总结

> **v3.8.0 GA Gate 的核心判断标准：系统是否从「双路径 SQL engine」收敛为「单路径 ACID database」**
