# Proposal — Issue #4636: 关联标量子查询 (SELECT list)

## 问题

`SELECT id, (SELECT b.val FROM b WHERE b.id = a.id) AS bval FROM a`
应每行返回对应 b.val，实际报错：

> Scalar subquery in SELECT list is only supported for `SELECT <literal>`
> (issue #4686). The from-table form is not yet implemented.

PR #4712 仅落地了 no-table literal 形态（`SELECT (SELECT 1)`）。

## 根因

1. `execute_subquery_for_scalar` 只接受 no-table 形态，from-table 一律报错。
2. projection 循环运行在 `storage.write()` 锁内（V311-10 为 SEQUENCE 序列
   而加），在该闭包里递归 `execute_select`（内部取 storage 读锁）会与
   parking_lot 不可重入 RWLock 死锁——这就是当初只做 literal 形态的原因。

## 方案

复用既有的 correlated IN/EXISTS 机制（`substitute_outer_refs_in_select`
+ `execute_select`，同 `eval_in_subquery_membership` 模式）：

1. 新增 `execute_subquery_for_scalar_from_table(subq, outer_row, outer_ti)`：
   - 收集子查询自身列（projection 名 + FROM 表 schema 列）做影子保护
     （BUG-3b 同款），防止内层裸列被外层行值误替换；
   - `substitute_outer_refs_in_select` 把外层引用（裸 `id` 与限定
     `a.id`，跳过子查询自身 qualifier）改写为 Literal(外层行值)；
   - 对改写后（已无关联）的 select 走 `execute_select`，取首行首列；
     空结果 → `Value::Null`（标量子查询标准语义）；多行取首行（与
     WHERE 路径既有 hook 行为一致）。
2. 时序规避死锁：在 **取 storage 写锁之前**（与 WindowCall 预计算同位
   置）按外层行逐行预计算每个 from-table 标量子查询列，结果按
   `row_idx` 位置对齐；projection 循环内仅做位置查找，不递归执行。
3. literal（no-table）形态保持原 lazy 路径不变（行无关，仍走闭包）。

## 范围与限制

- 覆盖 SELECT list 顶层 `Expression::Subquery` 列（issue 复现形态、
  uncorrelated 聚合形态、空结果 NULL）。
- 嵌在更大表达式内的子查询（如 `(SELECT ..) + 1`）仍走原路径报错，
  留待后续。
- GROUP BY/HAVING 聚合投影路径未扩展（本次不在 issue 范围）。

## 验证（证据）

- 新增 `tests/integration/sql/v312_75_correlated_scalar_subquery_test.rs`
  6 用例全过：qualified 复现 / unqualified inner 影子保护 / 无匹配
  NULL / uncorrelated 聚合 / literal 回归 / correlated EXISTS 回归。
- 更新 `v312_67_scalar_subquery_in_select_test.rs` 的
  `with_table_returns_error` → `with_table_executes`（断言从报错改为
  返回 30/30/30）。
- 二进制实跑 issue 复现：csv 输出 `1,10` / `2,20`。
- 回归：q21_exists_hash_path 2 过、v312_66 7 过、parser_e2e 249 过、
  cte_e2e 11 过、sqlrustgo lib 115 过；q13/q21_cell/q2_q17 为 TPC-H
  fixture 缺失（IoError NotFound），v312_62 3 败为基线已有，均与本改动无关。
