# 批次 42 证据 —— 崩溃测试必须崩在它声称的边界上（#5556）

| 字段 | 值 |
|---|---|
| gate_policy_eval_id | `v410-b42-fault-occurrence-001` |
| source_agent | openclaw-mcode (MiniMax Code) |
| source_run | 批次 42（本机实测，Mac-mini-7.local arm64 macOS 27.0.0） |
| timestamp | 2026-10-25T00:52:00+08:00 |
| 基线 | `572f2b0ad25102d33ee5f56ea35170459aee5a21`（批次 41 之后、批次 42 之前） |
| 本批提交 | `25f23dc55af4a6666c5c61d09cbc68fd5a60c63d` |
| 工作树 | `wt-v410-b40`（分支 `fix/ddl-case-and-idle-reaper-batch40`） |
| 对照工作树 | `wt-v410-b42base`（detach `572f2b0ad2`，BASE 侧同机同负载对照） |
| 关联 issue | #5556（崩溃测试挂死）；本批新登记 #5585（CREATE INDEX 不认会话当前库） |

---

## 0. 一句话

#5556 报的是"崩溃测试挂死"。本批把它拆成**三个互相独立**的缺陷，逐个给根因、修法和 A/B 测量。

最要命的那条**不是"慢"**，而是：**崩溃从来没有落在它声称的边界上**。
测试断言的是一个"崩溃发生在脚本*之后*"的世界 —— 断言本身因此是假的。

| 方向 | 修前 | 修后 |
|---|---|---|
| **挂死** | 故障点在**每次**到达时都 hold 120 s ⇒ 一条脚本里的 N 条写语句付 N × 120 s | 每个点**只在指定的 occurrence** hold 一次；hold 时长可配 |
| **边界** | 阻塞式客户端 ⇒ 脚本等不到回应 ⇒ harness 只能在脚本**结束之后**才杀 server，SIGKILL 落在脚本之后 | 客户端进后台 ⇒ harness 立刻反应 ⇒ SIGKILL 落在被测的提交边界**上** |
| **孤儿** | 没有 `Drop` ⇒ server 子进程自旋到死，拖住整个测试二进制 | `impl Drop for Server` 回收孤儿 server |
| **断言** | `sum==10` —— 一个"崩溃在脚本之后"的假象 | `sum==6` —— **更强**：显式事务那一行（未提交）必须不存在 |
| **耗时**（同机同条件对照，见 §6.1） | `crash_commit_fsync_b1` **240.84 s** / `crash_wal_index_b1` **≥ 1800 s（超时被杀）** | **0.50 s** / **5.30 s** |

> **同时闭环 #5376.** 读完 #5376 才发现：**它就是本批修的这条**，
> 而且它的「建议 1 / 建议 2」与本批的修法逐字吻合
> （"把 seed SQL 放到后台线程执行"、"reach() 的 120 秒自旋是 backstop 而不是同步手段"）。
> ⇒ #5556 与 #5376 是**同一根因**，本批一并收口。

---

## 1. 三个子问题

### 1.1 子问题一：故障点在**每次**到达时都 hold 120 秒

**根因.** 旧的 `reach()` 在匹配到故障点后写信号，然后**无条件**自旋到 120 秒截止：

```rust
write_signal(&cfg);
let deadline = Instant::now() + Duration::from_secs(120);
while Instant::now() < deadline { sleep(20ms); }
```

一个脚本里有 N 条会写 WAL / 提交事务的语句，就有 N 次到达 ⇒ N × 120 秒。
`crash_commit_fsync_b1` 的两个用例各付一次 ⇒ 串行实测 **240.84 s**（= 2 × 120 s）。

**修法.** 引入 **per-point 的 occurrence 计数**：进程只在**指定的那一次**到达时 hold，其余到达直接通过。

- `FaultConfig` 增字段 `occurrence`，由 `SQLRUSTGO_FAULT_OCCURRENCE` 决定，默认 `1`。
- 计数用 `FaultPoint::index()` 给出的**每点独立**的原子量，不用全局计数：
  一个进程会先后经过多个故障点（`a06_failpoint_reachability` 正是这么测的），
  全局"已触发"标志会让第二个点**永远不再宣告自己**。
- 每点再加一个 `FIRED` 标志：hold 过一次之后，后续到达直接通过 ——
  边界是进程生命里的**一个点**，不是一种模式。

### 1.2 子问题二：崩溃从未落在它声称的边界上（本批最要命的一条）

**根因.** 旧测试用**阻塞式** mysql 客户端（`sql()`）跑脚本，但 server 在故障点 hold。
于是：

1. 脚本永远等不到回应；
2. harness 只能在**脚本结束之后**才发现信号、才杀掉 server；
3. ⇒ SIGKILL 落在**脚本之后**，而不是落在被测的那个提交边界**上**。

也就是说：这个测试**从来没有测过它名字说的东西**。它测的是"脚本跑完之后崩掉，数据还在"。

**这条为什么致命.** 旧断言是 `sum==10`（种子 1+2+3 = 6，显式事务那行 4 = 10）。
`sum==10` 成立的前提是"那次显式事务**已经提交了**" —— 这正是
"崩溃落在脚本之后"的世界观。在"崩溃落在提交边界上"的世界观里，
那次事务**必须不存在**，正确答案是 `sum==6`。

于是旧断言不是"不够严"，而是**方向相反**：它把 bug 当成了期望。

**修法.**

1. 新增 `sql_background()`：客户端进后台线程执行脚本，主流程不等它；
   客户端存活上限 `CLIENT_LIFETIME = 30 s`，超时后被杀掉，返回 `Receiver`。
2. 显式事务用例改用 `with_occurrence(1)` —— 实测唯一会触发 `AfterCommitFsync` 的那次提交
   （见 §1.5 的触发点普查）。
3. 断言改为 `sum==6`，并写明**为什么**旧值是假象。
4. `control` 用例（不提交新事务）同样改后台客户端；删掉 kill 前的 `sum=="3"` 断言
   —— 那一刻 server 正持锁卡在提交边界上，那个断言已经无意义；
   保留**重启后**的 `after=="3"`（那才是它真正要说的：没提交 ⇒ 数据不变）。

### 1.3 子问题三：孤儿 server 拖住整个测试二进制

**根因.** 旧 `Server` 没有 `Drop`。测试 panic 之后，server 子进程还在 120 秒的 hold 里自旋；
测试二进制的 stdout/stderr 管道被子进程持有 ⇒ cargo 必须等它退出 ⇒
**整个测试二进制**被拖住 120 秒，哪怕断言早就失败完了。

这条解释了为什么 `crash_wal_index_b1` 在 BASE 上是 **≥ 1800 s（超时被杀）** 而不是"秒级失败"。

**修法.** `impl Drop for Server`：进程还活着就 `kill()` + `wait()`。

---

## 2. 配套改动

### 2.1 hold 时长可配（`SQLRUSTGO_FAULT_HOLD_SECS`，默认 120）

有些用例的崩溃**必须落在脚本之后**（否则被测对象还没建好），
那种情况下 120 秒的 hold 是 120 秒的纯等待。`index_update` 用例改用 `5 s`。

为什么是 5 s：mysql 客户端自身的读超时是 30 s。hold 若 ≥ 30 s，
客户端先超时**重连**；重连后的会话**没有 `USE d1`** ⇒ 脚本死于
`ERROR 1146 Table not found: t`。5 s 让 hold 在客户端超时之前结束。

### 2.2 `file_storage::save_index_in` 改为原子写

旧实现直接 `File::create(目标路径)` 然后往里写 JSON ⇒ 崩溃会留下**截断**的索引文件。
下次启动 `load_index_in` 解析失败（`serde_json` → `InvalidData`），
后果不是"这个索引没了"，而是"**这张表完全读不了**"（全扫描和索引查询一起失败）。

改成：写临时文件 `{table}_idx_{col}.json.tmp` ⇒ `rename` 就位。
`rename` 在同一目录内是原子的 ⇒ 崩溃后文件要么是上一个完整版本，要么不存在，
**绝不会**是半截的。

**范围声明（重要）.** 索引文件自身的**持久性语义未变** —— 它本来就没有 fsync，
本批**也不新增** fsync。本批只修掉"半截文件"这一个成因。
（是否该给索引文件补 fsync ⇒ 见 #5584 的待裁决项。）

**副作用核查.** 仓里有两处按文件名扫索引文件，两处都不会把 `.json.tmp` 误认：

| 位置 | 判据 | `.json.tmp` | 结论 |
|---|---|---|---|
| `file_storage.rs:1473`（`load_tables`） | `path.extension() == "json"` | extension 是 `tmp` | 跳过 ✅ |
| `file_storage.rs:1527`（`load_all_indexes`） | `file_name.ends_with(".json")` | 结尾是 `.tmp` | 跳过 ✅ |

---

## 3. 既有红：`index_update` 的根因**一字未变**

`index_update_crash_does_not_serve_dangling_rows` 修前修后**都红**，而且：

- **失败信息完全一致**（BASE 与 FIXED 的 stderr 逐字相同）：
  ```
  seed sql: script failed: CREATE DATABASE d1; USE d1; CREATE TABLE t(...); CREATE INDEX idx_v ON t(v);
  ERROR 1146 (42000) at line 1: Table not found: t
  ```
- 变的只有**耗时**：≥ 1800 s → 5.30 s。

即：本批把这个红从「**挂死**」变成「**快速失败**」，根因**一个字节都没动**。
按"既有红不背锅也不掩盖"的规矩，它**不能**在本批被"修绿" ——
放水（改断言、跳过用例）才是掩盖。

根因是**产品缺陷**，不是测试缺陷 ⇒ 登记为 #5585（见 §4）。

---

## 4. 本批新发现：`CREATE INDEX` 不认会话当前库 ⇒ 登记 #5585

`index_update` 用例的 seed 里，`CREATE INDEX idx_v ON t(v)` 报
`ERROR 1146 Table not found: t`。追到执行器：

`src/execution_engine_methods.rs:827` `execute_create_index` 用
`storage.get_table_info(table_name)` —— **不带库名**。

于是 `USE d1` 之后，`CREATE INDEX ... ON t(v)` 查的是**默认库**里的 `t`，
不是 `d1.t` ⇒ 找不到表。

两条旁证（都在本批实测）：

1. 在**默认库**里 `CREATE INDEX` "成功"，但 `SHOW INDEX` 只有 `PRIMARY`，
   没有 `idx0` ⇒ 索引**根本没建上**，"成功"是假的；
2. 该表名解析错误也正是 `index_update` 崩溃后重启报
   `ERROR 1105 Unknown database: d1` 的同族问题（DDL 不进 WAL，见 #5584）。

**影响面.** 任何"先 `USE db` 再 `CREATE INDEX`"的用法都建不上索引，
随后走索引的查询要么全表扫描、要么用错库的数据。

**为什么本批不修.** 这是**产品语义**决策（`CREATE INDEX` 是否应当遵循会话当前库），
不是"把前提从假设换成测量"能带过去的；且它影响的不止这一条路径。
⇒ 按"不代属主决定"，登记为 #5585，等属主裁决修法与优先级。

---

## 5. 改动清单

| 文件 | 改动 |
|---|---|
| `crates/storage/src/fault_injection.rs` | `FaultConfig` 增 `occurrence`（`SQLRUSTGO_FAULT_OCCURRENCE`，默认 1）；`reach()` 改为每点只 hold 一次；hold 时长可配（`SQLRUSTGO_FAULT_HOLD_SECS`，默认 120）；新增 per-point `OCCURRENCES` / `FIRED` 原子量与 `FaultPoint::index()` |
| `crates/storage/src/file_storage.rs` | `save_index_in` 改为原子写（临时文件 + `rename`） |
| `crates/storage/tests/fault_coordination.rs` | 新增 `sql_background()`（后台客户端 + `CLIENT_LIFETIME=30 s` + `Receiver`）、`Arm::with_occurrence()`、`Arm::with_hold_secs()`；新增 `impl Drop for Server` 回收孤儿 server；`sql` 与 `with_hold_secs` 加 `#[allow(dead_code)]`（本模块被多个测试二进制各自 `#[path]` include，各二进制只用到其中一部分） |
| `crates/storage/tests/crash_commit_fsync_b1.rs` | 主用例改后台客户端 + `with_occurrence(1)`；断言 `sum==10` → `sum==6` 并写明旧值是假象；control 用例改后台客户端，删掉 kill 前的 `sum=="3"` 断言，保留重启后的 `after=="3"` |
| `crates/storage/tests/crash_wal_index_b1.rs` | wal_append 用例改后台客户端 + `with_occurrence(4)`；index_update 用例保留阻塞客户端 + `with_hold_secs(5)`（崩溃必须在脚本之后） |

---

## 6. 回归矩阵

| 对象 | 命令 | 结果 |
|---|---|---|
| `crash_commit_fsync_b1` | `cargo test -p sqlrustgo-storage --test crash_commit_fsync_b1` | **2 passed / 0 failed**，**0.50 s** |
| `crash_wal_index_b1` | `cargo test -p sqlrustgo-storage --test crash_wal_index_b1` | 1 passed / **1 failed（既有红，根因未变）**，**5.30 s** |
| `a06_failpoint_reachability` | `cargo test -p sqlrustgo-storage --test a06_failpoint_reachability` | **2 passed**，2.04 s（故障点可达性未被 occurrence 改动破坏） |
| fmt | `cargo fmt -p sqlrustgo-storage -- --check` | **CLEAN** |
| clippy | `cargo clippy -p sqlrustgo-storage --all-targets --all-features` | 无新增 warning（两个测试二进制各 2 个**既有** warning：`Scratch::keep` / `Arm::with_sync_mode`） |
| 临时文件副作用 | 静态核查 `file_storage.rs:1473` / `:1527` 两处文件名扫描 | `.json.tmp` 两处均被跳过 ✅ |

### 6.1 BASE / FIXED A/B 对照（**同机、同负载、同参数**）

对照方法：在 `wt-v410-b42base`（detach `572f2b0ad2`，即批次 42 之前）上
`cp -Rc` 克隆同一份 `target/`（APFS copy-on-write，编译产物一致），
用**完全相同的命令**跑：

```bash
cargo test -p sqlrustgo-storage --test <binary> -- --test-threads=1
```

| 测试二进制 | BASE（`572f2b0ad2`） | FIXED（`25f23dc55a`） | 加速 |
|---|---|---|---|
| `crash_commit_fsync_b1` | 2 passed，**240.84 s** | 2 passed，**0.50 s** | **482×** |
| `crash_wal_index_b1` | 1 passed / 1 failed，**≥ 1800 s（超时被杀）** | 1 passed / 1 failed，**5.30 s** | **≥ 340×** |

**`crash_wal_index_b1` 的 BASE 数字是保守下界，理由如下.**
测量在 **1800 s 时被超时终止**（测试进程仍在运行、未结束）⇒ 它**至少**跑了
1800 秒。两个用例的**分段时间**都已直接观测到，与"每次到达都 hold 120 s"
的机制**吻合**：

| BASE 分段 | 观测 | 与机制的关系 |
|---|---|---|
| `crash_commit_fsync_b1` 整体 | **240.84 s** | = **2 × 120 s**（两个用例各到达一次） |
| `crash_wal_index_b1` 的 wal_append 用例 | server 进程 **12:41 → 12:49**（480 s） | = **4 × 120 s**（故障点在该脚本里被到达 **4** 次） |

⇒ 仅 wal_append 一段就是 **4 × 120 = 480 s**；index_update 一段的 seed
（`CREATE TABLE` 建主键索引 + `INSERT` 更新索引）到达次数更多，
两段合计**超过 1800 s**（被超时终止，未能测到终点）。

与 FIXED 侧的 **5.30 s** 相比 ⇒ **≥ 340×**。

> **这个下界本身是一条独立的发现.** 旧代码下，**仅** `crash_wal_index_b1`
> 这一个测试二进制就跑了 **≥ 1800 s 仍未结束**。CI 的 `test` job 预算
> **9000 s**，`test-crates` 预算 **5400 s** —— 一个二进制就吃掉
> `test-crates` 预算的三分之一。这不是"CI 慢"，是**测试本身在自旋**。

这 4 次到达也正是 FIXED 侧选 `with_occurrence(4)` 的依据 ——
occurrence 的数字**来自实测到达次数**，不是猜的。

**BASE 侧的失败信息与 FIXED 侧逐字相同**（`index_update` 的
`ERROR 1146 (42000) at line 1: Table not found: t`）——
变的只有耗时：**从「挂死」变成「快速失败」**。

### 6.2 孤儿进程：不泄漏（实测，不是推断）

`impl Drop for Server` 的效果用**前后进程数对照**实测（`ps` 计数，
匹配 `wt-v410-b40/target/debug/sqlrustgo-mysql-server`）：

| 时点 | 孤儿 server 进程数 |
|---|---|
| 跑 `crash_commit_fsync_b1` **之前** | 11 |
| 跑完 **之后** | **11**（无新增） |

（跑之前那 11 个是**加 `Drop` 之前**的历史遗留，不是本次产生的。
旧代码**必然泄漏**：BASE 侧对照跑完后，多出 3 个 `wt-v410-b42base/…` 的孤儿。）

⇒ 孤儿回收是**承重的**：修前每次测试都留下一个永远不退出的 server
（占内存、占监听端口），修后不新增。

---

## 7. 可复现命令

```bash
# BASE / FIXED 对照
cd /Volumes/workspace/dev/sqlrustgo-worktrees/wt-v410-b42base   # BASE: 572f2b0ad2
cd /Volumes/workspace/dev/sqlrustgo-worktrees/wt-v410-b40       # FIXED: 25f23dc55a

# 改过 fault_injection.rs 后必须重建带 feature 的 server，
# 否则 server 读不到新的 env（occurrence / hold_secs）
cargo build -p sqlrustgo-mysql-server --features sqlrustgo-mysql-server/fault-injection

# 三个故障注入目标
cargo test -p sqlrustgo-storage --test crash_commit_fsync_b1     -- --nocapture --test-threads=1
cargo test -p sqlrustgo-storage --test crash_wal_index_b1         -- --nocapture --test-threads=1
cargo test -p sqlrustgo-storage --test a06_failpoint_reachability -- --nocapture --test-threads=1

# fmt / clippy
cargo fmt -p sqlrustgo-storage -- --check
cargo clippy -p sqlrustgo-storage --all-targets --all-features
```

---

## 8. 需要属主裁决的

1. **#5585（新建）**：`CREATE INDEX` 是否应遵循会话当前库？修法与优先级？
   本批**只登记、不改行为**。
2. **#5584（已登记，本批补一条证据）**：显式事务的 COMMIT 是否应触发
   `AfterCommitFsync`？索引文件是否应补 fsync？半建成索引的恢复语义？
3. **索引文件的 fsync**：`save_index_in` 本批只做了原子写，**没有**补 fsync
   （原样保留既有持久性语义）。是否需要补 ⇒ 属主裁决。
