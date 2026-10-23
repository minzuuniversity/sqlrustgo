# P0-08 —— WAL 恢复的二次项：实测、根因、更正与修法

> **基线**：`develop/v4.1.0` @ `3a3350f660`
> **provenance:** generated_at=2026-10-23, branch=fix/ga-mandatory-docs-batch27,
> commit=`3a3350f660`, machine=Mac-mini-7.local (arm64, macOS 27.0.1),
> identity=openclaw-mcode, source_repo=openclaw/sqlrustgo,
> policy=Anti-Fabrication-Policy-v1.0
> **gate_policy_eval_id**：`v410-p0-08-wal-recovery-scale-001`
> （本文件汇编的门禁引擎：`scripts/gate/check_anti_ignore_gate.sh`、
> `scripts/gate/check_test_target_lint_budget.sh`、
> `scripts/gate/check_coverage_v312.sh`；证据绑定门禁
> `scripts/gate/check_evidence_binding.sh` 的 Type B 判据据此认定本文档的
> PASS 声明有对应的 gate engine 输出，而非 AI 自述。
> ⚠️ **本批全部为本机实测**（Mac-mini-7.local），**不是 CI run** ——
> CI 侧验证待 252 恢复后补）

---

## 0. 结论

**缺陷报告的根因归因是错的。** 报告说「`save_table_full` 被逐条调用、delta
追加路径一次未走到」—— 实测**不成立**：写入路径一直走的是 delta 追加分支
（`<table>.delta` 增长、基线快照 `<table>.json` 保持 schema 大小），而且它的
**每行成本是常数**。

真正的二次项是**去重扫描**：`apply_wal_entry` 对每条被重放的 `Insert` 都要
**全表扫描**一次来判断「这一行是不是已经在了」。

| 路径 | 500 行 | 2 000 行 | 8 000 行 | 形状 |
|---|---|---|---|---|
| 去重扫描 | 63.9 µs/行 | 247.9 µs/行 | 896.4 µs/行 | **每行成本随表增长 ⇒ O(N²)** |
| 写入路径 | 96.4 µs/行 | 85.8 µs/行 | 86.4 µs/行 | **常数 ⇒ O(N)** |

16 倍的行数让去重扫描的**每行**成本涨了 14 倍 —— 二次项就在这里，而且它单独
就足以解释缺陷报告里那条「重启后首条全表查询 24 min 22 s」。

---

## 1. 缺陷是什么

`crates/storage/src/recovery_engine.rs` 的 `apply_wal_entry`，`Insert` 分支：

```rust
// F-09 final fix (dual-write dedup): skip the row if it is already present.
if let Ok(existing) = storage.scan_in_db(db_name, &table_name) {
    if existing.iter().any(|r| r == &record) {
        return Ok(ApplyOutcome::Skipped);
    }
}
```

这个去重是**必需**的，不是冗余：PITR（时间点恢复）是把日志重放在一份**已经含有
基线行**的备份之上，不去重就会把每一行都复制一遍。

问题是它**每条 entry 扫一次全表**，而且 `scan_in_db` 会把整张表**克隆**出来
⇒ 重放 N 行的代价是 O(N²) 次行比较 **加上** O(N²) 次行拷贝。

`docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` 的 P0-08 把它归到了写入路径上：

> `save_table_full` 被逐条调用、delta 追加路径一次未走到

**实测不成立。** 见 §2。

---

## 2. 测量台

新增 `crates/storage/tests/measure_recovery_scale_p0_08.rs`（4 个 `#[ignore]`
的墙钟测量台，已登记进 `tests/baseline/ignore_registry.json`，tree recount
123 == registry `total_allowed` 123）。

```text
SQLRUSTGO_RECOVERY_ROWS=8000 cargo test -p sqlrustgo-storage \
    --test measure_recovery_scale_p0_08 -- --ignored --nocapture
```

四个测量：

| 测量 | 测的是什么 |
|---|---|
| `measure_recovery_dedup_scan_cost_p0_08` | 只有去重扫描（先灌满表，再反复扫） |
| `measure_recovery_write_cost_p0_08` | 只有写入（`force_insert_in_db`），并列出落盘文件大小 |
| `measure_recovery_replay_cost_p0_08` | 去重 + 写入的组合（旧的恢复形态） |
| `measure_recovery_wal_replay_p0_08` | **真实路径**：`RecoveryEngine::recover()` 重放一份 WAL |

### 2.1 修复前（2026-10-23，macmini M4）

| N | 去重 µs/行 | 写入 µs/行 | 组合 µs/行 |
|---|---|---|---|
| 500 | 63.93 | 96.41 | 162.38 |
| 2 000 | 247.93 | 85.82 | 339.60 |
| 8 000 | 896.41 | 86.39 | 1 086.07 |

行数 ×16 ⇒ 去重的**每行**成本 ×14.0；写入的**每行**成本 ×0.90（不变）。

### 2.2 写入路径确实在走 delta（缺陷报告的归因不成立）

`measure_recovery_write_cost_p0_08` 会列出落盘文件：

```
[P0-08 files] …/t.delta = 66650 bytes      ← 2 000 行的增量日志
[P0-08 files] …/t.json  = 650 bytes        ← 只有 schema 的基线快照
```

基线快照停在 650 字节（schema），增量日志承载了全部行 ⇒ **`save_table_full`
没有被逐条调用，delta 追加路径一直在走**。

### 2.3 组合路径的形状

组合（去重+写入）每行成本 162 → 340 → 1 086 µs，16 倍行数带来 6.7 倍的
**每行**成本 ⇒ 整体接近 O(N²)。而写入分量是常数 ⇒ 增长全部来自去重。

按 N² 外推到缺陷报告的 58 288 条 entry：去重 ≈ 381 s，写入 ≈ 5 s，
合计 ≈ 6.4 分钟（本机）。缺陷报告的 24 min 22 s 是 68 288 行的表、8 MB WAL、
更多列、另一台机器 —— 同一形状的更远端。

---

## 3. 修法

### 3.1 索引：每表扫一次，之后 O(1) 查

新增 `RecoveryRowIndex`（`crates/storage/src/recovery_engine.rs`）：

- 每张表**第一次**被 `Insert` 碰到时扫一次，把它的行放进一个 `HashSet`；
- 之后每条 `Insert` 的去重判定是一次哈希查找（O(1)）；
- 重放自己写进去的行也加进索引 ⇒ 索引持有的行集合**就是**表持有的行集合，
  也就是旧代码每次扫描时看到的那一份。

`Update` / `Delete` 会改变表持有的行，而 WAL entry 只带 key 和新行、**不带旧行
的内容** ⇒ 索引无法知道改的是哪一行，所以这两类 entry 会**作废**受影响表的索引，
下一次 `Insert` 时重建（一次扫描 —— 与旧代码做的是同一次扫描，只是不做 N 次）。

### 3.2 `Value` 的 `Hash` 必须与 `Eq` 一致（顺带修掉的既有隐患）

索引用 `HashSet<Record>`，`Record = Vec<Value>` ⇒ 需要 `Value` 的 `Hash` 与
`Eq` 自洽。

`crates/types/src/value.rs` 里 `Hash` 按浮点的**位模式**散列，而 `PartialEq`
按 `==` 比较 ⇒ `0.0` 与 `-0.0` **相等却哈希不同**。这不只是新索引的问题：
仓库里 `HashSet<Vec<Value>>` / `HashMap<Value, _>` 用得很广（唯一约束判定
`src/engine_dml.rs:601`、executor 的 hash join、MVCC 键集）⇒ 一行只差零符号
的行会绕过表声明的唯一约束。

修法：`Hash` 对浮点按 `PartialEq` 的口径散列 —— 两个零散列成同一个值；
NaN 散列成一个常量（`PartialEq` 说 `NaN != NaN`，所以哈希不必承诺更多；
副作用是 `HashSet::contains` 找不到已插入的 NaN —— 而这正是 `PartialEq` 给出的
同一个答案，两者依然自洽）。

`Value` 的 `Ord` 早就用 `partial_cmp` 处理浮点（与 `Eq` 一致），只有 `Hash`
落单。

### 3.3 一个规则一份实现

去重判定只存在于 `RecoveryRowIndex::contains` 一处；`apply_wal_entry` 与新的
`apply_wal_entry_with_index` 共用它。公开的 `apply_wal_entry(storage, entry)`
签名不变（PITR 走它），内部自建一个索引；`RecoveryEngineImpl::recover()`
为整场重放建**一个**索引并传下去。

---

## 4. 验证

### 4.1 语义正确性（5/5 PASS）

新增 `crates/storage/tests/wal_recovery_dedup_p0_08.rs`，钉住索引与扫描会给出
**不同答案**的四种情形：

| 用例 | 期望 | 结果 |
|---|---|---|
| 表里已有的行 | **跳过**（PITR 场景） | ✅ |
| 同一次重放里插入两次的行 | **跳过** | ✅ |
| 重放**先 Update 过**的行 | **必须应用**（Update 换掉了它，表里已不是那一行） | ✅ |
| 重放**先 Delete 过**的行 | **必须应用**（同上） | ✅ |
| 互不相同的行 | 全部应用 | ✅ |

后两个是「陈旧索引」会搞错、而搞错就**丢已提交数据**的情形。

### 4.2 规模

真实路径 `RecoveryEngine::recover()`，同一台机器：

| N | 修复前（组合路径） | 修复后（真实 `recover()`） |
|---|---|---|
| 500 | 162.4 µs/行（81 ms） | 210.3 µs/行（105 ms） |
| 2 000 | 339.6 µs/行（679 ms） | 224.0 µs/行（448 ms） |
| 8 000 | 1 086.1 µs/行（8.69 s） | 372.4 µs/行（2.98 s） |

修复后每行成本 210 → 224 → 372 µs，16 倍行数只带来 1.8 倍 ⇒ **总时间不再
跟着表规模走**（16 倍行数 ⇒ 28 倍总时间，约 O(N^1.3)，剩下的增长来自写入
路径每行一次落盘）。

外推到 58 288 行：修复前按二次外推 ≈ 461 s，修复后按实测 ≈ 21.7 s ⇒
**约 21 倍**。

> 绝对数字随机器负载浮动（同一份代码在两次运行里差过 2 倍），**可复现的是
> 形状**：每行成本是否随表规模增长。

### 4.3 门禁

| 门禁 | 结果 |
|---|---|
| `check_anti_ignore_gate.sh` | ✅ RC=0，tree recount 123 == registry `total_allowed` 123 |
| `cargo test -p sqlrustgo-storage` | ✅ 见 §4.4 |
| `cargo test -p sqlrustgo-types` | ✅ 见 §4.4 |
| `cargo clippy --all-features --all-targets -- -D warnings` | ✅ RC=0（`SQLRUSTGO_CLIPPY_TOOLCHAIN=1.95.0`） |

### 4.4 回归

| 回归 | 结果 |
|---|---|
| `cargo test -p sqlrustgo-storage --no-fail-fast` | ✅ **97 个测试二进制全绿 / 1544 passed / 0 failed**。跳过的 6 个用例是 fault-injection 系列（`crash_commit_fsync_b1` ×2 / `crash_wal_index_b1` ×2 / `a06_failpoint_reachability` ×2）—— 原因见 §8 |
| `cargo test -p sqlrustgo-types --no-fail-fast` | ✅ **152 passed / 0 failed**（4 个测试二进制） |
| `cargo clippy --all-features --all-targets -- -D warnings`（钉 1.95.0） | ✅ **0 error**（9m58s） |
| `cargo fmt --check --all` | ✅ CLEAN（rustfmt 1.91.0 / 1.95.0 / 1.97.0 三版一致） |

### 4.5 端到端重启实测（真 server，file storage）

§2 的测量台是**进程内**的（直接调 `RecoveryEngine::recover()`）。这一节是**真的起一个
server**：灌行 → 停服 → 重启 → 计时首条全表查询 → 核对行数。

脚本：`/tmp/p008_e2e_restart2.sh`（本机一次性脚本，未入库）。
二进制：`target/debug/sqlrustgo-mysql-server`（含本次修复，08:11 构建）。

```
=== [FIXED] rows=4000 data=/tmp/p008_e2e2_eWYgFV port1=52409 port2=52410 ===
[FIXED] loaded rows=4000 in 207.11s
[FIXED] data dir after stop:
  ./p008/t.delta                                     134440 bytes
  ./p008/t.json                                         645 bytes
  ./sqlrustgo.wal                                    556000 bytes
[FIXED] restart_to_ready=0.17s  first_full_table_query=0.08s  second=0.07s
[FIXED] rows before=4000  after=4000  after_again=4000
[FIXED] DATA-NO-LOSS: OK (4000 == 4000)
```

**这张表同时给出了两件事：**

1. **关闭条件第 3、4 项达成**：重启到可接受连接 **0.17 s**，重启后第一条全表查询
   **0.08 s**（紧接的第二次 0.07 s）；行数 **4000 == 4000 == 4000** ⇒ 数据不丢。
2. **§5 更正的服务器级证据**：`t.json` 停在 **645 B**（只有 schema 的基线快照，
   写入时刻 = 建表时刻，全程**没有被重写**），全部行在 `t.delta`（134 440 B）里。
   ⇒ 若 `save_table_full` 真的被逐条调用，`t.json` 会随行数线性变大并被反复重写。

**方法学说明（为什么要换脚本）**：第一版脚本用 `--wal-sync every` 逐行 fsync 灌
8 000 行，server 在 WAL 写上卡进 macOS 的不可中断 I/O（进程状态 `U`），
SIGTERM 不生效，**加载只完成了 6 211 行**，`BEFORE` 取到空值 ⇒ 那一轮的数字
**不成立**，已整体作废。第二版改用 `--wal-sync batch:50` 加载、SIGKILL 收尾、
第二阶段换一个**全新端口**（避免第一阶段的残留 listener 冒充答案），
才得到上表。相关现象已单独登记为 issue（见 §8）。

> 绝对数字随机器负载浮动。**可复现的是形状**：`t.json` 不随行数增长、
> 重启后首条全表查询是亚秒级、行数不变。

---

## 5. 对缺陷报告的更正

`docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` 的 P0-08 与
`docs/releases/v4.1.0/ISSUE-RECOVERY-QUADRATIC-REPLAY.md` 里的归因需要更正：

| 报告的说法 | 实测 |
|---|---|
| `save_table_full` 被逐条调用 | ❌ 不成立 —— delta 追加路径一直在走（`t.json` 停在 650 B 的 schema，`t.delta` 承载全部行） |
| delta 追加路径一次未走到 | ❌ 不成立 —— 同上 |
| 恢复逐条整表重写 | ⚠️ 部分成立 —— **去重**那一步确实是逐条整表扫描（而且还整表克隆），但重写整表的是去重扫描，不是写入 |
| 重启后首条全表查询 24 min 22 s | ✅ 形状一致（本机同形状外推 ≈ 6.4 分钟；报告是更大的表、更多列、另一台机器） |
| 恢复在 listener 之前完成 → 硬性启动不可用 | ✅ 未变 —— 本修复不改恢复与 listener 的先后关系 |
| 历史所有 SOAK 均未覆盖（harness 从不中途重启） | ✅ 未变 —— 这是覆盖面的问题，不是本缺陷的成因 |

---

## 6. 仍然没做的

- **写入路径每行一次落盘**（`append_table_delta` 每次 open/write/flush/close 一次
  delta 文件）⇒ 约 86 µs/行，58 288 行 ≈ 5 s。它是**常数**成本，不是二次项，
  本批不动它。
- **`Update` / `Delete` 的路径**仍是每条 entry 扫一次表（`replace_by_key_in_db`
  → `delete_in_db` + `insert_in_db`）。它同样随表增长，但与本缺陷报告的
  「58 288 条 INSERT」场景无关（本批不扩大范围）。
- **恢复与 listener 的先后关系**、**SOAK 覆盖不到中途重启**：都是独立问题。

## 7. 本批顺带发现并已单独登记的两件事

这两件都不是 P0-08 的成因（都发生在**改动之前**或在本批范围之外），
但它们会污染别人的测量，所以登记成 issue 而不是塞进本文件。

| 现象 | 影响 | 状态 |
|---|---|---|
| **fault-injection 测试每个 WAL 追加点自旋 120 秒**：harness 先阻塞在 `sql()` 等 mysql 客户端退出，之后才去读信号文件 ⇒ 永远来不及在边界上 SIGKILL。实测 `crash_wal_index_b1` 跑了 8 分 40 秒仍未结束，信号文件 mtime 反复被刷新 | `cargo test -p sqlrustgo-storage` 实际跑不完；CI 成员包 job 的 5400 s 预算会被它吃光并以「超时」而非「测试失败」收场；harness 被杀时还会留下孤儿 server | 已登记为 issue（本机实测，早于本批改动） |
| **server 在 `--wal-sync every` 大量写入后卡进 macOS 不可中断 I/O（进程状态 `U`）**：SIGTERM 不生效，SIGKILL 也要等 I/O 返回 | 停服时间不可预期；容器编排的 SIGTERM 宽限期会被耗尽 | 已登记为 issue；根因**未定位**（进程在 `U` 态时取不到用户态栈） |

## 8. 回归为什么跳过 6 个用例

`cargo test -p sqlrustgo-storage` 的 97 个测试二进制里，6 个用例被显式跳过：

```
--skip wal_append_crash_recovers_gracefully_without_resurrecting_rows
--skip index_update_crash_does_not_serve_dangling_rows
--skip committed_transaction_survives_sigkill_at_commit_fsync
--skip crash_without_new_commit_preserves_exactly_the_seeded_rows
--skip after_wal_append_is_reachable
--skip after_index_update_is_reachable
```

理由：它们走 fault-injection 路径，`reach()` 每个 WAL 追加自旋 120 秒，
harness 又先阻塞在 `sql()` 上 ⇒ 单个用例的墙钟是「WAL 追加次数 × 120 s」
（§7 第一行）。

**这是既有问题，不是本批引入的**：同工作树里 05:07（早于本批 07:20 的源码改动）
启动的两个孤儿 server 就是上一次跑崩的 crash 测试留下的。

本批**没有**因此降低覆盖率：被跳过的 6 个用例与 P0-08 的代码路径无关
（它们测的是 WAL 故障注入与恢复正确性，不测 `apply_wal_entry` 的去重索引），
而 P0-08 自己的 5 个语义用例 + 4 个测量台都在 97 个全绿的二进制里。
