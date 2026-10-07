# 同步脚本「先 fetch 再校验」验证证据

- source_agent: mcode (MiniMax-M3.1-Flash-Preview)
- source_run: 2026-10-07 会话（承接 #5090 合并后的同步事故）
- timestamp: 2026-10-07
- 被测脚本: `scripts/sync/5remotes_sync.sh`
- 基线: `ed181dfc0b39`（#5090 合并后 5 远端一致）

## 事故背景

PR #5090 合并后 gitea252 上 `develop/v4.1.0` = `ed181dfc0b39`，但本地
`refs/remotes/gitea252/develop/v4.1.0` 仍是合并前的 `c4aed21d93f4`。旧脚本
**先读本地 ref 取 SHA、后 fetch**，于是把陈旧的 `c4aed21d93f4` 当作目标，
force-push 到全部 5 个远端，分支被回退。本会话内同类事故发生两次。

## 改动

Step 0 提前到任何 SHA 读取之前，并加两道 fail-closed 校验：

1. `git fetch "$source_remote"` 或 `git fetch --all` 失败 → `exit 5`
2. 对每个分支比对 `git rev-parse "$remote/$branch"` 与
   `git ls-remote`（直接问服务器，不读缓存 ref）：
   - 不一致 → `exit 5`
   - `ls-remote` 返回空（查不到，等于不知道远端状态）→ `exit 5`

原 Step 1 的 fetch 相应删除。

## 验证

三条路径，均实跑。

### 路径 1：正常路径（真实 5 远端）

```
$ bash scripts/sync/5remotes_sync.sh develop/v4.1.0
Step 0: fetch before reading any SHA
[develop/v4.1.0] source SHA = ed181dfc0b3978f90fd8b64437ef356263a6baf7 (from gitea252)
Step 2: gitcode/gitee/github : already at ed181dfc0b39 ✓ ×3
Step 3: 250 / 252 容器 update-ref : already at ed181dfc0b39 ✓ ×2
Step 4: ✓ all 10 pairs consistent
EXIT=0
```

### 路径 2：陈旧 ref（离线 fake 远端，复现 #5090 条件）

离线构造：bare fake 远端真实持有 `b19a35c8`，本地 ref 手工置为
`4247114c`。fetch refspec 收窄到不刷新该分支，以保证 fetch 成功却仍陈旧。

```
local ref : 4247114cde1c43316940e2bb43b9ed9b6dd4e285
remote has: b19a35c85b5c63a08b6038af84863fcb653b51a4

$ bash scripts/sync/5remotes_sync.sh develop/v4.1.0 fake
ERROR: local fake/develop/v4.1.0 is 4247114cde1c43316940e2bb43b9ed9b6dd4e285
       but fake actually holds b19a35c85b5c63a08b6038af84863fcb653b51a4
       Refusing to sync: pushing the local value would rewind the
       branch on every remote. Fetch and re-run.
EXIT=5
```

未发出任何 push。

### 路径 3：fetch 失败

source remote URL 不可达时，`exit 5` 并说明原因，同样不推送。

### 变异验证 M-D：删除整个 stale-ref 守卫

从脚本副本中整段移除 `local_sha` / `actual_sha` 校验块（936 字节），
还原成修复前行为，并在副本中把 `GITEA_HOSTS` 改为不可达地址以隔离真实容器。

```
[develop/v4.1.0] source SHA = 4247114cde1c43316940e2bb43b9ed9b6dd4e285 (from fake)
  gitcode/develop/v4.1.0: pushing 4247114cde1c43316940e2bb43b9ed9b6dd4e285
  gitee/develop/v4.1.0: pushing  4247114cde1c43316940e2bb43b9ed9b6dd4e285
  github/develop/v4.1.0: pushing 4247114cde1c43316940e2bb43b9ed9b6dd4e285
```

守卫一删，陈旧 SHA 立即被当作目标并向 3 个 unprotected 远端发起推送 ——
这 3 个远端在真实拓扑里是**不设保护**的，推送会成功，分支即回退。

**M-D CAUGHT。** 对照路径 2 的 exit 5：校验有效时一推不发。

（变异运行中 3 次 push 报 failed，仅因隔离用的 probe clone 未配置这 3 个
remote，与守卫逻辑无关；要证明的「用哪个 SHA 去推」已如上可见。）