待裁定问题（只依据材料判定，材料不足输出 INCOMPLETE）

**待决**：SQLM-1 —— SQLite 兼容模式（`crates/sqlrustgo-cli/src/sqlite_mode.rs`）的
**嵌套事务语义**：`ROLLBACK` 到底该不该一次性出栈到最外层？

**背景（现场事实，不需要再裁）**：
- `dispatch_one` 用 `self.tx_depth: u32` 记嵌套层数：`BEGIN` 加一、`COMMIT` 减一、
  `ROLLBACK` 减一（两个分支代码完全相同 ⇒ clippy `if_same_then_else` 判红）。
- 该计数器只服务于**错误路径**：语句执行出错时，若 `tx_depth > 0` 则强制
  `ROLLBACK` 并把 `tx_depth` 清零（sqlite_mode.rs:733-737）。
- SQLite **没有嵌套事务**：`BEGIN; BEGIN;` 在 sqlite3 里直接报
  `cannot start a transaction within a transaction`；嵌套语义由 SAVEPOINT/RELEASE/
  ROLLBACK TO 提供。也就是说，本仓的 `tx_depth` 是在**模拟**一层并不存在的嵌套。
- 现有测试：`crates/sqlrustgo-cli/tests/` 下**没有任何**嵌套事务用例 ⇒ 当前行为
  没有任何测试钉住；`grep -rn tx_depth crates/sqlrustgo-cli` 只命中该文件。

**要裁的是这两件事**：
  (1) **口径**：本仓的 SQLite 兼容模式对 `BEGIN; BEGIN; ROLLBACK; COMMIT;`
      应该是什么行为？两个候选：
      (a) **整体出栈**（ROLLBACK 把 `tx_depth` 置 0，因为 SQLite 没有嵌套事务，
          `ROLLBACK` 结束的是最外层事务）——与参考实现更接近；
      (b) **逐层递减**（现状）——CLI 侧模拟嵌套计数，COMMIT 再减一层。
      判据要能落到可执行：**给出该序列在参考 sqlite3 上逐条语句的实际输出与错误文本**，
      以及本仓应当对齐到哪一边。
  (2) **要不要把「不支持嵌套 BEGIN」直接暴露**？即 `BEGIN` 时若 `tx_depth > 0`
      就按 SQLite 报错（`cannot start a transaction within a transaction`），
      从而**根本不需要** `tx_depth` 这个计数器（`tx_depth` 只剩错误路径的兜底作用）。
      若选这条，需要说明兼容代价（`sqlrustgo-cli` 是否被当作多语句脚本执行器用）。

**材料**：
- `crates/sqlrustgo-cli/src/sqlite_mode.rs`（现场：`dispatch_one` 的 tx_depth 维护
  分支在 713-725 行；错误路径在 733-737 行）。
- 归档 run `20261106-ga5594-004` 的 `gate-clippy` 日志（该 lint 的原始现场；
  本批只合并了同形分支并加注释，**未改行为**）。
- 看板行 `SQLM-1`（本问题的登记，验收判据写在行内）。
- 参考实现的现场核对需要自己跑（本仓环境有 sqlite3 / MariaDB 可用）。

**判据**：对 (1)(2) 分别给出「选 (a)/(b) 或其他 + 一句话理由 + 可执行的验收
序列」，并明确「若维持现状 (b)，必须把它登记为与 SQLite 的已知偏差，不得当作已对齐」。
材料不足 ⇒ INCOMPLETE。

---

## 参考实现实测（2026-11-07，omp-macmini，本机跑的真实输出）

⇒ 本节只补「材料」，不做裁定。命令与输出原样贴出，裁定方可直接复跑。

### A. sqlite3（`/usr/bin/sqlite3`，无 `-bail` ⇒ 出错后继续跑完批次）

脚本（`probe.sql`）：
```sql
CREATE TABLE t(x INT);
INSERT INTO t VALUES(1);
BEGIN;
INSERT INTO t VALUES(2);
BEGIN;            -- 第 5 行
INSERT INTO t VALUES(3);
ROLLBACK;
SELECT COUNT(*) FROM t;
COMMIT;
SELECT COUNT(*) FROM t;
```

```
$ sqlite3 :memory: < probe.sql ; echo "rc=$?"
Error near line 5: cannot start a transaction within a transaction
1
Error near line 9: cannot commit - no transaction is active
1
rc=1
```

逐条读法：嵌套 `BEGIN` **被拒**（报 `cannot start a transaction within a transaction`，
并**点名行号**）；第 7 行的 `ROLLBACK` 结束的是那**唯一一层**事务 ⇒ 第 6 行的
`INSERT(3)` 被回滚 ⇒ 计数 1（只有 `INSERT(1)` 活着）；第 9 行的 `COMMIT` 因为没有活动事务
而报 `cannot commit - no transaction is active`；批次**继续**执行到结尾，退出码 1。

⇒ **SQLite 侧不存在「ROLLBACK 出栈几层」这个问题**：它根本没有第二层。
嵌套靠 SAVEPOINT / RELEASE / ROLLBACK TO，不靠 `BEGIN` 计数。

### B. MariaDB（`12.3.2-MariaDB`，`mariadb -f`）

同一序列（`-e` 单行拼接）⇒ 两次 `COUNT(*)` 都是 **2**：
第二个 `BEGIN` 在 MySQL 语义里**隐式提交**当前事务，因此 `INSERT(2)` 已被提交，
随后的 `ROLLBACK` 回滚的是第二个事务（`INSERT(3)`）。

⇒ 两个参考实现**对嵌套 BEGIN 的处理方向相反**（SQLite 拒绝 / MySQL 隐式提交），
但对 `BEGIN; INSERT; ROLLBACK; SELECT`（不嵌套）这一情形**一致**：计数 **1**。

### C. 本仓 `sqlrustgo sqlite --batch`（`target/debug/sqlrustgo`）

```
$ ./target/debug/sqlrustgo sqlite <scratch-db> --batch < probe.sql ; echo "rc=$?"
Error: sqlrustgo:error:runtime: Execution error: Transaction already in progress
rc=1
```
不嵌套的那一支（`probe2.sql`：`BEGIN; INSERT; ROLLBACK; SELECT COUNT(*)`）：
```
__agg_1
--------
1
```
⇒ 与两个参考实现**一致**（回滚生效）。

### D. 实测暴露出的、与裁定并列的三条偏差

| # | 偏差 | 证据 |
|---|---|---|
| 1 | 错误文本不同：SQLite 是 `cannot start a transaction within a transaction`，本仓是 `Transaction already in progress`，且**不带行号**（SQLite 带 `near line 5`） | B/C 两段输出 |
| 2 | 出错后**批次终止**：SQLite 默认继续跑完（后面还有两条语句并输出结果），本仓在第一条错误处停下 ⇒ 脚本后半段静默不执行 | C 的输出只有一行错误 |
| 3 | 纯语法错误（`SELECT * FROM nope_table;`）退出码 **1**，嵌套 BEGIN 这一支也是 **1** ⇒ 退出码这一层没有虚报（本条是**没问题的**，写出来是为了说明测过） | 实测 `rc=1` |

⇒ 对裁定 (2) 的直接影响：本仓**已经**在拒绝嵌套 BEGIN（不是"允许嵌套后逐层递减"），
所以 `tx_depth` 的「加一」分支在 sqlite 模式下**进不去**；
`tx_depth` 现在实际只剩「错误路径兜底 + COMMIT/ROLLBACK 各减一层」，
而这两件事在 SQLite 语义下都应当等价于「事务结束」。
⇒ 也就是说候选 (b)（逐层递减）在现网路径上**已经不可达**，裁 (a) 的代价是删计数器 +
把错误文本对齐 SQLite，而不是改语义。裁 (a) 后 (1) 的验收序列就是本文 §A 那条脚本，
期望输出逐字符对齐 sqlite3（含行号与两条错误文本）。
