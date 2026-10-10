# IAIA-410 Formalization Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.
> **source_agent:** codex
> **source_run:** `iaia-410-formalization-20261010`
> **timestamp:** `2026-10-10T00:24:00+08:00`
> **evidence_ref:** `docs/plans/2026-10-10-iaia-410-formalization-design.md@a65f87ceed`
> **evidence_hash:** `sha256:97490c7ed1c08c2ba46a193860a71c1d363ec5af2474c884e6dd82879b676dc7`
> **conflict_resolution:** none

**Goal:** 建立 IAIA-410 正式审计 SSOT、可验收的 Gitea Issue DAG 和可复现证据契约。

**Architecture:** 以稳定的 `IAIA_410_SSOT.md` 定义审计协议，以 Gitea Master/子 Issue 管理执行依赖，以不可变 run manifest 和 finding schema 承载证据；历史计划不再承担实时状态 SSOT。所有结论遵守 ADR-001、AFP、ADR-008 和 ADR-014，不从文档声明推导 PASS。

**Tech Stack:** Markdown、YAML schema 示例、Gitea/tea CLI、Git worktree、仓库文档 gate。

---

### Task 1: 建立 Gitea Issue DAG

**Files:**
- Reference: `docs/plans/2026-10-10-iaia-410-formalization-design.md`
- Reference: `docs/releases/v4.1.0/IAIA_410_AUDIT_PLAN.md`

**Step 1:** 创建 A01–A12 子 Issue；每条包含范围、依赖、交付物、证据要求和关闭条件。

**Step 2:** 创建 IAIA-410 Master Issue，链接全部子 Issue 和已有 finding Issue。

**Step 3:** 用 `tea issues` 逐条查询编号、标题和 open 状态，保存核验输出。

**Step 4:** A01–A10 在提交证据后进入 `evidence_ready` 并保持 open；A11 verdict 完成后再按关闭验证流程逐项关闭。

### Task 2: 编写正式审计 SSOT

**Files:**
- Create: `docs/releases/v4.1.0/IAIA_410_SSOT.md`
- Modify: `docs/releases/v4.1.0/IAIA_410_AUDIT_PLAN.md`

**Step 1:** 写入 SSOT 元数据、权威顺序、范围矩阵和术语。

**Step 2:** 写入阶段 DAG、角色分离、六类审计工作流和一票否决规则。

**Step 3:** 写入 run manifest、finding、独立复核和冲突裁决 schema。

**Step 4:** 写入 Issue DAG、交付物、关闭协议和与 `STAGE.yaml` 的关系。

**Step 5:** 在旧计划顶部增加迁移说明，仅指向 SSOT，不改写历史执行内容。

### Task 3: 文档核验

**Files:**
- Verify: `docs/releases/v4.1.0/IAIA_410_SSOT.md`
- Verify: `docs/releases/v4.1.0/IAIA_410_AUDIT_PLAN.md`

**Step 1:** 运行 `bash scripts/gate/check_docs_links.sh`，记录退出码。

**Step 2:** 运行 `bash scripts/gate/check_docs_consistency.sh`，记录退出码；如为既有失败，区分基线与本次变更。

**Step 3:** 检查 `git diff --check`、链接路径、Issue URL 和 provenance 字段。

**Step 4:** 请求独立 reviewer 审查 Issue 完整性、SSOT 权威边界和证据契约。

### Task 4: 提交与交付

**Files:**
- Commit: all files above

**Step 1:** 运行 `git status --short` 和 `git diff --stat`，确认无非预期修改。

**Step 2:** 提交文档变更，commit message 关联 Master Issue。

**Step 3:** 推送 `codex/docs-iaia-410-ssot` 到 Gitea 并创建面向 `develop/v4.1.0` 的 PR。

**Step 4:** 在 Master Issue 中补充 PR、commit、验证命令、退出码和 evidence hash；不在 PR 合并前关闭任何 Issue。
