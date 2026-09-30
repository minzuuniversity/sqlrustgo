# Incident — Gitea Actions CI 全红: v4.x 无触发器 + 无 Runner (2026-10-01)

> **状态**: 诊断完成, 修复待执行 (需 runner 基础设施, 非仓库内可解)
> **影响范围**: `develop/v4.0.0` / `develop/v4.1.0` / `main` 等 2026-09-19 之后的全部提交
> **关联**: PR #4921 合并时无 CI 证据; §5.4.1 "CI 必须通过" 条款对 v4.x 实际未生效

---

## 1. 现象

PR #4921 合并前查询 commit status:

```
$ curl .../repos/openclaw/sqlrustgo/commits/d108bd0dec/status
  state: pending          # 无任何实际 check
```

最近 5 次 Actions run **全部失败**, 且全部早于本次改动:

| run id | 分支 | 起始时间 | 结论 |
|--------|------|----------|------|
| 5758 | `main` | 2026-09-19T17:56:25Z | failure |
| 5757 | (PR) | 2026-09-19T17:55:07Z | failure |
| 5746 | `main` | 2026-09-11T05:20:49Z | failure |
| 5741 | `main` | — | failure |
| — | `main` | — | failure |

**关键异常**: 2026-09-19 之后仓库有大量提交 (含 PR #4920 合并的 `7ce8f18fc8`), 但**再无任何 run 产生**. 说明流水线根本没被触发, 而非触发后失败.

---

## 2. 根因 (两个独立阻断, 缺一不可)

### 根因 A: 工作流触发器不覆盖 v4.x

`.gitea/workflows/*.yml` 全部 8 个工作流中, 有分支触发条件的都停留在 v3.x:

| 工作流 | 触发的分支 |
|--------|-----------|
| `ci.yml` (Hermes Pipeline) | `develop/v2.8.0`, `v3.8.0`, `v3.10.0`, `v3.11.0`, `v3.12.0`, `beta/v2.8.0`, `ci/gitea-compat`, `ci/v3.8.0-*` |
| `gate.yml` (Evidence Graph Gate v4.1) | `develop`, `main`, `develop/v3.7.0`, `develop/v3.8.0` |
| `reconciliation.yml` | `develop`, `develop/v3.7.0`, `develop/v3.8.0` |
| `soak_168h.yml` / `soak_probe.yml` / `z6g4-*.yml` | 手动 `workflow_dispatch`, 无自动触发 |

**没有任何工作流监听 `develop/v4.0.0` 或 `develop/v4.1.0`.**

注意 `gate.yml` 的名字叫 "Evidence Graph Gate **v4.1**", 但它触发的却是 `develop/v3.7.0` / `v3.8.0` —— 名称与触发条件脱节, 极易误判为"v4.1 已有门禁".

### 根因 B: 没有任何 Runner 注册

```
$ curl .../repos/openclaw/sqlrustgo/actions/runners
  {"runners":[], "total_count":0}
```

而所有 8 个工作流的 12 个 job 全部声明 `runs-on: [hp-z6g4]` —— 需要一台带 `hp-z6g4` 标签的 runner.

**即使补上触发器, 没有 runner 仍会永久排队**. 两个根因必须同时修复.

(组织级 runner 接口 `/api/v1/admin/runners` 返回 404, 该 Gitea 版本/权限下不可查; 仓库级查询已足够证明无可用 runner.)

---

## 3. 为什么这构成治理违规

`docs/governance/BRANCH_GOVERNANCE.md` §5.4.1 规定 `develop/*`:

```
- 禁止直接 push develop/vX.Y.Z
- 必须通过 PR
- 至少 1 个 review
- CI 必须通过
```

"CI 必须通过" 对 v4.x **从未真正执行过** —— 没有 run 产生, 自然无从失败或通过.

叠加另一处已确认的缺口: 合并 PR #4921 时 `develop/v4.1.0` **没有任何 branch protection 记录** (7 条保护规则中不含它, 而 §2.1 表明确要求 `develop/*` 启用 Require review + Disable force push). 两项缺失叠加, 使得 #4921 在**无 reviewer 审批、无 CI** 的状态下被 admin 直接合入.

> 该违规已既成事实, 详见 PR #4921 及 `2026-09-30-V410-ALPHA-UNSUPPORTED-BY-GATE-EVIDENCE.md` (同类问题: ALPHA 由 `current_stage` 误推进, 无实跑证据).

---

## 4. 已采取的补救 (本次)

| 项 | 状态 | 证据 |
|----|------|------|
| `develop/v4.1.0` 补 branch protection | ✅ 已建并**实测生效** | `enable_push=false`, `required_approvals=1`, `block_admin_merge_override=true`; 试探推送被拒: `Not allowed to push to protected branch develop/v4.1.0` |
| 保护规则阻止无审批合并 | ✅ **实测生效** | 合并 PR #4922 被拒: `Does not have enough approvals` |

branch protection 已使"无审批直接合入"不再可能, 补上了治理链的最后一环. 但 **CI 门禁仍空缺**, 需基础设施支持.

---

## 5. 待办 (需 runner 基础设施)

修复顺序有依赖, 不可颠倒:

1. **注册 runner**: 部署 `act_runner` 并打上 `hp-z6g4` 标签
   ```bash
   # 在 Gitea 管理后台: Settings → Actions → Runners → Add runner
   # token 用 /api/v1/user/settings/keys 之外的管理接口签发
   ```
2. **补触发器**: 在 `ci.yml` / `gate.yml` / `reconciliation.yml` 的 `push.branches`
   与 `pull_request.branches` 中加入 `develop/v4.0.0` / `develop/v4.1.0`
   (建议直接用通配 `develop/v*` 以免每次发版重复踩坑)
3. **对齐 `gate.yml` 名称**: 现名 "Evidence Graph Gate v4.1" 却监听 v3.7/v3.8, 需核对到底该监听哪条线
4. **核实历史失败原因**: 5 次 failure 的日志需在 runner 恢复后重跑确认; 当前 API 返回 `jobs: 0`, 无步骤级日志可查
5. **考虑启用 status check**: runner 恢复后, 在 `develop/v4.1.0` 的 branch protection 上打开
   `enable_status_check` 并绑定 `status_check_contexts`, 让"CI 必须通过"从文档条款变成服务端硬拦截

---

## 6. 复现命令

```bash
A="openclaw:details8848"; U="http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo"

# 根因 A: 列出各工作流实际触发的分支
for f in .gitea/workflows/*.yml; do
  echo "=== $f ==="
  sed -n '/^on:/,/^env:\|^jobs:/p' "$f" | grep -E '^\s+- '
done

# 根因 B: 确认无 runner
curl -s -u "$A" "$U/actions/runners"

# 现象: 确认 2026-09-19 后无新 run
curl -s -u "$A" "$U/actions/runs?limit=5" | python3 -c \
  "import json,sys;[print(r['id'], r.get('head_branch'), r.get('started_at'), r.get('conclusion')) for r in json.load(sys.stdin)['workflow_runs']]"
```

---

## Provenance

- 诊断时间: 2026-10-01
- 诊断方式: Gitea REST API (`/actions/runs`, `/actions/runners`, `/branch_protections`) + 仓库内 `.gitea/workflows/*.yml` 静态核对
- 均为**实测输出**, 非推断. 唯一未能取证项: 历史 5 次 failure 的步骤级日志 (API 返回 `jobs: 0`, runner 已注销, 日志不可读)
- 依据: `BRANCH_GOVERNANCE.md` §2.1 / §5.4.1
