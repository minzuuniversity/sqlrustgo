# CI/CD Platform

基于 Gitea + act_runner + Nomad 的 CI/CD 平台，经历从单机手动管理到调度系统编排的演进。

## 架构演进

### 阶段1: 裸 Runner（已废弃）
- 手动 `docker run` 启动 runner
- 16+ 个失联容器堆积
- token 过期无人知晓
- 无健康检查、无自愈

### 阶段2: systemd 管理（2026-04-29）
- `hermes-runner.service` + wrapper 自动重注册
- `sqlrustgo-runner:v1` 完整工具链镜像
- Gitea `app.ini` 防炸配置
- PR 清理 cron 脚本

### 阶段3: Nomad 调度（2026-04-29）
- Nomad v1.9.6 Server+Client
- 双节点: hp-z6g4 (80核) + z440 (28核)
- CI Auto-scaler 按 CI 队列长度动态扩缩
- 支持 0→10 runner 弹性伸缩

## CI Pipeline


## 混沌工程验证

6 项实验通过 5 项，系统达到"故障持续发生仍能工作"的可靠性目标。

## 相关页面

- [gitea-devstack](../wiki/gitea-devstack) — Gitea 基础设施
- [nomad-scheduler](../wiki/nomad-scheduler) — Nomad 调度系统
- [governance-system](../wiki/governance-system) — 治理系统与 CI 集成