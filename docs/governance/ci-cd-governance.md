# CI/CD 门禁治理规范

> 规范版本: v1.0.0
> 生效日期: 2025-05-27
> 规范来源: `gate_spec_v350.md` §六

## 一、治理目标

确保所有 CI/CD 配置（workflow 文件、分支保护、门禁脚本）始终与当前活跃开发分支保持同步，防止因配置漂移导致的门禁失效。

## 二、强制规则

### 规则 1: check_gate_sync.sh 必须通过

**任何向 `develop/v3.5.0` 或 `rc/v3.5.0` 分支的合并（merge），必须先通过 `scripts/gate/check_gate_sync.sh` 的全部 6 项检查。**

```bash
# 在本地验证
bash scripts/gate/check_gate_sync.sh

# 预期输出: Result: ✅ PASSED
```

### 规则 2: Gitea Actions 为唯一 CI 引擎

- 所有 CI 工作流文件必须放在 `.gitea/workflows/` 目录（**不是** `.github/workflows/`）
- 不得使用 GitHub Actions；所有配置针对 Gitea Actions 语法
- Gitea Actions 的 status check context 格式为: `workflow_filename/job_key`

### 规则 3: 分支保护必须与工作流匹配

对于 `develop/v3.5.0` 分支，branch protection 的 `status_check_contexts` 必须覆盖以下工作流的所有非 `workflow_dispatch` job：

| 工作流 | 必须覆盖的 Job |
|--------|--------------|
| `ci.yml` | `lint-build`, `test`, `postcheck`, `formal-verification`, `sqlite-diff` |
| `layered-ci.yml` | `l0-build`, `l1-unit-tests`, `l2-incremental-tests`, `l3-full-gate` |
| `pr-gate.yml` | `pr-gate`, `pr-gate-integration` |

## 三、check_gate_sync.sh 六项检查

| # | 检查项 | 失败后果 |
|---|--------|---------|
| 1 | Workflow 分支触发条件是否覆盖当前活跃分支 | 🔴 分支保护无法触发 |
| 2 | 无过时 workflow 引用已废弃分支 | ⚠️ 浪费 CI 资源 |
| 3 | Branch protection 的 status_check_contexts 非空 | 🔴 无法防止强制推送 |
| 4 | coverage_threshold.toml 覆盖所有必须 crate | 🔴 覆盖率门禁失效 |
| 5 | Gate 脚本（alpha/beta/ga/rc v350）存在 | 🔴 门禁无法运行 |
| 6 | CI job contexts 与 branch protection contexts 完全匹配 | 🔴 门禁不完整 |

## 四、配置变更流程

```
┌─────────────────────────────────────────────┐
│ 1. 修改任何 .gitea/workflows/*.yml 文件      │
└─────────────────┬───────────────────────────┘
                  ▼
┌─────────────────────────────────────────────┐
│ 2. 运行 check_gate_sync.sh 验证             │
│    bash scripts/gate/check_gate_sync.sh     │
└─────────────────┬───────────────────────────┘
                  ▼
         [是否通过？]
          ├─ YES → 继续提交
          └─ NO  → 修复后重验
                  │
                  ▼
┌─────────────────────────────────────────────┐
│ 3. git add → git commit → git push          │
│    → 创建 PR → 必须通过 check_gate_sync.sh   │
└─────────────────┬───────────────────────────┘
                  ▼
┌─────────────────────────────────────────────┐
│ 4. PR 合并前，Gitea branch protection       │
│    必须包含所有相关 job contexts            │
└─────────────────────────────────────────────┘
```

## 五、违反处理

| 违规类型 | 处理方式 |
|---------|---------|
| 合并了未通过 check_gate_sync.sh 的 PR | 回滚合并，重新走流程 |
| 分支保护 contexts 与实际 CI 不匹配 | 立即重建分支保护 |
| 使用 `.github/workflows/` 而非 `.gitea/workflows/` | 迁移到 Gitea Actions 并删除 GitHub workflow |

## 六、相关文件

| 文件 | 作用 |
|------|------|
| `scripts/gate/check_gate_sync.sh` | 门禁同步性检查脚本 |
| `scripts/gate/coverage_threshold.toml` | 覆盖率阈值配置 |
| `.gitea/workflows/*.yml` | Gitea Actions 工作流 |
| `docs/governance/gate_spec_v350.md` | v3.5.0 门禁规范 |
| `docs/releases/v3.5.0/TEST_PLAN.md` | v3.5.0 测试计划 |

## 七、版本约定

| 版本 | 活跃分支 | 门禁脚本前缀 |
|------|---------|-------------|
| v3.4.0 | `develop/v3.4.0` | `check_*_v340.sh` |
| v3.5.0 | `develop/v3.5.0` | `check_*_v350.sh` |
| v3.6.0 | `develop/v3.6.0` | `check_*_v360.sh` |

每次创建新版本门禁时，同步更新本文件的版本约定表。
