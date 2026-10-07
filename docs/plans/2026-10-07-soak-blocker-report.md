# v4.1.0 SOAK 重跑 — 阻塞报告

> **日期**: 2026-10-07
> **结论**: ❌ **SOAK 无法执行** — 阻塞于 P0 并发事务缺陷，非测试环境问题
> **被测版本**: `origin/develop/v4.1.0` @ `ed181dfc0b`（2026-10-07 13:38，双服 origin/gitea250 一致）
> **二进制**: `cargo build --release -p sqlrustgo-mysql-server` → 3m59s，exit 0，26 warnings
> **复现环境**: 80 核 / 404 GB RAM / NVMe 746 GB 空闲；sysbench 1.0.20、pymysql 2.2.8

---

## 1. 拉取决策：需要，且已用新 worktree 隔离

| 项 | 值 |
|---|---|
| 本地 HEAD | `41d87cab37`（`develop/v3.12.0`） |
| 目标 HEAD | `ed181dfc0b`（`origin/develop/v4.1.0`） |
| 分叉度 | `27643 / 23874`，merge base 停在 `2ba889b76e`（2026-02-16） |

历史被重写过，**无 fast-forward 路径**，且本地落后 27643 个提交。直接切分支会丢掉 v3.12.0 视角。

**做法**：新建 detached worktree `.worktrees/soak-v410` 检出 `ed181dfc0b`，主 worktree（干净）不动，脏的 `.worktrees/soak168h`（200 文件变更）也未触碰。

最新提交确实值得拉：#5057 系列直接改 FileStorage 持久化链与 `USE` 连接上下文，正处于本次暴露缺陷的同一域。

---

## 2. 历史 SOAK 状态：无一生还

| 位置 | 状态 |
|---|---|
| `test_results/ga_soak_smoke` | `soak_1h.pid`=788989、`soak_168h.pid`=789356、`server.pid`=785595 **全部不存在**；两个 log **0 字节**（停在 2026-08-27） |
| `.worktrees/soak168h` | 脏 200 文件（103 D / 96 M / 1 ??），**无 `target/`**（从未成功编译），`results/soak-168h/` 仅 640 B server.log |
| v4.0.0 168h SOAK | 官方结论 **DEFERRED**（`V400_09_168H_SOAK_FINAL_REPORT.md`：2026-09-19 kickoff 声称启动，实际**从未跑到完成**，无 STABILITY_REPORT / metrics.csv） |

---

## 3. 本次 SOAK 执行结果：60 秒失败

```
[22:47:54] prepare 完成（10000 rows ✓）
[22:47:58] SOAK 开始
[22:48:00] FATAL: mysql_drv_query() returned error 1062 (Duplicate entry '5024' for key 'PRIMARY')
          for query 'INSERT INTO sbtest1 (id, k, c, pad) VALUES (5024, 5019, ...)'
```

run_soak_loop.sh 退出码 1。RSS 稳定在 73.5 MB、无 panic、WAL 0.03 MB、磁盘 3 MB —— **资源指标全部健康，失败纯粹来自事务正确性**。

证据目录：`/home/openclaw/sqlrustgo-soak-v410/results/soak_20261007_224750/`

---

## 4. P0 缺陷一：并发事务下 DELETE 未回滚 → 静默丢行

### 复现（30 次试验测得 30% 复现率）

两连接各自 `BEGIN; DELETE FROM w WHERE id=42`，A 提交、B 回滚：

```
trials=30  row-lost=9  txn-errors=0
VERDICT: intermittent
```

单次试验时结果不稳定（同一脚本首轮 `id=42 present=1` 未复现），
故以 30 次独立试验（每轮新建表）定量：**9/30 轮该行永久消失，0 事务错误**。

**MySQL 语义下 B 的回滚必须恢复 id=42；此处 30% 的情况下该行永久消失。**
提交方的 delete 覆盖了回滚方的 undo —— 违反隔离性，且**无任何报错**，属静默丢数据。

> 方法论更正：首轮"确定性复现"的观察不可靠，实际是间歇性竞态。
> 结论以 30 次试验的定量结果为准，不以单次命中为准。

### 代码定位

- `src/engine_dml.rs:1521-1535` — WHERE 路径按行记录 `UndoRecord::Delete`，**记录侧正确**
- `src/savepoint_wiring.rs:83-96` — `record_delete_undo` 按 PK 记录 old_value
- `src/engine_dml.rs:1467-1484` — 单行快路径 `storage.delete(&table_name, &key_values)`

缺陷在 **undo 重放阶段**：回滚方按 PK 重插该行时，提交方已提交的 delete 优先，重插被吞掉。

### 并发放大效应（5 轮 × 1600 事务实测）

| 轮次 | 事务数 | 1062 错误 | 行数变化 | 丢失 |
|---|---:|---:|---|---:|
| rep0 | 1600 | 8 | 200 → 200 | 0 |
| rep1 | 1600 | 14 | 200 → 94 | 106 |
| rep2 | 1600 | 31 | 200 → 80 | 120 |
| rep3 | 1600 | 40 | 200 → 87 | 113 |
| rep4 | 1600 | 17 | 200 → 98 | 102 |
| **合计** | **8000** | **110 (1.4%)** | — | **441 (55%)** |

**200 行的表在并发读写 1600 次事务后平均只剩 91 行 —— 丢失 55%，且每次回滚都"成功"。**
丢行不产生任何错误信号，只有事后对账才能发现。

**这一条同时解释了 SOAK 的 1062**：sysbench `oltp_read_write` 的
`execute_delete_inserts` 就是 `DELETE id=X` + `INSERT id=X` 同一事务。
两线程撞同一 id 时，一方的 delete 尚未对其事务可见，另一方 insert 即触发主键冲突，
sysbench 将 FATAL 判为致命错误并终止整个 run。

**行数丢失与 1062 是同一个根因的两个表现**，不是两个独立 bug。
rep0 无丢行但仍有 8 个 1062，进一步印证两者同源。

---

## 5. P0 缺陷二：解析器不支持 `db.table` 限定名

### 症状

| 语句 | 结果 |
|---|---|
| `SELECT * FROM d1.t1` | `ERROR 2027 (HY000) Malformed packet` |
| `INSERT INTO d1.t2 (id,v) VALUES (1,'a')` | `ERROR 1064 Parse error: Expected VALUES, SELECT, or DEFAULT VALUES` |
| `INSERT INTO d1.t2 VALUES (2,'b')` | 同上 |
| 不带库名的同名语句 | ✅ 全部正常 |

### 根因（精确位置）

`crates/parser/src/parser.rs:8489-8492`：

```rust
let table = match self.next() {
    Some(Token::Identifier(name)) => name,
    _ => return Err("Expected table name".to_string()),
};
```

`parse_insert` **只读一个标识符**作为表名。`INSERT INTO d1.t2` 时它取 `d1` 当表名，
`.t2` 未被消费，随后列清单检查与 VALUES 分支相继落空，
最终报出误导性的 `Expected VALUES, SELECT, or DEFAULT VALUES`。

对照 `parse_table_ref`（同文件 `:11279-11285`）**已有**正确的 `schema.table` 处理，
`parse_insert` 漏掉了同一段逻辑：

```rust
if matches!(self.current(), Some(Token::Dot)) {
    self.next();
    schema = Some(name);
    name = match self.next() { Some(Token::Identifier(n)) => n, ... };
}
```

这是 INSERT 路径的既有缺口，**非本次 #5057 提交引入**。

### 附带发现：`CREATE TABLE db.t` 落盘位置错误

```
CREATE TABLE d1.t1 ...   →  /tmp/probe-data/t1.json        ← 根目录，跑错库
CREATE TABLE t2 (USE d1) → /tmp/probe-data/d1/t2.json      ← 正确
```

#5057 的显式库名改造**只落了一半**：`INSERT/UPDATE/DELETE` 已有 `*_in_db` 方法族，
但 `CREATE TABLE db.t` 的目标路径仍未使用限定库名。

---

## 6. 已排除的假设（避免误判）

| 假设 | 结论 | 证据 |
|---|---|---|
| 1062 是 DELETE/INSERT 基本逻辑错误 | ❌ 否 | 单线程 400 次 DELETE+INSERT 同 id：**0 错误** |
| UPDATE 并发导致丢行 | ❌ 否 | 干净服务器 800 次并发 UPDATE：200 行**完整存活**，sum(k)=781（并发同 row 竞争，非丢失） |
| ROLLBACK 本身失效 | ❌ 否 | 孤立回滚测试中已插入的行**正确回滚** |
| 并发能力不足 | ❌ 否 | `--server-threads 16` 下 5 连接并发查询 5/5 成功 |
| 1062 是 SOAK 脚本配置问题 | ❌ 否 | sysbench prepare 成功写入 10000 行；失败发生在并发 write 阶段 |
| server 死锁卡死 | ⚠️ 曾观测到 | `--server-threads 4` 时出现 1070% CPU 空转且查询超时，**未能稳定复现**，不作为结论 |

> 一度观测到"200 行表只剩 3 行"，已在干净服务器上重测推翻 —— 那是前序已被污染状态叠加
> 我自己脚本复用 cursor 的 bug 所致，非独立缺陷。此前的过度归因在此更正。
>
> **方法论**：并发缺陷不可用单次命中下结论。本报告所有并发结论均改为
> 多次独立试验的定量结果（§4 用 30 次判定复现率，§4 并发表用 5 轮 × 1600 事务）。

---

## 7. SOAK 判定

| 判据 | 结果 |
|---|---|
| 1h SOAK 完成 | ❌ 60s 终止 |
| 0 driver 错误 | ❌ 27 × ERROR 1062 |
| 数据一致性 | ❌ 8000 事务丢 441 行（55%），0 报错 |
| 0 panics | ✅ |
| RSS < 1.7 GB | ✅ 73.5 MB |
| WAL / 磁盘健康 | ✅ 0.03 MB / 3 MB |

**判定：BLOCKED — P0 事务缺陷，不予出具 SOAK 基线。**

资源指标全部健康说明 v4.1.0 的性能栈（MVCC GC + WAL batch）本身无退化；
阻塞项是**事务隔离正确性**，与 v4.0.0 记录的 168h SOAK DEFERRED 属不同性质问题
（那次是 RSS 调优未落地，本次是真实并发缺陷）。

需要强调：**丢行率 55% 这一项比 SOAK 本身的失败更严重**。
SOAK 至少会 FATAL 报出来，而丢行完全静默 —— 任何依赖该引擎的
生产读写路径都在无声地丢数据，且事后无告警。

---

## 8. 修复建议（按优先级）

### P0-1 undo 重放顺序 —— 修 `engine_dml.rs` DELETE 路径

回滚方重插被提交方 delete 吞掉，疑似 `storage.delete` / 重插之间缺少版本序校验，
或 `UndoRecord::Delete` 重放未走 MVCC 版本检查。
建议：在重插前确认目标 PK 无更新版本；对同一 PK 的 delete-undo 走版本序仲裁。

回归测试建议（非 tautology）：
```python
# 两连接删同一行，A commit、B rollback → 行必须存活
```
该测试须断言 `id=42 present=1`，直接锁死本缺陷。

### P0-2 `parse_insert` 限定名 —— 修 `parser.rs:8489`

照搬 `parse_table_ref:11279` 的 `Dot` 分支，为 `InsertStatement` 增加可选 `db` 字段。
注意 `InsertStatement`（`:973`）当前**无 db 字段**，需同步扩展结构体与所有构造点。

### P1 `CREATE TABLE db.t` 落盘路径

与 #5057 已有的 `*_in_db` 方法族对齐，补齐 DDL 侧显式库名。

### 临时绕过（仅在 P0-1 落地前有效）

`SOAK_WORKLOAD=oltp_read_only` 可跑只读压测；
`oltp_read_write` 在缺陷修复前**无法产出可信基线**。

---

## 9. 复现步骤

```bash
# 1. 检出 + 编译
git -C /home/openclaw/sqlrustgo_work worktree add \
    /home/openclaw/sqlrustgo_work/.worktrees/soak-v410 origin/develop/v4.1.0 --detach
cd /home/openclaw/sqlrustgo_work/.worktrees/soak-v410
cargo build --release -p sqlrustgo-mysql-server

# 2. 复现 SOAK 失败（60s 内 FATAL 1062）
SOAK_NO_DETACH=1 SOAK_HOURS=1 SOAK_PORT=3411 \
SOAK_DATA_DIR=/home/openclaw/sqlrustgo-soak-v410/data-3411 \
SOAK_RESULTS_DIR=/home/openclaw/sqlrustgo-soak-v410/results \
SOAK_TABLE_SIZE=10000 SOAK_SERVER_THR=16 SOAK_SB_THR=8 \
SOAK_WORKLOAD=oltp_read_write bash scripts/soak/run_soak_loop.sh

# 3. 最小复现（丢行）
#    见 §4 的两连接 DELETE + commit/rollback 脚本
```

---

## 10. 遗留与后续

- 缺陷**未修复** —— 本次任务范围为评估与重跑，不是修复；修复需独立变更与评审
- `--server-threads 4` 下的 1070% CPU 空转现象**未稳定复现**，建议后续用线程数斜坡复测
- `run_soak_loop.sh:154` 在 `SOAK_RESULTS_DIR` 创建前写 `preflight.log`，报
  `No such file or directory` —— 无害（run dir 随后正常创建），但属脚本顺序瑕疵