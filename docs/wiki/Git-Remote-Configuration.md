# Git Remote 配置指南

> 面向 SQLRustGo 三节点分布式 AI 协作开发环境的 Git 远程配置。
> 目标：确保 Hermes / OpenCode / Claude Code 等 AI Agent 能正确配置 Git 远程、SSH Key 和 PAT 令牌。

---

## 1. 节点概览

| 节点 | 主机 | IP | Git 目录 | Agent |
|------|------|-----|----------|-------|
| **Mac Mini (Brain)** | Mac Mini M4 | 192.168.0.240 | `~/workspace/dev/yinglichina163/sqlrustgo` | **Hermes Agent** (主控) |
| **Z6G4 (Worker)** | HP Z6G4 | 192.168.0.252 | Gitea Server (Docker) | Gitea CI Runner |
| **Z440 (Worker)** | HP Z440 | 192.168.0.250 | Gitea CI Runner (Nomad) | Gitea CI Runner |

Mac Mini 上还有 OpenCode 目录：
```
~/workspace/dev/openheart/sqlrustgo  # OpenCode Agent 使用
```

> **注意**：Hermes 和 OpenCode 工作在不同的 repo 克隆中，通过 Gitea PR 协调。**千万不要在同一个 repo 克隆中并行工作。**

---

## 2. Gitea 远程 URL

### 2.1 正确配置

**方式 A：SSH 别名（推荐）**

```bash
# Hermes repo
cd ~/workspace/dev/yinglichina163/sqlrustgo
git remote set-url origin git@gitea-macmini:openclaw/sqlrustgo.git

# OpenCode repo
cd ~/workspace/dev/openheart/sqlrustgo
git remote set-url origin git@gitea-macmini:openclaw/sqlrustgo.git
```

**方式 B：完整 SSH URL**

```bash
git remote set-url origin ssh://git@192.168.0.252:222/openclaw/sqlrustgo.git
```

⚠️ **必须使用端口 222**（Gitea SSH 端口），不是端口 22。

### 2.2 必须的 SSH 别名配置

`~/.ssh/config` 中必须包含：

```
# Hermes Mac Mini 身份
Host gitea-macmini
    HostName 192.168.0.252
    Port 222
    User git
    IdentityFile ~/.ssh/id_ed25519_macmini
    IdentitiesOnly yes
    StrictHostKeyChecking accept-new

# OpenCode 身份
Host gitea-devstack
    HostName 192.168.0.252
    Port 222
    User git
    IdentityFile ~/.ssh/id_ed25519_openheart
    IdentitiesOnly yes

# Root 访问（非 Git）
Host z6g4
    HostName 192.168.0.252
    User openclaw
    Port 22
    IdentityFile ~/.ssh/id_ed25519
```

> **⚠️ 已知问题**：`~/.ssh/config` 中存在重复的 `Host gitea-macmini` 条目，第二个会覆盖第一个。确保只保留一个正确的条目。

---

## 3. SSH 密钥映射

| SSH 私钥 | 公钥指纹 | Gitea 账户 | 挂载的 Agent | 用途 |
|-----------|----------|------------|-------------|------|
| `~/.ssh/id_ed25519_macmini` | `SHA256:XgyEMgkWcB2RiZ/jRQmoH254jApR4I0hrtQpQDkhh8M` | **hermes-macmini** | Hermes Agent | 默认 Git 操作、PR、CI |
| `~/.ssh/id_ed25519_openheart` | `SHA256:SebqqJS5gKEm1yk/od5cCX3iUewOzit/k38Ly0uoNTI` | **openclaw** (管理员) | OpenCode Agent | PR 提交、合并、管理操作 |
| `~/.ssh/id_ed25519` | `SHA256:Wv9Q8HZ0rJR4OBGnkD5tSnYmHr2/7BySs2Vhcl/HdGM` | **未注册** | 本地用户 | Z6G4/Z440 系统 SSH 登录 |
| `~/.ssh/id_ed25519_maintainer` | — | **未注册** | — | Gitee/GitHub 镜像 |

### 3.1 测试连接

```bash
# 测试 Hermes Mac Mini 身份
ssh -p 222 -i ~/.ssh/id_ed25519_macmini git@192.168.0.252

# 测试 OpenCode 身份
ssh -p 222 -i ~/.ssh/id_ed25519_openheart git@192.168.0.252

# 成功标志：收到 "Hi there, <account>! You've successfully authenticated"
```

### 3.2 注册新密钥

通过 Gitea Web UI 添加：
1. 登录 http://192.168.0.252:3000 → Settings → SSH/GPG Keys
2. 添加公钥内容 `cat ~/.ssh/id_ed25519_xxx.pub`
3. 或者用 API：

```bash
GITEA_TOKEN="$(cat ~/.ssh/hermes-cli-gitea.PAT)"
ssh_pub=$(cat ~/.ssh/id_ed25519_newkey.pub)
curl -X POST "http://192.168.0.252:3000/api/v1/user/keys?token=$GITEA_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"title\":\"my-new-key\",\"key\":\"$ssh_pub\"}"
```

---

## 4. PAT 令牌映射

| 文件名 | 令牌（前缀） | Gitea 账户 | 权限 | 用途 |
|--------|------------|------------|------|------|
| `~/.ssh/openclaw-gitea-write.PAT` | `37a0cc30...` | **openclaw** (管理员) | `write:repo`, `write:user` | API 写入、PR 创建合并 |
| `~/.ssh/hermes-cli-gitea.PAT` | `c766f1b8...` | **hermes-macmini** | `read:repo` | Hermes CLI 只读操作 |
| `~/.ssh/openclaw-gitea.PAT` | `5518d175...` | **openclaw** (管理员) | `read:repo` | 只读 API 查询 |
| `~/.ssh/BraveHeart-Gitcode.PAT` | `8z85f2eB...` | BraveHeart@Gitcode | — | GitCode 镜像推送 |

### 4.1 PAT 使用示例

```bash
# 读取 Wiki 页面
curl -s "http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo/wiki/pages?token=$(cat ~/.ssh/openclaw-gitea.PAT)"

# 创建 Issue
curl -X POST "http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo/issues?token=$(cat ~/.ssh/openclaw-gitea-write.PAT)" \
  -H "Content-Type: application/json" \
  -d '{"title":"Issue Title","body":"Issue Body"}'

# 合并 PR
curl -X POST "http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo/pulls/123/merge?token=$(cat ~/.ssh/openclaw-gitea-write.PAT)"
```

### 4.2 PAT 创建新令牌

1. 登录 Gitea → Settings → Applications → Manage Access Tokens
2. 选择权限范围：
   - **只读**：`read:repository`, `read:user`
   - **写入**：`write:repository`, `write:user`, `sudo`
3. 保存令牌到 `~/.ssh/<name>.PAT`，权限 `600`

---

## 5. Git 身份配置

### 5.1 本地仓库配置

Hermes repo 的本地 Git 身份：

```bash
cd ~/workspace/dev/yinglichina163/sqlrustgo
git config user.name "yinglichina8848"
git config user.email "openheart@gaoyuanyiyao.com"   # ⚠️ pre-commit 要求
git config user.signingkey ""
git config commit.gpgsign false
```

OpenCode repo 的本地 Git 身份：

```bash
cd ~/workspace/dev/openheart/sqlrustgo
git config user.name "Hermes Agent"
git config user.email "openheart@gaoyuanyiyao.com"   # pre-commit 要求
git config commit.gpgsign false
```

> **⚠️ pre-commit 强制规则**：email 必须为 `openheart@gaoyuanyiyao.com`，否则 commit 被拒绝。

### 5.2 全局 Git 配置

```bash
git config --global user.name "CI Bot"
git config --global user.email "ci@example.com"
git config --global pull.rebase true
git config --global push.default current
```

全局配置仅用于非仓库的上下文。在 sqlrustgo 仓库内，local config 覆盖 global。

---

## 6. SSH fetch 故障排查

### 6.1 诊断步骤

```bash
# 1. 验证 SSH 连接
ssh -p 222 -i ~/.ssh/id_ed25519_macmini -v git@192.168.0.252
# 输出 "Hi there, hermes-macmini! You've successfully authenticated" → 连接成功

# 2. 测试 git upload-pack（轻量）
ssh -p 222 -i ~/.ssh/id_ed25519_macmini git@192.168.0.252 \
  "git-upload-pack /openclaw/sqlrustgo.git"
# 输出 refs 信息 → 成功

# 3. 测试 git ls-remote（轻量）
git ls-remote origin refs/heads/develop/v2.8.0

# 4. 测试 git fetch（完整传输）
GIT_SSH_COMMAND="ssh -p 222 -i ~/.ssh/id_ed25519_macmini -o IdentitiesOnly=yes" \
  git fetch origin develop/v2.8.0 --depth=1
```

### 6.2 已知错误

**"fetch-pack: unexpected disconnect while reading sideband packet"**

原因：SSH 连接建立成功，git-upload-pack 能正常工作，但 `git fetch` 的 pack 数据传输阶段 SSH session 被中断。

可能原因：
1. Gitea 的 authorized_keys `command` 限制（Gitea git-shell 拦截）
2. SSH agent 多 Key 冲突（`IdentitiesOnly=yes` 可缓解）
3. Gitea 容器内 git-upload-pack 超时

**临时解决方案（HTTP 绕过）：**

```bash
# 用 HTTP 协议添加临时 remote
git remote add gitea-http http://192.168.0.252:3000/openclaw/sqlrustgo.git
# 注：HTTP 需要 write PAT token 放到 URL 中写操作
git fetch gitea-http develop/v2.8.0 --depth=5

# 删除临时 remote
git remote remove gitea-http
```

---

## 7. Gitea Web UI 和账户

| 账户 | 角色 | 密码 | SSH Key | 用途 |
|------|------|------|---------|------|
| **openclaw** | 管理员 | `details8848` | `opencode-agent`, `HP-Z6G4-*` | 项目管理、CI/CD、Wiki |
| **hermes-macmini** | 普通用户 | — | `Mac Mini Hermes Agent` | Hermes Agent 日常操作 |
| **hermes-z6g4** | 普通用户 | — | Z6G4 本地密钥 | Z6G4 Agent 操作 |
| **ci-bot** | 机器人 | — | — | CI/CD 自动操作 |

> **Web UI**: http://192.168.0.252:3000/openclaw/sqlrustgo

---

## 8. 多 Agent 协作模型

```
Mac Mini (Hermes Agent)            Z6G4 (Gitea + Nomad)
┌─────────────────────┐           ┌─────────────────────┐
│  Hermes CLI (main)  │───SSH───→│  Gitea (port 3000)  │
│  OpenCode CLI       │───SSH───→│  Nomad + Runner     │
│  TRAE AI (并行)     │───HTTP──→│  {commit/push/pull}  │
└─────────────────────┘           └─────────────────────┘
         │                                │
         │ Git Push via Gitea PR          │
         └─────────── PR ────────────────→│
                                           │
                    ┌──────────────────────┘
                    ▼
           Z440 (Nomad Client)
        ┌─────────────────────┐
        │  Gitea Runner ×2    │
        │  CI/CD 执行者       │
        └─────────────────────┘
```

**核心规则：**
1. Hermes 和 OpenCode 在不同 clone 中工作
2. **永远不要直接在同一个 clone 中并行**（会导致文件冲突）
3. 修改通过 Gitea PR 同步
4. pre-commit hook 强制 email 验证
5. Gitea 端口 222 = SSH Git 操作，端口 3000 = Web UI + HTTP API
6. CodeMirror 镜像每 15 分钟同步到 GitCode/Gitee/GitHub

---

## 9. 相关页面

- [Gitea Infrastructure](../wiki/Gitea-Infrastructure) — Gitea 部署架构
- [Multi Agent Orchestration 2026-04](../wiki/Multi-Agent-Orchestration-2026-04) — 多 Agent 协调方案
- [Hermes Agent](../wiki/Hermes-Agent) — Hermes Agent 配置
- [CI CD Platform](../wiki/CI-CD-Platform) — CI/CD 流水线
- [GBrain Knowledge](../wiki/GBrain-Knowledge) — 结构化知识库
- [Home](../wiki/Home) — 项目总览
