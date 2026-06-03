# SPEC-001 — B4 Format Truthfulness 修复

> **PR Number**: SPEC-001 (独立修复，不属于 PR-800/830 系列)
> **PR Title**: 修复 B4 Format Truthfulness 违规 — 消除 auto-fix 掩盖
> **Version**: v3.8.0
> **Branch**: `fix/v3.8.0-b4-format-truthfulness` (从 `33c418af7` 切出)
> **Auditor**: Hermes Agent
> **Created**: 2026-06-02
> **Status**: DRAFT — 待执行

---

## 1. 概述

### 1.1 问题

`cargo fmt --all -- --check` 在 main HEAD (`33c418af7`) 退出码 1，存在 5 个文件 6 处 format 违规。

**这是 v3.8.0 Beta Gate 报告与文档不一致的根因** — 文档声称 `cargo fmt --all -- --check` PASS（EXIT:0），但实际 EXIT:1。之前的工作流使用 `cargo fmt --all` 自动修复再检查，**掩盖了 HEAD 本身的格式问题**，违反 ADR-001 Truthfulness Framework。

### 1.2 当前状态（基线 33c418af7）

| # | 文件 | 违规性质 | 行号 |
|---|------|----------|------|
| 1 | `crates/executor/src/execution/context.rs` | 末尾多空行 | 100 |
| 2 | `crates/executor/src/execution/drift.rs` | match arm 格式化 | 140 |
| 3 | `crates/executor/src/execution/engine.rs` | 末尾多空行 | 9 |
| 4 | `crates/executor/src/execution/facade.rs` | use 排序 | 1 |
| 5 | `crates/executor/src/execution/facade.rs` | 末尾多空行 | 26 |
| 6 | `crates/executor/src/execution/result.rs` | 末尾多空行 | 29 |
| 7 | `crates/executor/src/merge.rs` | 函数调用换行 | 96 |
| 8 | `crates/executor/src/merge.rs` | fn body 换行 | 273 |
| 9 | `crates/executor/src/merge.rs` | 末尾空行 | 590 |

**核实命令**:
```bash
cargo fmt --all -- --check
echo "EXIT: $?"  # 期望: 0
```

**当前 EXIT**: 1

### 1.3 违规根因

历史工作流 (见 `docs/releases/v3.8.0/LEGACY_FIXES_VERIFICATION_REPORT.md` §4.2 B4):
```
$ cargo fmt --all -- --check
EXIT: 1  (违规: merge.rs x4, engine_builder.rs x1, execution_engine.rs x2)

$ cargo fmt --all && cargo fmt --all -- --check
EXIT: 0  (掩盖了 HEAD 本身的格式问题)
```

根因: 提交者使用 `cargo fmt --all` 自动修复后检查，未意识到新文件 (33c418af7 后新增的执行模块测试) 又引入了 format 违规。

---

## 2. 功能范围

### 2.1 必须做 (Must Do)

| 任务 | 实施方式 | 验证方法 |
|------|----------|----------|
| 修复 5 个文件 6 处 format 违规 | `cargo fmt --all` (不修改) | `cargo fmt --all -- --check` → EXIT 0 |
| 提交修复（保留 auto-fix 痕迹透明）| 单个 commit `style: cargo fmt` | git diff 显示 9 个 hunk 改动 |
| 更新 LEGACY_FIXES_VERIFICATION_REPORT.md §4.2 B4 状态 | 标记 B4 = ✅ FIXED | 文档内 grep |

**预存在缺陷声明** (本 SPEC 不修复):
- `crates/executor/src/merge.rs:11` 存在 unused import `SqlError` (main HEAD 33c418af7 引入)
- `crates/executor/src/merge.rs:457` 存在 unused import `super::*` (mod tests 块)
- 这两个 pre-existing clippy 警告与 format 修复**无关**, 将通过 **SPEC-008 (Pre-existing Clippy Warnings)** 单独处理

### 2.2 禁止做 (Must NOT Do)

- ❌ 修改任何代码逻辑（仅 format 修复）
- ❌ 强制 push 到 main / develop/v3.8.0
- ❌ 修改 BETA_GATE_CONTRACT.md（避免文档与执行状态不一致重演）
- ❌ 在 SPEC 中添加未经验证的状态声明

### 2.3 不在范围内 (Out of Scope)

- 修复 B4 之外的任何遗留问题（G1~G5、INT-2/3/4、AV-001~007）→ 各自 SPEC
- 改进 CI 自动化（Pre-commit hook 强制 fmt check）→ 单独 SPEC

---

## 3. 技术设计

### 3.1 修复策略

**最小修改原则**: 仅调用 `cargo fmt --all` 修复，不手动编辑。

**风险评估**:
- `cargo fmt` 不会改变语义
- 仅调整空白、缩进、use 排序
- 9 个 hunk 改动均可通过 `git diff` 审查

### 3.2 验证矩阵

| 验证项 | 命令 | 通过条件 |
|--------|------|----------|
| Format check | `cargo fmt --all -- --check` | EXIT 0 |
| 编译（保留全特性）| `cargo build --all-features` | "Finished" 无 error |
| Clippy | `cargo clippy --all-features -- -D warnings` | 0 warnings |
| 单元测试 | `cargo test -p sqlrustgo-executor --lib` | 327/327 PASS（不变）|
| 单元测试 | `cargo test -p sqlrustgo-storage --lib` | 283/283 PASS（不变）|

### 3.3 提交规范

```bash
git add -A
git commit -m "style(executor): cargo fmt --all — fix B4 format truthfulness

修复 5 个文件 6 处 format 违规:
- context.rs / engine.rs / result.rs / facade.rs: 末尾多空行
- drift.rs: match arm 格式化
- facade.rs: use 排序
- merge.rs: 函数调用/fn body 换行

SPEC-001: B4 Format Truthfulness
基线: 33c418af7 → 验证后 EXIT 0"
```

---

## 4. 实施计划

### 4.1 执行步骤

| Step | 操作 | 工具 |
|------|------|------|
| 1 | 在 worktree 内执行 `cargo fmt --all` | terminal |
| 2 | 验证 `cargo fmt --all -- --check` EXIT 0 | terminal |
| 3 | 运行 clippy + executor/storage 单元测试 | terminal |
| 4 | `git add -A && git commit` | terminal |
| 5 | 推送分支到 gitea | `git push gitea fix/v3.8.0-b4-format-truthfulness` |
| 6 | 创建 Gitea PR (base: `develop/v3.8.0`) | curl POST API |
| 7 | 报告 PR 链接 + commit SHA | — |

### 4.2 时间表

| 阶段 | 预计耗时 |
|------|----------|
| 修复 | < 1 min |
| 验证 | 3-5 min（build + clippy + test）|
| 提交 + 推送 | < 1 min |
| 创建 PR | < 1 min |
| **总计** | **< 10 min** |

---

## 5. 验收标准 (Acceptance Criteria)

### 5.1 必须全部满足

- [ ] **AC-1**: `cargo fmt --all -- --check` 在 worktree 内 EXIT 0
- [ ] **AC-2**: 改动仅为 format 修复（无语义变化），`git diff --stat` 仅显示 5 个文件
- [ ] **AC-3 (调整)**: `cargo clippy -p sqlrustgo-executor` 警告数量与基线 33c418af7 相同 (2 个 pre-existing unused imports, 将在 SPEC-008 处理), **未引入新警告**
- [ ] **AC-4**: `cargo test -p sqlrustgo-executor --lib` 仍 327/327 PASS
- [ ] **AC-5**: `cargo test -p sqlrustgo-storage --lib` 仍 283/283 PASS
- [ ] **AC-6**: PR base = `develop/v3.8.0`（不直推 main）
- [ ] **AC-7**: 推送完成后 3 平台分支一致 (gitea/gitcode/gitee)

### 5.2 完成定义 (DoD)

- [ ] Commit 已推送到 gitea
- [ ] Gitea PR 已创建（ID 已记录）
- [ ] 验证日志已保存到 `artifacts/gate/v3.8.0/SPEC-001-verification.log`
- [ ] 本 SPEC 文件随代码一并提交（同一 commit 或单独 commit）

---

## 6. 风险与缓解

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| `cargo fmt` 误改语义 | 极低 | 低 | 仅 format 不会改 AST；测试覆盖已锁定 |
| Push 失败（Gitea 服务挂）| 中 | 中 | 等待恢复后重试；不阻塞后续 SPEC |
| CI 检查发现新问题 | 中 | 低 | 在 PR 评论中说明"仅 format 修复，独立评审" |

---

## 7. 关联

- **上游**: `docs/releases/v3.8.0/LEGACY_FIXES_VERIFICATION_REPORT.md` §4.2 B4
- **下游**: `docs/releases/v3.8.0/BETA_GATE_REPORT.md` (后续更新 B4 状态)
- **Gitea Issue**: 无单独 Issue（属于 LEGACY_FIXES_VERIFICATION_REPORT 中已记录的已知缺口）
- **后续**: SPEC-002 (PR-830F Lifecycle) — 紧随本次完成

---

*本 SPEC 依据 ADR-001 Truthfulness Framework 编写，所有状态变更必须基于实际执行证据。*
