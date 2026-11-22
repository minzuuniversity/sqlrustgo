# 权威认证轮 009 —— **未跑满**（seq1 真红 + 其余由操作者中止）

> 本轮**没有**跑满 35 项。它同时包含两件事，必须分开说：
> ① **seq1 是真的判红**（D7 维度的 G10 探针）—— 有明确的根因，**不是**被测代码的缺陷；
> ② seq20 之后由**操作者主动中止** —— seq20 卡在一条三层嵌套的 cargo 调用链上，
>    再等下去只会撞上单门禁预算。

## 1. 轮次事实

| 项 | 值 |
|---|---|
| run_id | `20261117-ga-gates-009` |
| 被测树 | `1579950048b37628eb19bb9f2fe624ab87198ada`（与 007/008 **同一棵**） |
| plan sha256 | `sha256:6466a0ec156075daf3bc53ece379c4ecc2b13b63da27cd17a1f39cb2254da809`（与 007/008 **完全一致** ⇒ 分母与判据未漂移） |
| 已执行 | **19 / 35**：**18 PASS + 1 FAIL**（seq20–seq35 **从未开始**） |
| 中止时点 | 2026-11-17T08:09Z（seq20 运行中） |
| 留存证据 | `results.jsonl`（19 行）、`020-d6-batch-hang.txt`（D6 批次日志 655 行）、`020-hang-stack-*.txt`（4 份 macOS `sample` 栈快照） |

## 2. seq1 判红 —— 根因已定位（**不是被测代码的缺陷**）

```
D7-Reliability: 4/5 (blockers: 1)
  [G10] Audit Log + Time Travel ...  ✗ [FAIL] G10 Audit/Time-Travel:
     Compiling sqlrustgo v4.1.0 ... Finished `test` profile in 22.87s
      Running tests/integration/stress/audit_log_test.rs
```

`scripts/gate/check_rc_ga_gate.sh:1511`：

```bash
AUDIT_TEST=$(timeout 60 cargo test --test audit_log_test --test time_travel_test 2>&1 | tail -3)
```

**60 s 的探针预算把 cargo 的编译时间算了进去。**

为什么那 22.87 s 的编译是多余的：本轮的预构建顺序是「先默认 features、后 `--all-features`」。
cargo 把 lib 的**最终产物**uplift 到 `target/debug/libsqlrustgo.rlib`，**这条路径不区分 features**；
于是 all-features 的构建覆盖了默认侧，**把默认 features 的 lib 单元置脏** ⇒ 轮次内第一次跑默认
features 的 cargo 调用被迫重编译 lib。

**排除「被测代码有缺陷」的三条证据**：

1. `audit_log_test` 的**测试二进制本身没有重编译**（输出里只有 lib 的 `Compiling`）⇒ 产物是热的；
2. 007/008 轮 G10 都是 **5/5 PASS**（同一棵树、同一份代码），差别只在缓存状态；
3. 判红的形态是「**进程被 `timeout` 杀掉**」（`tail -3` 只剩 Compiling/Finished/Running，
   没有 test result），**不是**断言失败 —— 日志里 `panic|assertion` 命中 0 处。

**⇒ 处置：把默认 features 的构建放最后**，让轮次开始时默认侧是干净的。
**不改门禁脚本、不改任何阈值、不改 35 项分母。**

## 3. seq20 卡住 —— 三层嵌套 cargo（**结构性问题，已实测取证**）

`seq20` = `check_full_gate_verification.sh`，其 D1–D5 维度会再跑一次 RC/GA 门禁。
实测的完整后代树（`sample` 栈快照见同目录 `020-hang-stack-*.txt`）：

```
check_full_gate_verification.sh (36233)
└── cargo test --no-fail-fast --test <51 个目标>      ← D6 批次 (55874)
    └── regression_test-059797ac006dab1a (77938)     ← 元测试：0.13 s CPU / 40 min
        └── cargo test --test physical_backup_test (14584)   ← 元测试**再起一层 cargo**
            └── physical_backup_test-69f4e737393ccdb2 (14817)
                ├── cargo test -p sqlrustgo-tools -- --nocapture (15654)
                │   └── incremental_backup_e2e_test (20460)
                └── cargo test -p sqlrustgo-tools physical_b... (15794)
                    └── incremental_backup_e2e_test (20459)
```

**`tests/integration/sql/regression_test.rs:326` 逐个测试文件 `Command::new("cargo")…`** ——
它是个**元测试**；`physical_backup_test` 又**并发**起两个嵌套 cargo。
三层嵌套共用同一个 target 目录 ⇒ 每层嵌套都要等锁、都要编译 ⇒ 实测**每层 ~3.5 min**，
整条链一小时都跑不完。

**栈证据**（`sample`）：
`regression_test` 阻塞在 `std::process::Command::output` → `read_output` → `poll()`
（等子进程输出），而子进程 cargo 阻塞在 `dispatch_semaphore_wait_slow`（等它自己的 jobserver）。
`lsof target/debug/.cargo-lock` 显示**当时没有人持有 build 锁** ⇒ 不是死锁，
是**嵌套调用 + 锁竞争 + 反复重编译**导致的数量级爆炸。

**这条链是「缓存被置脏」放大的**：lib 干净时，嵌套 cargo 不必重编译 ⇒ 每层是秒级；
lib 被置脏时，每层都要重编译 ⇒ 每层 3.5 min。**同一个门禁，007 轮的 seq20 能在 44.8 min 内跑完，
正是因为当时缓存是热的。**

## 4. 为什么中止而不是等它跑完

seq1 已经真红 ⇒ 本轮**不可能全绿**；seq20 的嵌套链按当前速率需要数小时，
必然撞上 `--timeout-sec 7200`（seq20 于 06:48 起跑、08:48 到期）⇒ 等下去只是把「一条已知的
超时」变成「一条已知的超时 + 多等一小时」。

seq33（`cargo test --all-features`）的判定**不依赖本轮**：修复树在**同一个提交** `1579950048`
上跑过 workspace 全量测试 ⇒ **7216 passed / 0 failed**（含 `concurrent_rollback_isolation`），
#5834 的修复已端到端验证。

## 5. 读这份证据时必须知道的事

- 本目录的 `results.jsonl` **只有 19 行**，不是 35 行。
- **不得**把「18 PASS / 1 FAIL」读成「这轮比别的轮差」——剩下 16 项**根本没有运行**。
- 本轮**不产生**总账结论；权威结论以跑满 35 项的后续轮为准。
- 本目录 `manifest.yaml` 的 `commands` 记满 35 项（分母来源）、`results` 只记 19 项（实际执行），
  `aborted.measured` 段记本轮的实测数据 —— **缺口本身就是证据**。

## 6. 后续实测更正（2026-11-17，010 轮起跑前）—— **本节更正第 3 节的归因**

第 3 节当时把 seq20 的停滞归因为「嵌套 + 锁竞争 + 反复重编译」的数量级爆炸。
010 轮起跑前的两项**新实测**把这段归因**收紧**了，结论方向不变，但原因要说准：

### 6.1 那条链里有一层**本来就要 11 分钟**

`physical_backup_test` 的第 8 处嵌套调用是：

```bash
cargo test -p sqlrustgo-tools -- --nocapture      # 跑 tools 包的**全部**测试
```

tools 包里含 `crates/tools/tests/v400_multi_model_soak.rs` —— 一个**压缩时间等价**的 soak：

| 档位 | 真实时长 | CI 等价时长（源码实测） |
|---|---|---|
| 24h | 86 400 s | **60 s**（行 370） |
| 72h | 259 200 s | **180 s**（行 411） |
| 168h | 604 800 s | **420 s**（行 442） |

⇒ 这一层的**合法耗时 = 60 + 180 + 420 = 660 s ≈ 11 min**，且它几乎不耗 CPU
（soak 在等时间，不在算东西）⇒ 与 sample 栈里「0.13 s CPU / 40 min」的形态完全吻合。

**009 的 4 份 sample 栈复看**也支持这一点：嵌套 cargo 停在
`cargo::ops::cargo_test::run_tests → run_unit_tests → ProcessBuilder::exec → wait4`
—— 它在**等自己的测试二进制退出**，不是在等锁。

⇒ **⇒ seq20 不是死锁。** 第 3 节写的「lsof 显示无人持 build 锁 ⇒ 不是死锁」结论仍然成立，
现在有了更完整的解释：**那一层本来就慢**。

### 6.2 真正的放大器是**磁盘**（本轮新发现，008/009 共通）

010 轮起跑前体检发现：`/Volumes/workspace` **97% 满（仅 34 GiB 可用）**，
该卷上并存约 30 个 sqlrustgo 工作树、每个都带一个 `target/`。在这种状态下实测：

- `ls target/debug/deps` 单次 **> 25 s 超时**；
- 元测试预检里**每一个嵌套 cargo 都要 45–60 s**（58 个条目中 42 个有效条目几乎全是
  45–114 s，而对应测试本身的 harness 报 `finished in 0.00s`
  ⇒ **时间不在测试上，在 cargo 启动 + 指纹扫描 + 文件系统颠簸上**）。

处置：回收废弃工作树与专用 target 目录的构建缓存（`cargo clean`，**纯产物、可重建**；
交付树/修复树/材料树的目标目录一律未动）⇒ 可用空间 **34 GiB → 236 GiB**。
同机同命令对照：`cargo build --all-features` 从 50 s → **1 s**（命中缓存），
`cargo build -p sqlrustgo-tools` 从 50 s → **6.82 s**。

⇒ **第 3 节说的「每层 ~3.5 min」应当读作「文件系统颠簸下的每层 45–60 s」**，
它不是链的结构成本，而是**启动条件**（磁盘余量）的成本。

### 6.3 元测试清单本身有历史漂移（**本轮不改测试代码，留给属主**）

`tests/integration/sql/regression_test.rs` 的清单里有 **57 个条目指向并不存在的
Cargo target 名**（`backup_test`、`buffer_pool_test`、`planner_test`…）。
实测：`cargo test --test backup_test` 立即报 `no test target named`（0.16 s 退出），
而 `Cargo.toml` 只声明了 482/487 个 `[[test]]`。有效条目 **42 个** + 6 个 `PKG:` 跨包条目。

这**不影响门禁判定**：meta-test 的聚合口径只数 passed/failed（42 个有效条目全绿 ⇒ 判绿），
但它意味着**元测试宣称覆盖的范围比实际覆盖的大**。
⇒ **本轮不动测试代码**（改动测试会改变门禁覆盖面，属于属主裁决范围），
登记为待办项。

### 6.4 对本轮结论的影响

**无影响**：本轮没有跑满 35 项，本来就不产生总账结论；seq1 的真红根因（第 2 节）
也没有被推翻 —— G10 的 60 s 探针被 22.87 s 重编译吃掉是**独立于磁盘**的事实。
本节只是把「为什么 seq20 没能跑完」说准，以便下一轮的启动条件能一次配对。

### 6.5 **真凶补齐：为什么「每次嵌套 cargo 45–60 s」**（rustc 启动要扫 76 万个文件）

`sample` 抓到的 rustc 栈（010 轮起跑前实测）：

```
rustc_session::config::build_session_options
  → rustc_session::search_paths::SearchPath::from_cli_opt
    → SearchPath::new
      → std::fs::ReadDir → readdir_r → __getdirentries64     ← 723/726 采样（99.5%）
```

rustc 每次启动都要 **readdir 扫描它的搜索路径**，其中就有
`target/debug/deps`。而历轮构建累积下来，这个目录实测：

```
ls target/debug/deps | wc -l   ⇒   764176      （2026-11-17 实测）
```

⇒ **每一次 rustc 启动都要付一次 76 万项的目录扫描**（30–60 s）。
⇒ 表现形式完全一致：
  - 每个**嵌套 cargo** 45–60 s（008 的「34.8 s/目标」、009 的「每层 3.5 min」）；
  - `ls target/debug/deps` 单次 > 25 s 超时（同样的 readdir）；
  - 预构建 482 个目标时「几乎不前进」：rustc 存活 3 min 只拿到 2 s CPU，
    新 rustc 不断顶上来 —— 它不是卡在编译，是卡在**数文件**。

⇒ **处置（已在 010 轮实测生效）**：`cargo clean` 清空 target/（纯产物、可重建）
+ `CARGO_INCREMENTAL=0`（增量目录本身就是文件数大户：每个单元一个会话目录、
  几十万个 .o 小文件）⇒ deps 目录只剩「二进制 + .d + rlib」量级 ⇒ readdir 回到毫秒级。

**⇒⇒ 这一条同时解释了 008 与 009 的两个「慢」数字**：它们不是测试慢，
也不是编译慢，是**目录项太多**。

### 6.6 tools 包那一层的合法成本是 **25 分钟**（实测，不是 11 分钟）

010 轮起跑前在同一棵交付树上实测：

```
( time timeout 1500 cargo test -p sqlrustgo-tools -- --nocapture )
real 25m0.011s      user 0m0.616s      sys 0m0.618s      rc=124
```

`rc=124` = 被 1500 s 的 timeout 在**最后的 Doc-tests 阶段**截断；在此之前
**所有测试二进制全绿**（含 soak 的 29 passed）。`user+sys ≈ 1.2 s` ⇒
这 25 分钟里几乎全在**等**（soak 的压缩时间等价），不是在算。

⇒ **`physical_backup_test` 里的那一层（`cargo test -p sqlrustgo-tools`）合法成本 ≈ 25 min。**
009 轮在 40 min 处中止时，元测试链里的 42 个有效条目（每个 45–60 s，≈ 30 min）
**刚好还没走到这一层** —— 中止时点落在链条的中段，不是卡死。
