# 迭代 5 · 批次 24 —— 让 CI 第一次**真的**跑测试（#5508 / #5515 / #5516 / #5533 / #5534）

- **分支**：`fix/ci-truth-batch24`
- **基线**：`develop/v4.1.0` @ `378f014529`
- **合并进来的既有批次**：#5508/#5446/#5447/#5403（批次 19）、#5515（批次 23）、#5516（批次 20）
- **本批新增**：#5533（`needs` 使 job 从未被调度）、#5534（gate.yml 静默检出错误的树）

---

## 0. 一句话

**这个仓库的 CI 从来没有跑过测试套件。** 原因是 `ci.yml` 里 `test` job 的
`needs: [lint-build]` 在这台 Gitea 上会让它永远停在 `Waiting`；于是每一次
「CI 全绿」都是因为那个 job 被 **skipped**，不是因为测试通过。

本批把这个事实修掉，并且顺手拆掉了 CI 对 GitHub 的依赖。

---

## 1. 根因 A —— 声明了非空 `needs:` 的 job 从未被调度（#5533）

### 1.1 现象

`ci.yml` 的 `test` job（唯一真正跑测试套件的 job）带着：

```yaml
  test:
    runs-on: [hp-z6g4]
    needs: [lint-build]
```

在这台 Gitea（28.1.0）上，这个 job 的 `status` 一直是 **5（Waiting）**，
`task_id` 恒为 0，`started_at` 恒为 0 ⇒ **runner 从未接到过它**。

### 1.2 证据：查服务器自己的 job 表

Gitea 的 job 状态表 `action_run_job` 直接给出答案：

```sql
SELECT name, status, count(*), min(created) AS first_seen
  FROM action_run_job
 WHERE needs = '["lint-build"]'
 GROUP BY name, status;
```

| name | status | 含义 | 次数 | 首次出现 |
|---|---|---|---|---|
| test | **1** | **success** | **0** | — |
| test | 2 | failure | 29 | 1782642434 |
| test | 3 | cancelled | 641 | 1780012079 |
| test | 4 | skipped | 745 | 1780063089 |
| test | 5 | waiting | 1 | 1792658030 |

⇒ 从 2026-05-29 至今，**1452 个实例，0 次成功**。其中 745 次被标成
**skipped** —— 也就是说，CI 报「绿」是因为它**根本没跑**。

### 1.3 同一张表上的旁证

全表里 `needs` 非空的 job 共 3815 个，`needs` 为空的只有 44 个；
而 `["gate-baseline"]`（旧版本 ci.yml 里的依赖）有 157 次 success ——
说明 `needs` **本来是能工作的**，问题出在**这台** Gitea 的调度器上，
而出问题的只有 `ci.yml` 这条线。

### 1.4 为什么之前没人发现

`mutation` 与 `postcheck` 两个 job 早前（#5455）把 `needs: [test]` 改成了
`needs: []` + `if: always()`，当时的注释写着：

> the `test` job has never succeeded, so the mutation analysis never ran once

方向完全正确 —— 但**根因没被拔掉**：让 `test` 死掉的是 `test` 自己身上的
`needs: [lint-build]`。那两处改动只是让**下游**先跑起来，让上游的死
变得更刺眼而已。

### 1.5 修法

沿用这个仓库已经在用的写法：

```yaml
  test:
    runs-on: [hp-z6g4]
    needs: []
    if: always()
```

`test-crates`（批次 23 新增的成员包 job）同样带 `needs: [lint-build]`，
一起改掉。

**代价**：测试套件与 `lint-build` 并行启动，而不是在它之后启动。
**收益**：测试套件**真的会跑**。

本批之后，受门禁的两个 workflow（`ci.yml` / `gate.yml`）里**不再有任何
非空 `needs:`**。

---

## 2. 根因 B —— checkout 有两个缺陷（#5448 / #5532 / #5534）

### 2.1 缺陷 B1：每个 job 都要从 GitHub 拉一次 `actions/checkout`

`ci.yml` 的 7 个 job 全部写 `uses: actions/checkout@v4`。runner 因此在
**每个 job 的第一步**都要 clone `https://github.com/actions/checkout`。

CI run 8724 的日志把这件事写得明明白白：

```
09:15:00.7477082Z   ☁  git clone 'https://github.com/actions/checkout' # ref=v4
09:15:00.7477616Z   cloning https://github.com/actions/checkout to /root/.cache/act/c3fe249fe73091a17d6638…
```

GitHub 一抖动，整条流水线就在**跑第一行项目代码之前**死掉。实测两次
（run 8718 / 8719）：

```
fatal: could not read Username for 'https://github.com'
error: RPC failed; unexpected EOF
error: TLS handshake timeout
```

两次都是 `postcheck` 与 `test-soak-detach` 死在 checkout 上。这类红
**与代码无关**，却会让人以为 PR 有问题。

### 2.2 缺陷 B2：`gate.yml` 静默检出**错误的树**

`gate.yml` 的两个 job（`evidence-graph-gate` / `legacy-gate`）原来是这样
检出的：

```yaml
git clone --depth=1 "http://192.168.0.252:3000/$REPO" repo   # ← 默认分支
cd repo
git fetch --depth=1 origin "$SHA" 2>/dev/null || true        # ← 必然失败
git checkout "$SHA" 2>/dev/null || git checkout HEAD          # ← 静默留在默认分支
```

**浅克隆不能按 SHA 取对象**（refspec 少了 `+`）。实测：

```
$ git clone --depth=1 http://192.168.0.252:3000/openclaw/sqlrustgo.git repo
$ cd repo && git fetch --depth=1 origin d6b4fe97e0f13235aa936d31883558f2ec49d3db
fatal: couldn't find remote ref d6b4fe97e0f13235aa936d31883558f2ec49d3db
```

失败被 `2>/dev/null || true` 吞掉，兜底的 `git checkout HEAD` 让 job
**留在 `develop/v4.1.0` 上**，而 job 本身显示「成功」。

⇒ **evidence-graph-gate 分析的从来不是 PR 的那次提交，而是默认分支。**
这直接影响 #5447（该门禁用占位数据把自己写成 PASS）的可核查性：它声称
分析的是哪棵树，从来都不是本次的树。

同一个 fetch 加上 `+` 前缀就成立 —— 这正是 `actions/checkout` 一直在用的
refspec（run 8724 的日志里能看到它）：

```
git fetch --no-tags --prune --no-recurse-submodules --depth=1 origin +d6b4fe9…
```

### 2.3 修法

统一的 checkout 实现（`ci.yml` 7 处 + `gate.yml` 2 处，**逐字节相同**）：

* 只向 **Gitea 实例**克隆，不再碰 GitHub；
* 用 `+<sha>` refspec 取本次 run 的 commit（`CI_DEPTH=0` 时取全历史，
  供证据绑定门禁的 `git cat-file -t` 用）；
* 检出后**校验** `git rev-parse HEAD == 本次 run 的 SHA` ——
  检不出、或检出的是别的 commit，都是**硬失败**，绝不静默换一棵树。

workflow 步骤在检出之前拿不到仓库里的脚本，这段实现只能内联；
「一个规则只有一份实现」因此由**门禁**保证：

`scripts/gate/check_ci_checkout_consistency.sh`

| 规则 | 判据 |
|---|---|
| C1 | 任何 workflow 都不得再用 `uses: actions/checkout` |
| C2 | 受门禁的 workflow 里，每个 Checkout 步骤的 `run:` 正文必须与规范实现**逐字节相同** |
| C3 | `CI_DEPTH` 只允许 `"0"`/`"1"`；`CI_CHECKOUT_DIR` 只允许相对路径 |
| C4 | 豁免的 workflow 必须**带理由**——没有理由的豁免即 FAIL（静默跳过正是 P 规则禁止的反模式） |

`--selftest` 4 项（全部实测通过）：

1. 规范实现确实检出指定 commit；
2. 够不到的 commit 是**硬错误**，不是静默换一棵树；
3. 带 `actions/checkout` 的 workflow 被抓到；
4. 正文漂移被抓到。

### 2.4 端到端实测（对真实 Gitea，正文逐字节取自门禁里的规范实现）

| 场景 | 结果 |
|---|---|
| PR 头 `d6b4fe97e0`（depth=1） | rc=0，HEAD=`d6b4fe97e0` ✓ |
| develop 顶 `378f014529`（depth=1） | rc=0，HEAD=`378f014529` ✓ |
| develop 顶（depth=0） | rc=0，历史 24483 个 commit ✓ |
| 够不到的 commit（`0000…`） | rc≠0（硬失败）✓ |

---

## 3. 本批为什么把三个既有批次合进一个 PR

批次 19 / 20 / 23 各自都已推送并有 PR（#5524 / #5530 / #5521）。
本批把它们**合并**进一条分支，理由是可核查的：

1. **`test` job 此前从未运行** ⇒ 这三个 PR 的「CI 绿」都是空的。
   分三次合并就要付三次 CI 周期，而每次周期在没有 `test` 的情况下
   **证明不了任何东西**。
2. **单 runner、单 worker 队列** ⇒ 一个 CI 周期 ≈ 2–4 小时。
   三次合并 = 6–12 小时，而这三个批次合在一起只要一个周期。
3. 三者**修的是同一类问题**（CI 与门禁失明），并且互相依赖：
   批次 23 的证据绑定门禁依赖批次 19 带来的 provenance 修复
   （`5020bca409`），否则该门禁在 develop 上有 339 条假 FAIL。

合并后的分支结构（每个批次一个 merge commit，边界清晰）：

```
c878a9c7ae  fix(ci): needs: [lint-build] 让 test / test-crates 从未被调度（#5533）
64d18b6787  fix(ci): checkout 改为直接向 Gitea 克隆并校验 commit（#5448/#5532/#5534）
d52d4d85df  merge(batch20): #5516
16961179c9  merge(batch23): #5515
882f6b9590  merge(batch19): #5508/#5446/#5447/#5403
378f014529  ← develop 基线
```

### 3.1 合并时的一处冲突及其定性

`tests/common/bin_resolver.rs` 在合并批次 19 时冲突。两侧内容：

| 一侧 | 做法 |
|---|---|
| develop（批次 15 引入） | 每个用例只碰**自己专属**的环境变量名（`SELFTEST_ENGINE_ENV` / `SELFTEST_PSQL_ENV` / `SELFTEST_ABSENT_ENV`），根本不碰共享变量 |
| 批次 19 | 一把 `ENV_LOCK` + `save_env`/`restore_env`，把改共享变量的用例串行化 |

develop 那一侧是**更优解**（进程级变量是全局的，让每个用例只用自己的
变量名才是根治；串行化只是把冲突**推迟**）。批次 19 的观测记录
（`left: "/opt/homebrew/bin/psql"`）里的断言文本是 `SQLRUSTGO_PSQL_BIN`
—— 那是**旧版**代码的断言；develop 版断言的是
`SQLRUSTGO_RESOLVER_SELFTEST_PSQL_BIN`。也就是说，**那个竞态是在批次 15
合入 develop 之前观测到的**，合入之后已被根治。

⇒ 冲突取 develop 侧。批次 19 的「根因 H」提交因此在本分支上成为空提交。

---

## 4. 本地门禁实测（批次 24 树上）

| 门禁 | 结果 |
|---|---|
| `check_ci_checkout_consistency.sh --selftest` | EXIT=0（4/4） |
| `check_ci_checkout_consistency.sh` | EXIT=0（6 项全 OK） |
| `check_workflow_expressions.sh` | EXIT=0 |
| `check_drift_not_pass.sh` | EXIT=0 |
| `check_cli_binary_names.sh` | EXIT=0 |
| `check_test_manifest_honesty.sh` | EXIT=0 |
| `check_path_casefold_conflict.sh` | EXIT=0 |

`ci.yml` / `gate.yml` 的 YAML 解析实测通过（`python3 -c "yaml.safe_load(...)"`），
两个 workflow 的 job 清单与 `needs`/`if` 全部核对过。

---

## 5. 本批**没有**解决、且必须单独记账的

| 事项 | 状态 |
|---|---|
| `test` job 第一次真正运行后**会暴露什么** | **未知** —— 这正是本批要拿到的事实。已知的 12 个红目标里，8 个由批次 19/20/23 修掉；`g13_oltp1_concurrent_select_test` 是负载型环境阻塞；`tls_read_spin_5099` 读 `/proc`，Linux 上会绿 |
| `mutation` job 的 BASELINE 失败（rc=101，无可辨识错误输出） | **待查** —— 见 §6 |
| #5451（cargo registry 缓存） | 未做。runner 镜像已自带 registry，但本次复现时 `cargo test` 仍在 `Updating crates.io index` 上耗时间 |
| runner 并行度 | 未动。runner 配置 `capacity: 4`（`/opt/devstack/runner/config.yaml`），宿主机 80 核 / 404 GB，当前负载 58 |

---

## 6. `mutation` job 的 BASELINE 失败（未结案）

CI run 8724 的 `mutation` job 红：

```
09:17:09.7417663Z == BASELINE: the unmutated 'diag_q11' must pass, otherwise CAUGHT is meaningless
09:19:21.8913170Z ERROR: the BASELINE run of 'diag_q11' failed (rc=101) — the mutation results would be meaningless
```

基线跑了 132 秒后以 rc=101 退出，而脚本用来诊断的
`grep -E "^test .* FAILED|panicked at|error\[|^error"` **一条都没匹配上** ——
即 cargo 的输出里既没有 `error`，也没有 `panicked at`，也没有 `FAILED`。

**已排除**：本机用同一份 fixture 直接跑 `diag_q11` 的预编译二进制 ⇒
**16 passed / 0 failed**（`/tmp/diagq11_local.log`）。

**正在做**：在 runner 主机上用**同一个 CI 镜像**（`sqlrustgo-runner:v1`）
复现，拿到 cargo 的原始输出。

---

## 7. 身份与协作

- 身份串：`openclaw-mcode`（Mac Mini / Minimax-Code）
- 本批在协作台账 #5531 下的认领标记：`<!-- claim: openclaw-mcode b24-needs -->`、
  `<!-- claim: openclaw-mcode b24-checkout-tree -->`
- 涉及 Issue：#5508 #5446 #5447 #5403 #5515 #5516 #5448 #5532 #5447 #5533 #5534
- **只写 `Refs`，不写 `Closes`** —— 关单由合并后复跑 + 证据评论决定。

Refs #5508
Refs #5446
Refs #5447
Refs #5403
Refs #5515
Refs #5516
Refs #5448
Refs #5532
Refs #5533
Refs #5534
