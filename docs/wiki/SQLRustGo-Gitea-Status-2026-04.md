# SQLRustGo & Gitea DevStack 状态总览

> 编译自 Hermes Agent 记忆、SQLRustGo 项目状态、Gitea 基础设施配置。
> 本页面整合 [hermes-agent](../wiki/hermes-agent)、[sqlrustgo](../wiki/sqlrustgo)、[gitea-devstack](../wiki/gitea-devstack) 三个实体页面的关键信息，
> 并记录截至 2026-04-30 的最新阻塞项和技术债务。

## 1. SQLRustGo 版本矩阵

| 版本 | 状态 | 关键里程碑 |
|------|------|-----------|
| v2.6.0 | **稳定基线** | 226 PASS, 0 FAIL, 0 IGNORED |
| v2.8.0 | **活跃开发** | governance 分支，baseline_verified 完成，R1-R7 规则生效 |
| v2.9.0 | **规划中** | RBAC 未实现, 10GB 测试缺失, GBrain 验证未开始 |

### 2.8.0 治理详情

- **分支**: `governance/v2.8.0` → 合并至 `develop/v2.8.0`
- **攻击面 (Attack Surface)**: AV1-AV9，其中 AV3/AV4/AV5 为 CRITICAL，AV1/AV2/AV7 为 HIGH
- **关键限制 L1**: 测试验证行为，不验证语义正确性
- **Proof Registry**:

| 证明 | 状态 |
|------|------|
| wal_crash_proof | UNVERIFIED |
| mvcc_proof | FAIL |
| executor_proof | VERIFIED |
| benchmark_proof | UNVERIFIED |
| blockers | 4 remaining |

### v2.9.0 阻塞项

1. **RBAC** — 基于行的访问控制未实现
2. **10GB 测试** — 10GB 规模向量测试缺失
3. **GBrain 语义验证** — GBrain 集成未开始
4. **Memory Sync Mode** — memory_index.json 同步机制未完成

## 2. Gitea DevStack 基础设施

### 网络与访问

| 参数 | 值 |
|------|-----|
| Gitea 主机 | 192.168.0.252 (HP Z6G4, 80核/409GB) |
| Gitea Web | http://192.168.0.252:3000 |
| Gitea SSH | ssh://git@192.168.0.252:222 |
| Gitea 版本 | 1.22.1 (Docker) |
| PostgreSQL | 17 (Docker Compose) |

### 账户体系

| 账户 | 角色 | 用途 |
|------|------|------|
| openclaw | 管理员 | 分支保护、合并 PR |
| hermes-macmini | write | Mac Mini 开发 |
| hermes-z6g4 | write | Z440/250 开发 |
| hermes-devstack | write | Z6G4/252 开发 |
| ci-bot | write | CI/CD 自动化 |

> ⚠️ hermes-z6g4 和 hermes-devstack 的 Git 身份需要验证是否已正确配置（用户名/hermes-agent 记忆中的名称可能与实际不符）。

### Runner 架构（Nomad 调度后）


Runner 镜像 `sqlrustgo-runner:v1` 包含: clippy, rustfmt, protoc, sysbench, postgresql-client, sqlite, py3-pip, jq, yq

### Gitea 防炸配置 (app.ini)


> ⚠️ Gitea 1.22 `GetDiffShortStat` 不能完全被 `ENABLE_GIT_DIFF_STATS=false` 绕过，建议升级至 1.23+。

### 混沌工程验证（2026-04-29）

| 实验 | 结果 | 说明 |
|------|------|------|
| 杀 Runner | ✅ 13s 自愈 | systemd → Nomad 自动恢复 |
| CPU 82% 压力 | ✅ 稳定 | Gitea 响应 34-44ms |
| 30GB 磁盘写入 | ✅ 稳定 | Gitea 响应 19-22ms |
| 30% 网络丢包 | ✅ 自愈 | TCP 自动重试 |
| Postgres 重启 | ✅ 免影响 | Gitea 连接池保持 |

6 项实验通过 5 项。

## 3. Nomad Auto-scaler 当前状态

### Open Issues（截至 2026-04-30）

| Issue | 状态 | 说明 |
|-------|------|------|
| Nomad entrypoint.sh TOKEN 未验证 | **未确认** | 文件路径 `/opt/nomad/entrypoint.sh` 不存在，需找到实际路径 |
| runner-data 目录未确认 | **未确认** | 需要验证 `/opt/nomad/` 下的目录结构 |
| docker.sock 权限 | **阻塞** | 252 上 liying 用户不在 docker 组，无法直接 `docker ps` |
| ci-autoscaler.sh 守护进程 | **未启动** | 需验证 `ci-autoscaler.sh` 是否在运行 |

### Auto-scaler 规格

- 脚本路径: `/opt/nomad/bin/ci-autoscaler.sh`
- 轮询间隔: 15 秒
- 扩缩策略: 每 2 个待处理 CI 任务 → 1 个 runner
- 范围: 0 → 10 runners

### 诊断命令


> ⚠️ sudo 需要密码 "details"，且 heredoc SSH 方式对 sudo无效，需用 Python-over-SSH 方式。

## 4. 开发环境冲突

| 端 | 路径 | 分支 | 风险 |
|----|------|------|------|
| Hermes (本会话) | `~/workspace/dev/openheart/sqlrustgo` | develop/v2.8.0 | 当前活跃工作区 |
| TRAE AI | `~/workspace/dev/yinglichina163/sqlrustgo` | v2.8.0 | 可能覆盖彼此修改 |

> ⚠️ 两个 AI 代理并行工作于同一 repo（不同克隆），需通过 Gitea PR 机制协调，避免直接 push 冲突。

## 5. Git 安全规则

SQLRustGo 有 pre-commit hook 检查 Git 身份：

- **必须**: `git config user.email "openheart@gaoyuanyiyao.com"`
- **违规**: 任何其他 email 导致 commit 失败，报错 "❌ Wrong Git identity!"
- **Gitea 远程**: `ssh://git@192.168.0.252:222/openclaw/sqlrustgo.git`

## 6. Parser 实现要点（v2.8.0 经验积累）

- **Lexer 改动**: `token.rs` 添加枚举 + `lexer.rs` match 语句需同步修改
- **parse_insert**: 先查 `Token::Replace`，再 `expect Token::Insert`
- **parse_partition**: 内联解析分区列，不调用 `parse_column_list()`

## 7. 相关页面

- [sqlrustgo](../wiki/sqlrustgo) — 项目实体页面（详细版本路线）
- [gitea-devstack](../wiki/gitea-devstack) — 基础设施实体页面（详细配置）
- [hermes-agent](../wiki/hermes-agent) — Agent 实体页面（记忆系统设计）
- [ci-cd-platform](../wiki/ci-cd-platform) — CI/CD 平台演进历史
- [nomad-scheduler](../wiki/nomad-scheduler) — Nomad 调度系统详情
- [governance-system](../wiki/governance-system) — v2.8.0 治理模型
- [proof-system](../wiki/proof-system) — 证明注册表系统