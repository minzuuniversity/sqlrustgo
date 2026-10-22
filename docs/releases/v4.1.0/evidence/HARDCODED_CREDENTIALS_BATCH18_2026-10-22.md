# 迭代 5 · 批次 18 —— 凭据不再进仓库，5 远端同步工具真的能推进 gitea250（#5520）

> 分支：`fix/hardcoded-credentials-5520-batch18`
> 基点：`develop/v4.1.0` = `effa1bcabd7f37e0a1f853fd45176a5e949c8d2f`
> 关闭：#5520

---

## 0. 一句话

本批次把 #5520 的**前提订正**了（develop 上其实早就没有明文凭据），
然后做了三件真事：
**清洗 5 个公开可读的凭据**（develop 的文档 + 别的分支的代码）、
**把「查不到」和「落后」分开**并让同步工具**真的能推进 gitea250**、
**把已有的凭据门禁扩到文档并接进 CI**（它此前根本没接进任何 workflow）。

---

## 1. 前提订正：初版正文的两条断言都不成立

初版 #5520 是在**主工作区**上 grep 得出的结论，而那个工作区停在
**他人的分支** `fix/4597-chaos-t8d-awk-portability`（`9fbb63bfbc`）上，不是 develop。

在 develop（`effa1bcabd`）上实测：

```bash
$ grep -rInE "openclaw:details8848|cc19b2ad677e18c96b9d049f6dc2b46e02176883" \
      . --exclude-dir=archive --exclude-dir=target --exclude-dir=.git
(空)
```

develop 上这些脚本早已改好（**#5381** 跟踪）：

| 脚本 | develop 上的写法 |
|---|---|
| `scripts/sync/5remotes_sync.sh` | `: "${GITEA_USER:?…}"` + `: "${GITEA_TOKEN:?…}"` 硬失败；凭据走 `http.extraheader`，**不进 URL** |
| `scripts/push_q21_merge_to_gitea.sh` | `${GITEA_USER}:${GITEA_TOKEN}`，未设置即 `:?` 退出 |
| `scripts/stability/sync_252_when_ready.sh` | 同上 |
| `scripts/check_repo_sync.sh` | `${GITEA_TOKEN:?…}` |

⇒ 初版正文里「develop 上有明文」「同步工具因明文失效而完全不可用」两条**已删除**。

---

## 2. 真实缺陷 A：明文凭据在**公开可读**的地方

### 2.1 develop 的**文档**里仍有 5 个凭据

文档不是安全区 —— `develop/v4.1.0` 在 **public** 的 GitHub 镜像
（`yinglichina8848/sqlrustgo-public`）上，**匿名可读**。

实测（匿名、零凭据）：

```bash
$ curl -s "https://api.github.com/repos/yinglichina8848/sqlrustgo-public/contents/\
docs/releases/v3.12.0/SCOPE_TABLE_v3.12.md?ref=develop/v4.1.0" | base64 -d | grep details8848
  L151: | W4: comments posted | `curl -s -u openclaw:details8848 'http://localhost:3000/...'`
```

### 2.2 完整清单：**5 个凭据**，逐个实测有效性

| # | 凭据 | 形态 | 出现位置 | 有效性实测 |
|---|------|------|----------|-----------|
| 1 | `openclaw:details8848` | Gitea 口令 | develop 的 14 个文档 + **gitcode 分支的 3 个脚本** | **401（已失效）** |
| 2 | `cc19b2ad677e18c96b9d049f6dc2b46e02176883` | Gitea token | develop 的 2 个文档 | **401（已失效）** |
| 3 | `04bcda86dd601364a53eec33dc37aa6efa98a5b7` | Gitea token | develop 的 5 个文档 | **401（已失效）** |
| 4 | `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` | **GitHub PAT** | develop 的 1 个文档 | **401 "Bad credentials"** |
| 5 | `5518d1752693805a72096591a3cdfee718b6d049` | Gitea token | **只在 gitcode 分支的 `scripts/soak/*.py`（可执行代码！）** | **401（已失效）** |

> 第 3/4/5 个是**本批次用新门禁挖出来的** —— 前两个在初版 issue 里就被发现了。

⇒ **五个全部已经失效**（等价于已轮换）。
⇒ 暴露窗口需要人工确认（§6 验收标准第 5 条）。

### 2.3 门禁为什么没挡住：它**根本没接进 CI**

```bash
$ grep -rn "check_no_hardcoded_secrets" .gitea/workflows/
(空)
```

`scripts/gate/check_no_hardcoded_secrets.sh` 存在、也写得不错（#5381 建的），
但**没有任何 workflow 调用它** ⇒ 它从来不会红 ⇒ 文档里的凭据一住就是好几个月。

而且它只扫 `scripts tools config .gitea`，**不扫 `docs/`** ——
而本次挖到的 4 个都在文档里。

---

## 3. 真实缺陷 B：同步工具**无法推进 gitea250**，且可能静默 exit 0

### 3.1 实测：250 对当前凭据全部 401

同一套凭据（本机 credential store 里那套，实测对 252 有效）打到两台 Gitea：

| 端点 | 带 Basic 认证 | 匿名 |
|---|---|---|
| `192.168.0.252:3000` API ref 查询 | **200** | 200 |
| `192.168.0.250:3000` API ref 查询 | **401** | **200** |
| `192.168.0.250:3000` git-upload-pack（push 握手） | **401** | — |

⇒ **250 上根本没有可用的 HTTP 凭据**（匿名能读，匿名不能推）。

### 3.2 于是脚本在 250 上走的是「跳过」分支

```
① current=$(curl ... -H "$auth_header" .../git/refs/heads/$branch)   ← 250 上 401 ⇒ current 为空
② if [ "$current" = "$sha" ]; then ... continue; fi                    ← 空 ≠ sha ⇒ 不跳过
③ git -c http.extraheader=... push "$sha:refs/heads/$tmp_ref"          ← 250 上 401 ⇒ 失败
④     echo "    ERROR: temp-ref push to $host failed"; continue         ← ★ 直接 continue
⑤ ssh ... git update-ref refs/heads/$branch $sha                       ← ★ 永远执行不到
```

⇒ **第 ④ 步的 `continue` 把第 ⑤ 步整个跳过了 ⇒ gitea250 永远不会被这个脚本推进。**

### 3.3 而它仍然可能 exit 0（静默假绿）

修复前在 develop 树上的实测：

```
  [192.168.0.250 / 4bfae0aa5d24 / /git/openclaw/sqlrustgo.git]
    develop/v4.1.0: current=, target=effa1bcabd7f
fatal: could not read Username for 'http://192.168.0.250:3000': Device not configured
    ERROR: temp-ref push to 192.168.0.250 failed
  ...
  ✓ all 10 pairs consistent
SYNC_EXIT=0
```

⇒ 250 上的推送**失败了**，脚本却报「all 10 pairs consistent」并 **exit 0**。
本例之所以仍然一致，只是因为 250 之前已被别的路径推进过。

---

## 4. 修法

### 4.1 凭据解析：一个实现，三个脚本共用

**新** `scripts/sync/gitea_credential_common.sh`：

| 规则 | 内容 |
|------|------|
| C1 | 来源优先级：环境变量 `GITEA_USER`/`GITEA_TOKEN` > 本机 **git credential store**（`git credential fill`） |
| C2 | 凭据**绝不**出现在 URL 里（`http://user:pass@host` 会进 `.git/config`、进程列表、CI 日志） |
| C3 | 取不到凭据时**明确失败**，绝不回落到任何写死的值 |
| C4 | 任何函数都**不得**把凭据打印到 stdout/stderr |

「一个规则只能有一份实现」：三个脚本过去各自内嵌了一份明文 ⇒ 现在共用这一份。

### 4.2 同步工具：区分「查不到」与「落后」，并加一条不依赖 HTTP 凭据的路

**探测**：先带凭据探，失败则**匿名**探（250 的仓库匿名可读）。
两次都失败 ⇒ **明确报错并置 `ALL_FAILED`**，不再把「未知」当成「落后」。

**对象传输**：HTTP 推临时 ref 失败 ⇒ **回退到 bundle over SSH**
（`git bundle` 经 SSH 灌进容器 ⇒ `git fetch` ⇒ `update-ref`）——
这条路**完全不需要 HTTP 凭据**，而 250 的 SSH 通道是通的。

> 实测中踩到的三个坑，都已写进代码注释：
> 1. `git bundle create` 的**头部必须是 ref 名** —— 用裸 SHA 会说
>    `Refusing to create empty bundle`；
> 2. 接收端必须**显式点名 bundle 的头 ref**
>    （`git fetch <bundle> <head-ref>`）——
>    `git fetch <bundle>` 会去找一个 bundle 里没有的 `HEAD`，
>    `git fetch <bundle> FETCH_HEAD` 会把 `FETCH_HEAD` 当成待取的 ref；
> 3. `bundle_ref` 用了每次运行唯一的名字（`sync-transfer-head-$$`），
>    避免并发运行互相覆盖。

**失败不再被吞掉**：对象搬运失败 ⇒ `ALL_FAILED=1` ⇒ 退出码 2，
不再出现「推失败了但报一致」。

### 4.3 门禁：从「只扫代码、没接 CI」变成「扫文档 + 接 CI + 能自证」

**扩展既有实现**（`scripts/gate/check_no_hardcoded_secrets.sh`），不另写一份：

| 改动 | 为什么 |
|------|-------|
| 扫描路径加入 `docs/` | 4/5 个凭据在文档里 |
| 扫描 2 泛化：字面量的「名字:口令」对 | 原规则只认某一个口令的形状，换个口令就漏 |
| 扫描 3 排除 `<…>` 占位符 | `<user>:<password>` 是**正确**写法，不能误伤 |
| 扫描 4 排除 `<…>` | 同上 |
| 新增 `--repo-root` | 凭据可能留在**别的分支**上 —— 那在当前检出的树上根本看不见 |
| 新增 `--selftest` | 门禁必须能证明自己会红 |
| **接入 CI** | 它此前**从未**被任何 workflow 调用 |

### 4.4 清洗

| 范围 | 处理 |
|------|------|
| develop 的 19 个文档文件 | 凭据字面量 → `${GITEA_USER}:${GITEA_TOKEN}` / `${GITEA_TOKEN}` / `${GITHUB_TOKEN}`；形状占位符 → `<user>:<password>` |
| `#5380` 证据里的「注入形态」表 | 改成 `<user>:<password>`，**并加一条清洗说明**（表格结论一字未改，只是不再复述凭据） |
| gitcode 分支上的第 5 个凭据 | **不动** —— 那是他人的分支，动手会打断别的工作流；已登记在 §2.2 与 §6 |

---

## 5. 判别力

### 5.1 门禁自检 9/9

```
=== 自检: 注入已知泄露形态 ===
  PASS: 干净脚本 (期望 PASS，实际 PASS)
  PASS: 凭据变量 ← 十六进制字面量 (期望 FAIL，实际 FAIL)
  PASS: curl 字面量基本认证 (期望 FAIL，实际 FAIL)
  PASS: URL 内嵌凭据 (期望 FAIL，实际 FAIL)
  PASS: 字面量 名字:口令 对 (期望 FAIL，实际 FAIL)
  PASS: 文档里的字面量也被抓住
  PASS: 环境变量引用 (期望 PASS，实际 PASS)
  PASS: host:port (期望 PASS，实际 PASS)
  PASS: git@host SSH URL (期望 PASS，实际 PASS)
```

### 5.2 负对照：对**含明文的分支**扫描 ⇒ 必须红

```
$ bash scripts/gate/check_no_hardcoded_secrets.sh --repo-root <gitcode 分支的 scripts+docs>
FAIL — 凭据名变量被赋了长十六进制字面量:
  scripts/soak/mysql_ladder_soak.py:23:GITEA_TOKEN = "5518d1752693805a72096591a3cdfee718b6d049"
  scripts/soak/long_running_soak.py:23:GITEA_TOKEN = "5518d1752693805a72096591a3cdfee718b6d049"
  docs/superpowers/plans/2026-08-15-q21-chain-order-assert-fix.md:549:TOKEN=04bcda86...
FAIL — 发现字面量的「名字:口令」对:
  scripts/push_q21_merge_to_gitea.sh:36:  curl -s -u "openclaw:details8848" -X POST \
  scripts/stability/sync_252_when_ready.sh:18:AUTH="openclaw:details8848"
  scripts/sync/5remotes_sync.sh:261:            -u "openclaw:details8848" \
  ...
```

⇒ **同一个门禁，在清洗过的 develop 上绿，在含明文的分支上红。**
（这也是第 3/4/5 个凭据的发现方式。）

### 5.3 同步工具的端到端判别力：**250 被真的推进了**

**步骤 1** 故意把 250 的 `develop/v4.0.0` 置后一个提交：

```
250 现在 = ee8593afdeb3   目标 = ac3fa16afb4a
```

**步骤 2** 跑同步脚本（只针对 250，且**不导出任何环境变量**）：

```
Step 3: SSH container update-ref for gitea250 / gitea252
  [192.168.0.250 / 4bfae0aa5d24 / /git/openclaw/sqlrustgo.git]
    develop/v4.0.0: current=ee8593afdeb3, target=ac3fa16afb4a (probe: anonymous)
    HTTP temp-ref push to 192.168.0.250 failed — carrying the objects by a bundle over SSH instead
final: ac3fa16afb4a388fb3be09fee3a7e2b7e23e1a0c

  gitea250 = ac3fa16afb4a
  ✓ all 10 pairs consistent
SYNC_EXIT=0
```

**步骤 3** 复核：

```bash
$ git ls-remote gitea250 refs/heads/develop/v4.0.0
ac3fa16afb4a...
```

⇒ **修复前**：exit 0，但 250 **没被推进**（静默假绿）。
⇒ **修复后**：exit 0，且 250 **真的被推进了**。

### 5.4 幂等 + 无环境变量

```bash
$ env -u GITEA_USER -u GITEA_TOKEN bash scripts/sync/5remotes_sync.sh develop/v4.1.0
  [192.168.0.250 / …]  develop/v4.1.0: already at effa1bcabd7f ✓ (probe: anonymous)
  [192.168.0.252 / …]  develop/v4.1.0: already at effa1bcabd7f ✓ (probe: authenticated)
  ✓ all 10 pairs consistent
SYNC_EXIT=0
```

⇒ 凭据**只来自 credential store**（未导出任何环境变量）也能工作；
250 走匿名探测、252 走认证探测，两条探测路径都被走到。

### 5.5 仓库自带的测试与变异测试仍然全绿

```
$ bash scripts/sync/test_5remotes_sync_ssh.sh
Total: PASS=11  FAIL=0
✅ SSH 通道健壮

$ bash scripts/sync/mutate_5remotes_sync_ssh.sh
MUT CAUGHT=3  SURVIVED=0
```

> ⚠️ 过程中确实**引入过**一次回归：自检首轮 `PASS=8 FAIL=3` ——
> 原因是测试只 copy 被测脚本，**没 copy 新增的共用 helper** ⇒
> `source` 失败 ⇒ 凭据解析函数未定义 ⇒ 主机被跳过。
> 修法有两处：测试补上 copy，脚本在 helper 缺失时**明确报错退出**（fail-fast），
> 而不是静默地「看起来像没有凭据」。
> ⇒ 这也印证了「门禁必须自带判别力」：仓库自带的测试确实能抓住我的回归。

---

## 6. 本批次的验证结果

| 项 | 实测 |
|---|---|
| `scripts/gate/check_no_hardcoded_secrets.sh --selftest` | **9/9 PASS** |
| `scripts/gate/check_no_hardcoded_secrets.sh`（清洗后的 develop） | **PASS**（4 条扫描全绿） |
| 同上，`--repo-root` 指向含明文的分支 | **FAIL**，报出 5 个凭据的位置 |
| `scripts/sync/test_5remotes_sync_ssh.sh` | **PASS=11 FAIL=0** |
| `scripts/sync/mutate_5remotes_sync_ssh.sh` | **MUT CAUGHT=3 SURVIVED=0** |
| `bash -n scripts/sync/5remotes_sync.sh` | 语法 OK |
| `scripts/gate/check_workflow_expressions.sh`（改后的 ci.yml） | **PASS**（7 个 workflow 全部可解析） |
| 同步工具（5 远端已一致，不导出环境变量） | **exit 0**，10 对一致 |
| 同步工具（250 故意落后） | **exit 0**，250 **真的被推进**到目标 SHA |
| 5 个凭据的有效性 | 全部 **401**（已失效） |
| 编译/测试影响 | 本批次**只改 shell 脚本与 markdown**，不触碰任何 Rust 代码 ⇒ 无编译/测试影响 |

---

## 7. 边界：本批次**没有**做的事

| 没做 | 为什么 | 去哪 |
|---|---|---|
| 没有清洗 gitcode 分支上的第 5 个凭据 | 那是**他人的分支**（主工作区正停在它上面）；推 commit 过去会打断别人的工作流 | 已登记在 §2.2 与 issue #5520；该分支的属主可用 `--repo-root` 自查 |
| 没有改写 git 历史 | 凭据已在历史里；改写历史代价与风险都远大于收益 | 凭据**已全部失效** ⇒ 轮换即是彻底的处置 |
| 没有删除 `check_no_hardcoded_secrets.sh` 另写一份 | 「一个规则只能有一份实现」⇒ 扩展既有实现 | — |
| 没有为 `push_q21_merge_to_gitea.sh` / `sync_252_when_ready.sh` 接入共用 helper | 它们只用到「用户:口令」形态，没有 250 那种「凭据不可用」场景；接入会扩大本批次的改动面 | 后续批次（如再有重复） |
| 没有碰批次 11 的引擎缺陷 | 另一条线（R2 回归待跑） | 批次 11 |

---

## 8. 引用

- `scripts/sync/gitea_credential_common.sh` —— 本批次**新增**的凭据解析唯一实现
- `scripts/sync/5remotes_sync.sh` —— 探测二段式 + bundle over SSH 兜底 + 失败不再被吞
- `scripts/sync/test_5remotes_sync_ssh.sh` —— 补 copy 共用 helper（自检首轮的红来自这里）
- `scripts/gate/check_no_hardcoded_secrets.sh` —— 扩展：扫文档 / `--repo-root` / `--selftest`
- `.gitea/workflows/ci.yml` —— 该门禁**首次**接入 CI（`lint-build` job）
- `docs/releases/v4.1.0/evidence/SYNC_SCRIPT_5380_2026-10-20.md` —— 加了清洗说明
- #5520 —— 本批次关闭的 issue
- #5381 —— 之前那次凭据泄露的跟踪 issue（develop 上的代码修复出自它）
