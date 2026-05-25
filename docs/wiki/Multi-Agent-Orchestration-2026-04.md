# 多 Agent 协同开发系统 — 平台现状与自动化方案

> 编译自三平台实地侦察（Z440 / Z6G4 / Mac Mini）+ Gitea API 数据。
> 本页面记录当前手工协调状态的问题，并提出自动化协同方案。

## 1. 三平台当前状态

### 平台矩阵

| 属性 | Z440 (192.168.0.250) | Z6G4 (192.168.0.252) | Mac Mini |
|------|----------------------|----------------------|----------|
| 用户 | `ai` | `openclaw` | `liying` |
| repo 路径 | `/home/ai/dev/sqlrustgo` | `/home/openclaw/dev/yinglichina163/sqlrustgo` | `~/workspace/dev/openheart/sqlrustgo` |
| repo 命名空间 | **openheart** (错误!) | **yinglichina163** | **openheart** |
| 分支 | `develop/v2.8.0` | `feature/issue-25-column-filter-select` | `develop/v2.8.0` |
| Hermes | ✅ 运行中 | ✅ 运行中 | ✅ 运行中 |
| OpenCode | ✅ 运行中 (167h) | ✅ 运行中 | ✅ 运行中 |
| 远程 | `origin` (HTTP, 内嵌密码) | `origin` + `gitea` + `gitcode` | `origin` (SSH) |
| SSH key | ❌ 未配置 | ✅ `id_ed25519_openheart` | ✅ `id_ed25519_openheart` |
| 未推送 commit | **1 commit** (`432d63fa`) | **7 commits** (`0b0e25b7` 起) | 无 |
| PR 状态 | ❌ 无 | ❌ 无（column-filter-select 未提交 PR） | ✅ PR #34 已合并 |

### 关键发现

#### Z440 — 命名空间错误

`/home/ai/dev/sqlrustgo` 是 `openheart` 命名空间的克隆，但 Gitea 的 `openclaw` 账户（Hermes 在用）应该用 `yinglichina163` 命名空间。这导致：


所有平台都在用 `openclaw` 账户的 HTTP 凭证访问 `openclaw` 命名空间，但 repo 实际是 `openclaw/sqlrustgo`（不是 `openclaw/yinglichina163.sqlrustgo`）。需要确认 repo 的正确归属。

#### Z6G4 — 7 个 commit 未推送

`feature/issue-25-column-filter-select` 分支有 7 个 commit 未 push，主题是 column privilege DML（INSERT/UPDATE 权限检查）。另外有 worktree `fix/mysql-handshake-v2.8.0`。

#### Mac Mini — 最新状态

- `develop/v2.8.0` 与 origin 同步
- PR #34 刚合并（fix/parser-tests）
- Wiki 已同步到 Gitea
- GBrain 已更新

### 当前分支分布（Gitea API 可见）


## 2. 手工协调的问题

### 问题清单

| # | 问题 | 风险等级 | 影响 |
|---|------|----------|------|
| P1 | Z440 repo 命名空间可能错误（openheart vs yinglichina163） | **HIGH** | push 后可能覆盖彼此修改 |
| P2 | Z6G4 7 commits 未推送，长期悬空 | **HIGH** | 工作成果可能丢失 |
| P3 | 无跨平台任务分配机制 | **HIGH** | 同一任务被两个平台重复做 |
| P4 | Z440 无 SSH key，依赖 HTTP 密码 | **MEDIUM** | token 过期后 CI 中断 |
| P5 | 15+ 孤分支无 owner | **MEDIUM** | 技术债务积累 |
| P6 | CI auto-scaler 未运行（NOMAD 问题） | **MEDIUM** | CI 队列堆积 |
| P7 | PR 创建需要手工复制 PAT/token | **LOW** | OpenCode 无法自主提交 PR |

### 手工协调流程（现状）


问题：用户变成中间人，三个 Agent 无法感知彼此状态。

## 3. 自动化协同方案

### 核心设计原则


### 方案架构


### 任务分配约定

| 平台 | 任务前缀 | 任务域 |
|------|----------|--------|
| Z440 (`ai`) | `task/z440/*`, `perf/*`, `fix/z440/*` | 性能优化、基础设施、Z440 特有 |
| Z6G4 (`openclaw`) | `task/z6g4/*`, `feat/*`, `fix/*` | 核心功能、数据库引擎功能 |
| Mac Mini (`liying`) | `task/mac/*`, `docs/*`, `chore/*` | 文档、CI/CD、治理、PR 审核 |

### 分支命名规范（强制）


Gitea branch protection: `develop/v2.8.0` 只接受符合命名规范的 PR 合并。

### 自动化 PR 生命周期

#### 触发器（任一条件）

1. `git push origin <branch>` → Gitea webhook → CI 开始
2. CI 通过 → Hermite (Mac Mini) 自动创建 PR（如果 PR 不存在）
3. CI 通过 + PR review 通过 → 自动合并

#### Auto-PR Script（在 Mac Mini 上作为 cron 运行）


#### Auto-Merge Script（CI 通过后触发）


### 跨平台通知机制

当 Z6G4 完成一个任务并 push 后，Mac Mini 的 Hermes 应收到通知：


### Gitea PAT 配置（各平台统一）

所有平台的 Hermes 使用 `~/.ssh/openclaw-gitea.PAT` 通过 Gitea API 操作：


## 4. 立即可执行的动作

### 立即（手动，一次性）

| Action | 执行位置 | 命令 |
|--------|----------|------|
| 修复 Z440 remote | Z440 (`ai`) | `git remote set-url origin http://yinglichina8848:<TOKEN>@192.168.0.252:3000/openclaw/sqlrustgo.git` |
| 推送 Z6G4 悬空 commits | Z6G4 (`openclaw`) | `git push origin feature/issue-25-column-filter-select` |
| 为 Z6G4 创建 PR | Mac Mini | `curl -X POST ... /pulls` (见上方) |
| 修复 Z440 SSH | Z440 | `ssh-keygen -t ed25519 -C "ai@z440" && copy to Gitea` |

### 短期（24h 内）

1. **清理孤分支** — 9+ 无 owner 分支，征求用户确认后删除或认领
2. **运行 auto_pr_creator** — 在 Mac Mini 上配置 cron，每 5 分钟检查一次
3. **启动 CI auto-scaler** — 解决 Nomad entrypoint.sh TOKEN 问题

### 中期（1 周内）

1. **规范化分支命名** — 所有新分支必须带平台前缀
2. **Gitea branch protection** — `develop/v2.8.0` 强制要求 PR + CI 通过 + 至少 1 reviewer
3. **统一 PAT 管理** — 各平台的 `~/.ssh/openclaw-gitea.PAT` 同步

## 5. 相关页面

- [sqlrustgo-gitea-status-2026-04](../wiki/sqlrustgo-gitea-status-2026-04) — SQLRustGo + Gitea 当前状态
- [ci-cd-platform](../wiki/ci-cd-platform) — CI/CD 平台架构
- [nomad-scheduler](../wiki/nomad-scheduler) — Nomad 调度系统详情
- [sqlrustgo-distributed-dev](../wiki/sqlrustgo-distributed-dev) — 分布式开发环境配置