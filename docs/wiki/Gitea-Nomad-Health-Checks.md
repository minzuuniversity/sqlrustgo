# Gitea + Nomad 健康检查清单

> 预防已知错误、快速诊断 CI/CD 离线问题的标准检查流程。

## 前置原则

修改任何 CI/CD 组件前：

1. **备份当前配置**: `/opt/backups/`
2. **记录修改到 `~/wiki/log.md`**
3. **测试后提交配置文件到 Gitea**


## 2. Nomad 运行时健康检查

### 2.1 Job 状态


### 2.2 节点状态


### 2.3 Docker 容器状态



## 4. 容器镜像完整性检查

### 4.1 Registry 可用性


### 4.2 Runner 镜像检查



## 6. 配置管理检查

### 6.1 配置文件归档

确认以下配置文件已提交到 Gitea:

| 文件 | 位置 | 归档仓库 |
|------|------|----------|
| docker-compose.yml | `/opt/devstack/` | `openclaw/sqlrustgo.wiki` |
| app.ini | `/home/openclaw/` | `openclaw/sqlrustgo.wiki` |
| nomad.hcl | `/etc/nomad.d/` | `openclaw/sqlrustgo.wiki` |
| runner.nomad | `/home/openclaw/` | `openclaw/sqlrustgo.wiki` |
| entrypoint.sh | sqlrustgo-runner:v1 镜像 | `openclaw/sqlrustgo.wiki` |
| Dockerfile.nomad | `/home/openclaw/` | `openclaw/sqlrustgo.wiki` |

### 6.2 版本标签



## 8. 自动化检查脚本

位置: `~/wiki/gbrain/scripts/gitea-nomad-check.sh` (TODO: 实现)


---

## 相关

- [ci-cd-platform-deployment](../wiki/ci-cd-platform-deployment) — 完整部署指南
- [gitea-devstack](../wiki/gitea-devstack) — Gitea 基础设施
- [nomad-scheduler](../wiki/nomad-scheduler) — Nomad 调度器
- [proof-system](../wiki/proof-system) — 证明系统

## 检查清单 v23 (2026-04-30 扩容后)

- [ ] Nomad job count=4, 双节点各分配 1-3 个 runner
- [ ] `GITEA_RUNNER_ONCE` 已删除（runner 常驻 daemon 持续轮询）
- [ ] 镜像标签 `nomad-root-v2` 在 registry 中存在
- [ ] Dockerfile 无 `ENV CONFIG_FILE` 遗留配置（应设为空字符或删除）
- [ ] z440 节点无旧版镜像缓存（新 tag 强制拉取）
- [ ] 各 runner 注册日志出现 `declare successfully`
- [ ] Gitea 日志无 Register 500 错误（预期会有少量，持续出现需排查）