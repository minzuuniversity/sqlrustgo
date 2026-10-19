## Why

`scripts/sync/5remotes_sync.sh` 在本机**无限挂起**（实测 900s 超时仍未返回），
5 远端同步无法完成。

此前会话据挂起现象得出「github / gitee / gitcode 三个远端在本机 SSH 不可达」的结论并记录进
`docs/governance/lessons/`。**该结论经本次实测证伪**：三个远端认证完全正常，
挂起的唯一根因是 **ssh-agent 失效**。

### 根因（实测隔离，非推测）

本机 `SSH_AUTH_SOCK=/var/run/com.apple.launchd.mngm6DFt8Q/Listeners`，
socket 文件仍在但 agent 已死（`ssh-add -l` 为空）。
socket 文件的存在使 `connect()` 不立即失败，ssh 在公钥认证阶段**直接阻塞**。

| 命令（同一台机器 / 同一网络 / 同一密钥） | 结果 |
|---|---|
| `ssh -T git@github.com` | **挂起，无输出** |
| `ssh -o IdentityAgent=none -T git@github.com` | 认证成功 |
| `ssh -o IdentitiesOnly=yes -o IdentityAgent=none -T git@{github,gitee,gitcode}.com` | 3/3 认证成功 |
| `ssh -o IdentitiesOnly=yes -o IdentityAgent=none {liying@192.168.0.250,liying@192.168.0.252} true` | 2/2 成功 |

TCP 22 端口对三个公网主机全部 OPEN；主机密钥与 `known_hosts` 一致。

脚本的全部 ssh 调用点都**没有** `IdentityAgent=none`：
`discover_gitea_target()`、Step 3 的探活、Step 3 的 `update-ref`，
以及经 `git push` 走的 SSH 通道（`GIT_SSH_COMMAND` 未设则继承同样失效的 agent）。

### 次生缺陷

1. `discover_gitea_target` 在容器内跑 `find / -maxdepth 6`——250 容器上实测 **9 分钟未完成**，
   单这一条就吃光脚本 900s 预算。而两台机器的仓库布局是已知且互不相同的：
   - 250：`/git/<owner>/<repo>.git`
   - 252：`/data/git/repositories/<owner>/<repo>.git`
2. 脚本内硬编码明文凭据（另立 #5381）。

## What Changes

* **`scripts/sync/5remotes_sync.sh`**
  * 引入单一 SSH 选项来源：所有 ssh 调用与 git 的 SSH 通道共用，
    强制携带 `IdentityAgent=none` + `IdentitiesOnly=yes` + `BatchMode=yes` +
    `ConnectTimeout` + `ServerAliveInterval/CountMax`，
    **不再依赖调用方的 agent 状态**。
  * `GIT_SSH_COMMAND` 在调用方未设置时由脚本自行设置——否则 `git push`/`git fetch`
    仍会继承失效 agent 而挂起。
  * 容器仓库发现改为**定向探测**（已知布局优先），`find` 收窄为兜底且限定目录。
  * 移除硬编码明文凭据，改由 `GITEA_USER` / `GITEA_TOKEN` 提供，缺失时**明确报错退出**。
  * 新增 `GITEA_HOSTS` / `GITEA_PUSH_URL` 覆盖开关——既是可测试性 seam，
    也是「同步到别处 / 只同步部分主机」的真实需求。
* **新增 `scripts/sync/test_5remotes_sync_ssh.sh`**
  * S1 静态检查：ssh 调用点选项完整性、无明文凭据、无全盘 `find /`
  * S2 行为测试：stub `ssh` **忠实模拟失效 agent 的阻塞行为**，
    在 `SSH_AUTH_SOCK` 指向死 socket 时复现挂起——修复前必须失败
  * S3 行为测试：断言远端命令确实构造了 `update-ref <branch> <sha>` 与临时 ref 清理

## Capabilities

### Modified Capabilities

- `release-5-remote-sync`: 同步工具的 SSH 通道必须自洽，不受调用方环境影响；
  凭据必须外置；同步结果必须可由远端真值（`ls-remote`）验证

## Impact

- **Modified**: `scripts/sync/5remotes_sync.sh`
- **New**: `scripts/sync/test_5remotes_sync_ssh.sh`、`openspec/changes/fix-5380-sync-ssh-hang/`
- **Cross-reference**: #5381（凭据轮换需 owner 配合）、#5380（本变更的上游 issue）
- **行为兼容性**: 凭据不再有硬编码缺省值 → 未设置 `GITEA_USER`/`GITEA_TOKEN`
  的操作者会看到明确报错而非静默使用泄漏凭据。这是**有意的行为变更**。

## Acceptance criteria

- [ ] `bash scripts/sync/test_5remotes_sync_ssh.sh` 在修复前失败、修复后通过
- [ ] 脚本在 `SSH_AUTH_SOCK` 指向失效 socket 的环境下完成全流程，不挂起
- [ ] 真实 5 远端同步成功：`git ls-remote <remote> refs/heads/develop/v4.1.0`
      对 5 个远端全部返回同一 SHA
- [ ] 脚本内 `grep -nE 'cc19b2ad|details8848'` 命中数为 0
- [ ] 容器发现单次耗时 < 5s
