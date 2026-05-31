# PR-800 ACCEPTANCE — COM_QUERY AST Routing

> **PR Number**: PR-800  
> **PR Title**: COM_QUERY AST Routing  
> **Version**: v3.8.0 Phase 0 Architecture Freeze  
> **Branch**: `develop/v3.8.0` (SHA: `745f24f1`)  
> **Auditor**: Hermes Agent  
> **Created**: 2026-05-30  
> **Status**: DRAFT — For Review  

---

## 1. 概述

本文档定义 PR-800 的验收标准（Acceptance Criteria）。每个验收标准必须有明确的验证方法和通过标准。

---

## 2. 功能验收标准

### 2.1 AC-1: Parser 正确解析所有 SQL 类型

**标准**: Parser::parse() 能正确解析 SELECT/INSERT/UPDATE/DELETE/DDL 语句，生成正确的 AST。

**验证方法**:
```bash
cargo test -p sqlrustgo-parser --lib -- test_pr800_
grep -E "test_pr800_|passed|failed" /tmp/parser_output.txt
```

**通过标准**: 所有 test_pr800_* 测试 PASS（预计 30+ tests）

**证据格式**:
```
AC-1: Parser parse tests
Command: cargo test -p sqlrustgo-parser --lib -- test_pr800_
Result: 32 passed, 0 failed
Status: ✅ PASS
```

---

### 2.2 AC-2: Planner 正确生成 PhysicalPlan

**标准**: Planner::plan() 能为所有 AST 类型生成正确的 PhysicalPlan。

**验证方法**:
```bash
cargo test -p sqlrustgo-planner --lib -- test_pr800_
```

**通过标准**: 所有 test_pr800_* 测试 PASS

---

### 2.3 AC-3: COM_QUERY 通过 AST Routing 执行

**标准**: mysql-server 收到 COM_QUERY 后，SQL 经过 Parser → Planner → LocalExecutor → StorageEngine 标准路径。

**验证方法**:
```bash
# 方法 1: 代码路径检查
grep -rn "parser::parse\|planner::plan\|local_executor.execute" src/ | grep -v "#.*parse\|#.*plan"
# 期望: 在 mysql-server 或 execution_engine 中有调用

# 方法 2: E2E 测试
cargo test -p sqlrustgo-e2e --all-features
# 期望: 28/28 PASS
```

**通过标准**: E2E 测试 28/28 PASS

---

### 2.4 AC-4: eng.execute(raw_sql) 路径已删除

**标准**: `eng.execute(raw_sql)` 直接执行路径不存在于代码中。

**验证方法**:
```bash
# 检查直接解析 raw_sql 的代码不存在
grep -rn "execute.*raw_sql\|raw_sql.*execute\|eng\.execute(sql)" src/execution_engine.rs | grep -v "#.*//"
# 期望: 0 matches

# 检查 execute() 函数签名
grep -n "pub fn execute" src/execution_engine.rs
# 期望: 仅存在用于向后兼容的 shim 函数（内部调用 execute_statement）
```

**通过标准**: 直接 raw_sql 执行路径 = 0 matches

---

### 2.5 AC-5: LocalExecutor 被主路径调用

**标准**: `local_executor.execute()` 在 COM_QUERY 处理路径中被调用。

**验证方法**:
```bash
grep -rn "local_executor\|LocalExecutor" src/ | grep -v "mod.rs\|lib.rs\|#.*LocalExecutor"
# 期望: 在 execution_engine.rs 或 mysql-server 中有调用
```

**通过标准**: local_executor 被主路径调用（非孤岛组件）

---

## 3. 向后兼容验收标准

### 3.1 BC-1: 现有 API 兼容

**标准**: 现有调用 `eng.execute(sql)` 的代码在 PR-800 后仍然工作（内部路由到新路径）。

**验证方法**:
```bash
cargo test --lib -p sqlrustgo --all-features
# 期望: 所有 tests PASS
```

**通过标准**: sqlrustgo crate 所有 lib tests PASS

---

### 3.2 BC-2: 现有 E2E 测试兼容

**标准**: 现有的 E2E 测试文件全部 PASS。

**验证方法**:
```bash
cargo test -p sqlrustgo-e2e --all-features 2>&1 | grep -E "test result|running|passed|failed"
```

**通过标准**: E2E test result: ok (所有测试通过)

---

## 4. 代码质量验收标准

### 4.1 CQ-1: 代码格式化

**标准**: 所有新增/修改的代码符合 `cargo fmt` 格式。

**验证方法**:
```bash
cargo fmt --all -- --check
```

**通过标准**: Exit code 0

---

### 4.2 CQ-2: Clippy 无警告

**标准**: `cargo clippy --all-features -- -D warnings` 无警告。

**验证方法**:
```bash
cargo clippy --all-features -- -D warnings 2>&1 | grep -E "warning|error"
```

**通过标准**: 0 warnings, 0 errors

---

### 4.3 CQ-3: Build 通过

**标准**: `cargo build --release --workspace` 成功。

**验证方法**:
```bash
cargo build --release --workspace 2>&1 | tail -5
```

**通过标准**: "Finished release profile" 或 "Compiling X crates... done"

---

## 5. 架构验收标准

### 5.1 AR-1: 单执行路径

**标准**: 所有 SQL 执行都经过 Parser → Planner → Executor 标准路径，不存在其他执行路径。

**验证方法**:
```bash
# 检查是否有 bypass Parser 的路径
grep -rn "execute.*sql.*&str\|execute_statement" src/ | grep -v "execute_statement"
# 期望: 仅 execute_statement 被调用
```

**通过标准**: 无绕过 Parser 的路径

---

### 5.2 AR-2: ExecutionEngine 重构为 Router

**标准**: ExecutionEngine 从"直接执行器"变为"路由协调器"，仅负责协调 Parser/Planner/Executor。

**验证方法**:
```bash
# ExecutionEngine 行数应该减少（从 6829 行减少到 <6000 行）
wc -l src/execution_engine.rs
# 期望: <6000 行（PR-900 才到达 <1500 行）
```

**通过标准**: execution_engine.rs 行数 <6000 行

---

## 6. 覆盖率验收标准

### 6.1 COV-1: Parser 覆盖率

**标准**: sqlrustgo-parser crate 覆盖率 ≥85%。

**验证方法**:
```bash
cargo llvm-cov test -p sqlrustgo-parser --all-features --tests --output-path /tmp/cov.json
# 或
cargo llvm-cov test -p sqlrustgo-parser --all-features --lib
```

**通过标准**: parser 覆盖率 ≥85%

---

### 6.2 COV-2: Planner 覆盖率

**标准**: sqlrustgo-planner crate 覆盖率 ≥85%。

**验证方法**:
```bash
cargo llvm-cov test -p sqlrustgo-planner --all-features --tests
```

**通过标准**: planner 覆盖率 ≥85%

---

### 6.3 COV-3: Executor 覆盖率

**标准**: sqlrustgo-executor crate 覆盖率不下降（维持 v3.7.0 的 83%）。

**验证方法**:
```bash
cargo llvm-cov test -p sqlrustgo-executor --all-features --tests
```

**通过标准**: executor 覆盖率 ≥83% (v3.7.0 baseline)

---

## 7. 验收检查清单

### 7.1 功能验收

| AC | 验收项 | 验证命令 | 状态 |
|----|--------|----------|------|
| AC-1 | Parser 正确解析 | `cargo test -p sqlrustgo-parser -- test_pr800_` | ⏳ |
| AC-2 | Planner 生成 PhysicalPlan | `cargo test -p sqlrustgo-planner -- test_pr800_` | ⏳ |
| AC-3 | COM_QUERY AST Routing | `cargo test -p sqlrustgo-e2e` | ⏳ |
| AC-4 | eng.execute(raw_sql) 已删除 | `grep -rn "raw_sql" src/execution_engine.rs` | ⏳ |
| AC-5 | LocalExecutor 被调用 | `grep -rn "local_executor" src/` | ⏳ |

### 7.2 向后兼容验收

| BC | 验收项 | 验证命令 | 状态 |
|----|--------|----------|------|
| BC-1 | 现有 API 兼容 | `cargo test --lib -p sqlrustgo` | ⏳ |
| BC-2 | E2E 测试兼容 | `cargo test -p sqlrustgo-e2e` | ⏳ |

### 7.3 代码质量验收

| CQ | 验收项 | 验证命令 | 状态 |
|----|--------|----------|------|
| CQ-1 | Format | `cargo fmt --all -- --check` | ⏳ |
| CQ-2 | Clippy | `cargo clippy --all-features -- -D warnings` | ⏳ |
| CQ-3 | Build | `cargo build --release --workspace` | ⏳ |

### 7.4 架构验收

| AR | 验收项 | 验证命令 | 状态 |
|----|--------|----------|------|
| AR-1 | 单执行路径 | `grep -rn "execute.*sql.*&str" src/` | ⏳ |
| AR-2 | ExecutionEngine <6000行 | `wc -l src/execution_engine.rs` | ⏳ |

### 7.5 覆盖率验收

| COV | 验收项 | 验证命令 | 状态 |
|-----|--------|----------|------|
| COV-1 | Parser ≥85% | `cargo llvm-cov test -p sqlrustgo-parser` | ⏳ |
| COV-2 | Planner ≥85% | `cargo llvm-cov test -p sqlrustgo-planner` | ⏳ |
| COV-3 | Executor ≥83% | `cargo llvm-cov test -p sqlrustgo-executor` | ⏳ |

---

## 8. SSOT 引用

- `docs/releases/v3.8.0/PR-800_SPEC.md` — PR-800 规格说明
- `docs/releases/v3.8.0/PR-800_TEST_PLAN.md` — PR-800 测试计划
- `docs/releases/v3.8.0/DEVELOPMENT_PLAN.md` — PR DAG
- `src/execution_engine.rs` — 当前 ExecutionEngine 实现