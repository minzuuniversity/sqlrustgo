# develop/v3.9.0 分支保护规则配置记录

> **配置日期**: 2026-06-24
> **配置者**: Hermes Agent
> **Gitea 实例**: `http://192.168.0.250:3000/openclaw/sqlrustgo` (backup 250, Z6G4 主 Gitea 物理 down 时启用)
> **API endpoint**: `POST /api/v1/repos/openclaw/sqlrustgo/branch_protections`
> **Rule ID**: priority 10

---

## 1. 背景

develop/v3.9.0 是 v3.9.0 GA 的 active development 分支。在 PR #3308 合并期间观察到:

- 该分支原本**没有任何保护规则**(api 列出的 protected branches 不包含 develop/v3.9.0)
- 任何 collaborator 都能直接 push,绕过 PR 流程
- 任何 collaborator 都能 force-push,潜在丢历史

参考 `docs/governance/RC_BRANCH_PROTECTION.md` 的治理标准,以及 2026-06-07 `docs/releases/v3.9.0/incidents/GITEA_252_OUTAGE_20260607.md` 中的"Gitea outage 期间仍需要 admin 能热修"的特殊场景需求,设计了如下规则。

---

## 2. 规则配置

| 字段 | 值 | 说明 |
|---|---|---|
| `branch_name` | `develop/v3.9.0` | 目标分支 |
| `rule_name` | `develop/v3.9.0` | 规则名 |
| **`enable_push`** | **`false`** | **默认禁止直接 push** |
| `enable_push_whitelist` | (隐含 true) | 配合 push_whitelist_usernames |
| `push_whitelist_usernames` | `["openclaw"]` | 仅 openclaw(本仓库 owner/admin)可例外 |
| `push_whitelist_teams` | `[]` | 不对 team 开放 |
| `push_whitelist_deploy_keys` | `false` | 不对 deploy key 开放 |
| **`enable_force_push`** | **`false`** | **完全禁止 force-push** |
| `enable_force_push_allowlist` | (隐含 true) | 配合 force_push_allowlist_usernames |
| `force_push_allowlist_usernames` | `["openclaw"]` | 预留:future 若需开 force-push,仅 openclaw |
| `enable_merge_whitelist` | `false` | 任何人都可触发 merge button |
| **`required_approvals`** | **`1`** | **至少 1 个审批,强制走 PR 流程** |
| **`block_on_rejected_reviews`** | **`true`** | 被 reviewer reject 的 PR 不能合并 |
| **`block_on_outdated_branch`** | **`true`** | 落后 base 分支的 PR 不能合并,逼 rebase |
| `enable_status_check` | `false` | 当前没有 CI,留 false 避免 lock |
| `status_check_contexts` | `[]` | (留空) |
| `require_signed_commits` | `false` | 未启用 GPG 签名要求 |
| `block_admin_merge_override` | `false` | admin 在紧急情况下仍能绕过审批 |

---

## 3. 设计意图

### 3.1 默认路径:PR + 审批

任何普通 contributor 想要把代码合入 develop/v3.9.0:
1. 创建 feature branch (e.g. `fix/xxx`)
2. push 到自己的分支
3. 开 PR `feature → develop/v3.9.0`
4. 获得至少 1 个 approval
5. PR 通过 merge button 合并

`block_on_rejected_reviews: true` 保证被拒 PR 不能绕过;`block_on_outdated_branch: true` 保证 PR 不会基于过期的 base 分支。

### 3.2 例外路径:openclaw 直接 push (FAST-FORWARD only)

为了应对:
- Z6G4 252 物理 down 期间,主 Gitea 不可达,backup 250 上的 develop/v3.9.0 需要 admin 继续热修
- 紧急 hotfix 不能等 PR 流程

`enable_push: false` 但 `push_whitelist_usernames: ["openclaw"]` 允许:
- openclaw **fast-forward** push 自己的 commit(不会丢历史)
- 其他人必须走 PR 流程

### 3.3 Force-push 永远禁止

**`enable_force_push: false`** 是硬约束,不允许任何人(包括 openclaw)force-push develop/v3.9.0。这保证:
- 历史永远保留
- 已合并的 PR 不能被"擦除"
- 即便 `force_push_allowlist_usernames: ["openclaw"]` 预留了 escape hatch,目前 `enable_force_push_allowlist` 留空,Gitea 拒绝 force-push

如果将来真要启用 force-push(例如 Gitea migrate 期间),需要:
1. `PATCH /api/v1/repos/openclaw/sqlrustgo/branch_protections/{rule_name}` 设 `enable_force_push: true` AND `enable_force_push_allowlist: true`
2. 在事故报告里记录为什么要打开

---

## 4. 验证

### 4.1 普通 collaborator 直接 push — 应被拒

测试用户 `test_no_push`(有 write 权限)尝试 push 到 develop/v3.9.0:

```
remote: error: Not allowed to push to protected branch develop/v3.9.0
! [remote rejected] HEAD -> develop/v3.9.0 (pre-receive hook declined)
```

✅ 通过。

### 4.2 openclaw fast-forward push — 应允许

```
$ git push backup develop/v3.9.0
ok (up-to-date)
```

✅ 通过(测试期间 backup 上 develop/v3.9.0 已经 up-to-date,无新 commit)。

### 4.3 openclaw force-push — 应被拒

```
$ git push --force-with-lease backup HEAD:develop/v3.9.0
remote: error: branch develop/v3.9.0 is protected from force push
! [remote rejected] HEAD -> develop/v3.9.0 (pre-receive hook declined)
```

✅ 通过。

### 4.4 PR 流程仍正常 — PR #3308 合并成功

本次 PR #3308 (`fix(mysql-server): real multi-statement COM_QUERY execution + split_sql_statements`) 在保护规则配置**前**已合并。合并 commit `261c4b862` 已存在于 develop/v3.9.0 (`bbc57fbbf` 之前)。后续 PR 仍可正常走 PR + approval + merge 流程。

---

## 5. 与已有保护规则的对比

| 分支 | `enable_push` | `required_approvals` | `block_on_rejected_reviews` | `block_on_outdated_branch` | `enable_force_push` |
|---|---|---|---|---|---|
| develop/v3.5.0 (旧) | true (allowlist openclaw) | 1 | false | false | true (allowlist openclaw) |
| develop/v3.4.0 (旧) | true (allowlist openclaw) | 1 | false | false | true (allowlist openclaw) |
| **develop/v3.9.0 (新)** | **false (allowlist openclaw)** | **1** | **true** | **true** | **false** |
| main | true (allowlist openclaw) | 0 | false | false | true (allowlist openclaw) |

develop/v3.9.0 比旧版更严格:
- `enable_push: false`(旧 develop: true),更严格 — 普通 contributor **完全不能** push
- `block_on_rejected_reviews: true`(旧: false),防止 PR 被 reviewer 拒绝后绕过
- `block_on_outdated_branch: true`(旧: false),防止 PR 基于过期的 base
- `enable_force_push: false`(旧: true),更安全 — 防止历史丢失

---

## 6. 维护说明

### 6.1 查看当前规则

```bash
curl -s -u "hermes:TOKEN" \
  "http://192.168.0.250:3000/api/v1/repos/openclaw/sqlrustgo/branch_protections" \
  | python3 -m json.tool | grep -A 50 'develop/v3.9.0'
```

### 6.2 更新规则

```bash
curl -s -u "hermes:TOKEN" \
  -X PATCH \
  -H "Content-Type: application/json" \
  -d '{"required_approvals": 2, ...}' \
  "http://192.168.0.250:3000/api/v1/repos/openclaw/sqlrustgo/branch_protections/develop/v3.9.0"
```

### 6.3 删除规则(rollback)

```bash
curl -s -u "hermes:TOKEN" \
  -X DELETE \
  "http://192.168.0.250:3000/api/v1/repos/openclaw/sqlrustgo/branch_protections/develop/v3.9.0"
```

### 6.4 Z6G4 252 物理恢复后

Z6G4 252 主 Gitea 恢复后,需要把这条规则同步到 252。操作:

1. 在 250 上 dump 规则 JSON(curl GET)
2. ssh 到 252,执行同样 POST
3. 验证 252 上 develop/v3.9.0 也有此规则

---

## 7. 参考

- `docs/governance/RC_BRANCH_PROTECTION.md` — RC 分支保护标准(参考)
- `docs/releases/v3.9.0/incidents/GITEA_252_OUTAGE_20260607.md` — Z6G4 outage 报告,admin 紧急 push 场景依据
- `docs/governance/RELEASE_LIFECYCLE.md` — v3.9.0 GA 流程
- Gitea API: `https://docs.gitea.com/api/v1.22/#tag/repository/operation/repo-create-branch-protection`
