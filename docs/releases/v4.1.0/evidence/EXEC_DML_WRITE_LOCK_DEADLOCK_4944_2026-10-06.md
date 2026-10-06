# #4944 收尾之一：写锁内取表扫描导致的自死锁（AUTOINCREMENT / REPLACE INTO）

- **日期**：2026-10-06
- **Issue**：#4944（G11 legacy 回归矩阵 / WP-C）
- **分支**：`fix/4944-autoincrement-executor-hang`
- **基线**：`develop/v4.1.0` @ `650420db37`

---

## 1. 起点：核实 G11 的真实状态

#4944 正文称 WP-D / WP-F / WP-G 三项 deferred、尚无覆盖。实跑核对后，这个说法**已过时**：

| 矩阵文件 | `#[test]` | `#[ignore]` | 提交 |
|---|---|---|---|
| `crates/parser/tests/wp_a_legacy.rs` | 28 | 1 | — |
| `crates/types/tests/wp_b_legacy.rs` | 36 | 0 | — |
| `crates/executor/tests/wp_c_legacy.rs` | 14 | **11** | `73cca84b4f` |
| `crates/executor/tests/wp_d_legacy.rs` | 13 | 0 | `03878ebd0d` |
| `crates/transaction/tests/wp_e_legacy.rs` | 15 | 0 | — |
| `crates/storage/tests/wp_f_legacy.rs` | 8 | 0 | `e8ed305fc4` |
| `crates/executor/tests/wp_g_legacy.rs` | 12 | **9** | `ae86f33ddf` |

WP-A…WP-G 的「占位断言换成真断言」都已完成。**但 WP-C 14 个里有 11 个、WP-G 12 个里有 9 个被
`#[ignore]` 掉，根本没有在跑** —— 而这些 `#[ignore]` 记录的恰恰是真实未修的缺陷。

本 PR 处理其中最严重的一个：**执行器挂死**。

---

## 2. 缺陷：持写锁时请求表扫描 → 自死锁

`ExecutionEngine::storage_read()` 取的是**读**锁。它先试 `try_read()`，失败则退化为阻塞的
`read()`。当调用线程**已经持有写锁**时，`try_read()` 必然失败，而 `parking_lot::RwLock`
不可重入 —— 于是语句永久阻塞在自己身上。

代码里其实已经写明了这一点（`src/execution_engine_methods.rs`）：

> `parking_lot::RwLock` is not reentrant, and `storage_read()` falls back to a blocking
> `read()` when `try_read()` fails — which it always does when the calling thread already
> holds the lock. ... calling `scan_for_reader` from inside such a scope deadlocks.

对应的安全变体 `scan_for_reader_with(&storage, ..)` 就是为此存在的，而且**同一个函数里相邻
的重复键检查扫描已经在用它**。踩坑的两处只是用错了变体：

| 位置 | 语句 | 症状 |
|---|---|---|
| `src/engine_dml.rs` AUTOINCREMENT 的 `MAX(id)` 扫描 | `INSERT` 进带 `AUTOINCREMENT` 列的表 | 执行器永不返回 |
| `src/engine_dml.rs` REPLACE 的冲突扫描 | `REPLACE INTO ...` | 执行器永不返回 |

## 3. 实测证据（非推断）

**AUTOINCREMENT** —— 把 `wp_c_legacy` 里三个被 ignore 的相关测试放出来跑：

```
test issue_4682_system_tables::sqlite_sequence_after_autoincrement ... running for over 60 seconds
test issue_4672_autoincrement::autoincrement_allocates_sequential_ids ... running for over 60 seconds
test issue_4672_autoincrement::autoincrement_continues_after_delete ... running for over 60 seconds
```

**REPLACE INTO** —— `tests/integration/dml/replace_test.rs`（起真实 CLI 子进程）：

```
基线：  test_replace_into_affects_rows        ... has been running for over 60 seconds
        test_replace_into_existing_row        ... has been running for over 60 seconds
        test_replace_into_new_row             ... has been running for over 60 seconds
        test_replace_into_with_autoincrement  ... has been running for over 60 seconds
修复后：test result: ok. 4 passed; 0 failed   (21.89s)
```

### 一个必须说明的观察

`replace_test.rs` 在**没有** `SQLRUSTGO_REPO_ROOT` 环境变量时是 0.00s「快速失败」——
因为 `run_sql` 直接返回 `Err`，而测试用 `unwrap_or_default()` 把错误吞成了空串，
断言 于是报「should have banana, not apple: 」（值为空）。设上该变量才会真正跑 SQL 并挂死。

也就是说这批测试有**两种失败形态**（快速失败 / 挂死），取决于环境变量。不设变量时得到的
「4 个 FAILED」与 REPLACE 的正确性无关，纯属环境缺失 —— 不能据此判断 REPLACE 是否工作。

## 4. 修法

两处都改用持锁变体 `scan_for_reader_with(&storage, ..)`，即函数内相邻代码已经在用的那个。

全仓 `scan_for_reader(` 剩余调用点逐一核对，确认没有第三处：

| 位置 | 是否持写锁 | 结论 |
|---|---|---|
| `engine_dml.rs` REPLACE | 是 | **死锁，已修** |
| `engine_dml.rs` AUTOINCREMENT | 是 | **死锁，已修** |
| `engine_dml.rs` ~1335（DELETE undo 快照） | 否 —— 上一个 `storage.write()` 在独立 `{}` 块内，块已结束 | 安全 |
| `engine_dml.rs` ~1473（多行 DELETE） | 否 —— 写锁在 `if` 分支，扫描在 `else` 分支，互斥 | 安全 |

## 5. 测试

新增 `crates/executor/tests/engine_dml_write_lock_scan_deadlock_4944.rs`（5 项，全绿，0.01s）。

同时把 `wp_c_legacy` 里两个 AUTOINCREMENT 测试的 `#[ignore]` 摘掉 —— 它们此前是被当成
「已知 GAP」标记的，现在是真通过的测试。wp_c_legacy 因此从 **4 通过 / 10 忽略** 变为
**6 通过 / 8 忽略**。

原有的诊断注释是错的，也一并更正：

> ~~HANG: AUTOINCREMENT causes the executor to hang. PR #4927 added per-table AtomicU64
> counters for AUTO_INCREMENT; AUTOINCREMENT (SQLite) appears to take a different code path
> that loops.~~

实际不是「走了不同代码路径导致循环」，而是「全表扫描取 `MAX(id)`」这一步在写锁内取了读锁。

## 6. 变异验证

**M12** —— 把这两处退回 `scan_for_reader`（即重新引入死锁）：

```
test replace_into_existing_row_replaces_it has been running for over 60 seconds
```

**CAUGHT**。

注意这个变异的失败形态与其他变异不同：它**挂死**而不是报错。所以证据里没有 `test result:
FAILED` 这一行，只有超时与 `has been running for over 60 seconds` 提示。这本身就是一个治理
问题，见下。

## 7. 顺带暴露的门禁盲区

`.gitea/workflows/ci.yml` 的 `Test summary` 步骤靠 grep `test result: FAILED` 计数后
`exit 1`。**挂死的测试永远产不出 `FAILED` 行** —— `cargo test` 只会一直等，直到整个 job 超时。

也就是说：一个无限挂死的测试可以让 CI 长时间红着，却不会以门禁能识别的方式报出来；门禁
看到的只是「job 超时」，而不是「哪条语句死了锁」。本 issue 之前把 AUTOINCREMENT 挂死标成
`#[ignore]` GAP 而不是修掉，正是这条盲区的产物。

（上一条 #5059 评论里我核实过该门禁「不是 fail-open」—— 那个结论针对**失败**用例成立。
**挂死**是另一条路径，不在那个 grep 的覆盖范围内。两条评论合起来才完整。）

## 8. 未纳入本 PR

- **WP-G 的 9 个 `#[ignore]`** —— CHAR 比较相关的真实缺陷（#4846 headline 等），语义面较广，
  应单独处理。
- **WP-C 剩余 9 个 `#[ignore]`** —— 主要是 executor 静默接受 `CREATE PROCEDURE` /
  `CREATE FUNCTION`。
