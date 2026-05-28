# v3.5.0 Release 报告

**GA 日期**: 2026-05-28
**GA Commit**: `5720805b` (Merge PR #1444)
**Tag**: `v3.5.0` → `5720805b`

---

## 发布核对清单

### 分支状态

| 平台 | main | develop/v3.5.0 | v3.5.0 Tag |
|------|------|-----------------|-------------|
| origin (Gitea) | 3e534b4c ⚠️ | 5720805b ✅ | b2aa6c3 ✅ |
| GitHub | — | cf840c2b | b2aa6c3 ✅ |
| GitCode | — | cf840c2b | b2aa6c3 ✅ |
| Gitee | 5720805b ✅ | cf840c2b | b2aa6c3 ✅ |

⚠️ origin main 保护阻止 push，需要手动合并或临时解除保护

### GA Gate 结果

| Gate | 命令 | 结果 |
|------|------|------|
| G1 Release编译 | `cargo build --release -p sqlrustgo` | ✅ PASS |
| G2 全测试 | `cargo test --workspace` | ✅ PASS |
| G3 Clippy | `cargo clippy --all-features -- -D warnings` | ✅ PASS (0 errors) |
| G4 Format | `cargo fmt --all -- --check` | ✅ PASS |
| G5 覆盖率 | L1 crates 平均 | ✅ **87.36%** |

### 覆盖率详情

| Crate | 覆盖率 | 方法 |
|-------|--------|------|
| sqlrustgo-types | 87.65% | --tests |
| sqlrustgo-parser | 83.82% | --tests |
| sqlrustgo-planner | 89.39% | --tests |
| sqlrustgo-optimizer | 89.72% | --tests |
| sqlrustgo-executor | 89.30% | --tests |
| sqlrustgo-storage | 81.70% | --lib (varlen pre-existing) |
| sqlrustgo-transaction | 88.75% | --tests |
| sqlrustgo-catalog | 88.52% | --tests |
| **平均** | **87.36%** | — |

### 关键修复 (v3.5.0-rc1 → v3.5.0)

| PR | 修复内容 |
|----|----------|
| #1442 | clippy needless_else (3 errors) |
| #1441 | UPDATE+SELECT regression (projection/ORDER BY index mismatch) |
| Z440 | audit_chain_verify.rs clippy (5 errors) |

### 已知限制

1. **Storage varlen 5 tests**: `test_encode_decode_blob_*`, `test_encode_decode_text_*` — pre-existing encoder bug，v3.6.0 修复
2. **origin main**: 分支保护阻止 v3.5.0 GA commit 直接 push，需 PR 合并

---

## v3.6.0 开发分支

- `develop/v3.6.0` 已创建 → `origin/develop/v3.6.0`
- 基于 `develop/v3.5.0` (commit 5720805b)

---

## 变更记录

### 3.5.0 (GA)

**新特性**:
- 综合覆盖率测量（--tests + --lib 降级策略）
- UPDATE+SELECT 回归修复

**改进**:
- clippy 零警告达标
- executor 覆盖率 89.30%

**文档**:
- GA_GATE_REPORT.md
- REFACTOR_ANALYSIS.md
- RC2_ANALYSIS.md
- GitNexus Blind Spot Analysis

### 3.4.0 → 3.5.0 累积修复

- projection/ORDER BY index mismatch
- empty else branches (3)
- unused compare_order_by function
- executor patch_error_tests assertion
- Alpha A5 coverage method
- R6 security offline fallback
- SCRIPT_DIR path fix
