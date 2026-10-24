# 批次 37 证据 —— GROUP BY 键的名字解析（别名不再当常量，未知标识符不再 panic）

| 字段 | 值 |
|---|---|
| gate_policy_eval_id | `v410-b37-group-by-binder-001` |
| source_agent | openclaw-mcode (MiniMax Code) |
| source_run | 批次 37（本机实测，Mac-mini-7.local arm64 macOS 27.0.0） |
| timestamp | 2026-10-24T18:20:00+08:00 |
| 基线 | `develop/v4.1.0` = `7eff3f7558da` |
| 工作树 | `wt-v410-b37`（分支 `fix/group-by-binder-5462-5463`） |
| 关联 issue | #5462（GROUP BY 别名被当字符串常量）、#5463（GROUP BY 未知标识符 ⇒ server panic） |

---

## 0. 一句话

`GROUP BY` 的键列表**从未经过名字解析**。于是：

* **#5462** — `GROUP BY <SELECT 列表别名>` 被当成**字符串常量**执行 ⇒
  每一行产生同一个分组键 ⇒ 整个输入被压成 **1 组**，且**不报任何错**；
* **#5463** — `GROUP BY <未知标识符>` **不被 binder 拒绝**（SELECT 列表有这条检查，
  `GROUP BY` 没有）⇒ 非法语句一路走到执行器，在那里触发行宽断言
  `panic!` ⇒ **server 线程死掉**，该 server 上**所有连接**同时断掉。

两个缺陷**同一个根因**。本批把它收敛成**一份实现**
（`engine_utils::resolve_group_by_keys`），并在**引擎层**与**wire 层**各留一套
判别力测试。

---

## 1. 根因（代码级定位）

### 1.1 binder 漏掉了 `group_by`

`src/engine_utils.rs::validate_select_columns_referenced` 检查了
SELECT 列表 / WHERE / HAVING / ORDER BY 四处引用，**唯独没有 `group_by`**：

```rust
for col in &select.columns { check_expr_references(expr, &exists)?; }   // SELECT
if let Some(ref w) = select.where_clause { ... }                          // WHERE
if let Some(ref h) = select.having { ... }                                // HAVING
for ord in &select.order_by { ... }                                       // ORDER BY
// ← 没有 group_by
```

### 1.2 执行器把未解析的标识符当成常量

`src/engine_select.rs` 的分组主循环直接对 AST 求值：

```rust
let key: Vec<Value> = group_exprs.iter()
    .map(|expr| evaluate_expression(expr, row, &table_info).unwrap_or(Value::Null))
    .collect();
groups.entry(key).or_default().push(row_idx);
```

而 `eval_identifier` 对「在 schema 里找不到的标识符」**回退成
`Value::Text(名字本身)`**（这是 #4041 为兼容 SQLLogicTest fixture 保留的
legacy 行为，`src/engine_utils.rs` 的文档注释里写得很清楚）。
⇒ `GROUP BY lf` 的键恒为 `Text("lf")` ⇒ 5 行全落进 1 组。

⇒ 这两条合起来正好解释了两个 issue 的现象。

---

## 2. RED（修复前实测）

新增测试：`src/engine_group_by_binder_tests.rs`（引擎层，6 例）、
`crates/mysql-server/tests/group_by_binder_5462_5463_test.rs`（wire 层，5 例）。

### 2.1 引擎层

```
$ cargo test --lib engine_group_by_binder
running 6 tests
test ...::test_5463_group_by_unknown_identifier_in_select_list_is_still_rejected ... ok
test ...::test_5462_group_by_real_column_is_unchanged ... ok
test ...::test_5463_group_by_unknown_identifier_does_not_break_later_statements ... FAILED
test ...::test_5462_group_by_alias_of_plain_column_groups_by_the_column ... FAILED
test ...::test_5463_group_by_unknown_identifier_is_a_binder_error ... FAILED
test ...::test_5462_group_by_alias_of_expression_groups_by_the_expression ... FAILED

---- test_5462_group_by_alias_of_plain_column_groups_by_the_column stdout ----
assertion `left == right` failed: GROUP BY <alias> must produce one group per distinct
underlying value, got ["Text(\"lf\"),Integer(5)"]
  left: 1
 right: 3

---- test_5463_group_by_unknown_identifier_is_a_binder_error stdout ----
GROUP BY <unknown column> must be rejected by the binder:
  ExecutorResult { rows: [[Integer(5)]], affected_rows: 1, columns: [...] }

test result: FAILED. 2 passed; 4 failed; 0 ignored; 0 measured; 164 filtered out
```

⇒ `Text("lf")` **就是 issue 里描述的那个键**（分组键 = 字符串 `"lf"`）。

### 2.2 wire 层（真实 server + MySQL wire 协议）

```
$ cargo test -p sqlrustgo-mysql-server --test group_by_binder_5462_5463_test
test group_by_real_column_is_unchanged ... ok
test group_by_alias_of_an_expression_groups_by_the_expression ... FAILED
test group_by_unknown_column_returns_an_error_packet_and_the_server_survives ... FAILED
test group_by_alias_of_a_column_groups_by_the_column ... FAILED
test group_by_unknown_column_does_not_take_down_other_connections ... FAILED

---- group_by_alias_of_a_column_groups_by_the_column stdout ----
assertion `left == right` failed: GROUP BY <alias> must group by the underlying
column's value, not by the alias text (which would collapse everything into one group)
  left: ["lf,5"]
 right: ["A,2", "N,2", "R,1"]

test result: FAILED. 1 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out
```

---

## 3. 修法（一份实现）

### 3.1 新增唯一实现

`src/engine_utils.rs`：

```rust
pub fn resolve_group_by_keys(
    group_by: &[Expression],
    select_columns: &[SelectColumn],
    table_info: &TableInfo,
) -> SqlResult<Vec<Expression>>
```

规则（SQL-1999 §10.4 / MySQL 8.0 §12.20.3）：

1. 键是 **FROM 关系的列** ⇒ 原样保留；
2. 键是 **SELECT 列表输出别名** ⇒ 换成**该别名的定义表达式**；
3. 都不是 ⇒ **binder 错误** `Binder error: column '<name>' not found in schema`
   —— 与 SELECT 列表**同一条规则、同一条消息**。

「是不是 schema 里的列」复用既有的 `find_column_index`（已处理限定名
`t.col`、多级 join 的累计列名 `a_join_b.a.tag`），不另起一套。

### 3.2 调用点（分组之前）

`src/engine_select.rs` 的分组阶段入口：

```rust
let resolved_group_by = crate::engine_utils::resolve_group_by_keys(
    &select.group_by, &select.columns, &table_info)?;
```

`GROUPING SETS` 的成员用**同一个** `resolve_group_by_expression` 解析，
保证「set 成员」与「GROUP BY 键」仍然结构可比。

⇒ 非法键**在任何一行被分组之前**就被拒，执行器永远看不到它。

---

## 4. GREEN（修复后实测）

| 检查 | 结果 |
|---|---|
| `cargo test --lib engine_group_by_binder` | **6 passed / 0 failed** |
| `cargo test -p sqlrustgo-mysql-server --test group_by_binder_5462_5463_test` | **5 passed / 0 failed** |
| `cargo test --lib`（根 crate 全量） | **170 passed / 0 failed** |
| `cargo clippy --all-features --all-targets -- -D warnings` | **exit 0，0 告警** |
| `cargo fmt --check`（本批 5 个文件） | **全部 OK** |

---

## 5. 变异自测（A/B 判别力）

把 `resolve_group_by_keys` 临时改成**不做解析**（直接返回原列表），
其余代码不动：

| 臂 | 引擎层 | wire 层 |
|---|---|---|
| **A（关闭修复）** | **2 passed / 4 failed** | **1 passed / 4 failed** |
| **B（恢复修复）** | **6 passed / 0 failed** | **5 passed / 0 failed** |

A 臂的失败输出与 issue 描述逐字吻合（`["lf,5"]` / 单组 / 键为别名字面量），
说明**这些测试真的能抓到缺陷，不是恒通过**。

---

## 6. 全仓回归的 1 处红 —— 判定为**本地环境限制，不是代码红**

`cargo test --workspace` 的唯一失败：

```
---- coverage_gate_derives_the_features_storage_actually_needs stdout ----
thread '...' panicked at tests/integration/gate/alpha_gate_blocker_test.rs:78:5:
the derivation must find the fault-injection feature. Got: ""
```

逐条排除：

| 判据 | 证据 |
|---|---|
| 是不是本批引入 | `git diff 7eff3f7558da -- tests/integration/gate/alpha_gate_blocker_test.rs scripts/gate/check_coverage_v312.sh` ⇒ **空**（两个文件都没被本批碰过） |
| 推导逻辑本身坏了吗 | 同一条推导改用临时文件 source ⇒ 输出 `bin_storage_default fault-injection test-support` ⇒ **推导是好的** |
| 为什么本机拿到空 | 该测试用 `source <(...)` 取函数；**macOS 自带 bash 3.2 的 `source <(...)` 读不到内容**。最小复现：`bash -c 'source <(echo "f() { echo HI; }"); f'` ⇒ `bash: f: command not found` |
| CI 上会不会红 | CI 作业镜像 `sqlrustgo-runner:v1` 的 bash 是 **5.3.3**（Alpine），`source <(...)` 正常；该测试自 2026-10-22 21:28 落地起在 CI 跑过多次 |

⇒ 判定：**本地环境红（macOS bash 3.2），CI 绿**。不计入代码红。

---

## 7. 顺带发现（另立 issue，本批不改）

跑 `cargo test --workspace` 时，`tests/integration/compat/a04_mysql_diff_matrix_test.rs`
**覆写了已提交的证据文件**
`docs/releases/v4.1.0/evidence/a04-5226/wire/*.txt`：当 PostgreSQL 参考容器
不可用时，它把整份记录替换成一行
`ENGINE_UNAVAILABLE: container a04-pg-ref not running`。

⇒ 在没有 pg 参考容器的机器上跑一次全仓测试，**就把已归档的证据抹掉了**。
本批已把被改写的 21 个文件 `git checkout --` 还原，并在 PR diff 里确认
**不含**这些文件。已另立 issue 跟踪。

---

## 8. 变更范围

| 文件 | 性质 |
|---|---|
| `src/engine_utils.rs` | **新增** `resolve_group_by_keys` / `resolve_group_by_expression` / `select_list_definition`（唯一实现） |
| `src/engine_select.rs` | 分组阶段入口调用解析器；`GROUPING SETS` 用解析后的集合 |
| `src/lib.rs` | 注册新测试模块 |
| `src/engine_group_by_binder_tests.rs` | **新增**：引擎层 6 例 |
| `crates/mysql-server/tests/group_by_binder_5462_5463_test.rs` | **新增**：wire 层 5 例 |

**零**生产语义之外的改动；**零**门禁脚本改动；**零**豁免标记。

Refs #5462
Refs #5463
