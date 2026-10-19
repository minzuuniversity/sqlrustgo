# Design — fix-5380-sync-ssh-hang

## 1. 失效 agent 为什么会让 ssh「挂住」而不是「快速失败」

OpenSSH 在启动时读取 `SSH_AUTH_SOCK`，并向该 unix socket 请求已加载的身份列表。
当 agent 进程已死但 socket 文件仍在时：

- `connect()` 不会立刻返回 `ECONNREFUSED`——unix socket 的行为是「文件存在即认为可连」
- 于是 ssh 阻塞在 agent 往返上，**没有任何输出**

`BatchMode=yes` 只禁止*交互式*口令/口令短语提示，**不禁止** agent 往返。
`IdentitiesOnly=yes` 只限制*提供哪些身份*，同样**不禁止** agent 往返。
唯一能彻底切断这条路径的是 `IdentityAgent=none`——它让 ssh 完全不去碰 agent。

> 这是本设计唯一不可协商的选项。其余选项（`StrictHostKeyChecking`、`ConnectTimeout`）
> 解决的是别的失败模式，不能替代它。

## 2. 选项来源要唯一

脚本共有 3 处 ssh 调用 + 1 条 git SSH 通道（`git push` / `git fetch`）。
若每处各写各的，漏一处就复现本次故障。因此：

```
SSH_OPTS=( -o BatchMode=yes
           -o ConnectTimeout=10
           -o IdentitiesOnly=yes
           -o IdentityAgent=none
           -o StrictHostKeyChecking=accept-new
           -o ServerAliveInterval=15
           -o ServerAliveCountMax=3 )
```

- 全部 ssh 调用：`ssh "${SSH_OPTS[@]}" ...`
- git 通道：`GIT_SSH_COMMAND="ssh ${SSH_OPTS[*]}"`（仅当调用方未设置时）
- `ServerAliveInterval/CountMax` 把「连上了但服务端不应答」的挂起上界压到 ~45s

`StrictHostKeyChecking` 由原 `no` 改为 `accept-new`：
`no` 会**静默接受任何变更过的主机密钥**（抗不了中间人），`accept-new` 只自动接受**首次见到**的密钥、
对**已变更**的密钥报错拒绝。已知 5 个主机在 `known_hosts` 中均已存在，本次实测 `accept-new` 全部通过。

## 3. 凭据外置：为什么不留"默认值"

保留硬编码默认值等于**没修**——脚本在生产环境仍然会使用那个泄漏的 token，
且运维者永远不知道自己该去设什么。缺省即报错的语义与脚本既有的
「失败关闭、不猜」哲学一致（见脚本内 exit 5 的注释）。

```
GITEA_USER   Gitea 用户名（HTTP basic auth）
GITEA_TOKEN  Gitea token / 口令
```

两者都缺失时：Step 3 需要的「读当前 ref」与「推临时 ref」都会失败，
脚本以可操作的错误信息退出，并说明该设哪个变量。

## 4. 容器仓库发现：从全盘 find 改为定向探测

`find / -maxdepth 6` 要遍历容器内所有挂载点（实测 9 分钟未完成）。
而布局是**已知且有限**的：

```
/git/*/sqlrustgo.git                        # 250
/data/git/repositories/*/sqlrustgo.git      # 252
/data/git/gitea-repositories/*/sqlrustgo.git # 变体
```

定向探测 O(1)；`find` 保留为兜底但**限定在 `/git` 与 `/data/git` 之内**且 `maxdepth 4`。

## 5. 可测试性 seam

脚本原本无法被测试：主机地址硬编码、远端是真实网络、容器发现依赖真实 docker。
引入两个覆盖开关后，测试可以把 5 个远端替换成本地 bare repo：

| 开关 | 默认 | 用途 |
|---|---|---|
| `GITEA_HOSTS` | `192.168.0.250:liying 192.168.0.252:liying` | 同步哪些主机 |
| `GITEA_PUSH_URL` | 由 host + 凭据计算 | 临时 ref 推到哪 |

它们不是"为测试而加的死代码"：只同步部分主机、推到镜像仓库都是真实需求。

## 6. 测试如何忠实模拟失效 agent

stub `ssh` 的判定逻辑：

```
若 SSH_AUTH_SOCK 非空 且 参数中没有 IdentityAgent=none  →  阻塞（模拟真 ssh）
否则                                                  →  按命令语义响应
```

这正是真 ssh 的行为差异，因此该 stub 是一个**行为替身**而非断言脚本：
如果脚本漏传 `IdentityAgent=none`，stub 就会像真 ssh 一样挂住，测试超时失败。

stub 还要能执行脚本发来的远端命令——否则测试只能验证「命令字符串长什么样」，
验证不了「命令跑起来对不对」。做法：把 `docker exec <cid> bash -c '<inner>'`
剥成 `<inner>`，并让 `discover` 阶段返回本地 bare repo 路径，
于是 `<inner>` 里的 `cd $repo_path` 自然落在真实目录上。

## 7. 风险与回退

- **风险**：选项收紧后，若某主机的 `known_hosts` 条目陈旧，`accept-new` 会拒绝连接。
  缓解：报错信息明确指出「主机密钥已变更」，并给出修复命令。
- **回退**：改动集中在选项数组一处，回退即恢复旧数组。
- **凭据行为变更**：未设 `GITEA_USER`/`GITEA_TOKEN` 的操作者会从「静默使用泄漏凭据」
  变为「明确报错」。这是有意的，且与 #5381 的轮换方向一致。
