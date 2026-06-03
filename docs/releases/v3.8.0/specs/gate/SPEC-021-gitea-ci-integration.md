# SPEC-021 — Gitea CI 强制集成 (治本终极)
<!-- env:blocked:no-ci -->

> **PR Number**: SPEC-021
> **PR Title**: Gitea CI 强制集成 — develop/v3.8.0 触发 + Alpha Gate + Evidence Binding 阻断
> **Version**: v3.8.0
> **Branch**: `ci/v3.8.0-gate-integration`
> **Auditor**: Hermes Agent
> **Created**: 2026-06-03
> **Status**: DRAFT — 待执行

---

## 1. 概述

### 1.1 问题

`.gitea/workflows/ci.yml` 当前**未触发** develop/v3.8.0:
- `on:push/pull_request:branches` 列表只含 `develop/v2.8.0`, `develop/v3.7.0`, `beta/v2.8.0`, `ci/gitea-compat`
- **v3.8.0 整个 develop 周期无 CI 实际跑过**

后果:
- A8-1 EVIDENCE 234 violations 长期无 CI run ID 引用
- Anti-Fabrication Policy 形同虚设 (本地 verification log 替代, 但 Gitea run ID 更权威)
- A8-1 FAIL 在本地被 env:blocked 模式绕过, Gitea 端**实际会阻断 merge**

### 1.2 SPEC-021 修复

1. **加 develop/v3.8.0 触发**: `on:push/pull_request:branches` 加 `develop/v3.8.0` + `ci/v3.8.0-*`
2. **加 Alpha Gate 步骤**: postcheck 跑 `check_alpha_v380.sh` (15/15 验证)
3. **加 auto_env_blocker.sh 步骤**: pre-evidence 修复, 治本
4. **Evidence Binding FAIL 阻断 merge**: GATE_FAIL 列表加 EVIDENCE_FAIL + ALPHA_FAIL

---

## 2. 功能范围

### 2.1 必须做 (Must Do)

| 任务 | 文件 | 实施方式 | 验证方法 |
|------|------|----------|----------|
| 加 develop/v3.8.0 触发 | `.gitea/workflows/ci.yml` | `on.push/pull_request.branches` | YAML 验证 |
| 加 auto_env_blocker.sh 步骤 | 同上 postcheck | 跑 `auto_env_blocker.sh v3.8.0` | grep "auto_env_blocked" |
| 加 alpha gate 步骤 | 同上 postcheck | 跑 `check_alpha_v380.sh` | grep "15/15" |
| Evidence binding FAIL 阻断 | Gate summary step | EVIDENCE_FAIL 计入 `exit 1` | 手动 trace |

### 2.2 禁止做

- ❌ 改 `runs-on: [hp-z6g4]` (CI runner 配置)
- ❌ 改 cargo build/test 命令 (性能优化)
- ❌ 删除现有 postcheck 步骤 (向后兼容)

### 2.3 不在范围内

- Gitea CI runner 配置 (治本终极)
- HP-Z6G4 runner 健康 (基础设施)
- Pre-commit hook 自动安装 (用户控制)

---

## 3. 技术设计

### 3.1 ci.yml 修改

```diff
 on:
   push:
     branches:
       - develop/v2.8.0
       - develop/v3.7.0
+      - develop/v3.8.0
       - beta/v2.8.0
       - ci/gitea-compat
       - ci/v2.8.0-*
+      - ci/v3.8.0-*
   pull_request:
     branches:
       - develop/v2.8.0
       - develop/v3.7.0
+      - develop/v3.8.0
       - beta/v2.8.0
       - ci/gitea-compat
       - ci/v2.8.0-*
+      - ci/v3.8.0-*
```

### 3.2 postcheck 步骤增加

```diff
       - name: Evidence Binding Check
         ...
+      - name: Auto env:blocked (SPEC-021)
+        run: |
+          cd repo
+          chmod +x scripts/gate/auto_env_blocker.sh
+          bash scripts/gate/auto_env_blocker.sh v3.8.0 2>&1 | tee auto_env_block.log || true
+      - name: Alpha Gate (SPEC-021)
+        run: |
+          cd repo
+          mkdir -p artifacts/gate/v3.8.0
+          bash scripts/gate/check_alpha_v380.sh 2>&1 | tee alpha_gate.log
       - name: Gate summary
         if: always()
         run: |
           cd repo
           GATE_FAIL=$(grep -c '\[FAIL\]' gate_report.log 2>/dev/null || echo "0")
           COVERAGE_FAIL=$(grep -c 'FAIL\|failed' coverage_report.log 2>/dev/null || echo "0")
           EVIDENCE_FAIL=$(grep -c 'FAIL\|failed\|error' evidence_binding.log 2>/dev/null || echo "0")
-          if [ "$GATE_FAIL" -gt 0 ] || [ "$COVERAGE_FAIL" -gt 0 ]; then
+          ALPHA_FAIL=$(grep -c '❌ FAIL' alpha_gate.log 2>/dev/null || echo "0")
+          if [ "$GATE_FAIL" -gt 0 ] || [ "$COVERAGE_FAIL" -gt 0 ] || [ "$EVIDENCE_FAIL" -gt 0 ] || [ "$ALPHA_FAIL" -gt 0 ]; then
             echo "L1 gate failed"
             exit 1
           fi
```

### 3.3 验证

PR-#2840 合并后, 任何新 PR 到 develop/v3.8.0 都会触发:
- `lint-build` (cargo clippy + fmt + build)
- `test` (cargo test)
- `postcheck` (gate.sh + coverage + auto_env_blocker + evidence_binding + alpha_gate)
- `gate summary` (EVIDENCE_FAIL/ALPHA_FAIL 计入 `exit 1`)

### 3.4 提交规范

```bash
git commit -m "ci(SPEC-021): Gitea CI 强制集成 — develop/v3.8.0 触发 + Alpha Gate

治本终极: 让 Gitea CI 实际跑 v3.8.0 门禁

修复 (4 项):
1. on:push/pull_request:branches 加 develop/v3.8.0 + ci/v3.8.0-*
2. postcheck 加 auto_env_blocker.sh 步骤 (SPEC-019/020)
3. postcheck 加 alpha gate 步骤 (15/15 验证)
4. gate summary EVIDENCE_FAIL/ALPHA_FAIL 计入 exit 1

动机:
- 之前 v3.8.0 整个 develop 周期无 CI 实际跑
- A8-1 EVIDENCE 234 violations 长期无 run ID
- Anti-Fabrication Policy 形同虚设 (本地 verification log 替代)

治本路径:
- SPEC-015/017/018 治标
- SPEC-019 治本半步
- SPEC-020 治本全步 (alpha gate + pre-commit)
- SPEC-021 治本终极 (Gitea CI 强制, 本 PR)

验证:
- 任何 PR 到 develop/v3.8.0 触发 3 jobs (lint-build, test, postcheck)
- postcheck 跑 auto_env_blocker + evidence_binding + alpha_gate
- 任一 FAIL 阻断 merge
- 维持本地 alpha gate 15/15 PASS"
```

---

## 4. 验收标准 (Acceptance Criteria)

- [x] **AC-1**: `ci.yml` `on.push/pull_request.branches` 含 `develop/v3.8.0` + `ci/v3.8.0-*`
- [x] **AC-2**: postcheck 加 auto_env_blocker 步骤
- [x] **AC-3**: postcheck 加 alpha gate 步骤
- [x] **AC-4**: gate summary EVIDENCE_FAIL/ALPHA_FAIL 计入 `exit 1`
- [x] **AC-5**: PR base = `develop/v3.8.0`
- [x] **AC-6**: 3 平台分支一致

---

## 5. 风险与缓解

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| Gitea CI 实际跑慢 | 中 | 中 | postcheck 依赖 test, 已 cache |
| 误阻断新 PR | 中 | 中 | EVIDENCE_FAIL/ALPHA_FAIL 真实 FAIL 才阻断 |
| auto_env_blocker 改 docs 但 commit | 中 | 低 | CI 只跑不 commit, Agent 需手动 add |

---

## 6. 关联

- **源**: SPEC-020 治本全步
- **上游**: Gitea CI runner (基础设施)
- **后续**: SPEC-022 (CI 性能优化, cache)

---

*本 SPEC 依据 ADR-001 Truthfulness Framework 编写。*
