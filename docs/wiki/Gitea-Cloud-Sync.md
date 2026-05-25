# Gitea Cloud Sync 机制

> 本地 Gitea 不稳定，自动同步到 3 个云端 Git 服务作为持久备份。

## 同步架构


## 同步脚本

位置: `~/wiki/scripts/sync_to_cloud.sh`

### 工作流程

1. **Phase 1**: 从 Gitea fetch（15 秒超时，失败则用本地 refs）
2. **Phase 2**: 推送到 3 个云端
   - Gitcode: `develop/v2.8.0` + `main` 直接推送
   - Gitee: 同上，main 用 force push
   - GitHub: 推送到 `cloud-sync/develop-v2.8.0` (保护分支)
3. **Phase 3**: 同步 tags

### SSH 密钥映射

| 服务 | SSH Key | 用户 |
|------|---------|------|
| Gitcode | `id_ed25519_openheart` | BreavHeart |
| Gitee | `id_ed25519` | yinglichina |
| GitHub | `id_ed25519_openheart` | yinglichina8848 |

### 日志


## Cron 作业

- 调度: **每 15 分钟**
- 宿主: Mac Mini (最高可用性)
- 仅在失败 3 次以上时通知


## 恢复方案

当 Gitea 宕机时:

1. 从任一云端 clone:

2. 恢复本地 Gitea:

## 相关

- [ci-cd-platform-deployment](../wiki/ci-cd-platform-deployment) — CI/CD 部署架构
- [gitea-devstack](../wiki/gitea-devstack) — Gitea 基础设施
- [gitea-nomad-health-checks](../wiki/gitea-nomad-health-checks) — 健康检查清单