# 迭代 5 · 批次 14 —— 空闲连接回收器不再制造「假的引擎崩溃」（#5464）

**关闭**：#5464
**基线**：`develop/v4.1.0` @ `6e943b77d51e`

---

## 0. 一句话

服务端有一个 30 秒空闲连接回收器；测试却把「引擎查询 ↔ 预言机计算」
**交错**执行 ⇒ 预言机计算期间引擎连接完全空闲 ⇒ 被回收 ⇒
**下一条**引擎语句报传输错误 ⇒
**被报错的那条语句与真正的故障毫无关系**。

本批次把两侧改成**两段式**：先把引擎结果全部取回来，再算预言机。

---

## 1. 机制

`crates/mysql-server/src/lib.rs`：

```rust
/// v3.12.0 Issue #4682: scan interval for the idle-connection
/// reaper. The reaper force-closes any connection whose last
/// activity was more than `read_idle_timeout_secs()` ago.
fn read_idle_timeout_secs() -> u64 {
    std::env::var("SQLRUSTGO_IDLE_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|n| *n >= 1)
        .unwrap_or(30)
}
```

⇒ 默认 **30 秒**。`SQLRUSTGO_IDLE_TIMEOUT_SECS=0` **不是**「禁用」
（`.filter(|n| *n >= 1)` 会把它过滤掉，然后落回默认 30）。

---

## 2. 为什么会误报

原来的两个阶段都是**交错**的：

```text
for 每张表 / 每条语句:
    引擎侧查询          ← 引擎连接被使用
    预言机计算          ← 引擎连接完全空闲（这段可能 41 秒）
    比较
```

预言机计算期间，引擎连接**一次字节都没有读过**。
只要某一条预言机计算超过 30 秒，回收器就把它关掉 ⇒
**下一条**引擎语句立刻失败：

```
read packet header: failed to fill whole buffer
```

⇒ 被报错的那条语句与真正的故障毫无关系（#5464 的原始实测：
Q21 的预言机算了 **41 秒**，于是 **Q22** 报了错；而 Q22 单独跑是好的）。

---

## 3. 修法：两段式

| | 原来 | 现在 |
|---|---|---|
| Pass A | 逐条「引擎 → 预言机 → 比较」 | **所有语句**先在引擎上跑完，结果留在内存里 |
| Pass B | —— | 再逐条问预言机，然后比较 |

⇒ 引擎连接**只在两条引擎语句之间**可能空闲，而那期间没有预言机计算 ⇒
空闲窗口消失。

**语义完全不变**：每条语句仍然被比较 / 跳过 / 失败恰好一次，
末尾的断言（`compared + not_run + failed == EXPECTED_QUERIES`、
`compared == expected_scoped`、`skipped_scope.len() == expected_skipped`）
一字未改。

### 改了哪三个文件

| 文件 | 改的是什么 |
|---|---|
| `tests/integration/tpch/tpch_sf01_22_vs_sqlite_test.rs` | Phase 1（表行数）与 Phase 2（22 条语句）都改成两段式 |
| `tests/integration/tpch/tpch_sf01_22_vs_3engines_test.rs` | 同上 |
| `tests/integration/tpch/tpch_sf01_22_vs_3engines.rs` | 同上 |

> 三个文件**都是**已声明的 `[[test]]` 目标 ⇒ 都进 CI，
> 所以三处一起改，不能只改一个。

---

## 4. 为什么**不**去改引擎侧的空闲阈值

issue 里把引擎侧列为「可选，但值得讨论」。本批次的判断是**不改**，
理由与代价都记在这里：

* **改默认值的代价**：30 秒是 #4682 为解决「客户端突然丢弃后 CLOSE_WAIT
  堆积 / `read_exact` 干等 600 秒」而特意选的。
  把它调大（例如 300 秒）会延长那类连接的滞留时间 ⇒ 那个问题可能复发。
* **MySQL 的 28800 秒不能直接照搬**：那是「连接空闲」；
  #4682 处理的是「**连接被客户端突然丢弃**」——
  后者靠对端消失来发现，靠超时来兜底，两者不是一回事。
* **正确的方向是心跳**：#4682 注释里提到「客户端突然丢弃」，
  而 #5464 的场景是「客户端在算长任务」——
  **两者可以区分**，靠 `COM_PING` 之类的心跳就能区分，
  而不是靠一个统一的空闲阈值。

⇒ 引擎侧的改动另立 issue 跟踪（见 §7），本批次**不**顺手改。

---

## 5. 判别力

| 变异 | 观察到的结果 |
|---|---|
| **把 Phase 2 改回交错**（预言机计算夹在两条引擎语句之间） | 预言机计算 > 30 秒时，**下一条**引擎语句报 `read packet header: failed to fill whole buffer`；对应 issue 记录的原始实测就是 Q21 预言机 41 秒 ⇒ **Q22 报错** |

本批次是**测试结构**的修正，没有「生产代码改动 ⇒ 变异 ⇒ 红灯」这种形态。
判别力体现在**结构本身**：

* 交错写法下，预言机计算一旦超过空闲阈值，红灯会落在**与故障无关**的语句上；
* 两段式写法下，引擎连接在整段预言机计算期间空闲，但**之后不再使用**它 ⇒
  回收器关掉它不影响任何断言。

⇒ 门禁（这个测试本身）从「会因为参考实现的耗时而误报」
变成「只对引擎的真实行为报错」。

---

## 6. 门禁

| 检查 | 结果 |
|---|---|
| `cargo test --test tpch_sf01_22_vs_sqlite_test`（带 SF=0.01 fixture） | **46 passed / 0 failed**；`compared=19 mismatched=0 known=0 not_run=3 failed=0 truncated_tables=0` |
| `cargo test --test tpch_sf01_22_vs_3engines_test` | **48 passed / 0 failed**（14.98s） |
| `cargo test --test tpch_sf01_22_vs_3engines` | **48 passed / 0 failed**（16.60s） |
| `cargo fmt --check --all` | PASS（exit 0） |
| `cargo clippy --test tpch_sf01_22_vs_sqlite_test --test tpch_sf01_22_vs_3engines_test --test tpch_sf01_22_vs_3engines -- -D warnings` | **exit 0** |
| 本批次改动面 | 3 个测试文件（**零生产代码**） |

> 本批次**不改任何生产代码** —— 这是有意的：
> 缺陷在测试结构里，引擎的空闲回收器按设计工作（#4682）。

---

## 7. 本批次**没有**做的事

1. **没有**改引擎侧的空闲阈值 / 心跳机制（见 §4，另立 issue 跟踪）。
2. **没有**动 SQLite 预言机的索引 —— 仓库里**早已有** 14 个 TPC-H 索引
   （`tests/common/sqlite_oracle.rs:125` 的 `TPC_H_INDEXES`，
   `open()` 在 :177 调用 `create_indexes()`），
   issue 里「补索引」的建议**已经实现**；
   本批次实测预言机侧每个语句都在毫秒~秒级完成。
3. **没有**动 `tpch_sf1_22_vs_3engines_test.rs`（SF=1 变体）——
   它的结构与 SF=0.01 变体不同（没有预言机交错），不在本 issue 的作用面内。

---

## 8. 引用

- `crates/mysql-server/src/lib.rs` — `read_idle_timeout_secs` / `read_reaper_interval_secs`
- `tests/integration/tpch/tpch_sf01_22_vs_sqlite_test.rs` — Phase 1 / Phase 2 两段式
- `tests/integration/tpch/tpch_sf01_22_vs_3engines_test.rs` / `tpch_sf01_22_vs_3engines.rs` — 同上
- `tests/common/sqlite_oracle.rs` — `TPC_H_INDEXES`（预言机索引，早已存在）
- `tests/common/tpch_wire_harness.rs` — `run_query_timed` / `WireOutcome`
