# Gitea on DevStack

Gitea 运行于 HP Z6G4 服务器 (192.168.0.252, 80核/409GB)，作为 SQLRustGo 项目的主代码托管和 CI/CD 平台。

## 网络配置

| 参数 | 值 |
|------|-----|
| 主机 | 192.168.0.252 (HP Z6G4) |
| 系统 SSH | port 22 (liying 用户) |
| Gitea SSH | port 222 (git 用户) |
| Gitea Web | http://192.168.0.252:3000 |
| Gitea 版本 | 1.22.1 (Docker, gitea/gitea) |
| 数据库 | PostgreSQL 17-alpine (Docker Compose) |
| Docker Compose | `/opt/devstack/docker-compose.yml` |

## 部署架构 (Docker Compose)


**关键数据卷:**
- `devstack_gitea-git` → /data/git/repositories — Git 仓库裸数据
- `devstack_gitea-data` → /data/gitea — 配置、日志、索引
- `devstack_postgres-data` → PostgreSQL 数据文件

## CI/CD 基础设施

### Runner 架构


## 账户体系

### 全部账户 (2026-05-02)

| ID | 账户 | 管理员 | 登录方式 | SSH 密钥 | 用途 |
|----|------|--------|---------|---------|------|
| 1 | **openclaw** | ✅ | 密码 | 3 keys | 管理员，密码 `details8848` |
| 2 | hermes-macmini | ❌ | SSH only | ✅ Mac Mini | Mac Mini Hermes Agent |
| 3 | hermes-z6g4 | ❌ | SSH only | ✅ Z6G4 | Z6G4 Hermes Agent |
| 4 | hermes-devstack | ❌ | SSH only | ✅ DevStack | DevStack Hermes Agent |
| 5 | ci-bot | ❌ | SSH only | 无 | CI/CD runner (环境变量) |
| 6 | **z440-opencode** | ❌ | 密码+SSH | ✅ **2026-05-02 新增** | Z440 Opencode, 密码 `opencode8848` |

### SSH 密钥指纹

| 账户 | 指纹 | 对应机器 |
|------|------|---------|
| openclaw | `SHA256:SebqqJS5...` | 管理节点 |
| openclaw | `SHA256:ix/HiMGs...` | 管理节点 |
| openclaw | `SHA256:Z233lVR2...` | 管理节点 |
| hermes-macmini | `SHA256:XgyEMgkW...` | Mac Mini |
| hermes-z6g4 | `SHA256:ovGpUSLP...` | Z6G4/250 |
| hermes-devstack | `SHA256:MHXMQB9f...` | DevStack |
| z440-opencode | `SHA256:PpLxEs0c...` | **Z440 (2026-05-02)** |

## Pre-Receive Hook 故障与修复 (2026-05-02)

### 故障现象

`git push` 被拒绝，报错：

所有推送方式 (HTTP/SSH/file://) 均受影响。Gitea Web UI 正常，`runEnvironmentToIni` 正常运行。

### 根因

Gitea 1.22.1 的 `runEnvironmentToIni()` 每次容器启动时用环境变量重写 `app.ini`。`docker run` 时如果未设置 `GITEA__repository__ROOT` 环境变量，则 `[repository]` 段被完全丢弃。

Pre-receive hook 调用 `/usr/local/bin/gitea hook --config=/data/gitea/conf/app.ini pre-receive`，读取配置时找不到 `[repository] ROOT`，尝试使用默认路径 `/data/gitea/data/gitea-repositories/`，但仓库实际位于 `/data/git/repositories/`（Docker volume `devstack_gitea-git`），导致解码失败。

### 修复

1. **重建 Gitea 容器**，添加所有必要环境变量：


2. **创建 symlink**（双重保护）：

3. **更新 docker-compose.yml** 添加 `GITEA__repository__ROOT` 环境变量，确保 `docker compose up` 不丢失配置。

### 关键教训

- Gitea 容器 `app.ini` 受 `runEnvironmentToIni()` 控制，手动修改会被覆盖
- 所有定制配置必须通过 **环境变量** 注入（`GITEA__section__KEY=value`）
- 对比：原始 `docker-compose.yml` 中 `GITEA__database__PASSWD=***` 是字面量（非被 mask），密码必须显式设置为正确值 `GiteaDB@2026!`

## 分支保护

| 分支 | Status Check | Approvals |
|------|-------------|-----------|
| `develop/v2.8.0` | ✅ "Hermes Pipeline" | 1 |
| `develop/v2.9.0` | ✅ "Hermes Pipeline" | 1 |
| `main` | ❌ | 0 |
| `release/v2.8.0` | ❌ | 0 |

## 备份策略 (2026-05-02 起)

两层级自动备份：

### Level 1: Z6G4 本地备份 (每日 03:00)

脚本: `/opt/scripts/gitea-backup.sh`
- PostgreSQL pg_dump → `/opt/backups/gitea-db-*.sql.gz`
- 配置归档 → `/opt/backups/gitea-config-*.tar.gz`
- 保留 14 天

### Level 2: 远程镜像同步 (每日 04:00)

从 Mac Mini 执行，同步所有分支到：
- **GitCode**: `git@gitcode.com:BreavHeart/sqlrustgo.git`
- **Gitee**: `git@gitee.com:yinglichina/sqlrustgo.git`
- **GitHub**: `git@github.com:minzuuniversity/sqlrustgo.git`

## 已知问题

- Gitea 1.22 `GetDiffShortStat` 不能完全被 `ENABLE_GIT_DIFF_STATS=false` 绕过（建议升 1.23+）
- runner `--once` 模式依赖 act_runner 版本
- Docker runner Register 返回 500 (cosmetic, FetchTask 正常)
- Nomad runner 部署 jobs 680-689 卡住 (Failed 59, Healthy 0)

## 相关页面

- [sqlrustgo](../wiki/sqlrustgo) — 托管的数据库引擎项目
- [ci-cd-platform](../wiki/ci-cd-platform) — CI/CD 平台架构
- [nomad-scheduler](../wiki/nomad-scheduler) — Nomad 调度系统
- [gitea-cloud-sync](../wiki/gitea-cloud-sync) — 云同步配置
- [multi-agent-orchestration-2026-04](../wiki/multi-agent-orchestration-2026-04) — 多平台协作