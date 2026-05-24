# GMP-Platform v1.3.0 门禁规范

> **版本**: 1.0
> **日期**: 2026-05-24
> **状态**: 生效
> **适用范围**: v1.3.0
> **supersedes**: `gate_spec_v120.md`（v1.2.0版）

---

## 一、门禁体系概述

### 1.1 四级门禁模型

```
L0-Alpha → L0-Beta → L0-RC → L0-GA
```

| 门禁 | 名称 | 阶段目标 | 通过标准 |
|------|------|----------|----------|
| L0-Alpha | Alpha Gate | 核心功能可运行 | 13/13 检查 PASS |
| L0-Beta | Beta Gate | 功能冻结，全链路可用 | 14/14 检查 PASS |
| L0-RC | RC Gate | 发布候选，性能达标 | 12/12 检查 PASS |
| L0-GA | GA Gate | 正式发布 | 全部 PASS |

### 1.2 通过条件

| 条件类型 | 说明 |
|----------|------|
| **全部通过** | 所有门禁检查项必须 PASS |
| **有豁免记录** | 不满足的项必须在 `GATE_EXEMPTIONS.md` 有审批通过的豁免记录 |
| **无阻塞性 OPEN Issue** | Milestone 下的 OPEN Issue 不能阻塞门禁 |

### 1.3 v1.3.0 变更说明

- **移除**: A-F2 Web Dashboard (`server.py`) → 替换为 `gmp-server` 健康检查
- **新增**: A-F3 gmp-event-bus crate 检查
- **新增**: A-F4 gmp-api crate 检查
- **更新**: gmp-retrieve → gmp-cli (二进制名称变更)
- **移除**: 旧版 mobile_cli_demo.py 检查

---

## 二、L0-Alpha Gate (A)

### 2.1 入口条件

- Workspace 全量编译成功
- 所有 crate clippy 零警告
- 单元测试 ≥90% 通过

### 2.2 检查清单

| # | 检查项 | 命令/方法 | 通过标准 | 证据格式 |
|---|--------|----------|---------|---------|
| A1 | Build (workspace) | `cargo build --release --workspace` | 成功 | `{exit_code, time}` |
| A2 | Incremental Build | `cargo build --release -p gmp-cli` | 成功 | `{exit_code}` |
| A3 | Unit Tests | `cargo test --lib` | ≥90% 通过 | `{passed, failed, exit_code}` |
| A4 | Format Check | `cargo fmt --all -- --check` | 无格式错误 | `{diff_count, exit_code}` |
| A5 | Clippy Check | `cargo clippy --all-targets -- -D warnings` | 零警告 | `{warnings, exit_code}` |
| A6 | Python Lint | `python3 -m py_compile scripts/*.py` | 全部通过 | `{files, exit_code}` |
| A-F1 | gmp-cli CLI | `./target/release/gmp --version` | 返回版本 | `{version}` |
| A-F2 | gmp-server | `gmp-server` 二进制存在 | 可执行 | `{path}` |
| A-F3 | gmp-event-bus | `cargo build -p gmp-event-bus` | 成功 | `{exit_code}` |
| A-F4 | gmp-api | `cargo build -p gmp-api` | 成功 | `{exit_code}` |
| A-S1 | Gate Spec | `[ -f docs/governance/gate_spec_v130.md ]` | 文件存在 | `{path}` |
| A-S2 | Development Plan | `[ -f docs/releases/v1.3.0/DEVELOPMENT_PLAN.md ]` | 文件存在 | `{path}` |
| A-S3 | Branch Model | `git branch -r \| grep develop/v1.3.0` | 分支存在 | `{branch}` |

### 2.3 Alpha 通过结果

```
--- A1: Build (full workspace) ---        [PASS]
--- A2: Incremental Build (gmp-cli) ---  [PASS]
--- A3: Unit Tests ---                    [PASS]
--- A4: Format Check ---                  [PASS]
--- A5: Clippy Check ---                  [PASS]
--- A6: Python Lint ---                   [PASS]
--- A-F1: gmp-cli CLI ---                [PASS]
--- A-F2: gmp-server ---                  [PASS]
--- A-F3: gmp-event-bus ---              [PASS]
--- A-F4: gmp-api ---                    [PASS]
--- A-S1: Gate Spec ---                  [PASS]
--- A-S2: Development Plan ---           [PASS]
--- A-S3: Branch Model ---               [PASS]
========================================
  L0-Alpha Gate Results: 13/13 PASSED
```

---

## 三、L0-Beta Gate (B)

### 3.1 入口条件

- L0-Alpha 全部 PASS
- 21-docs 文档完整性检查 PASS
- P0 Issue #124 (Layer 0 管线) 可选完成

### 3.2 检查清单

| # | 检查项 | 命令/方法 | 通过标准 | 证据格式 |
|---|--------|----------|---------|---------|
| B1 | Full Test Suite | `cargo test` | ≥95% 通过 | `{passed, failed}` |
| B2 | 21 Docs Complete | `docs/releases/v1.3.0/DOCS_INDEX.md` | ≥20 项 | `{count}` |
| B3 | Feature Matrix | `FEATURE_MATRIX.md` | 文件存在 | `{path}` |
| B4 | Integration Status | `INTEGRATION_STATUS.md` | 文件存在 | `{path}` |
| B5 | Progress Report | `PROGRESS_REPORT.md` | 文件存在 | `{path}` |
| B6 | Release Gate Checklist | `RELEASE_GATE_CHECKLIST.md` | 文件存在 | `{path}` |
| B7 | Test Plan | `TEST_PLAN.md` | 文件存在 | `{path}` |
| B8 | Layer 0 Pipeline | `FEATURE_MATRIX.md` | 文档化 | `{section}` |
| B9 | gmp-event-bus | `cargo build -p gmp-event-bus` | 成功 | `{exit_code}` |
| B10 | gmp-api | `cargo build -p gmp-api` | 成功 | `{exit_code}` |
| B11 | gmp-retrieval | `cargo build -p gmp-retrieval` | 成功 | `{exit_code}` |
| B12 | gmp-reranker | `cargo build -p gmp-reranker` | 成功 | `{exit_code}` |
| B13 | gmp-llm | `cargo build -p gmp-llm` | 成功 | `{exit_code}` |
| B14 | gmp-eval | `cargo build -p gmp-eval` | 成功 | `{exit_code}` |

### 3.3 Beta 通过结果

```
--- B1: Full Test Suite ---              [PASS]
--- B2: 21 Docs Complete ---            [PASS]
--- B3: Feature Matrix ---               [PASS]
--- B4: Integration Status ---          [PASS]
--- B5: Progress Report ---              [PASS]
--- B6: Release Gate Checklist ---      [PASS]
--- B7: Test Plan ---                   [PASS]
--- B8: Layer 0 Pipeline ---            [PASS]
--- B9: gmp-event-bus ---               [PASS]
--- B10: gmp-api ---                    [PASS]
--- B11: gmp-retrieval ---              [PASS]
--- B12: gmp-reranker ---              [PASS]
--- B13: gmp-llm ---                   [PASS]
--- B14: gmp-eval ---                  [PASS]
========================================
  L0-Beta Gate Results: 14/14 PASSED
```

---

## 四、L0-RC Gate (R)

### 4.1 入口条件

- L0-Beta 全部 PASS（或有豁免记录）
- 场景通过率基线已建立
- 功能冻结

### 4.2 检查清单

| # | 检查项 | 命令/方法 | 通过标准 | 证据格式 |
|---|--------|----------|---------|---------|
| R1 | Release Summary | `[ -f docs/releases/v1.3.0/00-release-summary.md ]` | 文件存在 | `{path}` |
| R2 | User Manual | `[ -f docs/releases/v1.3.0/USER_MANUAL.md ]` | 文件存在 | `{path}` |
| R3 | Deployment Guide | `[ -f docs/releases/v1.3.0/DEPLOYMENT_GUIDE.md ]` | 文件存在 | `{path}` |
| R4 | Audit Report | `[ -f docs/releases/v1.3.0/AUDIT_REPORT.md ]` | 文件存在 | `{path}` |
| R5 | Migration Guide | `[ -f docs/releases/v1.3.0/MIGRATION_GUIDE.md ]` | 文件存在 | `{path}` |
| R6 | CLI Binary | `[ -f target/release/gmp ] \|\| [ -f target/release/gmp-cli ]` | 存在 | `{path}` |
| R7 | Server Binary | `[ -f target/release/gmp-server ]` | 存在 | `{path}` |
| R8 | Cargo Audit | `cargo audit` | 无 HIGH/CRITICAL | `{vulns}` |
| R9 | All Crates Build | `cargo build --release --workspace` | 成功 | `{exit_code}` |
| R10 | LICENSE | `[ -f LICENSE ]` | 文件存在 | `{path}` |
| R11 | README Version | `grep "1.3.0" README.md` | 版本匹配 | `{match}` |
| R12 | CHANGELOG | `[ -f CHANGELOG.md ]` | 文件存在 | `{path}` |

### 4.3 RC 通过结果

```
--- R1: Release Summary ---              [PASS]
--- R2: User Manual ---                 [PASS]
--- R3: Deployment Guide ---            [PASS]
--- R4: Audit Report ---                [PASS]
--- R5: Migration Guide ---             [PASS]
--- R6: CLI Binary ---                 [PASS]
--- R7: Server Binary ---               [PASS]
--- R8: Cargo Audit ---                [PASS]
--- R9: All Crates Build ---           [PASS]
--- R10: LICENSE ---                   [PASS]
--- R11: README Version ---            [PASS]
--- R12: CHANGELOG ---                 [PASS]
========================================
  L0-RC Gate Results: 12/12 PASSED
```

---

## 五、L0-GA Gate (G)

### 5.1 入口条件

- L0-RC 全部 PASS（或有豁免记录）
- 无阻塞性 OPEN Issue

### 5.2 检查清单

| # | 检查项 | 命令/方法 | 通过标准 |
|---|--------|----------|---------|
| G1 | Tag Exists | `git tag -l \| grep "^v1.3.0$"` | Tag 存在 |
| G2 | Tag on HEAD | `git describe --tags --exact-match` | v1.3.0 |
| G3 | Main Branch | `git branch -r \| grep origin/main` | 存在 |
| G4 | Issues Closed | Gitea API: milestone v1.3.0 | 无 P0/P1 OPEN |
| G5 | PRs Merged | Gitea API: milestone v1.3.0 | 无 OPEN PR |
| G6 | Tag Signed | `git tag -v v1.3.0` | Good signature (可选) |
| G7 | Binary Artifacts | `ls target/release/{gmp,gmp-cli,gmp-server}` | ≥2 存在 |
| G8 | Gate Spec | `[ -f docs/governance/gate_spec_v130.md ]` | 存在 |
| G9 | CONTRIBUTING | `[ -f CONTRIBUTING.md ]` | 存在 |
| G10 | Final Build | `cargo build --release --workspace` | 成功 |

### 5.3 GA 通过结果

```
--- G1: Tag Exists ---                  [PASS]
--- G2: Tag on HEAD ---                [PASS]
--- G3: Main Branch ---                [PASS]
--- G4: Issues Closed ---              [PASS]
--- G5: PRs Merged ---                 [PASS]
--- G6: Tag Signed ---                 [PASS/INFO]
--- G7: Binary Artifacts ---           [PASS]
--- G8: Gate Spec ---                  [PASS]
--- G9: CONTRIBUTING ---               [PASS]
--- G10: Final Build ---               [PASS]
========================================
  L0-GA Gate Results: 10/10 PASSED
```

---

## 六、豁免机制

如特殊情况导致无法达标，必须：

1. 在 `docs/governance/GATE_EXEMPTIONS.md` 创建豁免记录
2. 包含：豁免 ID、描述、关联 Issue、审批状态、复审日期
3. P0/P1 问题不可豁免
4. 豁免必须有明确的复审日期（不超过 30 天）

---

## 七、版本化门禁规范索引

| 版本 | 门禁文档 | 状态 |
|------|----------|------|
| v1.0.0 | `gate_spec.md` | 已归档 |
| v1.1.0 | `gate_spec.md` | 已归档 |
| v1.2.0 | `gate_spec_v120.md` | 已归档 |
| v1.3.0 | `gate_spec_v130.md` | 生效 |

---

## 八、门禁脚本与规范同步

脚本路径: `scripts/gate/check_{phase}_v130.sh`

| 脚本 | 对应规范 |
|------|----------|
| `check_alpha_v130.sh` | §二 L0-Alpha (13 项) |
| `check_beta_v130.sh` | §三 L0-Beta (14 项) |
| `check_rc_v130.sh` | §四 L0-RC (12 项) |
| `check_ga_v130.sh` | §五 L0-GA (10 项) |

**动态路径**: 所有 v130 脚本使用 `REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"` 动态解析路径，不再硬编码。
