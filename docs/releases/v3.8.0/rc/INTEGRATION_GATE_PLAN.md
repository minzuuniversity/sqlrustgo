# Integration Gate 接入计划
**Date**: 2026-06-01
**Author**: Hermes C
**Status**: DRAFT
**gate_policy_eval_id**: `run_20260601_007`

---

## 1. 背景

deepseek 审查揭示了**设计‑实现‑测试三维一致性**的系统性缺失：

- INT-4: 双执行路径不一致（mysql-server vs bench-cli）
- INT-1: DML 不经 WAL（设计上拦截，实现直接写入）
- 覆盖率测量差异: Z6G4 vs Z440 差 49pp

现有 Alpha/Beta/RC/GA 门禁缺少**语义层面**的实质性检查。SGL (Semantic Gate Layer) 填补了这个空白。

---

## 2. Integration Gate 定位

### 2.1 门禁阶段映射

```
Alpha ──► Beta ──► Integration ──► RC ──► GA
  A1-A6     B1-B4      ★ NEW         TPC-H   TPC-H
                                    SF0.1~1  SF10
```

### 2.2 各阶段检查重点

| 阶段 | 检查重点 | 工具 |
|------|----------|------|
| Alpha | 编译通过、测试通过、格式干净 | cargo build/test/fmt/clippy |
| Beta | WAL 执行路径可用、RECOVERY 测试 | B1-B4 + RECOVERY-001~008 |
| **Integration** | **语义契约、设计‑实现一致性** | **C-ARCH + SGL + WAL Invariants** |
| RC | 功能完整、TPC-H SF0.1~1 | TPC-H fingerprint 对比 |
| GA | 性能基线、稳定性 | TPC-H SF10 + 压力测试 |

### 2.3 Integration Gate vs Beta Gate

| 维度 | Beta Gate | Integration Gate |
|------|-----------|----------------|
| 检查方式 | 命令执行（cargo build/test）| 静态分析 + 语义契约 |
| 检查对象 | 代码可编译、可运行 | 设计约束是否被遵守 |
| 覆盖范围 | B1 Build / B2 WAL / B3 Clippy / B4 Format | C-ARCH / SGL / WAL / Harness |
| 失败特征 | 编译错误、测试失败 | 架构漂移、语义偏差 |

---

## 3. 检查项定义

### 3.1 Section 1: Architecture Static Checks (C-ARCH)

| Rule ID | 设计约束 | 检查方法 | 阈值 |
|---------|----------|----------|------|
| C-ARCH-01 | LocalExecutor 无 txn_manager 字段 | grep 结构体定义 | 0 matches |
| C-ARCH-02 | LocalExecutor 无 write_buffer 字段 | grep 结构体定义 | 0 matches |
| C-ARCH-03 | storage.insert 只在 sqlrustgo 核心 | grep + 排除 gmp/bench-cli | 可审计 |
| C-ARCH-04 | 无 eng.execute() 在核心路径外 | grep 调用点 | 可审计 |
| C-ARCH-05 | execution_engine.rs < 2000 行 | wc -l | < 2000 |

**Note**: C-ARCH-01 按 PR-830 架构设计豁免；C-ARCH-03/04 由 SGL-005 DRIFT 追踪。

### 3.2 Section 2: SGL Layer-3 Semantic Gate

| Check ID | 契约 | 检查方法 | 状态 |
|----------|------|----------|------|
| SGL-001 | B4 format 是 read-only | cargo fmt --check | PASS ✅ |
| SGL-002 | WAL-002: checkpoint advance 在 commit 路径 | grep advance_checkpoint | PASS ✅ |
| SGL-003 | WAL-003: truncate_before 在 commit 路径 | grep truncate_before | PASS ✅ |
| SGL-004 | WAL-004: DELETE replay 幂等 | grep 语义检查 | PASS ✅ |
| SGL-005 | TX-002: Storage direct bypass | grep storage.insert | DRIFT ⚠️ |

### 3.3 Section 3: WAL Lifecycle Validation

| Test | 描述 | 验证方法 |
|------|------|----------|
| INV-1 | Committed data survives crash | Rust test: test_commit_flush_crash_replays |
| INV-2 | Uncommitted data NOT recovered | Rust test: test_*_crash_rolls_back |
| INV-3 | ROLLBACK leaves no trace | Rust test: test_*_rollback |

### 3.4 Section 4: Execution Consistency Harness

**状态**: 工具已部署，待 SQL corpus 完备后启用。

```
python3 scripts/test/execution_consistency_harness.py \
  --corpus data/sql_corpus_small.json \
  --sqlrustgo ./target/debug/sqlrustgo
```

---

## 4. 接入现有治理体系

### 4.1 更新 BETA_GATE_CONTRACT.md

在 B1-B4 基础上增加 **B5: Integration Gate** 检查项：

```
|| B5 | Integration Gate | bash scripts/gate/check_integration_gate.sh | exit 0 | INTEG-001~004 |
```

### 4.2 更新门禁脚本

`scripts/gate/check_beta_gate.sh` 应在 B4 之后调用 `scripts/gate/check_integration_gate.sh`。

### 4.3 门禁执行流程

```
Beta Gate 执行顺序:
  1. B1: cargo build --release
  2. B2: WAL execution path (RECOVERY tests)
  3. B3: cargo clippy
  4. B4: cargo fmt
  5. B5: bash scripts/gate/check_integration_gate.sh  ← NEW
```

---

## 5. 执行验证

### 5.1 当前状态（develop/v3.8.0 HEAD: 126c48b1）

```
check_integration_gate.sh:
  PASS: 4/4 sections
  FAIL: 0

SGL: 4/5 PASS, DRIFT=1
  SGL-001~004: PASS ✅
  SGL-005: DRIFT (AV-001~AV-007 legacy)

WAL: 22/22 PASS (Rust) + 5/5 PASS (invariant script)

C-ARCH: 2/2 PASS (C-ARCH-02, C-ARCH-05)
```

### 5.2 验证命令

```bash
# 独立运行 Integration Gate
bash scripts/gate/check_integration_gate.sh

# 单独运行各 Section
bash scripts/gate/check_arch_invariants.sh
python3 scripts/gate/semantic_gate_check.py
bash scripts/test/wal_invariant.sh

# Beta Gate 完整流程
bash scripts/gate/check_beta_gate.sh
```

---

## 6. 豁免与例外机制

### 6.1 DRIFT 分类

| 类型 | 含义 | 处理方式 |
|------|------|----------|
| LEGACY DRIFT | 历史遗留架构问题 | 记录在案，RC 前评审 |
| INTENTIONAL DRIFT | 已知设计偏离，有文档 | 豁免，需 PR 审批 |
| UNINTENDED DRIFT | 意外引入的偏差 | 必须修复 |

### 6.2 当前 DRIFT 项

| DRIFT ID | 描述 | 类型 | Owner |
|----------|------|------|-------|
| AV-001~AV-007 | Storage direct bypass | LEGACY DRIFT | 待定 |

---

## 7. 后续工作

| 任务 | 状态 | 说明 |
|------|------|------|
| B5 接入 check_beta_gate.sh | 🔲 待办 | 修改 Beta Gate 脚本 |
| 更新 BETA_GATE_CONTRACT.md | 🔲 待办 | 文档化 B5 |
| 更新 RC/GA Checklist | 🔲 待办 | 引用 Integration Gate 结果 |
| TPC-H SF0.01 fingerprint | 🔲 待办 | execution_consistency_harness 数据 |
| 豁免机制文档化 | 🔲 待办 | GATE_EFFECTIVENESS_MATRIX 更新 |
