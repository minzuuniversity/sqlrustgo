# Nomad Scheduler

HashiCorp Nomad v1.9.6 作为 CI runner 的调度系统。

## 部署拓扑

| 节点 | IP | 角色 | 规格 | Nomad 角色 |
|------|-----|------|------|-----------|
| Z6G4 | 192.168.0.252 | Server+Client | 80核, 409GB | main |
| Z440 | 192.168.0.250 | Client | 28核 | fallback |

## 安装路径

- 二进制: `/usr/local/bin/nomad`
- 配置: `/etc/nomad.d/config.hcl`
- 服务: `nomad-server.service` (systemd, Z6G4)
- 服务: `nomad-client.service` (systemd, Z440)

## Runner Job


## Auto-scaler

`/opt/nomad/bin/ci-autoscaler.sh` 每 15 秒检查 Gitea 待处理 CI 任务数，动态调整 runner 数量:
- 有 CI 任务 → scale up (每2个任务1个runner)
- 无任务 → scale down (维持最小值)

## 相关页面

- [ci-cd-platform](../wiki/ci-cd-platform) — CI/CD 平台整体架构
- [gitea-devstack](../wiki/gitea-devstack) — Gitea 基础设施