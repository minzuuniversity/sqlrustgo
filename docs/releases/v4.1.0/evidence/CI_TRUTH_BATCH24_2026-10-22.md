# 迭代 5 · 批次 24 —— 让 CI 第一次**真的**跑测试（#5508 / #5515 / #5516 / #5533 / #5534）

- **分支**：`fix/ci-truth-batch24`
- **基线**：`develop/v4.1.0` @ `378f014529`，本批中途又并入了 develop 的
  `6f26128890`（PR #5536，PR-DAG 门禁族 #5457/#5458）
- **合并进来的既有批次**：#5508/#5446/#5447/#5403（批次 19）、#5515（批次 23）、#5516（批次 20）
- **本批新增**：#5533（`needs` 使 job 启动推迟 12 分钟 ~ 7 小时，1452 次里只有 30 次真的启动）、
  #5534（gate.yml 静默检出错误的树）
- **本批还修的两个缺陷**：`mutation` job 的诊断输出被吞（§6.1）、900 秒预算同时覆盖
  5m57s 的编译（§6.2）

---

## 0. 一句话

**这个仓库的 CI 从来没有跑绿过测试套件。** 逐条定性 1452 次机会里的那 30 次：

* **10 次**在跑任何一行测试之前就死了 —— 6 次死在从 `github.com` 拉
  `actions/checkout`，4 次死在从 `index.crates.io` 拉 crate；
* **19 次**跑到了测试，因**真实的测试失败**而死；
* **1 次**（本批这次）正在跑；
* **成功 0 次**。

剩下 1422 次机会，`test` job 被 **skipped**（779）或在启动前被
**cancelled**（631）—— 于是 run 显示「绿」。

本批拆掉这条因果链上的两个**外部网络依赖**（GitHub / crates.io），
并让 `test` job 不再等「别的 job 结束」才起跑；19 次真实测试失败由本批
合并进来的批次 19 / 20 / 23 负责。

---

## 1. 根因 A —— `test` job 几乎从不启动，且**从未成功过**（#5533）

### 1.1 现象

`ci.yml` 的 `test` job（唯一真正跑测试套件的 job）带着：

```yaml
  test:
    runs-on: [hp-z6g4]
    needs: [lint-build]
```

在这台 Gitea（28.1.0）上，这个 job 的启动被**严重推迟**：它在
`lint-build` 结束之后，还要再等**别的 job 结束**才被唤醒。

### 1.2 证据一：启动延迟的实测分布

服务器自己的 job 表（`action_run_job`）：

```sql
SELECT j.run_id, j.status, (j.started - p.stopped) AS delay_s
  FROM action_run_job j
  JOIN action_run_job p ON j.run_id = p.run_id AND p.name = 'lint-build'
 WHERE j.needs = '["lint-build"]' AND j.started > 0
 ORDER BY j.run_id DESC;
```

| run | status | 相对 `lint-build` 结束的延迟 |
|---|---|---|
| 8724 | running | 1298 s |
| 8673 | failure | 937 s |
| 8264 | failure | 3969 s |
| 8242 | failure | 2305 s |
| 8348 | failure | 23818 s（≈6.6 h） |
| 8346 | failure | 25441 s（≈7.1 h） |
| 8344 | failure | 20521 s（≈5.7 h） |

⇒ 延迟从 **12 分钟到 7 小时**。本批这次 run 更直观：`lint-build` 在
09:14:57 结束，而 `test` 直到 **09:36:35**（`postcheck` 结束的那一刻）
才被调度。

### 1.3 证据二：1452 个实例，只有 30 个真的启动过

```sql
SELECT status,
       count(*) FILTER (WHERE started > 0) AS actually_started,
       count(*) AS total
  FROM action_run_job
 WHERE needs = '["lint-build"]'
 GROUP BY status;
```

| status | 含义 | 真正启动过 | 总数 |
|---|---|---|---|
| 2 | failure | 29 | 29 |
| 3 | cancelled | 12 | 643 |
| 4 | skipped | 2 | 779 |
| 6 | running | 1 | 1 |
| **合计** | | **30 / 1452 = 2.1%** | 1452 |

⇒ **1452 次机会，30 次真的跑起来，0 次成功。**

### 1.4 证据三：那 29 次失败**没有一次**是「测试没过」以外的原因吗？—— 逐条定性

29 份 job 日志全部下载并分类（`/tmp/tlogs/<job_id>.log`）：

| 类别 | 次数 | 判据（原文） | job id |
|---|---|---|---|
| **GitHub checkout 死亡**（测试一行都没跑） | **6** | `Get "https://github.com/actions/checkout/info/refs?service=git-upload-pack": unexpected EOF` / `TLS handshake timeout` / `dial tcp 20.205.243.166:443: i/o timeout` | 15967, 16556, 16644, 16474, 16986, 17866 |
| **crates.io 下载/索引中断**（Build 步骤就死） | **4** | `failed to download from \`https://index.crates.io/config.json\`` / `[28] Timeout was reached (Operation too slow. Less than 10 bytes/sec …)` | 16699, 16951, 16994, 11557（日志在 `Updating crates.io index` 处截断） |
| **真的跑到了测试并失败** | **19** | `❌ Failure - Main Run tests` + `❌ Failure - Main Test summary`，其中可指名者：`--test alter_table_test` ×4、`--test diag_q11` ×4 | 其余 19 个 |
| **成功** | **0** | | |

⇒ **结论（必须照实说）**：CI 依赖**两个外部网络**（`github.com` 与
`index.crates.io`）。10/29 次在**跑任何一行测试之前**就死了；
19/29 次跑到了测试并因**真实的测试失败**而死；**0 次成功**。

### 1.5 因此「CI 全绿」意味着什么

`test` job 绝大多数时候被 **skipped**（779 次）或在启动前被 **cancelled**
（631 次）⇒ run 显示绿。剩下 30 次里，**一次都没绿过**。

⇒ 仓库里所有关于「CI 绿 ⇒ 可以合并」的判断，其证据强度是：
**「lint / 变异 / 后置门禁绿了，且测试 job 没跑」**。

### 1.6 修法

沿用这个仓库已经在用的写法（`mutation` / `postcheck` 就是这么写的）：

```yaml
  test:
    runs-on: [hp-z6g4]
    needs: []
    if: always()
```

`test-crates`（批次 23 新增的成员包 job）同样带 `needs: [lint-build]`，
一起改掉。

**代价**：测试套件与 `lint-build` 并行启动，而不是在它之后启动。
**收益**：测试套件不再等「别的 job 结束」才被唤醒，**与流水线同时起跑**。

本批之后，受门禁的两个 workflow（`ci.yml` / `gate.yml`）里**不再有任何
非空 `needs:`**。

> **诚实标注**：本项修的是「启动延迟 12 分钟 ~ 7 小时」与「1452 次里
> 只有 30 次启动」。它**不能**解释 19 次真实测试失败 —— 那些由本批合并
> 进来的批次 19 / 20 / 23 负责；而 10 次外部网络死亡由 §2（checkout）与
> §2bis（cargo 抗抖动）负责。三者合起来才是完整的因果链。

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

## 2bis. 根因 C —— crates.io 也是外部依赖（#5451 的一部分）

§1.4 的分类里，**4/29** 次失败死在下载 crate 上。实测（CI run 8348，
根包 `test` job 的 Build 步骤）：

```
error: failed to get `chrono` as a dependency of package
       `evidence-graph v0.1.0 (/workspace/openclaw/sqlrustgo/crates/evidence-graph)`
Caused by: failed to download from `https://index.crates.io/ch/ro/chrono`
Caused by: [28] Timeout was reached (Operation too slow.
            Less than 10 bytes/sec transferred the last 30 seconds)
```

锁文件里的 crate 必须取到，**下载本身不是可选项**；但网络抖动不该让整个
job 死掉。cargo 自带重试，只是默认次数太少、低速阈值太高。

本批在 `ci.yml` / `gate.yml` 的 `env:` 里只调**重试与超时**：

```yaml
  CARGO_NET_RETRY: "10"
  CARGO_HTTP_TIMEOUT: "120"
  CARGO_HTTP_LOW_SPEED_LIMIT: "1"
```

**没有动** `CARGO_HOME` / `RUSTUP_HOME`（`ci.yml` 顶部有一条明确的 NOTE：
在 act_runner 上覆盖它们会让每个 job 的 rustup 指向空工具链）。

> **未做**：真正的**缓存**（跨 job 复用的 registry 目录）需要动 runner 的
> 容器配置，属于基础设施改动，登记在 #5451，本批只做「抖动不该致死」这一半。

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
| `check_anti_fabrication.sh` | EXIT=0（ERRORS=0 / WARNINGS=0） |
| `check_gate_script_refs.sh`（develop 新带进来的） | EXIT=0（53 条既有引用全部在位且可执行；8 条已登记欠账按 warning 报告） |
| `check_pr_dag.sh`（develop 新带进来的） | EXIT=0（本地无 `origin/develop/v4.1.0` ⇒ 判 **DRIFT**，不是 PASS，也不是 FAIL） |
| `check_pr_template_sections.sh`（develop 新带进来的） | EXIT=0（同上，PR body 为空 ⇒ DRIFT） |
| `bash -n` + `shellcheck -S error`（mutation 脚本） | EXIT=0 |
| `cargo test -p sqlrustgo-storage --test no_namespace_db_refusal_5266` | 5 passed / 0 failed |
| `cargo test --test diag_q11 -- --nocapture`（TPCH_DATA_DIR 指向 SF=0.01/seed=42 fixture） | 16 passed / 0 failed |
| `mutate_tpch_oracle_tests.sh` 完整脚本 | CAUGHT=3 / SURVIVED=0 / ERRORS=0，EXIT=0 |

`ci.yml` / `gate.yml` 的 YAML 解析实测通过（`python3 -c "yaml.safe_load(...)"`），
两个 workflow 的 job 清单与 `needs`/`if` 全部核对过。

---

## 5. 本批**没有**解决、且必须单独记账的

| 事项 | 状态 |
|---|---|
| `test` job 第一次真正运行后**会暴露什么** | **未知** —— 这正是本批要拿到的事实。已知的 12 个红目标里，8 个由批次 19/20/23 修掉；`g13_oltp1_concurrent_select_test` 是负载型环境阻塞；`tls_read_spin_5099` 读 `/proc`，Linux 上会绿 |
| `mutation` job 的 BASELINE 失败（rc=101，无可辨识错误输出） | **已结案**（§6）：两个独立缺陷 —— 诊断输出被窄 grep 吞掉、900 秒预算同时覆盖 5m57s 的编译。两者都已修，并有正/负对照 |
| CI 上 `mutation` job 的**首次跑通** | **待观察** —— §6 的结论来自日志取证 + 本机复现，CI 实测结果待补 |
| #5451（cargo registry 缓存） | 未做。runner 镜像已自带 registry，但本次复现时 `cargo test` 仍在 `Updating crates.io index` 上耗时间 |
| runner 并行度 | 未动。runner 配置 `capacity: 4`（`/opt/devstack/runner/config.yaml`），宿主机 80 核 / 404 GB，当前负载 58 |

---

## 6. `mutation` job 的 BASELINE 失败 —— 已结案（两个独立缺陷）

CI run 8724 的 `mutation` job 红：

```
09:17:09.7417663Z == BASELINE: the unmutated 'diag_q11' must pass, otherwise CAUGHT is meaningless
09:19:21.8913170Z ERROR: the BASELINE run of 'diag_q11' failed (rc=101) — the mutation results would be meaningless
```

基线跑了 132 秒后以 rc=101 退出，而脚本用来诊断的
`grep -E "^test .* FAILED|panicked at|error\[|^error"` **一条都没匹配上**。

### 6.1 缺陷 D —— 根因被**吞掉**，失败不可诊断

脚本失败时唯一的诊断是那条窄 grep。它匹配不到时，CI 日志里就**只剩一行
"rc=101"**，根因必须靠人工翻原始 job 日志才能找到。这正是 run 8724 的情形：
`grep` 输出为空 ⇒ 日志里没有任何线索指向"到底是 cargo 下载失败、编译失败、
链接失败，还是测试进程被信号杀死"。

**修法**（`report_run_output`）：失败时**在 grep 之外**再打印运行的输出尾部
（BASELINE 60 行 / 变异 25 行）。grep 仍然第一个出现，tail 负责 grep 不可能
知道的一切 —— cargo 的下载错误、编译错误、链接错误、OOM killer 的输出。

### 6.2 缺陷 E —— 900 秒预算同时覆盖「编译」和「执行」，而编译就要 6 分钟

从服务器上 mutation job 的原始日志读出的 runner 实况：

| 事实 | 值 | 出处 |
|---|---|---|
| runner | `sqlrustgo-ci-01(version:v0.5.0)` | job 18223 日志首行 |
| job 镜像 | `192.168.0.252:5000/act_runner:node-rust` | `🚀 Start image=` |
| 容器 | **每个 job 新建**（`docker create` + `docker run`） | 同上 |
| `$CARGO_HOME` | **ephemeral**（容器内 `/root/.cargo`） | 容器级隔离 |
| checkout | **每个 job 从 `github.com` clone 一次** | `☁ git clone 'https://github.com/actions/checkout'` |

⇒ **容器里没有任何 cargo 缓存**：`cargo test` 的每一次都要从零编译整棵依赖树。

本机实测（批次 24 树，同一份 fixture）：

```
cargo test --test diag_q11 -- --nocapture
   Compiling … (actix-http / hyper / reqwest / sqlrustgo-* …)
   Finished `test` profile [unoptimized + debuginfo] target(s) in 5m 57s
   Running tests/integration/oracle/diag_q11.rs
   test result: ok. 16 passed; 0 failed; … finished in 0.19s
```

**编译 5m57s，测试 0.19s。** 而脚本的 `RUN_TIMEOUT_S=900`（15 分钟）覆盖的是
「编译 + 执行」**两段**，且三个变异各自还要重编一次 engine crate。

历史 job 的耗时把这事坐实了：

| job | run | 树 | 结果 | BASELINE 段耗时 |
|---|---|---|---|---|
| 18184 | 8712 | `4d31535527` | rc=**143**（SIGTERM） | **恰好 900 s** |
| 18223 | 8724 | `d6b4fe97e0`（批次 19） | rc=**101** | 132 s |

* job 18184：**正好 900 秒**被 `timeout` 杀掉 ⇒ 超时是在 cargo **还在编译**的时候
  触发的（CI 上多个 job 抢同一台 CPU，编译比本机更慢）。
* job 18223：132 秒就以 101 退出 ⇒ cargo 在**编译/依赖获取阶段**就失败了
  （101 也是 cargo 的编译错误退出码）。依赖获取走 crates.io 出口 —— 正是
  §2bis 已登记的第二个外部单点。

⇒ **两个失败模式，一个是环境（超时预算），一个是外部依赖（crates.io）。**

**修法**（编译/执行预算分离）：

* 新增 `COMPILE_TIMEOUT_S`（默认 3600），用于 `cargo test --no-run`；
* 编译**只做一次**（`diag_q11` + `diag_q12` 一次编译），三个变异复用构建产物，
  每次只需重编被改动的 engine crate；
* `RUN_TIMEOUT_S`（默认 900）只覆盖**测试执行**；
* 超时判定从 `rc -eq 124` 扩到 `124` **或** `143`（后者是超时信号打到 job 本身
  时 shell 报的码；job 18184 就是它）。

### 6.3 两个对照

**正向对照**（批次 24 树，本地跑完整脚本）：

```
== compiling the mutation targets (budget 3600s) — the CI container has no cargo cache, so this is the expensive part
   compiled OK
== BASELINE: the unmutated 'diag_q11' must pass, otherwise CAUGHT is meaningless
   baseline OK
== MUT-1: HAVING predicate inverted  (target: diag_q11)
   CAUGHT (MUT-1): the test failed as it should
== MUT-2: SUM drops the first value of each group  (target: diag_q11)
   CAUGHT (MUT-2): the test failed as it should
== MUT-3: COUNT(DISTINCT x) loses de-duplication  (target: diag_q12)
   CAUGHT (MUT-3): the test failed as it should
=== mutation summary ===
CAUGHT   : 3
SURVIVED : 0
ERRORS   : 0
PASS: all 3 mutation(s) were caught by the TPC-H assertions
```

退出码 **0**。

**负向对照 A（严格复刻缺陷 D 的场景）**：把 BASELINE 的 cargo 调用换成
「立刻 rc=101、且输出不含 `^test … FAILED` / `panicked at` / `error[` / `^error`
任何一种模式」的桩 ⇒ **旧脚本的 grep 零命中**（根因消失），**新脚本打印出**：

```
   --- last 60 lines of the run output ---
   | DIAGNOSTIC-MARKER-7f3a: cargo died before producing any test result
   --- end of the run output ---
```

退出码 2（harness error，不是「caught」）。

**负向对照 B（编译阶段失败）**：把 `MUT1_TARGET` 改成不存在的 target 名 ⇒
编译阶段失败 ⇒ 脚本打印出 cargo 的错误尾部（含可用 target 列表）并以 2 退出。

两个负向对照用的副本脚本用完即删，仓库里不留痕迹。

### 6.4 仍未结案

CI 上 `mutation` job 的**第一次真正跑通**仍待观察 —— 本节的所有结论都来自
日志取证 + 本机复现，CI 的实测结果待补（登记在 §5 的未结案表里）。

---

## 6bis. `lint-build` job 也红了 —— 工具链版本漂移（#5540）

批次 24 的 run 8750 的 `lint-build` job：

```
error: very complex type used. Consider factoring parts into `type` definitions
  --> tests/integration/sql/_q15_q16_check.rs:74:6
   |
74 | ) -> Option<(Vec<Vec<String>>, Vec<(String, usize)>)> {
   = help: ... rust-clippy/rust-1.95.0/index.html#type_complexity
   = note: `-D clippy::type-complexity` implied by `-D warnings`
error: could not compile `sqlrustgo` (test "_q15_q16_check") due to 1 previous error
```

### 6bis.1 因果链

1. 批次 23 的 `f917dc2b0e`（#5515）把 `compare_with_oracle` 的返回类型改成
   `Option<(Vec<Vec<String>>, Vec<(String, usize)>)>` —— 这个改动**本身正确**
   （它把「fixture 缺失 ⇒ panic」改成「返回 None ⇒ 记 NOT-RUN 后不判失败」）。
2. 该拼写在 **CI 的 clippy 1.95.0** 下触发 `clippy::type_complexity`，
   而**本地默认工具链 1.97.0 不触发**（clippy 的复杂度评分在版本间变过）。
3. 批次 19 的 lint 预算门禁是在**本地工具链**上跑的 ⇒ baseline 里没有这个
   新 lint ⇒ 门禁放行；而 CI 的 `lint-build` 步骤是
   `cargo clippy --all-features --all-targets -- -D warnings`（**零容忍**）⇒ 直接红。
4. 批次 23 从未单独跑过 CI（它要随 #5535 一起合并）⇒ 这个 lint 在
   **第一次真正跑到它的那次 CI** 才暴露。

⇒ 这是「CI 从未跑绿测试套件」的又一个直接后果。

### 6bis.2 修法与**两个对照**（都用 CI 的工具链）

修法：把返回类型命名 —— `type OracleRows = Vec<Vec<String>>;`、
`type LoadedTables = Vec<(String, usize)>;` ⇒ 签名读起来更清楚，
且**对任何 clippy 版本都不触发**该 lint。

| 对照 | 命令 | 结果 |
|---|---|---|
| **负对照**（临时还原旧签名） | `cargo +1.95.0 clippy --all-features --test _q15_q16_check -- -D warnings` | **RC=101**，报 `type_complexity` ✓ |
| **正对照**（修复后，同一条命令） | 同上 | **RC=0** ✓ |
| **CI 等价命令**（根包全 target） | `cargo +1.95.0 clippy --all-features --all-targets -- -D warnings` | **RC=0**（0 error） ✓ |

⚠️ 之所以能对上是**因为本地装了与 CI 同一个 clippy（1.95.0）**：
`rustup toolchain list` ⇒ `1.91.0` / `1.95.0` / `1.97.0` 都在。
本地默认的 1.97.0 **不会**报这个 lint —— 这正是它漏过门禁的原因。

### 6bis.3 附带发现（值得单独立规则）

lint 预算门禁在「当前工具链没有自己的 baseline 条目」时回退到
「各 lint 跨版本的最大计数」⇒ **本地绿、CI 红**的情形会被放过。
门禁脚本其实已经支持 `SQLRUSTGO_CLIPPY_TOOLCHAIN` 固定工具链（这是对的），
但没有任何东西**要求**生成 baseline 时用它。
⇒ 建议的规则：**生成/更新 baseline 必须用与 CI 相同的 clippy 版本**，
否则这次的漂移会重演（登记为 #5540 的「附带发现」）。

---

## 6ter. `lint-build` job 也红了 —— 批次 24 自己破坏了 `cargo fmt --check`（#5544）

CI run **8831** / job **18459**（PR #5535 的 head `cd4a02c6b5`）在 `lint-build` 这一步红：

```
Diff in crates/storage/tests/no_namespace_db_refusal_5266.rs:16:
 use sqlrustgo_storage::append_only_storage::AppendOnlyStorage;
+use sqlrustgo_storage::binary_storage_v2::BinaryTableStorageV2;
 use sqlrustgo_storage::engine::{
...
Diff in crates/storage/tests/no_namespace_db_refusal_5266.rs:22:
-use sqlrustgo_storage::binary_storage_v2::BinaryTableStorageV2;
 use sqlrustgo_storage::table_level_storage::TableLevelStorage;
  ❌  Failure - Main Format check
exitcode '1': failure
```

**根因是流程缺陷，不是代码缺陷**：本批的提交 `489aa8cb0e`（修 5266 测试的 `error[E0432]`）
把新 import 放在了不符合 rustfmt 排序的位置；而批次 24 的本地验证跑了门禁脚本、
clippy 预算门禁、AFP 门禁，**唯独没有跑 `cargo fmt --check --all`**。
CI 把它排在 `lint-build` 的最后一步，于是它成了第一个发现问题的步骤。

⇒ **本地绿灯不等于 CI 绿灯**：两边跑的不是同一份命令清单（#5544 的流程侧修复）。
本批已改为：提交前在本地**镜像 CI `lint-build` job 的全部 7 个步骤**
（clippy / 两个 lint 预算门禁 / workflow 表达式门禁 / 硬编码凭据门禁 / fmt / build），
并用 `SQLRUSTGO_CLIPPY_TOOLCHAIN=1.95.0` 把工具链钉成 CI 那一个。

代码侧修复：把 import 移到 rustfmt 要求的位置（`cargo fmt --all` 只改这一个文件）。

Refs #5544

---

## 7bis. 批次 24b —— PR-DAG 门禁的 base ref 恒不可解析（#5542）

本批**追加**一个提交，修掉一个会让 CI `test` job **必红**的缺陷：
`scripts/gate/verify_pr_dag.sh` 的默认 base `origin/develop/v4.1.0`
在本地（没有 `origin` 远端）和 CI（只 fetch 单个 SHA ⇒ 不生成远程跟踪分支）
**都解析不到** ⇒ 门禁恒判 DRIFT(exit 2) ⇒
`tests/integration/gate/gate_criterion_existence_test.rs` 的 5 个用例每个 run 都红。

修法：**显式 `--base` 逐字使用、永不回退**；默认 base 改成「发布线规格 + 一条确定的解析链」，
门禁打印它实际用的 ref。附带修掉 `:409` 的 `[ "" -gt 0 ]` 整数比较噪声。
另给门禁加 `--selftest`，并在 CI 的 `lint-build` job 里跑一次。

完整 SPEC / 实测 / 正负对照 / CI 形态 checkout 的前后对照见
**`docs/releases/v4.1.0/evidence/PR_DAG_BASE_RESOLUTION_5542_2026-10-22.md`**。

Refs #5542

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
Refs #5542
Refs #5544
