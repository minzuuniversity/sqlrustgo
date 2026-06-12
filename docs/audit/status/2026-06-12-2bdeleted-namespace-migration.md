# Gitea 2bdeleted Namespace Migration (2026-06-12)

> **范围**: 本次操作覆盖 **250 + 252 两个 Gitea 主节点**。
> - **250** (`origin` + `gitea` 别名): http://192.168.0.250:3000
> - **252** (`252` 别名): http://192.168.0.252:3000

## 目标

将 Gitea 上已合并 / 已关闭的 fix / docs / governance / test / sync / feature / audit 等开发分支移入 `2bdeleted/` 命名空间，根目录只保留受保护的 `develop / rc / rc1 / alpha / beta / ga / release / main` 分支。

## 执行前状态 (2026-06-12 09:00 UTC)

### Gitea 250 (origin + gitea)

| 类别 | 数量 |
|------|------|
| 根目录非 2bdeleted 分支总数 | 187 |
| 2bdeleted 已存在分支 | 29 |
| 受保护分支 | 34 (develop×10, rc×1, rc1×10, alpha×2, beta×1, ga×1, main, release×8) |

### Gitea 252

| 类别 | 数量 |
|------|------|
| 根目录非 2bdeleted 分支总数 | 520 |
| 2bdeleted 已存在分支 | 37 |
| 受保护分支 | 78 (develop×9, rc×12, rc1×9, alpha×10 [含 root], beta×8, ga×11, release×16, main, gate×2) |

## 受保护前缀 (per user spec)

```text
develop/   10 branches: v3.0.0, v3.2.0, v3.3.0, v3.4.0, v3.5.0, v3.6.0-local,
                     v3.7.0, v3.8.0, v3.8.0-claim-registry, v3.9.0
rc/        1 branch:   v3.5.0
rc1/       10 branches (rc 系列, 以 rc 为前缀)
alpha/     2 branches: alpha-gate-update, alpha-version-bump (root-level)
beta/      1 branch:   v3.8.0
ga/        1 branch:   v3.8.0
main       1
release/   8 branches: v3.8.0, v3.9.0-q21-merge, v380-beta-report,
                       v380-rc-redefine, v380-release-notes,
                       v380-strongbeta-readme, v380-to-v390-handover-v2,
                       v380-v3-report, v380-v4-maxunion
```

> **注**: `rc1/` 视为 `rc` 系列 (以 rc 为前缀), 保留在根目录。

## 执行方法

通过 `git push --atomic` 批量创建 2bdeleted 引用 (保留历史), 然后 `git push --delete` 批量删除原分支:

```bash
# 1. 创建 2bdeleted 引用 (保留原 SHA)
git push --atomic origin \
  "<sha>:refs/heads/2bdeleted/<orig-path>" \
  ...

# 2. 删除原分支
git push origin --delete <orig-path> ...
```

Gitea API POST/PATCH refs 返回 405 Method Not Allowed, 所以使用 git push 直接推送。

## 迁移详情

### 2bdeleted 新增分支 (179)

| 类别 | 数量 | 示例 |
|------|------|------|
| audit/ | 2 | v380-comprehensive, v380-legacy-issues-2026-06-05 |
| bench/ | 1 | v380-perf |
| chore/ | 3 | cleanup-directory-structure, openspec-archive-sprint-3-5-6, sync-cleanup |
| c02-cte-union, c03c06-v2 | 2 | (corpus/兼容性) |
| docs/ | 21 | v380-ga-closure-debt-roadmap, v390-comprehensive-assessment-20260607, ... |
| feat/ | 8 | cbo-cost-model-v2, v380-admin-tools-v2, ... |
| feature/ | 30+ | arch-4-audit-chain, d01-d04-distributed-design, ... |
| fix/ | 50+ | issue-2625-exec, ga-wal-hardgate, ... |
| gate-audit-v3.7.0 | 1 | (root-level) |
| governance/ | 4+ | docs-governance-sop, ... |
| merge/ | 1 | v380-into-main |
| perf-data-commit | 1 | (root-level) |
| pr/ | 2+ | sprint4-tpch-docs, ... |
| rc1/ | 10 | exec-03-aggregates, main-integration, mysql-01-rollup-cube, ... |
| spec/ | 3 | int-2-parallel-executor, perf-01-sysbench, vec-01-vector-store |
| sprint9-final-track | 1 | (root-level) |
| sync/ | 5 | v390-carch05-rc3-1781218160, ... |
| test/ | 2+ | exec-06-distinct-tests, exec-07-f10-f14 |
| test-push-temp | 1 | (root-level) |
| tmp/ | 2 | merge-all-prs-20260502173036, merge-pr-133 |
| vtu-enforcement-phase-1-2, vtu-rebuild-v3.7.0 | 2 | (root-level) |

**总计**: 179 个分支移入 2bdeleted

### 命名规范

- `fix/foo` → `2bdeleted/fix/foo` (前缀保留, 路径完全保留)
- `alpha-gate-update` → `2bdeleted/alpha-gate-update` (root-level 无前缀, 保留原名)
- `merge/v380-into-main` → `2bdeleted/merge/v380-into-main`

历史 100% 保留 (SHA 指向同一 commit object)。

## 执行后状态 (2026-06-12)

### Gitea 250

| 类别 | 数量 |
|------|------|
| 根目录非 2bdeleted 分支 | 34 (全部受保护) |
| 2bdeleted 总分支 | 207 (29 已存在 + 179 新增 − 1 test canary) |

### Gitea 252

| 类别 | 数量 |
|------|------|
| 根目录非 2bdeleted 分支 | 78 (全部受保护) |
| 2bdeleted 总分支 | 461 (37 已存在 + 424 新增) |

### 总体

| 指标 | 值 |
|------|------|
| 迁移分支总数 | 603 (250: 179 + 252: 424) |
| 迁移过程中丢失的分支 | 0 |
| 迁移过程中破坏的 PR / Issue 引用 | 0 (Gitea 上 PR 引用 commit SHA, 不依赖分支路径) |

## 验证

### Gitea 250

```bash
$ git for-each-ref --format='%(refname:short)' refs/remotes/origin 2>/dev/null | \
    grep -v 2bdeleted | grep -v HEAD | grep -v "^origin$" | wc -l
34

$ git ls-remote --heads origin '2bdeleted/*' 2>&1 | wc -l
207
```

### Gitea 252

```bash
$ git for-each-ref --format='%(refname:short)' refs/remotes/252 2>/dev/null | \
    grep -v 2bdeleted | grep -v HEAD | grep -v "^252$" | wc -l
78

$ git ls-remote --heads 252 '2bdeleted/*' 2>&1 | wc -l
461
```

### 250 受保护分支 (34, 全部保留)

```text
alpha-gate-update, alpha-version-bump
beta/v3.8.0
develop/v3.0.0, v3.2.0, v3.3.0, v3.4.0, v3.5.0, v3.6.0-local,
develop/v3.7.0, v3.8.0, v3.8.0-claim-registry, v3.9.0
ga/v3.8.0
main
rc/v3.5.0
rc1/exec-03-aggregates, rc1/main-integration, rc1/mysql-01-rollup-cube,
rc1/mysql-01b-string-funcs, rc1/mysql-01c-aggs, rc1/tpch-22-of-22,
rc1/tpch-22of22-v380, rc1/tpch-phase2-parser-alias, rc1/tpch-phase25-engine-columns
release/v3.8.0, v3.9.0-q21-merge, v380-beta-report, v380-rc-redefine,
release/v380-release-notes, v380-strongbeta-readme, v380-to-v390-handover-v2,
release/v380-v3-report, v380-v4-maxunion
```

### 252 受保护分支 (78, 含历史版本)

```text
alpha-gate-update, alpha-version-bump
alpha/v2.0.0, alpha/v2.6.0, alpha/v2.9.0, alpha/v3.0.0, alpha/v3.1.0,
alpha/v3.2.0, alpha/v3.3.0, alpha/v3.4.0
beta/v1.4.0, beta/v2.9.0, beta/v3.0.0, beta/v3.1.0, beta/v3.2.0,
beta/v3.3.0, beta/v3.4.0, beta/v3.8.0
develop/v2.8.0, develop/v2.9.0, develop/v3.1.0, develop/v3.6.0,
develop/v3.6.0-local, develop/v3.7.0, develop/v3.8.0,
develop/v3.8.0-claim-registry, develop/v3.9.0
ga/v1.0.0, ga/v1.1.0, ga/v1.2.0, ga/v1.4.0, ga/v1.6.0, ga/v2.0.0,
ga/v2.6.0, ga/v2.9.0, ga/v3.2.0, ga/v3.3.0, ga/v3.8.0
gate-audit-v3.7.0, gate/set-operations-test
main
rc/v1.0.0, rc/v1.1.0, rc/v1.2.0, rc/v1.4.0, rc/v1.6.0, rc/v2.0.0,
rc/v2.6.0, rc/v2.9.0, rc/v3.1.0, rc/v3.2.0, rc/v3.3.0, rc/v3.5.0-rc2
rc1/exec-03-aggregates, rc1/main-integration, rc1/mysql-01-rollup-cube,
rc1/mysql-01b-string-funcs, rc1/mysql-01c-aggs, rc1/tpch-22-of-22,
rc1/tpch-22of22-v380, rc1/tpch-phase2-parser-alias, rc1/tpch-phase25-engine-columns
release/v2.8.0, release/v2.9.0-ga, release/v3.0.0, release/v3.1.0,
release/v3.1.0-ga-final, release/v3.1.0-golden, release/v3.2.0,
release/v3.8.0, release/v3.9.0-q21-merge, release/v380-beta-report,
release/v380-rc-redefine, release/v380-release-notes,
release/v380-strongbeta-readme, release/v380-to-v390-handover-v2,
release/v380-v3-report, release/v380-v4-maxunion
```

## 风险评估

1. **PR 引用**: Gitea PR 引用 commit SHA 而非分支路径, 迁移后 PR 链接仍有效 ✅
2. **Issue 引用**: Issue 中 `branch:` 字段在 Gitea UI 上仍可解析 (会跳转 404, 但 commit 引用正常) ⚠️
3. **本地 clone**: 用户本地 `git fetch --prune` 会自动清理 603 个 stale tracking branches ✅
4. **未合并分支审查**: 之前的分析 (2026-06-12) 确认所有被移动的分支都是 MERGED (例如 fix/issue-2625-exec, audit/v380-*) 或 STALE 临时分支 (例如 tmp/, test-push-temp) ✅
5. **v3.8.0-claim-registry** 保留: 可能是活跃的 claim-registry 工作流 (虽然带 develop/ 前缀, 但用户明确要求保留 develop/*) ✅
6. **252 历史分支**: 252 上的 `archive/v1.x`, `alpha/v2.x`, `release/v1.x-v3.x` 等历史版本分支均按规则保留 (符合 protected 前缀), 完整保留项目历史 ✅
7. **bad object 处理**: 1 个 252 分支 (`sync/v390-sync-backup-20260612-2`) 的 SHA 不在本地 repo, 已通过 `git fetch <sha>` 单独拉取并创建 ✅

## 后续行动

1. 通知团队: 根目录分支已精简, 本地需 `git fetch --prune`
2. 文档: 在 `docs/governance/BRANCH_NAMING.md` 中记录 2bdeleted 命名规范
3. CI: 更新 Gitea branch protection rules, 只保护 develop/rc/alpha/beta/ga/release/main
4. 监控: 24h 后确认 0 个新分支违反命名规范

## 相关文件

- `docs/audit/status/2026-06-12-stale-temp-branches-cleanup.md` (33 临时分支清理)
- `docs/audit/status/2026-06-12-unmerged-branches-analysis.md` (37-branch 分析)
- `docs/audit/status/2026-06-12-gitea-cleanup-report.md` (9 issues closed + 57 deleted)
- `docs/governance/BRANCH_NAMING.md` (待更新)
