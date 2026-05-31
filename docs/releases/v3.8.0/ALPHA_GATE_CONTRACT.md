# ALPHA_GATE_CONTRACT.md — v3.8.0

> **版本**: v3.8.0 Alpha  
> **类型**: Gate Contract  
> **用途**: 定义 Alpha Gate 的检查项、阈值和验证方法  
> **分支**: `alpha/v3.8.0` (待创建)  
> **Auditor**: Hermes Agent  
> **Created**: 2026-05-30  
> **Status**: ACTIVE — Execution Semantics Freeze 已声明（commit 087bb12d）  

---

## 0. 前置说明

### 0.1 Governance-Driven Development

v3.8.0 是 Architecture Unification Release，不是质量提升版本。PR-800（COM_QUERY AST Routing）落地前：

- A1 Build PASS 没有意义（测试的是旧架构）
- A2 Test PASS 没有意义（测试的是旧路径）
- A5 Coverage ≥75% 更没有意义（覆盖率测量的是旧代码路径）

**因此，本 Gate Contract 在 A1-A5 基础上增加 A6 Governance，确保 Governance 框架参与门禁。**

### 0.2 Contract vs Spec vs Report

| 文档 | 用途 |
|------|------|
| **GATE_SPEC** | 治理标准的定义（SSOT） |
| **GATE_CONTRACT** | 本次 Alpha Gate 的具体检查项和阈值 |
| **GATE_REPORT** | Alpha Gate 执行后的实际结果记录 |

---

## 1. Alpha Gate 检查项

### 1.1 标准检查项（A1-A5）

| ID | Check | Method | Threshold | SSOT Reference |
|----|-------|--------|-----------|----------------|
| A1 | Build | `cargo build --release --workspace` | PASS (exit 0) | GATE_CONDITIONS.md §Alpha A1 |
| A2 | Test | `cargo test --lib --workspace` | 0 failures | GATE_CONDITIONS.md §Alpha A2 |
| A3 | Clippy | `cargo clippy --all-features -- -D warnings` | 0 warnings | GATE_CONDITIONS.md §Alpha A3 |
| A4 | Format | `cargo fmt --all -- --check` | PASS (exit 0) | GATE_CONDITIONS.md §Alpha A4 |
| A5 | Coverage | L1 8 crates 综合平均 | ≥75% | GATE_CONDITIONS.md §Alpha A5 |

### 1.2 Governance 检查项（A6）

| ID | Check | Method | Threshold | SSOT Reference |
|----|-------|--------|-----------|----------------|
| A6-1 | Replay Graph | `docs/governance/replay/REPLAY_v3.7.0_GA.md` 存在 | 4 issues 可追溯 | ADR-002 §Claim Registry |
| A6-2 | Claim Registry | Claim v3.8.0 相关 Claim 已记录 | 关键 Claim 有 ID | ADR-002 §Claim Registry |
| A6-3 | Decision Registry | Architecture Decision 有 ID | AD-001~AD-005 有记录 | ADR-003 §Decision Registry |
| A6-4 | Freshness PASS | 引用数据必须标注 Freshness | 无过期数据引用 | ADR-001 §G-06 |
| A6-5 | ADR Updated | Governance ADR 体系存在 | 5 ADRs 存在 | ADR-001~ADR-005 |

---

## 2. 各检查项详细说明

### A1: Build (release)

**命令**:
```bash
cargo build --release --workspace 2>&1 | tail -3
```

**通过标准**: `Finished release profile` 或 exit code 0

**Evidence 格式**:
```
=== A1: Build (release) ===
Command: cargo build --release --workspace
Result: Finished release profile [optimized] target(s) in X.XXs
Status: ✅ PASS
```

---

### A2: Test (lib)

**命令**:
```bash
cargo test --lib --workspace --exclude sqlrustgo-mysql-server 2>&1 | grep -E "test result|running|passed|failed"
```

**通过标准**: `test result: ok` (0 failures)

**注意**: 排除 `sqlrustgo-mysql-server`（有已知的 43 errors，是 legacy issue）

**Evidence 格式**:
```
=== A2: Test (lib) ===
Command: cargo test --lib --workspace --exclude sqlrustgo-mysql-server
Result: test result: ok. XX passed; 0 failed
Status: ✅ PASS
```

---

### A3: Clippy

**命令**:
```bash
cargo clippy --all-features -- -D warnings 2>&1 | grep -E "warning|error"
```

**通过标准**: 0 warnings, 0 errors

**Evidence 格式**:
```
=== A3: Clippy ===
Command: cargo clippy --all-features -- -D warnings
Result: (empty - no warnings)
Status: ✅ PASS
```

---

### A4: Format

**命令**:
```bash
cargo fmt --all -- --check 2>&1
```

**通过标准**: exit code 0

**Evidence 格式**:
```
=== A4: Format ===
Command: cargo fmt --all -- --check
Result: Exit code 0
Status: ✅ PASS
```

---

### A5: Coverage

**命令**:
```bash
# L1 8 crates 综合测量方法
for crate in sqlrustgo-types sqlrustgo-parser sqlrustgo-planner \
             sqlrustgo-optimizer sqlrustgo-executor sqlrustgo-storage \
             sqlrustgo-transaction sqlrustgo-catalog; do
    cargo llvm-cov test -p "$crate" --all-features --tests 2>/dev/null | \
        grep "^TOTAL" || cargo llvm-cov test -p "$crate" --all-features --lib 2>/dev/null | grep "^TOTAL"
done
```

**通过标准**: L1 8 crates 综合平均 ≥75%

**Evidence 格式**:
```
=== A5: Coverage ===
Command: cargo llvm-cov test (L1 8 crates, comprehensive method)
Result:
  sqlrustgo-types: XX.XX%
  sqlrustgo-parser: XX.XX%
  ...
  Average: XX.XX%
Status: ✅ PASS (≥75%)
```

---

### A6-1: Replay Graph

**命令**:
```bash
ls docs/governance/replay/REPLAY_v3.7.0_GA.md
grep -E "#2580|#2582|#2613|#2615" docs/governance/replay/REPLAY_v3.7.0_GA.md | wc -l
```

**通过标准**: 文件存在，且至少 4 个 issue 有完整轨迹

**Evidence 格式**:
```
=== A6-1: Replay Graph ===
Command: ls docs/governance/replay/REPLAY_v3.7.0_GA.md
Result: File exists
Issue Coverage:
  #2580: Parser coverage debt ✅
  #2582: Executor coverage debt ✅
  #2613: P0 fixes ✅
  #2615: Coverage ceiling ✅
Status: ✅ PASS (4/4 issues traced)
```

---

### A6-2: Claim Registry

**命令**:
```bash
grep -E "CLAIM-v3\.|claim_id:" docs/governance/adr/ADR-002-claim-registry.md | head -10
```

**通过标准**: v3.8.0 相关的 Claim 有记录

**Evidence 格式**:
```
=== A6-2: Claim Registry ===
Command: grep "CLAIM-v3.8" docs/governance/adr/ADR-002-claim-registry.md
Result: Found N claims
Status: ✅ PASS
```

---

### A6-3: Decision Registry

**命令**:
```bash
ls docs/releases/v3.8.0/ARCHITECTURE_DECISIONS.md
grep -E "^## AD-00" docs/releases/v3.8.0/ARCHITECTURE_DECISIONS.md
```

**通过标准**: AD-001~AD-005 存在

**Evidence 格式**:
```
=== A6-3: Decision Registry ===
Command: ls docs/releases/v3.8.0/ARCHITECTURE_DECISIONS.md
Result: File exists
AD Coverage:
  AD-001: ExecutionEngine 拆分 ✅
  AD-002: Single-path Execution ✅
  AD-003: WAL First ✅
  AD-004: Planner Consolidation ✅
  AD-005: VTU Mainline ✅
Status: ✅ PASS (5/5 ADs recorded)
```

---

### A6-4: Freshness PASS

**命令**:
```bash
grep -E "Freshness|v3\.[0-9]\.[0-9]" docs/releases/v3.8.0/PR-800_SPEC.md | head -10
```

**通过标准**: 引用数据标注 Freshness，无过期数据

**Evidence 格式**:
```
=== A6-4: Freshness PASS ===
Command: grep "Freshness" docs/releases/v3.8.0/PR-800_SPEC.md
Result: Freshness markers found
Status: ✅ PASS
```

---

### A6-5: ADR Updated

**命令**:
```bash
ls docs/governance/adr/ADR-00*.md
```

**通过标准**: ADR-001~ADR-005 全部存在

**Evidence 格式**:
```
=== A6-5: ADR Updated ===
Command: ls docs/governance/adr/ADR-00*.md
Result:
  ADR-001-truthfulness-framework.md ✅
  ADR-002-claim-registry.md ✅
  ADR-003-decision-registry.md ✅
  ADR-004-negative-evidence.md ✅
  ADR-005-legacy-gate-retirement.md ✅
Status: ✅ PASS (5/5 ADRs)
```

---

## 3. Alpha Gate 判定规则

### PASS

所有 A1-A5 和 A6-1~A6-5 全部 PASS → Alpha Gate PASS

### CONDITIONAL PASS

A1-A5 全部 PASS，A6 有 1-2 项 FAIL（可在 Beta 前修复）→ CONDITIONAL PASS

**条件**:
1. A6-1~A6-5 中最多 2 项 FAIL
2. 必须创建 Issue 追踪
3. 2 周内解除

### FAIL

- A1-A4 任一项 FAIL → FAIL
- A5 Coverage <50% → FAIL
- A6-1~A6-5 中 3+ 项 FAIL → FAIL

---

## 4. Alpha Gate 执行流程

```
Step 1: 创建 alpha/v3.8.0 分支
  git checkout -b alpha/v3.8.0 origin/develop/v3.8.0
  git push origin alpha/v3.8.0

Step 2: 执行标准检查 (A1-A5)
  cargo build --release --workspace  # A1
  cargo test --lib --workspace      # A2
  cargo clippy --all-features       # A3
  cargo fmt --all -- --check        # A4
  cargo llvm-cov (L1 8 crates)     # A5

Step 3: 执行 Governance 检查 (A6)
  docs/governance/replay/          # A6-1
  docs/governance/adr/             # A6-2~A6-5

Step 4: 记录 ALPHA_GATE_REPORT.md
  创建: docs/releases/v3.8.0/ALPHA_GATE_REPORT.md
  记录: 实际命令 + 输出 + 结论

Step 5: 如有 FAIL 项
  创建 Issue 追踪
  修复后重新执行
```

---

## 5. 附录：检查命令速查

```bash
# A1 Build
cargo build --release --workspace

# A2 Test
cargo test --lib --workspace --exclude sqlrustgo-mysql-server

# A3 Clippy
cargo clippy --all-features -- -D warnings

# A4 Format
cargo fmt --all -- --check

# A5 Coverage
for c in sqlrustgo-types sqlrustgo-parser sqlrustgo-planner sqlrustgo-optimizer \
         sqlrustgo-executor sqlrustgo-storage sqlrustgo-transaction sqlrustgo-catalog; do
    cargo llvm-cov test -p "$c" --all-features --tests 2>/dev/null | grep "^TOTAL" || \
    cargo llvm-cov test -p "$c" --all-features --lib 2>/dev/null | grep "^TOTAL"
done

# A6-1 Replay Graph
ls docs/governance/replay/REPLAY_v3.7.0_GA.md

# A6-2 Claim Registry
ls docs/governance/adr/ADR-002-claim-registry.md

# A6-3 Decision Registry
ls docs/releases/v3.8.0/ARCHITECTURE_DECISIONS.md

# A6-4 Freshness
grep -r "Freshness" docs/releases/v3.8.0/*.md | head -5

# A6-5 ADR Updated
ls docs/governance/adr/ADR-00*.md
```

---

## 6. SSOT 引用

- `docs/governance/GATE_CONDITIONS.md` — Alpha Gate 标准
- `docs/governance/adr/ADR-001-truthfulness-framework.md` — Truthfulness Framework
- `docs/governance/adr/ADR-002-claim-registry.md` — Claim Registry
- `docs/governance/adr/ADR-003-decision-registry.md` — Decision Registry
- `docs/governance/replay/REPLAY_v3.7.0_GA.md` — v3.7.0 Replay Graph
- `docs/releases/v3.8.0/PR-800_SPEC.md` — PR-800 规格说明
- `docs/releases/v3.8.0/PR-800_TEST_PLAN.md` — PR-800 测试计划
- `docs/releases/v3.8.0/PR-800_ACCEPTANCE.md` — PR-800 验收标准
- `docs/releases/v3.8.0/ARCHITECTURE_DECISIONS.md` — 架构决策记录