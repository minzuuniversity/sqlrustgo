# SQLRustGo v3.8.0 测试计划

> **版本**: v3.8.0
> **类型**: Architecture Unification Release
> **分支**: `origin/develop/v3.8.0`
> **创建日期**: 2026-05-30
> **Auditor**: Hermes Agent
> **Status**: ACTIVE — Execution Semantics Freeze (commit 087bb12d)

---

## 1. 测试策略

### 1.1 三层验证体系（替代 checklist）

> **核心原则**: 测试必须验证"执行模型正确性"，而非"功能存在性"

---

### Layer 1 — Unit Correctness（已有基础）

**目标**: 每个原子模块的正确性

| 模块 | 测试覆盖目标 | 门禁 |
|------|-------------|------|
| sqlrustgo-parser | AST 生成 100% node coverage | 100% |
| sqlrustgo-catalog | schema 元数据正确性 | 95%+ |
| sqlrustgo-executor | 执行计划正确性 | 90%+ |
| sqlrustgo-storage | 读写正确性 | 95%+ |
| wal-verification | WAL append/replay | 100% + crash test |
| transaction manager | 状态机（BEGIN/COMMIT/ROLLBACK） | 100% |

---

### Layer 2 — Execution Consistency（新增，v3.8.0 核心）

**目标**: 三条执行路径结果必须完全一致

```
SQL Test Corpus (500+ queries)
     ↓
┌─────────────────────────────────────┐
│  Path A: mysql-server               │
│  Path B: bench-cli (LocalExecutor)  │
│  Path C: Direct Executor call        │
└─────────────────────────────────────┘
     ↓
result_hash_diff engine
     ↓
assert: all paths same result
```

**测试矩阵**:

| 测试用例集 | 路径 | 门禁 |
|-----------|------|------|
| DDL (CREATE/ALTER/DROP) | A vs B vs C | hash 一致 |
| DML (INSERT/UPDATE/DELETE) | A vs B vs C | hash 一致 |
| SELECT (simple) | A vs B vs C | hash 一致 |
| SELECT (window/CTE/subquery) | A vs B vs C | hash 一致 |
| Transaction (BEGIN/COMMIT) | A vs B vs C | hash 一致 |

**门禁**:
```
execution_consistency_harness:
  PASS: all paths same result hash
  FAIL: any divergence → PR blocked
```

---

### Layer 3 — ACID Verification（新增，v3.8.0 独有）

**目标**: 数据库核心语义的正确性验证

#### 3.1 Transaction Isolation Suite

| 测试 | 描述 | 验证方法 | 门禁 |
|------|------|----------|------|
| T-ISO-01 | Dirty Read Prevention | txn1 write uncommitted → txn2 read → must NOT see | MUST PASS |
| T-ISO-02 | Non-repeatable Read | txn1 read → txn1 write → txn1 read → must same value | MUST PASS |
| T-ISO-03 | Phantom Read | txn1 range query → txn2 insert → txn1 range query → must NOT include new | MUST PASS |
| T-ISO-04 | Write-Write Conflict | txn1 write row X → txn2 write row X → one blocks or fails | MUST PASS |
| T-ISO-05 | Lost Update | concurrent update same row → final value correct | MUST PASS |

#### 3.2 Crash Simulation Suite

| 测试 | 操作 | 验证 | 门禁 |
|------|------|------|------|
| T-CRA-01 | commit 中 kill -9 | 重启后 WAL replay → 数据存在 | MUST PASS |
| T-CRA-02 | rollback 中 kill -9 | 重启后数据未改变 | MUST PASS |
| T-CRA-03 | partial write 中断 | 重启后数据一致或 empty | MUST PASS |
| T-CRA-04 | WAL replay ordering | 乱序写入 replay 后正确 | MUST PASS |
| T-CRA-05 | double commit | 重启后仅一次生效 | MUST PASS |

#### 3.3 Execution Divergence Test

| 测试 | 描述 | 门禁 |
|------|------|------|
| T-DIV-01 | Same SQL all paths | mysql-server == bench-cli == direct | MUST PASS |
| T-DIV-02 | NULL handling | all paths same NULL semantics | MUST PASS |
| T-DIV-03 | Type coercion | all paths same coercion result | MUST PASS |
| T-DIV-04 | Error handling | all paths same error code/msg | MUST PASS |

---

## 2. 门禁检查清单（按 PR 阶段嵌入）

### Phase 0 (PR-800, PR-810) — Alpha Gate

| Gate | 测试 | 标准 | 嵌入 PR |
|------|------|------|---------|
| A1 | cargo test --all-features | 0 failures | PR-800 |
| A2 | cargo clippy --all-features | 0 errors | PR-800 |
| A3 | grep "eng.execute.*raw_sql" | 0 matches | PR-800 |
| A4 | AST routing coverage | stmt type → handler 100% | PR-810 |

---

### Phase 1 (PR-820~PR-840) — Beta Entry Gate

| Gate | 测试 | 标准 | 嵌入 PR |
|------|------|------|---------|
| B1 | BEGIN/INSERT/COMMIT persistence | SELECT 返回插入数据 | PR-830 |
| B2 | T-CRA-01 crash recovery | 数据存在 after kill -9 | PR-830 |
| B3 | T-CRA-02 rollback crash | 数据未改变 | PR-840 |
| B4 | DML WAL trace | WAL log contains DML ops | PR-830 |
| B5 | T-ISO-01 dirty read | PASS | PR-840 |

---

### Phase 2 (PR-850, PR-860) — Beta Gate

| Gate | 测试 | 标准 | 嵌入 PR |
|------|------|------|---------|
| C1 | T-DIV-01 execution consistency | all paths same hash | PR-850 |
| C2 | E2E integration tests | 28/28 PASS | PR-850 |
| C3 | TPC-H SF=1 regression | Q1-Q22 < 5% degradation | PR-850 |
| C4 | grep storage.insert outside storage layer | 0 matches | PR-850 |
| C5 | mysql-server vs bench-cli DDL | hash 一致 | PR-850 |

---

### Phase 3 (PR-870, PR-880) — Beta Gate

| Gate | 测试 | 标准 | 嵌入 PR |
|------|------|------|---------|
| D1 | VTU vectorized vs non-vectorized | 结果一致 | PR-870 |
| D2 | T-DIV-02 NULL handling | all paths same | PR-870 |
| D3 | regression QPS test | < 5% degradation | PR-870 |
| D4 | execution_engine.rs lines | 开始下降（<6500） | PR-880 |

---

### Phase 4 (PR-890) — RC Gate

| Gate | 测试 | 标准 | 嵌入 PR |
|------|------|------|---------|
| E1 | T-ISO-01~05 isolation suite | ALL PASS | PR-890 |
| E2 | T-CRA-03~05 crash suite | ALL PASS | PR-890 |
| E3 | ROLLBACK 实际隔离 | 未提交数据不可见 | PR-890 |
| E4 | concurrent snapshot read | consistent | PR-890 |

---

### Phase 5 (PR-900) — RC Gate

| Gate | 测试 | 标准 | 嵌入 PR |
|------|------|------|---------|
| F1 | execution_engine.rs | < 1500 lines | PR-900 |
| F2 | coverage Z6G4 vs Z440 | delta < 10pp | PR-900 |
| F3 | stress test 24h | 0 panic | PR-900 |
| F4 | failure injection | 所有场景恢复 | PR-900 |
| F5 | cargo test --all-features | 0 failures | PR-900 |

---

### GA Final Gate

| Gate | 测试 | 标准 |
|------|------|------|
| G1 | Full test suite | 0 failures |
| G2 | T-ISO-01~05 | ALL PASS |
| G3 | T-CRA-01~05 | ALL PASS |
| G4 | T-DIV-01~04 | ALL PASS |
| G5 | TPC-H SF=1 | 22/22 PASS |
| G6 | execution_engine.rs | < 1500 lines |
| G7 | stress test 24h | 0 panic |
| G8 | coverage delta | < 10pp |

---

## 3. 新增工具需求

### 3.1 execution_consistency_harness

```
输入: SQL corpus (500+ queries)
输出:
  PASS: all execution paths same result hash
  FAIL: divergence details per path
```

### 3.2 crash_simulation_runner

```
输入: test scenario
操作: fork → execute SQL → kill -9 child → restart → verify
输出: PASS/FAIL + recovery details
```

### 3.3 isolation_test_suite

```
输入: isolation level definition
操作: 2+ concurrent transactions with defined order
输出: PASS (correct isolation) / FAIL (violation detected)
```

---

## 4. 测试数据要求

### 4.1 SQL Corpus

| 类型 | 数量 | 来源 |
|------|------|------|
| DDL | 100 | 已有 |
| DML | 150 | 已有 |
| SELECT simple | 100 | 已有 |
| SELECT complex (CTE/window) | 100 | 已有 |
| Transaction | 50 | 需新增 |
| **Total** | **500+** | |

### 4.2 TPC-H SF=1

用于性能回归测试：
- `/opt/tpch/tpch-dbgen/*_clean.tbl`
- 22 queries 必须全部 PASS
- Performance regression < 5%

---

## 5. 测试执行命令

```bash
# Layer 1: Unit tests
cargo test --all-features -- --nocapture

# Layer 2: Execution consistency
python3 scripts/test/execution_consistency_harness.py --corpus data/sql_corpus.json --paths mysql-server,bench-cli,direct

# Layer 3: ACID isolation
python3 scripts/test/isolation_test_suite.py --level snapshot

# Layer 3: Crash simulation
python3 scripts/test/crash_simulation_runner.py --scenarios data/crash_scenarios.json

# TPC-H regression
./target/release/sqlrustgo-bench-cli tpch-bench --queries all --ddl scripts/tpch/tpch_schema.sql

# Full gate
bash scripts/gate/check_alpha_v3.8.0.sh
bash scripts/gate/check_beta_v3.8.0.sh
bash scripts/gate/check_rc_v3.8.0.sh
```

---

## 6. 一句话总结

> **v3.8.0 测试体系的核心不是"功能是否存在"，而是"执行模型是否正确一致"**