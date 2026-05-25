# Gitea + Nomad CI/CD 部署架构

> 双节点 Nomad 调度 + Gitea Actions + Docker Compose runner 的混合 CI/CD 部署方案。

## 架构总览


## 组件角色

| 组件 | 版本 | 角色 | 节点 |
|------|------|------|------|
| Gitea | 1.22.1 | Git + Actions 调度 | Z6G4 (Docker) |
| Postgres | 17-alpine | Gitea 数据库 | Z6G4 (Docker) |
| Registry | 2 (Docker) | 镜像缓存仓库 | Z6G4 (Docker) |
| Nomad Server | 1.9.6 | 调度器 | Z6G4 (系统服务) |
| Nomad Client (hp-z6g4) | 1.9.6 | 工作节点 | Z6G4 |
| Nomad Client (z440) | 1.9.6 | 工作节点 | Z440 (250) |
| Gitea Runner ×4 | gitea/act_runner:node | CI 执行器（Docker Compose） | Z6G4 |
| SQLRustGo Runner | sqlrustgo-runner:nomad-v006 | Nomad 调度 CI 执行器 | Z6G4/Z440 |

## Docker Compose 部署

位置: `/opt/devstack/docker-compose.yml`


## Gitea 配置 (app.ini)

位置: `/home/openclaw/app.ini` (映射到容器 `/data/gitea/conf/app.ini`)

关键配置：


### Actions 防炸配置


## Nomad 部署

### Nomad Server 配置 (`/etc/nomad.d/nomad.hcl`)


### Nomad Job (runner.nomad)

位置: `/home/openclaw/runner.nomad`


## Runner 镜像构建链


## 工作流 (CI Pipeline)

Gitea 工作流文件: `.gitea/workflows/ci.yml` (在 SQLRustGo 仓库中)


## 历史事故记录

### Incident #1: entrypoint.sh TOKEN 变量截断 (2026-04-30)

**症状**: Nomad gitea-runner job 46 次失败，每次 `exit 1`，提示 "not set"

**根因**: `/entrypoint.sh` 中第 5 行被截断为:

**原因猜测**: 编写 entrypoint.sh 时 shell heredoc 中的变量名过长被截断，或编辑器损坏。

**修复**: 
1. 重写 `/tmp/entrypoint-fixed.sh` 使用完整变量名
2. 重建镜像 `sqlrustgo-runner:nomad-v006`
3. 推送至 registry

**预防检查**: 参见 [gitea-nomad-health-checks](../wiki/gitea-nomad-health-checks)

### Incident #2: Nomad job 约束使用短 ID (2026-04-30)

**症状**: Job 卡在 `pending`，`nomad job status` 显示 "Placement Failure"

**根因**: Job 级别约束使用了短 ID `98a7c88c`，但 Nomad 实际 UUID 是完整的 `98a7c88c-80ac-d9ac-6e78-26f0325ede05`


**修复**: 改用 `node.class` 约束替代 `node.unique.id`

**预防检查**: 参见 [gitea-nomad-health-checks](../wiki/gitea-nomad-health-checks)

### Incident #3: 镜像标签不存在 (2026-04-30)

**症状**: Job 使用 `nomad-v006` 标签，但 registry 只有 v003-v005

**根因**: 构建流程未更新 registry，新标签未被推送

**修复**: 重建并推送 nomad-v006 标签

**预防检查**: Nomad job 提交前验证 registry 标签存在

### Incident #4: USER 1000 权限不足 + CONFIG_FILE 污染 (2026-04-30)

**症状**: Runner 注册失败 `open .runner: permission denied`，后 `open /local/.runner: no such file or directory`

**根因**: 
- `sqlrustgo-runner:v1` 镜像设置 `USER 1000:1000`，但 Nomad 挂载的 `/data` 是 root 所有
- `run.sh` 使用 `cd /data` 写入 `.runner`，用户 1000 无写权限
- Dockerfile 中 `ENV CONFIG_FILE=/local/.runner` 被后续层继承，导致 `act_runner` 读取不存在的文件

**修复**: 
- 镜像改为 `USER root`，`ENV CONFIG_FILE=`（空字符串）
- Nomad job 删除 `GITEA_RUNNER_ONCE`，runner 常驻 daemon
- z440 缓存旧镜像 → 换 tag `nomad-root-v2` 强制拉取

**预防检查**: Nomad 提交前检查 Dockerfile 中无遗留 `ENV CONFIG_FILE` 配置

## 版本演进

| 版本 | 镜像 | count | --once | 结果 |
|------|------|-------|--------|------|
| v03 | nomad-v003 | 2 | 否 | ✅ 旧版正常工作 |
| v15 | nomad-v006 | 2 | 是 | ❌ node.id 短ID + 镜像不存在 |
| v16 | nomad-v006 | 1 | 是 | ❌ entrypoint TOKEN 截断 |
| v17 | nomad-v006 | 2 | 是 | ❌ USER 1000 写权限 |
| v18 | nomad-v006 | 2 | 是 | ❌ CONFIG_FILE 不存在的文件 |
| v19 | nomad-root | 2 | 是 | ❌ CONFIG_FILE 复用旧版 |
| v20 | nomad-root | 2 | 是 | ❌ shell 转义破坏 Token |
| v21 | nomad-root | 2 | 是 | ✅ z6g4 成功 / z440 缓存失败 |
| v22 | nomad-root-v2 | 2 | 是 | ✅ 双节点成功 |
| **v23** | **nomad-root-v2** | **4** | **否** | **✅ ✅ 当前稳定版** |

## 相关

- [gitea-nomad-health-checks](../wiki/gitea-nomad-health-checks) — 检查清单
- [ci-cd-platform](../wiki/ci-cd-platform) — CI/CD 平台概述
- [gitea-devstack](../wiki/gitea-devstack) — Gitea 基础设施
- [nomad-scheduler](../wiki/nomad-scheduler) — Nomad 调度器