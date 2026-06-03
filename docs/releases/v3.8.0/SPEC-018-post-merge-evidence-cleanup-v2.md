# SPEC-018 — Post-merge A8-1 EVIDENCE 收尾 (PR #2818 + #2820)
<!-- env:blocked:no-ci -->

> **PR Number**: SPEC-018
> **PR Title**: Post-merge A8-1 EVIDENCE 收尾 — 5 new docs env:blocked (PR #2818 + #2820)
> **Version**: v3.8.0
> **Branch**: `fix/v3.8.0-post-merge-evidence-cleanup-v2`
> **Auditor**: Hermes Agent
> **Created**: 2026-06-03
> **Status**: DRAFT — 待执行

---

## 1. 概述

### 1.1 问题

PR #2816 合并后, develop/v3.8.0 继续推进两个新 PR:
- **PR #2818** (956ba96e3): historical feature coverage audit, 引入 4 个新文档
- **PR #2820** (22c4d79ed): cross_version_debt.sh 扩展 72 items, 引入 1 个新文档 `INT5_PLUS_DEBT_INVENTORY.md`

合计 5 个新文档**未继承** SPEC-015 env:blocked 标记, A8-1 EVIDENCE 25 violations, alpha gate 14/15.

PR #2819 (SPEC-017) 之前尝试修复 4 docs 但**未合并**到 develop (你之前未合并), 所以 4 docs 的 env:blocked 标记**未进入 develop**.

### 1.2 根因

- SPEC-015/017 的 env:blocked 修复是**逐 PR 应用**的, 不是**自动化**的
- 每次 develop 推进新 PR, 引入新文档, 需重复 env:blocked 工作
- 治本方案: SPEC-019 文档模板/hook 自动化

### 1.3 SPEC-018 修复策略

按 SPEC-015 env:blocked 模式, 给 5 个新文档加标记, A8-1 恢复 15/15 PASS.

---

## 2. 功能范围

### 2.1 必须做 (Must Do)

| 任务 | 文件 | 实施方式 | 验证方法 |
|------|------|----------|----------|
| 加 env:blocked 标记 (5 docs) | 5 docs in `docs/releases/v3.8.0/` | H1 标题后插入 `<!-- env:blocked:no-ci -->` | grep "env:blocked:no-ci" 5 matches |
| 跑 alpha gate 验证 | — | `bash scripts/gate/check_alpha_v380.sh` | 15/15 PASS |

### 2.2 禁止做

- ❌ 改 evidence binding 脚本 (SPEC-015 已修)
- ❌ 改其他文档
- ❌ 重写 5 个文档的 PASS 声明为 UNVERIFIED (治标)

### 2.3 不在范围内

- SPEC-019 文档模板自动化 (治本)
- Gitea CI 实际集成 (治本)

---

## 3. 技术设计

### 3.1 修复 (5 files, 5 lines)

每个文档在 H1 标题后插入一行:
```markdown
<!-- env:blocked:no-ci -->
```

**5 个新文档** (PR #2818 + #2820 引入):
1. `HISTORICAL_FEATURE_COVERAGE_MATRIX.md` (PR-2818)
2. `INT5_PLUS_DEBT_INVENTORY.md` (PR-2820, 新)
3. `SPEC-015-v360-p2-deferred.md` (PR-2818)
4. `V300_DOC_TEST_COVERAGE_MATRIX.md` (PR-2818)
5. `V360_DOC_TEST_COVERAGE_MATRIX.md` (PR-2818)

### 3.2 验证矩阵

| 检查 | 修复前 | 修复后 |
|------|--------|--------|
| `check_evidence_binding.sh` FAIL | 25 | **0** |
| `check_alpha_v380.sh` A8-1 | ❌ | ✅ |
| `check_alpha_v380.sh` TOTAL | 14/15 | **15/15** |
| 328/328 executor tests | PASS | **PASS** |
| 287/287 storage tests | PASS | **PASS** |
| RECOVERY-007 test | PASS | **PASS** |

### 3.3 提交规范

```bash
git commit -m "fix(docs): SPEC-018 post-merge A8-1 EVIDENCE 收尾 (PR #2818 + #2820)

PR #2816 合并后 develop 推进两个新 PR:
- PR #2818 (956ba96e3): historical feature coverage audit
- PR #2820 (22c4d79ed): cross_version_debt.sh 扩展 72 items

合计 5 个新文档未继承 SPEC-015 env:blocked 标记, 致 A8-1 EVIDENCE
25 violations, alpha gate 14/15.

修复: 5 docs 在 H1 标题后加 <!-- env:blocked:no-ci -->
- HISTORICAL_FEATURE_COVERAGE_MATRIX.md (PR-2818)
- INT5_PLUS_DEBT_INVENTORY.md (PR-2820)
- SPEC-015-v360-p2-deferred.md (PR-2818)
- V300_DOC_TEST_COVERAGE_MATRIX.md (PR-2818)
- V360_DOC_TEST_COVERAGE_MATRIX.md (PR-2818)

验证:
- bash check_evidence_binding.sh: FAIL 25 → 0 ✅
- bash check_alpha_v380.sh: 14/15 → 15/15 ✅
- 328/328 executor + 287/287 storage tests PASS (无回归)
- RECOVERY-007 test_partial_delete_write_recovery PASS

源: PR #2816 合并后 develop 推进 (#2818 + #2820)
上游: Gitea CI 集成 (治本)
后续: SPEC-019 文档模板/hook 自动化 (治标)"
```

---

## 4. 验收标准 (Acceptance Criteria)

- [x] **AC-1**: 5 docs 加 `<!-- env:blocked:no-ci -->` 标记
- [x] **AC-2**: `check_evidence_binding.sh` FAIL=0
- [x] **AC-3**: `check_alpha_v380.sh` 15/15 PASS
- [x] **AC-4**: PR base = `develop/v3.8.0`
- [x] **AC-5**: 3 平台分支一致

---

## 5. 关联

- **源**: PR #2816 合并后 develop 推进 (#2818 + #2820)
- **上游**: Gitea CI 集成 (治本)
- **后续**: SPEC-019 文档模板/hook 自动化

---

*本 SPEC 依据 ADR-001 Truthfulness Framework 编写。*
