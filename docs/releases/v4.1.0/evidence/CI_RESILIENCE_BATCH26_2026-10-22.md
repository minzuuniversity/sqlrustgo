# 迭代 6 · 批次 26 —— CI 抗崩溃加固 + 三个「机器速度/机器布局」敏感测试的确定性化

- **分支**：`fix/ci-truth-batch24`（PR #5535）
- **基线**：`develop/v4.1.0` @ `b68671895c`
- **本批新增**：#5547（wal_legacy 余下 2 个墙钟断言测试）、#5548（wrapper 委托失败契约在 CI 里从未真正被验证）、
  #5549（g13 并发测试的速度敏感前提）、#5550（CI 并发把宿主机拖到内核 panic）、
  #5551（docker 地址池被占满 ⇒ CI job 一个都没跑）
- **本批收口**：#5400（门禁重写 + selftest）、#5401（storage db-scope 覆盖门禁）、#5502（文档指针门禁）
- **本批并入**：批次 17（#5423 的 env 竞争修复）—— 它的修法必须与本 PR 一起在 CI 上验

---

## 0. 一句话

上一轮 CI 之所以「全红」，**根本不是代码红**：2026-10-22 22:57:32 CST 宿主机
**内核 panic**，所有 CI 容器随宿主一起消亡。本批把这件事定性为**基础设施红**
（附完整证据），并同时修掉三类「换了台机器/换个负载就飘」的测试缺陷 ——
它们才是代码层真正的红。

---

## 1. run 8914–8921 的定性：**基础设施红，不是代码红**（#5550 的起因）

### 1.1 现象

四个分支同时推上去，触发 8 个 run（8914–8921），其中：

| job | 结论 | 日志最后一行 |
|---|---|---|
| lint-build (18645) | failure | **日志根本没上传**（`log_in_storage = f`, `log_size = 0`） |
| test (18646) | failure | `Compiling sqlrustgo v3.12.0-fix-zombie` |
| test-crates (18647) | failure | `Compiling sqlrustgo-executor v3.12.0-fix-zombie` |
| mutation (18648) | failure | `== compiling the mutation targets (budget 3600s) …` |
| postcheck (18651) | failure | `[L1] cargo build...` |
| evidence-graph-gate (18652) | failure | `Compiling chrono v0.4.45` |
| legacy-gate (18653) | failure | `[E3_5REMOTES_DRIFT] PASS` |
| test-soak-detach (18650) | **success** | —— |

**每个日志都停在编译中途**，没有一条测试断言失败。

### 1.2 服务器侧证据（`action_run_job` / `action_task` / `journalctl`）

* 7 个 job 的 `stopped` 时间戳：

  ```
  18645 lint-build         1792680800
  18646 test               1792681839
  18647 test-crates        1792681839
  18648 mutation          1792681838
  18651 postcheck          1792681838
  18652 evidence-graph-gate 1792681838
  18653 legacy-gate        1792681838
  ```

  除 lint-build 外，其余 6 个在**同一秒**被置为失败 —— 这是「一次性被杀」，
  不是 6 个各自独立的失败。

* `journalctl --list-boots`（服务器时区 UTC+8）：

  ```
  -4  21:03:30 → 21:34:14
  -3  22:38:30 → 22:41:44
  -2  22:42:57 → 22:56:44
  -1  22:57:16 → 23:03:43
   0  23:04:35 → …
  ```

  一天之内 **5 次重启**。

* boot -1 的日志尾部：

  ```
  Oct 22 22:57:32 gaoyuanai-HPZ6G4 kdump-tools[802]:  * running makedumpfile -F -c -d 31 /proc/vmcore | compress > /var/crash/202610222257/dump-incomplete
  Oct 22 22:57:32 gaoyuanai-HPZ6G4 kdump-tools[824]:  The kernel version is not supported.
  Oct 22 23:03:43 gaoyuanai-HPZ6G4 kdump-tools[824]:  makedumpfile Completed.
  ```

  ⇒ **内核 panic**（kdump 抓到了 vmcore），不是 OOM-killer、不是 docker 报错。

* 崩溃前 1 分钟，运维动作与负载叠加：

  ```
  Oct 22 22:56:38 … sudo: COMMAND=/usr/bin/docker compose -p runner-pool -f /opt/devstack/runner-pool/compose.yml up -d
  Oct 22 22:56:38 … kernel: br-aab77b2092fd: port 18(veth504f94f) entered disabled state
  Oct 22 22:56:41 … kernel: INFO: The task sync:112186 has been waiting for writeback completion for more than 22 seconds.
  Oct 22 22:57:32 … kdump-tools … /proc/vmcore
  ```

* 宿主机负载：`load average: 147.82, 86.18, 47.01`（80 核）。

### 1.3 结论

CI 容器**没有跑完任何一个 job**，更没有跑出任何一条测试断言。
按治理口径：本次 run = **NOT-RUN（基础设施红）**，**不产出任何 PASS 证据**。

已取消的排队 run（8916–8921）同样不产出任何 PASS，理由记在协作台账。

### 1.4 第二个基础设施红：run 8991 的「一个 job 都没跑」

宿主机重启后，**第一个** CI run（8991，head `0535a95e5e`）的 7 个 job 又全部
失败 —— 但这次日志只有 **10 行**，而且失败点在 act_runner **建 job 网络**那一步：

```
🚀  Start image=sqlrustgo-runner:v1
🐳  docker pull sqlrustgo-runner:v1
Image exists? true
Cleaning up network for job test, and network name is: GITEA-ACTIONS-TASK-6875-…-test-network
Error response from daemon: all predefined address pools have been fully subnetted
```

⇒ **测试套件一行都没执行**。这也是**基础设施红**，同样**不产出任何 FAIL 证据**。
详见新登记的 **#5551**（含服务器侧处置：清理脚本 + cron）。

---

## 2. 触发条件是什么 ⇒ #5550 的两条修法

根因是**资源超卖**：CI 容器**没有 cargo 缓存**（#5451 仍开着），所以每个 job
都从零重编整个 workspace（`cargo build` 约 7m30s，**纯 CPU 密集**）；
runner 池有 3 台 `hp-z6g4`，而 cargo 默认「一个核一个 rustc」，宿主机 80 核
⇒ 3 × 80 = **240 个 rustc 进程抢 80 个核**。再叠上 4 个 run 同时在跑
（28 个 job 排队抢 3 台 runner）和一次 `docker compose up -d` 重启 runner 池，
就把宿主机推进了内核 panic。

### 2.1 修法一：workflow 级 `concurrency` —— 每个 workflow 一次只跑一个 run

`.gitea/workflows/ci.yml` 顶层加：

```yaml
concurrency:
  group: sqlrustgo-ci-pipeline
  cancel-in-progress: false
```

`.gitea/workflows/gate.yml` 顶层加：

```yaml
concurrency:
  group: sqlrustgo-gate
  cancel-in-progress: false
```

* `group` 是**仓库级固定串**（不是 per-ref）：per-ref 的话 4 个分支仍然会同时跑，
  正是崩溃的条件；固定串把**同一个 workflow** 的并发压到 1。
* `cancel-in-progress: false`：新推送**排队**而不是杀掉在跑的那个 ——
  被杀的 run 同样不产出证据，而证据链必须完整。

#### 2.1.1 实测修订：两个 workflow **不能**共用一个 group

第一版把 `ci.yml` 与 `gate.yml` 都设成 `group: sqlrustgo-ci`。实测
（2026-10-23，Gitea 28.1.0，同一 PR 的两个 run 在**同一秒**创建）：

```
8997 | status=cancelled | started=0 | stopped=创建同秒   ← ci.yml
8998 | status=waiting                                 ← gate.yml
```

⇒ **Gitea 的 concurrency 语义与 GitHub 不同：新 run 加入 group 时，会把同 group 里
「尚未开始」的 run 取消掉**（`cancel-in-progress: false` 只保护「已经在跑」的那个）。
两个 workflow 共用一个 group ⇒ 同一 PR 上必然有一个被取消 ⇒ PR 上出现
cancelled 的 required check ⇒ **永远合不进去**。

⇒ 修法：**一个 workflow 文件一个 group**。跨分支的串行化（真正保护宿主机的
那一条）仍然成立 —— 每个 workflow 的 group 依然是固定串。
并发上限本来�� runner 池决定（3 台 `hp-z6g4`），配合
`CARGO_BUILD_JOBS=16` ⇒ 最多 3 × 16 = 48 个 rustc 抢 80 核。

### 2.2 修法二：`CARGO_BUILD_JOBS: "16"` —— 限制**每个容器**的编译并行度

cargo 默认按核数派 rustc。限到 16 之后，3 个并发 job 合计 48 ≤ 80 核，
机器不再被超卖；单次编译从「80 核 7m30s」变成「16 核约 10m」——
用一点墙钟换宿主机不崩，这笔账在 GA 之前必须付。

### 2.3 验证

| 检查 | 结果 |
|---|---|
| `python3 -c "yaml.safe_load(...)"` 两个 workflow | 解析通过，`concurrency`/`env` 字段落地 |
| `scripts/gate/check_workflow_expressions.sh` | **PASS**（7 个 workflow 全部扫描） |
| `scripts/gate/check_ci_checkout_consistency.sh` | **PASS**（C1–C4 全过，含「每个非门禁 workflow 都要有豁免理由」） |
| 运行时 | 推新 head 后：`8999`（ci.yml）在跑、`9000`（gate.yml）排队 —— **没有互相取消** |

### 2.4 服务器侧配套（#5551）

CI job 泄漏的 docker 网络/卷会耗尽 docker 的地址池，重启后的**第一个** run
会「一个 job 都没跑」就全灭。已落地：

* `/usr/local/bin/ci-docker-cleanup.sh` —— 只回收**没有任何容器在用**的
  `GITEA-ACTIONS-TASK-*` 网络与卷；
* `/etc/cron.d/ci-docker-cleanup` —— 每 10 分钟一次；
* 首次执行：卷 **193 → 89**，磁盘 **1.1T → 899G**。

---

## 3. #5547 —— `wal_legacy` 余下 2 个墙钟断言测试（coverage 门禁 exit=101 的根因）

### 3.1 症状

`scripts/gate/check_coverage_v312.sh` 报：

```
FAIL: could not measure sqlrustgo-storage (exit=101)
```

`exit=101` 是 `cargo test` 的失败退出码。实际失败用例（本地复现）：

```
test wal_legacy::tests::test_wal_perf_100_update ... FAILED
thread 'wal_legacy::tests::test_wal_perf_100_update' panicked at crates/storage/src/wal_legacy.rs:1604:9:
test result: FAILED. 914 passed; 1 failed; 1 ignored; 0 measured; 0 filtered out; finished in 17.05s
```

### 3.2 根因

`test_wal_perf_100_update` 断言「100 次 10KB UPDATE 的墙钟 < 1s」，
`test_wal_perf_recovery_1mb` 断言「1MB WAL 恢复的墙钟 < 1s」。
这两个都是**测量**，不是正确性 —— 而且它们住在 storage crate 的 **lib 测试二进制**里，
coverage 门禁正是用 `cargo llvm-cov test -p sqlrustgo-storage --no-fail-fast` 跑的：
插桩后同样的活儿慢好几倍，机器有负载时再慢好几倍。

#5545 已经把同族的 `test_wal_perf_1000_insert` 改成 `#[ignore]` + 登记，
但**这两个漏网了** —— 于是 coverage 门禁照红。

### 3.3 修法（与 #5545 完全同一条规则）

* 两个测试加 `#[ignore = "wall-clock measurement harness — run manually …"]`；
* 在 `tests/baseline/ignore_registry.json` 登记两条
  （`status: PARKED` / `category: EXTERNAL_PREREQ`，与 `measure_commit_clone_4948.rs`、
  `b22_flush_wall.rs`、`test_wal_perf_1000_insert` 同款）；
* `total_allowed` 117 → **119**；`known_deficiencies` 重算：
  active 69（不变）/ parked 48 → **50** / `EXTERNAL_PREREQ` 46 → **48** /
  `missing_reason_text` **34**（不变，新增的两条都带理由文本）。

### 3.4 验证

```
$ bash scripts/gate/check_anti_ignore_gate.sh
ignore_registry.json: total_allowed=119 (max=150), active=69 (max=80)
tree recount: real #[ignore] attributes in tests/ + crates/ = 119
rc=0
```

门禁是**自参照**的：它除了查上限，还会用同一套规则**重新数一遍树里的
`#[ignore]`**，树里 119 个、登记表 119 个，两边相等才算过 ——
所以「加了 `#[ignore]` 却忘了登记」不可能蒙混过关。

### 3.5 覆盖门禁本体的验收（这才是 #5547 的正主）

```
$ bash scripts/gate/check_coverage_v312.sh
...
Measuring coverage for sqlrustgo-storage ...
  features: bin_storage_default fault-injection
  sqlrustgo-storage: 80.61%
Measuring coverage for sqlrustgo-catalog ...      87.33%
Measuring coverage for sqlrustgo-optimizer ...    87.85%
Measuring coverage for sqlrustgo-types ...        95.28%
L1_8 average coverage: 87.80% (threshold: 80.0%)
cov rc=0
```

修复前是 `FAIL: could not measure sqlrustgo-storage (exit=101)` ——
现在 storage 的覆盖率**测得出来了**，整条门禁 rc=0。

### 3.6 storage lib 测试

```
$ cargo test -p sqlrustgo-storage --lib
test result: ok. 913 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out
```

修复前同一命令是 `914 passed; **1 failed**; 1 ignored`
（`test_wal_perf_100_update` FAILED）。

---

## 4. #5548 —— wrapper「不能委托时必须诚实失败」这条契约在 CI 里**从未被验证过**

### 4.1 症状

`tests/e2e/e2e_canonical_subprocess.rs::e2e_legacy_binary_sqlrustgo_deprecated` 红：

```
the wrapper must NOT report success when it cannot delegate; …
```

### 4.2 根因：前提造不出来

测试用「空 cwd + 空 `PATH`」来制造「找不到 `sqlrustgo-mysql-server`」。
但 wrapper 的候选表里**还有第三个候选**
（`crates/sqlrustgo-cli/src/lib.rs` → `mysql_server_bin()`）：

```
2. $CARGO_TARGET_DIR/{debug,release}/sqlrustgo-mysql-server
3. current_exe().parent()/sqlrustgo-mysql-server     ← 跟着 wrapper 自己走
4. ./target/{debug,release}/sqlrustgo-mysql-server   （相对 cwd）
5. $PATH
```

CI 里 wrapper 与 server 二进制**同在 `target/debug/`**，候选 3 永远命中 ⇒
「找不到」这个前提根本不成立 ⇒ wrapper 委托成功 ⇒ 断言「必须失败」红。

也就是说：**这条契约在 CI 里一次都没有被真正验证过**（和 #5477 当初
「找不到就静默 return」是同一类问题，只是这次反过来）。

### 4.3 修法（只改测试，不改生产行为）

把 wrapper **复制**到空目录再执行，于是 `current_exe().parent()` 指向空目录；
同时把 `CARGO_TARGET_DIR` 指向空目录、cwd 指向空目录、`PATH` 清空、
显式 `env_remove("SQLRUSTGO_SERVER_BIN")`。
五个候选全部落空 ⇒ wrapper 走「6. 什么都没找到」分支 ⇒
`Proc::new("sqlrustgo-mysql-server")` 启动失败 ⇒ 打印
`sqlrustgo-mysql-server not found …` 并以非零退出 —— 正是契约 2 要求的。

生产侧的候选表**一个字没动**。

---

## 5. #5549 —— g13 并发测试的前提是「这台机器有多快」，不是「代码对不对」

### 5.1 症状

`tests/integration/sql/g13_oltp1_concurrent_select_test.rs::g13_oltp1_concurrent_selects_run_in_parallel` 红：

```
baseline (no contention): thread A only ran 1 queries — the heavy query is too slow
for the test to be meaningful. Lower ROW_COUNT or the join depth.
```

### 5.2 根因

被测的 heavy query 是 500 行上的三路自连接（≈8M 次行比较）。原实现用
**固定 2 秒窗口**、并要求窗口内**至少跑完 2 次**：

```rust
const ALONE_DURATION: Duration = Duration::from_secs(2);
assert!(alone_q > 1, "… the heavy query is too slow …");
```

空闲的开发机上一次几十毫秒，窗口里能跑几十轮；**有负载的 CI 容器上一次就可能
超过 1 秒**，窗口里只剩 1 轮 ⇒ 断言红。于是**用来守护「测量是否有意义」的那条
断言本身成了失败点，而测量从未发生**。

### 5.3 修法：窗口由**实测**一次迭代推导出来

* 先单独跑一次 heavy query，量出 `one_iteration`；
* 窗口 = `clamp(one_iteration × 3, 2s, 60s)`；
* alone / contended **两侧用同一个窗口**，比值仍然可比。

契约没变（仍然是「有竞争时的吞吐 vs 无竞争时的吞吐」），只是把「窗口」从
「对机器的假设」变成「对机器的测量」。

---

## 5bis. 本地全量测试扫描 —— 找出 CI 还会红的两处

CI 的 job 日志在 Gitea 上会被截断（被杀的 job 只剩头部，
实测 `action_task.log_size` 只有 946–987 字节），所以「跑一次完整的测试套件」
是唯一可靠的办法。本地跑 `cargo test --all-features --no-fail-fast`
（446 个测试目标）拿到两个失败：

### 5bis.1 真红：env 竞争（#5423，批次 17 的修法，必须并进来）

```
---- execution_engine_tests::test_v410_atomic_parallel_degree_getter_clamps_to_one ----
panicked at src/execution_engine_tests.rs:1365:5:
assertion `left == right` failed
  left: 4
 right: 1
```

根因：`src/execution_engine_tests.rs` 用
`std::env::set_var("SQLRUSTGO_EXECUTOR_PARALLELISM", …)` 改写**进程级**环境表，
而根包 lib 测试二进制里 ~160 个用例是**并发**跑的 ⇒ 构造 engine 的那个用例
会读到别的用例刚写进去的值（`src/execution_engine.rs:376` 在构造时读 env）。

这正是 **#5423** 修的缺陷，修法在**批次 17 的分支**上（并行度改成显式注入
`ExecutionEngine::with_parallel_degree` + 纯函数 `resolve_parallel_degree`），
而那个分支还没进本 PR ⇒ 本 PR 的 CI 也会红。

⇒ **处置**：把批次 17 的分支并进本 PR（`chore(merge): 并入批次 17`），
并在 CI 上一起验。合并后新增的门禁
`scripts/gate/check_no_process_env_mutation.sh` 本地 PASS：

```
PASS: C1 — the root package does not mutate process-global environment variables
=== RESULT: [PASS] ===
```

### 5bis.2 假红：本地 macOS bash 3.2 的 `source <(...)` 不支持

```
---- coverage_gate_derives_the_features_storage_actually_needs ----
panicked at tests/integration/gate/alpha_gate_blocker_test.rs:78:5:
the derivation must find the fault-injection feature. Got: ""
```

该用例用 `source <(sed -n …)` 把门禁脚本里的两个函数抽出来单独调用。
本机是 **GNU bash 3.2.57**（macOS 自带），它在这个上下文里**不支持进程替换**：

```
$ bash -c 'source <(echo "f(){ echo hi; }"); f'
bash: f: command not found
```

CI 主机是 **GNU bash 5.2.21**（Linux），同一片段**正常工作**：

```
$ bash -c 'source <(echo "f(){ echo hi; }"); f'
hi
```

⇒ 这条**不是 CI 红**，是本地假红。**不修**（它不阻塞 CI，且该文件属于
另一 Agent 的 #5537 范围），但在此**如实记账**，免得下一个人把它当成真红去追。

---

## 6. #5400 —— 门禁重写：判据函数化 + `--selftest`

`scripts/gate/check_correctness_tests_production_storage.sh` 重写为 R1–R5 五个判据函数，
按 **match arm** 分别判定（默认 arm / `Some("binary")` / `Some("parallel")`），
语义化为「WAL manager + `FileStorage::new_with_external_wal` + WAL 包装层」「FileStorage + ParallelWalStorage」。

```
$ bash scripts/gate/check_correctness_tests_production_storage.sh --selftest
[PASS] selftest: 一份健康的装配满足 R1–R4 且不违反 R5
[PASS] selftest: 默认路径丢掉 WAL 包装层 ⇒ R2 变红
[PASS] selftest: 生产路径出现 MemoryStorage ⇒ R5 变红
[PASS] selftest: parallel 分支不再 file-backed ⇒ R4 变红
[PASS] selftest: binary 分支换成内存存储 ⇒ R3 变红
--- selftest summary ---  passed: 5  failed: 0
✅ selftest PASS

$ bash scripts/gate/check_correctness_tests_production_storage.sh
  [PASS] R1 生产默认路径 file-backed (MvccStorage::new(file_storage))
  [PASS] R2 生产默认路径带 WAL 持久化 (FileStorage::new_with_external_wal + WAL 包装层)
  [PASS] R3 binary 分支 file-backed (BinaryTableStorage::new_with_data)
  [PASS] R4 parallel 分支 file-backed (FileStorage::new_with_external_wal + ParallelWalStorage)
  [PASS] R5 生产存储选择块无 MemoryStorage — 不存在非 file 生产路径
=== Result: PASS=5 FAIL=0 ===
```

**5 个退化 fixture 证明判据还有判别力**（不是把判据放宽成永远绿）。

---

## 7. #5401 —— storage db-scope 覆盖门禁

* 门禁的 trait 提取原先把 `MemoryStorage` 的**固有 impl** 当成 trait 的一部分，
  于是要求包装层「覆盖」一个根本不在 trait 里的方法 ⇒ **永远红**。
  提取改为 `/^pub trait StorageEngine/{f=1} f{print} f && /^}$/{exit}`。
* 真正缺口 = **2 个方法 × 2 个包装层**：`get_view` / `list_views`
  ⇒ 已在 `crates/storage/src/wal_storage.rs`（`self.inner()`）与
  `crates/storage/src/mvcc_storage.rs`（`self.inner`）补上转发实现。
* 门禁自身的消息 bug（`printf '%s\n' "$WRAPPERS" | wc -l` 恒为 1）改为
  `echo $WRAPPERS | wc -w`。

```
$ bash scripts/gate/check_storage_db_scope_coverage.sh
PASS: 2 wrappers override all 16 database-scoped methods
rc=0
```

---

## 8. #5502 —— 文档指针门禁

`check_docs_pointer_paths.sh --scope docs/releases/v4.1.0` 原报 8 条新失效指针：

* 真正该修的：`ALTER_COLUMN_DEFAULT_BATCH13_2026-10-22.md` 指向的
  `crates/storage/src/bin_format.rs` 文件已不存在 ⇒ 改写为 BINT 落盘格式
  `crates/storage/src/bin_segment.rs`（并如实说明该文件唯一一处
  `default_value` 是内存侧 `ColumnDefinition` 字段，恒 `None`）；
* 登记 7 条确属债的进 `tests/baseline/doc_pointer_paths_releases_v4.1.0.json`
  （`matrix.json` 生成产物 / `totally_nonexistent_zzz.sh` 故意假名负向样例 /
  GATE_BLIND_SPOTS_BATCH23 的 4 个**包相对**路径 / 已移走的
  `2026-06-07-tpch-cell-diff-v390.json`）；
* 移除 1 条已解析的陈旧 entry（`verify_pr_dag.sh`）。

```
$ bash scripts/gate/check_docs_pointer_paths.sh --scope docs/releases/v4.1.0
  pointers in scope: 33 broken, 33 registered as known debt
  [OK] no broken pointer outside the recorded debt
  [PASS] Documentation Pointer-Path Gate
  [OK] the baseline was not modified (compare mode)
rc=0
```

---

## 9. 未达成 / 仍然欠着的

* **#5451 真缓存**（跨 job 共享 cargo registry）仍开着 —— 它是 CI 单次编译
  7m30s 的根因，也是本批只能靠「限并发 + 限并行度」来止损的原因。
  真解法要改 runner 容器的 `CARGO_HOME` 挂载。
* **BE5 / B-F5** 在 CI 上仍判不了（浅克隆 ⇒ DRIFT，设计如此），
  需要完整历史 checkout 才能真正执行，记账为独立欠账。
* 本批的 CI 证据要等**服务器恢复后**重跑（run 8914–8921 全部是
  NOT-RUN / 基础设施红，不产出任何 PASS）。

---

## 10. 身份与协作

- 身份串：`openclaw-mcode`（Mac Mini / Minimax-Code）
- 另一 Agent：`opencode-z440`（独立账号 id=13），本批**未触碰**其工作区文件
  （主工作区 `/Volumes/workspace/dev/sqlrustgo` 的 13 个在飞文件不在本批 diff 内）
- 涉及 Issue：#5400 #5401 #5502 #5547 #5548 #5549 #5550
- **只写 `Refs`，不写 `Closes`** —— 关单由合并后复跑 + 证据评论决定。

Refs #5400
Refs #5401
Refs #5502
Refs #5547
Refs #5548
Refs #5549
Refs #5550
