# SQLRustGo v3.8.0 GA Gate Checklist

> **版本**: v3.8.0
> **类型**: Architecture Unification Release
> **分支**: `origin/develop/v3.8.0`
> **Auditor**: Hermes Agent
> **标准**: v3.8.0 必须通过以下全部门禁方可发布 GA

---

## 0. GA Gate 概述

v3.8.0 GA 门禁的核心不再是「功能是否存在」，而是：

> **执行模型正确性 + 执行路径统一性 + ACID 语义正确性**

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

## 8. 门禁执行脚本

```bash
#!/bin/bash
# scripts/gate/check_ga_v3.8.0.sh

set -e

echo "=== v3.8.0 GA Gate ==="

echo "[L1] Unit Correctness..."
cargo test -p sqlrustgo-parser --lib --quiet
cargo test -p sqlrustgo-executor --lib --quiet
cargo test -p sqlrustgo-storage --lib --quiet
cargo test -p sqlrustgo-transaction --lib --quiet
cargo clippy --all-features --quiet
cargo fmt -- --check

echo "[L2] Execution Consistency..."
python3 scripts/test/execution_consistency_harness.py --corpus data/sql_corpus.json --paths mysql-server,bench-cli,direct
cargo test -p sqlrustgo-integration-tests --quiet

echo "[L3] ACID Verification..."
python3 scripts/test/isolation_test_suite.py --all
python3 scripts/test/crash_sim.py --all
python3 scripts/test/execution_divergence.py --all

echo "[L4] Architecture..."
bash scripts/test/arch_check.sh
bash scripts/test/vtu_path_check.sh

echo "[L5] Performance..."
./target/release/sqlrustgo-bench-cli tpch-bench --queries all
bash scripts/bench/qps_regression.sh

echo "[L6] Documentation..."
bash scripts/docs/ssot_cross_check.sh

echo "=== GA Gate PASSED ==="
```

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