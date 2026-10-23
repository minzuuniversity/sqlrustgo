# 5 远端同步工具：失败不再被吞，凭据不再进进程列表（#5520 · 批次 19）

> 分支：`omp-macmini/fix-5520-sync-tools-strict`
> 基点：`develop/v4.1.0` = `3a3350f660`
> 关闭：#5520（**不 merge** —— PR 由人审）

---

## 0. 一句话

批次 18 已经把「凭据不再进仓库」「同步工具真的能推进 gitea250」做掉了；
本批次补上**剩下的一半**：**失败被吞掉的地方**，
以及**凭据仍然出现在进程列表（argv）里**的地方 ——
并把「远端没有这个分支」与「我查不到」这两件事**彻底分开**。

---

## 1. 缺陷 A（凭据在公开分支上可读）：本批次的实测结论

### 1.1 可读性：确认（匿名，零凭据）

```bash
$ git ls-remote https://gitcode.com/BreavHeart/sqlrustgo.git \
      refs/heads/fix/4597-chaos-t8d-awk-portability
9fbb63bfbc0bc5221bdd762e281e937b27be86afd0	refs/heads/fix/4597-chaos-t8d-awk-portability
（exit 0 —— 匿名可读）

$ git clone --depth 1 --branch fix/4597-chaos-t8d-awk-portability \
      https://gitcode.com/BreavHeart/sqlrustgo.git gitcode-5520
（exit 0 —— 匿名克隆成功）
```

克隆下来的树上仍有 **5 处**明文凭据（4 个脚本）：

```
scripts/push_q21_merge_to_gitea.sh:36
scripts/check_repo_sync.sh:21
scripts/stability/sync_252_when_ready.sh:18
scripts/sync/5remotes_sync.sh:261
scripts/sync/5remotes_sync.sh:275
```

> **本批次没有改写 gitcode 上的任何历史**（那是 force push，本任务明令禁止，
> 且那条分支属于别人的工作流）。处置方式见 §5。

### 1.2 暴露窗口（实测，不是推测）

在匿名克隆里用 `git log -S` 定位引入提交：

| 凭据 | 引入提交 | 引入时间 | 仍可读的公开分支 |
|------|----------|----------|------------------|
| Gitea 口令 | `d7da931f09` `scripts: add push_q21_merge_to_gitea.sh recovery script` | **2026-06-09 20:24:29 +0800** | `fix/4597-chaos-t8d-awk-portability`（实测） |
| Gitea token(40hex) | `0a6f285f19` `ci(v400): add file-extension gate + repo-sync monitor` | **2026-09-10 01:46:57 +0800** | 同上 |

### 1.3 有效性（2026-10-23 实测）

```
192.168.0.252:3000  历史口令  → 401      历史 token → 401
192.168.0.250:3000  历史口令  → 401      历史 token → 401
192.168.0.252:3000  匿名读仓库 → 200
192.168.0.250:3000  匿名读仓库 → 200
本机 git credential store（username=token）→ 对 252 的 /api/v1/user 200
```

⇒ **两个历史凭据已全部失效**（等价于已轮换），且**现役凭据只在本机
credential store 里**，仓库内没有副本。
⇒ 暴露窗口 = [引入提交, ≤2026-10-22]（批次 18 首次实测 401；本批次 2026-10-23 复测仍 401）。
⇒ **「失效期间是否被使用过」无法从公开侧判定** —— 需要 Gitea 服务端的
访问日志（252/250 的 `gitea.log`）才能回答。本批次**没有**该权限，
因此**不能排除被使用** ⇒ 按「已泄露」处理（见 §5）。

---

## 2. 缺陷 B（同步工具吞错）：本批次定位到的**具体**吞错点

批次 18 已经修好了「临时 ref 推失败 ⇒ 直接 continue ⇒ 250 永远不被推进」。
本批次在**剩下的路径**上继续找，找到 4 处：

### 2.1 校验阶段比的是**本地 tracking ref**，不是服务器

```sh
# 修复前
sha=$(git rev-parse --verify "$r/$branch" 2>/dev/null | cut -c1-12)
```

本地那份可能过期 ⇒ 一个**根本没被推进**的远端也可能被判成「一致」。
更要命的是：远端**根本没有这条分支**时，`git rev-list --count` 报错，
`ab`/`ba` 变成空串，`[ "" -ne 0 ]` 抛 *integer expression expected*，
于是这一对**不被标记为分歧** —— 输出照样打印
「✓ all 10 pairs consistent」。

> 实测（对着修复前的 develop 跑离线用例）：
> `scripts/sync/5remotes_sync.sh: line 563: [: : integer expression expected`
> … `✓ all 10 pairs consistent`

### 2.2 「远端没有这个分支」被当成「我查不到」

探测用 `curl -sf`：**404 也变成「无输出」**，
于是「分支不存在」和「401/网络错误」塌成了同一个信号 ——
脚本要么把「不存在」当成错误直接退出（**建不了新分支**），
要么把「查不到」当成「落后」。

### 2.3 凭据在 **argv** 里

```sh
curl -sf -H "$auth_header" …                    # 凭据在 curl 的 argv
git -c "http.extraheader=$auth_header" push …   # 凭据在 git 的 argv
```

`ps aux` 能直接读到 ⇒ 违反 #5520 的验收项
「凭据全程不出现在进程列表 / .git/config / 日志里」。

### 2.4 另外两个脚本各自为政

`push_q21_merge_to_gitea.sh` / `sync_252_when_ready.sh` /
`check_repo_sync.sh` 仍然：

- 只认 `GITEA_USER`/`GITEA_TOKEN` 两个环境变量（**没有** credential store 兜底）；
- `git push … | tail -N` / `curl … | python3 …` —— **退出码是管道最后一环的**，
  失败被吞；
- `check_repo_sync.sh` 把 401（凭据被拒）当成「UNREACHABLE (network issue)」
  **且不计入失败** ⇒ 只剩 .250 可达时它会报 **PASS**；
- `push_q21_merge_to_gitea.sh` 第一行 `cd /Users/liying/workspace/dev/openheart/…`
  —— 那条路径在这台机器上不存在，脚本**每次都在第一行就死掉**。

---

## 3. 改法（一份凭据实现 + 三个脚本共用）

| 改动 | 文件 |
|------|------|
| 凭据解析**唯一实现**扩展两条「argv 里没有凭据」的通道：<br>`gitea_credential_curl_config_file()`（curl `-K`）<br>`gitea_credential_git_config_file()`（git `-c include.path=`） | `scripts/sync/gitea_credential_common.sh` |
| 凭据**在任何远端操作之前**解析，取不到 ⇒ **fail-fast 退出 2** | `scripts/sync/5remotes_sync.sh` |
| 探测二段式升级为**三态**：<br>`200`=分支存在 / `404`=分支不存在（**会被建出来**）/ 其它=**查不到**（明确报错） | 同上 |
| 校验阶段改为**逐个远端直接问服务器**（`ls-remote`），并区分<br>「问不到」(exit 3) / 「远端没有这条分支」(exit 1) / 「SHA 不同」(exit 1) | 同上 |
| 校验读的是**远端回读到的 SHA**（`git rev-parse`），不是 `cat` 出来的 loose ref 文件 | 同上 |
| 未受保护远端的推送：`current` 也改成**问服务器**，并用 `--force-with-lease=<ref>:<实测值>` | 同上 |
| 失败传播回归测试（**新增**） | `scripts/sync/test_5remotes_sync_strict.sh` |
| stub curl 改成**忠实模拟 200/404**（原来恒返回同一个 SHA ⇒ 「分支不存在」这条路径永远测不到） | `scripts/sync/test_5remotes_sync_ssh.sh` |
| 三个脚本接入共用凭据实现；`push` / `curl` 的失败一律**非 0 退出**；修掉写死的 `cd` | `scripts/push_q21_merge_to_gitea.sh`、`scripts/stability/sync_252_when_ready.sh`、`scripts/check_repo_sync.sh` |
| `check_repo_sync.sh`：401/403 与网络错误**都算失败**；分支不存在单列；分支清单补上 `develop/v4.1.0` | 同上 |
| 三个脚本都支持 `GITEA_252_URL` / `GITEA_250_URL` 覆盖（离线可测） | 同上 |

---

## 4. 验证证据

### 4.1 语法

```
$ for f in <7 个改动脚本>; do bash -n $f; done
scripts/sync/5remotes_sync.sh                     bash -n OK
scripts/sync/gitea_credential_common.sh           bash -n OK
scripts/sync/test_5remotes_sync_ssh.sh            bash -n OK
scripts/sync/test_5remotes_sync_strict.sh         bash -n OK
scripts/stability/sync_252_when_ready.sh          bash -n OK
scripts/check_repo_sync.sh                        bash -n OK
scripts/push_q21_merge_to_gitea.sh                bash -n OK
```

### 4.2 失败传播（本批次新增的测试）

```
$ bash scripts/sync/test_5remotes_sync_strict.sh
[T1] ssh update-ref 失败 ⇒ 必须非 0 退出（不得静默通过）
  ✅ PASS: ssh update-ref 失败 ⇒ exit 2（非 0）
  ✅ PASS: 输出点名了失败原因（ERROR: ssh update-ref …）
[T2] 有远端回答不了 ⇒ 必须非 0 退出（不能判成「一致」）
  ✅ PASS: 有远端读不到 ⇒ exit 3（非 0）
  ✅ PASS: 输出点名了读不到的远端
[T3] 没有可用凭据 ⇒ 必须明确报错并非 0 退出
  ✅ PASS: 没有凭据 ⇒ exit 2（非 0）
  ✅ PASS: 输出明确说了「没有可用凭据」
[T4] 凭据不得出现在进程列表（argv）里
  ✅ PASS: argv 里没有凭据原文
  ✅ PASS: argv 里没有凭据的 base64 形态
  ✅ PASS: .git/config 里没有凭据
[T5] 远端**没有这个分支** ⇒ 必须把它建出来（不是「查不到」）
  ✅ PASS: 分支不存在 ⇒ 被建出来并推进到目标 SHA（exit 0）
  ✅ PASS: 输出明确区分了「分支不存在」与「查不到」
Total: PASS=10  FAIL=0
```

### 4.3 判别力：同一套测试跑**修复前**的脚本 ⇒ 必须红

```bash
$ git show 3a3350f660:scripts/sync/5remotes_sync.sh > /tmp/oldtest/scripts/sync/5remotes_sync.sh
$ bash /tmp/oldtest/scripts/sync/test_5remotes_sync_strict.sh
[T4]  ❌ FAIL: 凭据（base64 形态）出现在 argv 里
       CURL_ARGV: -sf --max-time 5 -H Authorization: Basic c3R1Yi11c2VyOlNUUklDVC1URVNULVRPS0VOLTlkM2Y3YTJi http://192.168.0.250:3000/…
[T5]  ❌ FAIL: 分支不存在时没有把它建出来
       rc=2, 250 现在的 SHA=refs/heads/develop/v4.1.0, 目标=2ffc9989…
       scripts/sync/5remotes_sync.sh: line 563: [: : integer expression expected
       ✓ all 10 pairs consistent
Total: PASS=7  FAIL=4
```

⇒ 修复前的脚本：**凭据在 argv 里**，**分支不存在时建不出来**，
而且照样打印「✓ all 10 pairs consistent」。

### 4.4 仓库自带的回归 + 变异测试

```
$ bash scripts/sync/test_5remotes_sync_ssh.sh
Total: PASS=11  FAIL=0

$ bash scripts/sync/mutate_5remotes_sync_ssh.sh
MUT CAUGHT=3  SURVIVED=0
```

### 4.5 凭据门禁（develop 树）

```
$ bash scripts/gate/check_no_hardcoded_secrets.sh
=== 硬编码凭据扫描 (形状匹配, 非字面量) ===
PASS: 没有凭据名变量 ← 长十六进制字面量
PASS: 没有字面量的「名字:口令」对
PASS: curl 没有使用字面量凭据
PASS: 没有凭据被嵌进 URL
✅ 硬编码凭据检查通过
```

### 4.6 三个脚本的失败传播（离线，本地 stub API）

```
# push_q21：PR 创建失败（stub 返回 500）⇒ 非 0
$ GITEA_252_URL=http://127.0.0.1:18099 … bash scripts/push_q21_merge_to_gitea.sh
ERROR: PR creation failed (HTTP 500)
FAILED: one or more steps failed — see above. Exiting non-zero on purpose.
PUSH_Q21_EXIT=1

# push_q21：成功路径 ⇒ 0
PR #999 created: http://stub/pr/999
DONE
PUSH_Q21_EXIT=0

# push_q21：没有凭据 ⇒ 2
ERROR: no Gitea credential available for 192.168.0.252:3000.
PUSH_Q21_EXIT=2

# sync_252_when_ready：评论失败（stub 返回 500）⇒ 非 0
ERROR: Comment on #3531 failed (HTTP 500)
ERROR: a partially-synced 252 must not be reported as a completed sync.
STABILITY_EXIT=1

# sync_252_when_ready：没有凭据 ⇒ 2
STABILITY_NOCRED_EXIT=2

# check_repo_sync：没有凭据 ⇒ 3
ERROR: no Gitea credential available for 192.168.0.252:3000.
EXIT=3
```

### 4.7 真机只读验证：`check_repo_sync.sh`（不推任何东西）

```
$ bash scripts/check_repo_sync.sh
--- develop/v4.1.0 ---
  .252:        3a3350f660
  .250:        878e1d62c3
  gitcode:     775dc5da72
  github:      775dc5da72
  ⚠️  divergence: 878e1d62c3 vs 3a3350f660
  ⚠️  divergence: 775dc5da72 vs 3a3350f660
…
=== Summary ===
  Total checks:  15
  Failures:      4
❌ FAIL: 4 sync divergence detected
EXIT=1
```

⇒ 5 个远端**此刻确实不一致**（`develop/v4.1.0` 与 `main`），
本工具如实报出来 —— 这正是「不允许静默假绿」的意义。
（本批次**没有**去推 250/gitcode/gitee/github：那是别人的远端，
且本任务明令禁止。）

---

## 5. 边界：本批次**没有**做的事

| 没做 | 为什么 |
|------|--------|
| 没有清洗 gitcode 分支上的明文凭据 | 那是他人的分支；清洗要 force push，本任务明令禁止 |
| 没有改写任何 git 历史 | 同上；且凭据已全部失效 ⇒ 轮换即是彻底的处置 |
| 没有真正把 gitea250 推到目标 SHA | 本任务明令禁止向 250 推送。**真机验证需要谁来做**：<br>① 跑 `bash scripts/sync/5remotes_sync.sh develop/v4.1.0`（默认源 = gitea252），<br>它会走「HTTP 临时 ref 失败 ⇒ bundle over SSH ⇒ update-ref」这条路把 250 推进；<br>② 需要能 SSH 到 `liying@192.168.0.250` 并在其中 `docker exec` 的权限（本机有）。<br>本批次已用**离线 stub** 等价验证了这条路径（bundle 兜底 + update-ref + 回读校验 + 失败非 0）。 |
| 没有动 crates/ / workflow | 本批次只改 shell 脚本与 markdown ⇒ 无编译/测试影响 |
| 没有另写一份凭据门禁 | 复用既有 `scripts/gate/check_no_hardcoded_secrets.sh`（已支持 `--repo-root` 对别的分支体检） |

### 轮换结论（§4.C 的答复）

- 两个历史凭据 **2026-10-23 实测 401** ⇒ 等价于已轮换，**不需要**再轮换；
- 但**「失效期间是否被使用过」无法从公开侧排除**
  ⇒ 保守起见按「已泄露」处置：建议在 252/250 的 Gitea 服务端日志里
  按那两个用户名/凭据做一次回溯（这是**人工**动作，本批次无权做）；
- 公开分支上的历史残留：建议由该分支属主用
  `bash scripts/gate/check_no_hardcoded_secrets.sh --repo-root <该分支的工作树>`
  自查并自行处置。

---

## 6. 引用

- `scripts/sync/gitea_credential_common.sh` —— 凭据解析唯一实现（本批次加了 argv-free 两条通道）
- `scripts/sync/5remotes_sync.sh` —— 三态探测 + 逐远端直查 + 失败不吞
- `scripts/sync/test_5remotes_sync_strict.sh` —— **本批次新增**的失败传播回归测试
- `scripts/sync/test_5remotes_sync_ssh.sh` —— stub curl 改成忠实模拟 200/404
- `scripts/push_q21_merge_to_gitea.sh` / `scripts/stability/sync_252_when_ready.sh` / `scripts/check_repo_sync.sh` —— 接入共用凭据实现，失败一律非 0
- `docs/releases/v4.1.0/evidence/HARDCODED_CREDENTIALS_BATCH18_2026-10-22.md` —— 批次 18 的证据（凭据清洗 + 门禁接入 CI）
- #5520 —— 本批次关闭的 issue
