# SPEC-004 — G-01 验证链强制门禁

<!-- env:blocked:no-ci -->

> **PR Number**: SPEC-004 (独立 PR，不属于 PR-800/830 系列)
> **PR Title**: G-01 Validation Chain Enforcement — 8 维度 30 项自动化门禁
> **Version**: v3.8.0
> **Branch**: `feature/spec-004-g01-validation-chain` (从 `442619e74` 切出)
> **Auditor**: Claude (claude-macmini, governance-engineer)
> **Created**: 2026-06-03
> **Status**: DRAFT — 待执行

---

## 1. 概述

### 1.1 问题（基于 ISSUE-2741 + ISSUES_AUDIT_AND_GAP_ANALYSIS §1.1）

ISSUE-2741 P0 阻塞：测试设计审查无强制门禁。

**当前状态**：
- ✅ TEST_REVIEW_TEMPLATE.md（8 维度 30 项）已存在（commit b9e50f9ac）
- ❌ 8 维度 30 项仍靠人工 `TEST_REVIEW.md` 填写
- ❌ 没有 `scripts/gate/check_validation_chain.sh` 自动化
- ❌ Beta/RC Gate 不强制要求 TEST_REVIEW 完整

**ISSUE-2741 证据**：
- E-1: RECOVERY-006 断言 `COUNT(*) == 1` 不能证明 "DELETE 不恢复" 需求
- E-2: 集成 WAL 测试用间接断言（commit count vs data absent）
- E-3: GOVERNANCE.md 无测试设计审查门禁规则

### 1.2 深度调研（2026-06-03 develop/v3.8.0 @ 442619e74）

#### Finding-1: 现有 gate 脚本模式

```
$ ls scripts/gate/ | grep check_ | head -10
check_10_principles.sh
check_5_principles.sh
check_alpha.sh
check_arch_invariants.sh
check_architecture_freeze.sh
check_beta_gate.sh
check_docs_consistency.sh
check_docs_links.sh
check_evidence_binding.sh
check_g_02_source_marker.sh
check_g_06_freshness.sh
```

**模式分析**：
- `check_*.sh` 命名约定
- 退出码 0=PASS, 1=FAIL
- 输出 evidence（`grep` 命中行 + 文件路径）
- 增量计数 `PASS=$((PASS+1))` / `FAIL=$((FAIL+1))`

**参考脚本**：`check_g_02_source_marker.sh`（最相似 — 文档 claim 验证）

#### Finding-2: TEST_REVIEW_TEMPLATE 8 维度可自动化部分

| 维度 | 可自动化 | 原因 |
|------|----------|------|
| 2.1 Coverage Design | 部分 | grep 测试函数 vs SPEC 引用 |
| 2.2 Independence | ✅ | grep `tempfile::TempDir` / `setup` / `teardown` |
| 2.3 Assertion Quality | ✅ | grep `assert!` 模式 |
| 2.4 Real Executability | ✅ | grep `#[ignore]` + `TODO` + `// #[test]` |
| 2.5 Performance & Stability | 部分 | grep `panic!` exit codes |
| 2.6 Documentation | 部分 | grep `/// <PR-id>` doc-comments |
| 2.7 Security | ✅ | grep `tempfile` vs `File::create` / hardcoded secrets |
| 2.8 Integration & Gate | ✅ | grep `cargo test` in CI / Makefile |

**自动化覆盖率目标**：≥ 5/8 维度全自动化

#### Finding-3: 现有测试中 `#[ignore]` 数量

```
$ grep -rn "#\[ignore" --include="*.rs" tests/ crates/ 2>/dev/null | wc -l
~15-20
```

**ISSUE-2740 E-2 来源**：`crash_recovery_test.rs` 测试用 MemoryStorage，部分 RECOVERY-007 用 `#[ignore]`。这些 `#[ignore]` 应有 tracking issue 引用（SPEC-004 将强制）。

## 2. 设计

### 2.1 目标

创建 `scripts/gate/check_validation_chain.sh`，自动化验证：

| 维度 | 检查 | 工具 | 退出码 |
|------|------|------|--------|
| 2.1.4 | 覆盖矩阵 | 手工 | N/A (人工审) |
| 2.2.1 | 无外部依赖 | grep `File::create("/` / `TcpStream` | 0=无 |
| 2.2.4 | 资源清理 | grep `tempfile::TempDir` 配对 | 0=配对 |
| 2.3.1 | 断言具体 | grep `assert!([^,)]*\.is_ok\(\)\)` | 0=无 |
| 2.3.4 | 不依赖 print | grep `eprintln!` 在 `#[test]` 函数 | 0=无 |
| 2.4.2 | `#[ignore]` 有理由 | grep `#[ignore.*=.*"[^"]+"` | 0=全有 |
| 2.4.3 | 无 TODO 占位 | grep `// TODO: add test` | 0=无 |
| 2.4.4 | 无 commented-out 测试 | grep `^[[:space:]]*//[[:space:]]*#\[test\]` | 0=无 |
| 2.7.2 | 临时文件用 tempfile | grep `File::create` in tests/ | 0=无 |
| 2.7.3 | 无硬编码密钥 | grep `password.*=.*"[^"]\+"` in tests/ | 0=无 |
| 2.8.1 | clippy 0 warning | `cargo clippy --all-features -- -D warnings` | 0=PASS |
| 2.8.2 | fmt 0 error | `cargo fmt --all -- --check` | 0=PASS |

### 2.2 输出格式

```
=== G-01 Validation Chain Check ===
[2.2.1] No external dependencies...           PASS (0 violations)
[2.2.4] Resource cleanup (TempDir pairing)... PASS (12/12 paired)
[2.3.1] Assertion specificity...               PASS (0 weak assertions)
[2.3.4] No print-dependence...                 FAIL (3 eprintln! in #[test])
  Evidence: tests/foo.rs:42:    eprintln!("debug: {:?}", x);
[2.4.2] #[ignore] has reason...                PASS (15/15 have reason)
[2.4.3] No TODO placeholders...                PASS
[2.4.4] No commented-out tests...              PASS
[2.7.2] Tempfile usage...                      PASS
[2.7.3] No hardcoded secrets...                FAIL (1 found)
  Evidence: tests/auth_test.rs:15:    let pw = "secret123";
[2.8.1] clippy 0 warning...                    PASS
[2.8.2] fmt 0 error...                         PASS

=== Summary ===
PASS: 9
FAIL: 2
Total: 11

Exit 1 (FAIL) or 0 (PASS)
```

### 2.3 接入 Gate

修改 `scripts/gate/check_beta_gate.sh` 和 `scripts/gate/check_rc_ga_gate.sh`，
在现有检查项后追加：
```bash
bash scripts/gate/check_validation_chain.sh || exit 1
```

## 3. 实施路径

| 步骤 | 工作量 | 产物 |
|------|--------|------|
| 3.1 编写 `check_validation_chain.sh` | 1.5h | 200 行 bash |
| 3.2 本地 dry-run | 0.5h | evidence.json |
| 3.3 接入 Beta Gate | 0.5h | 修改 2 个 gate 脚本 |
| 3.4 创建 ADR-009 | 0.5h | docs/governance/adr/ADR-009-*.md |
| 3.5 更新 ISSUE-2741 状态 | 10min | 修改 1 处 |

**总计**：约半天

## 4. 完成标准

- [ ] `scripts/gate/check_validation_chain.sh` 创建
- [ ] 至少 5/8 维度自动化（实际 8/8 维度包含可自动化项）
- [ ] `check_beta_gate.sh` 和 `check_rc_ga_gate.sh` 接入
- [ ] 本地执行：记录 evidence.json 含 PASS/FAIL 计数
- [ ] `docs/governance/adr/ADR-009-g01-validation-chain-enforcement.md` 创建
- [ ] `docs/audit/issues/ISSUE-2741_validation_chain_missing.md` 状态从 UNVERIFIED → ENFORCED
- [ ] PR 合并 + Issue #2772 关闭

## 5. Risks

| Risk | Mitigation |
|------|------------|
| R-1: 误报（false positive） | 初期用 WARN-only 模式，运行 1 周后改 FAIL |
| R-2: 现有测试违反新规则 | Issue 列表化，分批修复 |
| R-3: 新规则过严导致 CI 噪声 | 渐进式开启：先 Beta 后 RC |

## 6. 引用

- ISSUE-2741: docs/audit/issues/ISSUE-2741_validation_chain_missing.md
- TEST_REVIEW_TEMPLATE: docs/governance/TEST_REVIEW_TEMPLATE.md (8 维度 30 项)
- 父任务: #2772 (Task #2747)
- 整改报告: docs/audit/V380_RECTIFICATION_PLAN_2026-06-03.md §3.2
- ADR-007: docs/governance/adr/ADR-007-wal-architecture-clarification.md

## 7. 变更历史

| 版本 | 日期 | 作者 | 说明 |
|------|------|------|------|
| 1.0 | 2026-06-03 | claude-macmini | 初始 SPEC：G-01 强制门禁设计 |
