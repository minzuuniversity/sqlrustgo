# 批次 40 证据 —— 落盘 DDL 列名大小写（#5509）/ 空闲回收器的「禁用」开关（#5514）

| 字段 | 值 |
|---|---|
| gate_policy_eval_id | `v410-b40-ddl-case-reaper-001` |
| source_agent | openclaw-mcode (MiniMax Code) |
| source_run | 批次 40（本机实测，Mac-mini-7.local arm64 macOS 27.0.0） |
| timestamp | 2026-10-24T22:10:20+08:00 |
| 基线 | `develop/v4.1.0` = `5d4817a06b9dbfa0ca232ab639975d31a887e539`（PR #5576 合并后） |
| 工作树 | `wt-v410-b40`（分支 `fix/storage-ddl-column-case-5509-batch40`） |
| 关联 issue | #5509（落盘 DDL 列名匹配大小写敏感）、#5514（30 s 空闲回收器误杀长任务）、**#5582（本批新建，见 §3）** |

---

## 0. 一句话

这一批修的是**同一份 SQL 在不同后端上结果不同**的缺陷，并顺手测出
**一个从未生效过的机制**——后者没有被顺手修掉，而是登记成了 #5582，
因为修它会把本 issue 抱怨的误杀换个路径重新引入。

| issue | 缺陷 | 本批处置 |
|---|---|---|
| **#5509** | `FileStorage` 的列级 DDL 用**大小写敏感**匹配，`MemoryStorage` 用大小写不敏感 ⇒ 同一份 SQL 在内存后端成功、在落盘后端报 `Column not found` | 9 处手写比较**收敛成 1 处**（复用存储 crate 已有的 SSOT `find_column_index_in`），并补齐两后端的**存储态**一致性 |
| **#5514** | `SQLRUSTGO_IDLE_TIMEOUT_SECS=0` 被 `.filter(\|n\| *n >= 1)` 过滤掉 ⇒ **没有「关闭回收器」的开关** | `0` 现在真的等于**禁用**（读超时也不武装），补纯判定函数 + 单测 + 端到端三例 + 文档 |
| **#5582**（新建） | 空闲回收器**从未真正关闭过任何连接**（`Weak` 的唯一强引用是临时值 ⇒ `upgrade()` 恒失败） | 只取证、不修：修它会把 #5514 的误杀引入 TLS 客户端（§3） |

---

## 1. #5509 —— 列名匹配这条规则有 9 份实现，其中 4 份是大小写敏感的

### 1.1 修之前的分布

| # | 位置 | 匹配方式 | 大小写 |
|---|---|---|---|
| 1 | `crates/storage/src/file_storage.rs` `drop_column` | `c.name == column` | **敏感** |
| 2 | `crates/storage/src/file_storage.rs` `set_column_default` | `c.name == column` | **敏感** |
| 3 | `crates/storage/src/file_storage.rs` `rename_column` | `c.name == old_name` | **敏感** |
| 4 | `crates/storage/src/file_storage.rs` `modify_column` | `c.name == column` | **敏感** |
| 5 | `crates/storage/src/engine.rs` `MemoryStorage::drop_column` | `c.name.to_lowercase() == column.to_lowercase()` | 不敏感 |
| 6 | 同上 `set_column_default` | 同上 | 不敏感 |
| 7 | 同上 `modify_column` | 同上 | 不敏感 |
| 8 | 同上 `rename_column` | 同上 | 不敏感 |
| 9 | `src/engine_ddl.rs` `AlterColumnOperation::SetDataType` | `c.name.to_lowercase() == name.to_lowercase()` | 不敏感 |

⇒ **同一个 `ALTER TABLE … DROP/MODIFY/SET DEFAULT/RENAME COLUMN`，
在内存后端上成功、在落盘后端上报 `Column not found`。**
落盘后端 = 生产后端。

仓库里**已经有一份**列名解析的 SSOT：
`crates/storage/src/engine.rs::find_column_index_in`（L843），
其文档自称 "Single source of truth for column-name resolution"，
第 2 层就是大小写不敏感匹配。SQL 层的 `SELECT` 走的也是它
（`src/engine_utils.rs::find_column_index`，同一套分层规则）。
⇒ **本批没有新写解析器**，只是把那 9 处手写比较全部指向它。

### 1.2 顺带发现的第二类不一致：**存储态**

匹配层统一之后还剩一处跨后端分歧：`modify_column` 写回的列名。

| 路径 | `MemoryStorage` | `FileStorage`（修前） | 结果 |
|---|---|---|---|
| `create_table` | 原样保留 | 原样保留 | 一致 |
| `add_column` | 原样保留（#4154 的明确设计） | 原样保留 | 一致 |
| `rename_column` | 小写化 | 小写化 | 一致 |
| `modify_column` | **小写化** | **原样保留** | **✗ 分歧** |

⇒ 同一段 DDL 在两个后端上 `SHOW COLUMNS` 会显示不同的列名。
本批让 `FileStorage::modify_column` 也小写化，**两后端逐字一致**。

### 1.3 变异 A/B —— 判据承重

| 变异 | 结果 |
|---|---|
| **BASE**（本批改动） | `file_storage_ddl_column_case_5509`：**11 passed / 0 failed** |
| **MUT-1**：`FileStorage::drop_column` 改回 `c.name == column` | **3 红**：`drop_column_finds_the_column_case_insensitively`、`the_ddl_history_does_not_change_whether_a_column_is_found`、`both_backends_agree_on_the_ddl_outcome`。跨后端断言的原文输出：`backends disagree for CREATE(B) / DROP(b): memory=Ok(()) file=Err("Execution error: Column not found: b")` —— **逐字复现 issue 正文描述的症状** |
| **RESTORE** | 11 passed / 0 failed |
| **MUT-2**：去掉 `FileStorage::modify_column` 的 `to_lowercase()` | **1 红**：`the_stored_name_does_not_depend_on_the_backend`，原文输出 `left: ["b"] / right: ["B"]` —— 跨后端 schema 分歧 |

### 1.4 新增回归用例（11 个）

`crates/storage/tests/file_storage_ddl_column_case_5509.rs`

| 用例 | 钉住什么 |
|---|---|
| `drop_column_finds_the_column_case_insensitively` | 落盘后端 `DROP COLUMN b` 命中存为 `B` 的列 |
| `set_column_default_finds_the_column_case_insensitively` | 同上（`SET DEFAULT`） |
| `drop_column_default_finds_the_column_case_insensitively` | 同上（`DROP DEFAULT`） |
| `modify_column_finds_the_column_case_insensitively` | 同上（`MODIFY COLUMN`） |
| `rename_column_finds_the_column_case_insensitively` | 同上（`RENAME COLUMN`） |
| `a_missing_column_still_errors_on_both_backends` | **放宽不得变成静默成功**：缺失列在两个后端都报错 |
| `the_exact_case_spelling_still_works` | 原样拼写仍然可用（放宽不是替换） |
| `both_backends_agree_on_the_ddl_outcome` | **核心断言**：同一份 SQL 在两个后端上结论相同（3 组拼写组合） |
| `the_stored_name_does_not_depend_on_the_backend` | 同一段 DDL 在两个后端存下**同样的列名** |
| `the_ddl_history_does_not_change_whether_a_column_is_found` | 建为 `B` / 建为 `b` / 改过名 三种来历都可被任一拼写命中 |
| `the_sql_layer_matches_the_column_case_insensitively_on_disk` | 端到端（落盘后端 + SQL 层）：镜像 DuckDB 一致性夹具 `duckdb_samples/case_insensitive_alter.test` |

> **为什么这个夹具重要**：`crates/sqlrustgo_sqllogictest` 的 runner **只用 `MemoryStorage`**
> （`crates/sqlrustgo_sqllogictest/src/main.rs`）⇒ 落盘后端的这一族行为**从来没被测过**，
> 不是「测过是绿的」，是**根本没测**。

---

## 2. #5514 —— `SQLRUSTGO_IDLE_TIMEOUT_SECS=0` 现在真的等于「禁用」

### 2.1 修之前的语义

```rust
std::env::var("SQLRUSTGO_IDLE_TIMEOUT_SECS")
    .ok()
    .and_then(|s| s.parse::<u64>().ok())
    .filter(|n| *n >= 1)      // ← 0 在这里被吃掉
    .unwrap_or(30)
```

⇒ `0` ⇒ `30`。**没有「关闭回收器」的开关。**

更关键的是：这个值同时喂给**两个**机制，而 issue 只描述了其中一个：

1. **回收器线程**（`spawn_idle_connection_reaper`）：静默超过阈值就 `shutdown(Both)`；
2. **每连接 socket 读超时**（`handle_connection` 里的 `set_read_timeout`）：
   静默超过阈值 ⇒ 阻塞读返回 `TimedOut` ⇒ 命令循环退出 ⇒ 连接关闭。

⇒ 只关掉其中一个（例如只关回收器）**不足以**让「客户端在算长任务」的连接活下来，
因为 socket 读超时照样会掐。**本批两个一起关。**

### 2.2 修之后的语义

| `SQLRUSTGO_IDLE_TIMEOUT_SECS` | 结果 | 含义 |
|---|---|---|
| 未设置 | `Some(30)` | 默认（与修前一致） |
| `0` | `None` | **禁用**：不武装读超时、回收器不回收任何连接 |
| `N ≥ 1` | `Some(N)` | 阈值（运行时可改，回收器每轮重读） |
| 不可解析 | `Some(30)` | 回落到默认（配置写错**不得**悄悄关掉保护） |

`0` 的语义对齐 MySQL 的 `wait_timeout=0`。

判定被抽成**纯函数**，因此「禁用」这一档可以脱离 socket / 服务器 / sleep 来测：

```rust
fn idle_reap_decision(idle_secs: u64, timeout_secs: Option<u64>) -> Option<u64>
```

⇒ `None` ⇒ 无论静默多久都不回收；`Some(t)` ⇒ 静默 `> t` 才回收（并返回 `t`，
让日志与判定不会漂移）。

### 2.3 端到端三例（`crates/mysql-server/tests/idle_reaper_heartbeat_5514.rs`）

| 用例 | 场景 | 断言 | 耗时 |
|---|---|---|---|
| `a_heartbeat_keeps_the_connection_alive` | 阈值 2 s、每 300 ms 发一次 `COM_PING`，共 12 轮（静默时长约 3.6 s ≫ 阈值） | 12 次心跳**全部成功**，且之后仍能执行查询 | ~3.6 s |
| `a_silent_client_is_reaped` | 阈值 2 s，客户端连接后**不说话** 5 s | 随后的查询**必须失败**（连接已被关闭）⇒ #4682 的症状不复发 | ~5 s |
| `zero_disables_the_idle_timeout` | `SQLRUSTGO_IDLE_TIMEOUT_SECS=0`，静默 **33 s** | 连接**仍然活着**且能执行查询 | ~33 s |

实测：`3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 41.93s`。

> **为什么第三例要睡 33 秒**：旧代码把 `0` 映射到 **30 s** 默认，
> 所以从外部唯一能区分「禁用」与「旧的 0 → 30」的办法，就是**熬过 30 s**；
> 更短的时间在旧代码下也会通过 ⇒ 钉不住任何东西。
> 「映射本身」由单测 `read_idle_timeout_zero_disables_the_reaper` 钉住，
> 这一例钉的是**可观察后果**。

> **一个必须说清的事实**：临时服务器路径（`start_ephemeral`）**不启动**回收器
> （只有 `run_server_v2` 这条生产路径启动）。测试因此显式调用
> `spawn_idle_connection_reaper()`，否则心跳用例会**假绿**
> ——见 §2.4 的 MUT-B。

### 2.4 变异 A/B

| 变异 | 结果 |
|---|---|
| **BASE** | 单测 6 passed / 0 failed；e2e 3 passed / 0 failed（41.93 s） |
| **MUT-A**：`read_idle_timeout_secs` 改回旧写法（`0` ⇒ 30） | 单测**红**：`left: Some(30) / right: None`；e2e `zero_disables_the_idle_timeout` **红**：`Io(Custom { kind: UnexpectedEof, error: "unexpected EOF during read_exact" })` —— **正是 issue 描述的「客户端看到传输层错误」** |
| **MUT-B**：把 `touch_connection`（读成功后刷新 idle 计时）整个去掉 | **仍然绿** |

**MUT-B 仍然绿不是测试的弱点，而是事实**，必须写进证据：

真正兜底的是**每连接 socket 读超时**——它被**每一次成功读**重新武装，
所以「客户端还在说话」的连接本来就不会超时；回收器的 tracker 在今天的实现里
**根本关不掉任何连接**（§3）。因此「心跳保活」这条 AC 的承重者是读超时，
不是 tracker。

### 2.5 文档（AC4）

`docs/releases/v4.1.0/DEPLOYMENT_GUIDE.md`

* 环境变量表：补齐**默认值 / 单位（秒）/ `0` 的含义 / 不可解析值的回落 /
  为什么长任务客户端需要 `0` / `0` 的代价（半开连接会一直占住一个服务线程，
  只应在可信网络关闭）**；
* 排障表新增一条：「客户端只是在算任务却被断连」⇒ `SQLRUSTGO_IDLE_TIMEOUT_SECS=0`
  或调大，或让客户端发 `COM_PING`；
* 排障表里原有的 `CLOSE_WAIT` 条目**改写**：原文写「idle reaper did not run」，
  而它**从来没有 run 过**（§3）—— 改成如实描述（真正兜底的是读超时，
  TLS 连接不受 `SO_RCVTIMEO` 约束）。

---

## 3. 附带发现：空闲回收器**从未真正生效**（新建 #5582）

### 3.1 根因

```rust
// handle_connection()
let cid = register_connection(Arc::new(stream.try_clone().unwrap()));
//                                     ^^^^^^^^^^^^^^^^^ 临时值

pub fn register_connection(stream: std::sync::Arc<std::net::TcpStream>) -> u64 {
    let weak = std::sync::Arc::downgrade(&stream);  // 只留 Weak
    CONNECTION_REGISTRY.write().insert(conn_id, (ConnectionTracker::new(), weak));
    conn_id
}                                                    // ← 唯一的强引用在此被丢弃
```

⇒ `Weak::upgrade()` **恒返回 `None`** ⇒ 回收器每轮只是把注册表条目删掉，
**socket 一个都没关**。代码注释里 "This is the load-bearing fix for the high-CPU bug"
与事实不符。

### 3.2 实测（探针，已从代码移除）

```text
PROBE A (production shape): conn_id=1 weak.upgrade().is_some()=false
PROBE B (strong ref kept):  conn_id=2 weak.upgrade().is_some()=true
```

⇒ 生产形态下 `upgrade()` 恒失败。旁证即 §2.4 的 MUT-B。

### 3.3 为什么**不在本批修**

把 `Weak` 变成可 `upgrade`，回收器就**开始真的关闭连接**。而它唯一的判定规则是
「静默超过阈值」——与 socket 读超时**完全等价**（两者都在任何一次成功读后重置）。
差别只在 **TLS**：

* 非 TLS：读超时已经兜住了，回收器生效后行为等价；
* **TLS：rustls 忽略 `SO_RCVTIMEO`** ⇒ 代码注释自己写着这一点
  ⇒ 目前「客户端在算长任务」的 TLS 连接**能活下来**；
  回收器一生效，就会在 30 s 静默后把它掐掉 ——
  **这正是 #5514 抱怨的场景 B 误杀，只是换了个路径重新引入。**

⇒ 先答出 #5514 第 2 点（服务端如何区分「在算长任务」与「已死」），
再让回收器生效。已登记为 **#5582**（含全部取证与建议），
并在该 issue 上留了 CLAIM 评论（comment **196715**）说明为何不在本批修。

---

## 4. 改动清单

| 文件 | 改动 |
|---|---|
| `crates/storage/src/file_storage.rs` | 4 处列级 DDL 匹配改用 `find_column_index_in`；`modify_column` 补 `new_def.name.to_lowercase()`；`set_column_default` 的过期注释重写 |
| `crates/storage/src/engine.rs` | `MemoryStorage` 的 4 处列级 DDL 匹配改用 `find_column_index_in`；`set_column_default` 的历史对照表标注为「#5472 当时的状态」 |
| `src/engine_ddl.rs` | `AlterColumnOperation::SetDataType` 的第 9 处手写比较改用 `find_column_index_in` |
| `crates/mysql-server/src/lib.rs` | `read_idle_timeout_secs()` → `Option<u64>`（`0` = 禁用）；新增纯函数 `idle_reap_decision()`；回收器线程按 `None` 跳过回收；socket 读超时在禁用时**不武装**；4 个新单测 + 旧单测改写；旧单测补上进程级 env 锁（消除并行 flake） |
| `crates/storage/tests/file_storage_ddl_column_case_5509.rs` | **新增** —— 11 个回归用例 |
| `crates/mysql-server/tests/idle_reaper_heartbeat_5514.rs` | **新增** —— 3 个端到端用例 |
| `docs/releases/v4.1.0/DEPLOYMENT_GUIDE.md` | 环境变量表 + 排障表（AC4） |
| `docs/releases/v4.1.0/evidence/BATCH40_DDL_CASE_REAPER_2026-10-24.md` | **新增** —— 本文档 |

**未改动**：`Cargo.toml`（两个新测试文件都在 cargo 自动发现的目录下，
不需要新增 `[[test]]` 声明；`crates/storage/tests/` 与 `crates/mysql-server/tests/`
本来就是自动发现的，顶层 `tests/` 才是必须显式声明的）。
`tests/baseline/test_count.json` 的 `active_tests` 计数会因新增 14 个 `#[test]` 而上升
（门禁 `check_test_count_monotonic.sh` 是**单调**判定 ⇒ 上升为 PASS，基线由门禁自己更新）。

---

## 5. 回归矩阵

| 命令 | 结果 |
|---|---|
| `cargo fmt --all --check` | ✅ 干净（全树） |
| `cargo clippy -p sqlrustgo-storage -p sqlrustgo-mysql-server -p sqlrustgo --all-targets` | ✅ **我改动的文件零告警**（仓库既有告警均在我改动区之外，例如 `crates/storage/src/engine.rs:4779+`、`crates/storage/examples/bench_binary.rs`） |
| `cargo test -p sqlrustgo-storage --test file_storage_ddl_column_case_5509` | ✅ **11 passed / 0 failed** |
| `cargo test -p sqlrustgo-mysql-server --lib idle` | ✅ **6 passed / 0 failed**（含 4 个新单测 + 2 个既有） |
| `cargo test -p sqlrustgo-mysql-server --test idle_reaper_heartbeat_5514 -- --test-threads=1` | ✅ **3 passed / 0 failed**（41.93 s） |
| `cargo test -p sqlrustgo-storage --no-fail-fast` | ⚠️ **1559 passed / 2 failed / 18 ignored** —— 两处失败的归属见下 |

### 5.1 两处失败的归属判定（**都不是本批引入**）

| 失败用例 | 判定 | 依据 |
|---|---|---|
| `crash_wal_index_b1::index_update_crash_does_not_serve_dangling_rows` | **基线既存红**（代码红，与本批无关） | **A/B 实测**：把本批 4 个源文件**还原到基线**后重跑，**逐字相同**地失败 —— 同一断言行 `crash_wal_index_b1.rs:180:25`、同一错误 `ERROR 1146 (42000) at line 1: Table not found: t`、同一耗时 0.42 s。批次 12 也做过同族对照（`ROLLBACK_PATH_BATCH12_2026-10-22.md` §9，BASE/FIXED 逐字相同）。症状落在 `CREATE INDEX` / `USE d1` 的库作用域上，与列名大小写无关 |
| `write_path_scaling_5181::report_single_row_delete_cost` | **环境阻塞（机器负载）**，非代码红 | 该用例是**吞吐断言**（`20000 rows` 的单行删改耗时上限）。负载下实测 `0.00555109s` ⇒ 红；**空闲机器上基线树 `0.00047576s`、改动树 `0.00056943s`，两边都绿**。本批不触碰写路径 |

⇒ 分类严格分开：**没有一条红是本批引入的**，也没有把环境阻塞当成 PASS。

---

## 6. 没做的事 / 需要属主裁决

| 事项 | 处置 | 理由 |
|---|---|---|
| #5509 第 2 点：`create_table_at` 小写化列名（让存储态不再取决于来历） | **未做** | ①issue 正文明确允许「只做第 1 点（匹配层不敏感、存储层保持原样）」；②它与 `MemoryStorage::add_column` 的**明确设计**（V313-followup-1 / #4154：「列名原样保留，以便大小写精确的 ALTER COLUMN 能消歧」）冲突 —— 若建表小写化而 `ADD COLUMN` 不小写，`CREATE (B)` + `ADD "B"` 会得到 `b` 与 `B` **两列**，`SELECT b` 与 `SELECT B` 会解析到**不同的列**，引入一个今天不存在的歧义；③它只影响**展示层**（`SHOW COLUMNS` / `DESCRIBE`），不影响行为（匹配已不敏感）。要一并做，需同时决定 `add_column` 的语义 ⇒ 属主裁决 |
| #5509 提到的「已落盘表的一次性迁移」 | **未做** | 迁移会**重写用户数据**；匹配层不敏感后行为已一致，迁移只影响展示。属主裁决 |
| #5514 第 3 点：重新论证默认阈值 30 s 是否合适 | **默认值未改** | 已补齐「配置项 + 文档」（AC4 的最低要求），并提供 `0` 开关。改默认值是语义决策，属主裁决 |
| #5514 第 2 点：引入心跳保活 | **机制上已可用**（任何命令都刷新计时，`COM_PING` 亦然），已由 e2e 钉住 | 服务端侧的「心跳超时 ⇒ 判死」语义与 #5582 的裁决**耦合**（§3.3），一起做才不会引入 TLS 误杀 ⇒ 归入 #5582 |
| #5582：让回收器真正生效 | **未做**（§3.3） | 会把 #5514 的场景 B 误杀引入 TLS 客户端 |
| GA 阶段跃迁审批 | 未做 | 三次跃迁需人类审批 |

---

## 7. 证据与可复现命令

```bash
# 工作树
cd /Volumes/workspace/dev/sqlrustgo-worktrees/wt-v410-b40

# #5509
cargo test -p sqlrustgo-storage --test file_storage_ddl_column_case_5509

# #5514
cargo test -p sqlrustgo-mysql-server --lib idle
cargo test -p sqlrustgo-mysql-server --test idle_reaper_heartbeat_5514 -- --test-threads=1

# 全量回归
cargo test -p sqlrustgo-storage --no-fail-fast     # 1559 passed / 2 failed（均见 §5.1）

# 格式 / 静态检查
cargo fmt --all --check
cargo clippy -p sqlrustgo-storage -p sqlrustgo-mysql-server -p sqlrustgo --all-targets
```

**变异脚本**（§1.3 / §2.4）逐条记录了「施加变异 → 观察变红 → 还原 → 复跑变绿」的完整过程；
变异均为**在本地副本上施加**，未进入提交。
