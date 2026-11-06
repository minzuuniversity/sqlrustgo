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
