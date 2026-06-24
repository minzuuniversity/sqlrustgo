# MySQL 协议多语句执行修复 (COM_QUERY multi-statement) - Issue

> **创建日期**: 2026-06-24
> **作者**: Hermes Agent
> **状态**: RESOLVED
> **Priority**: P1 (correctness gap, not GA blocker)
> **相关分支**: `fix/pre-existing-tech-debt`
> **Related**:
> - PR #3584 (`fix(mysql-server): revert broken COM_QUERY arm to restore server compile`) - 上一轮 revert
> - commit `2d882e05c` (`fix(mysql-server): column definition packet byte ordering, SELECT result-set routing, and multi_statement_test`) - 引入 multi_statement_test 但 handler 仍用 `eng.execute(&q)` 整段跑
> - commit `6c489ce3c` (`fix(parser): add multi-statement support for semicolon-separated queries`) - parser 支持多语句但 handler 未接
> - `GA_READINESS_STATUS_2026-06-21.md` 报告 multi_statement_test 处于"现状记录"状态(实际不验证多语句)

---

## 1. 问题概述

`crates/mysql-server/src/lib.rs` 的 COM_QUERY handler 在执行多语句(SQL 文本中含 `;` 分隔的多个 statement)时,**只执行第一个 statement,其余被静默丢弃**。

### 1.1 根因

handler 在 `parse_statements(&q)` 之后,虽然 `for stmt in stmts` 遍历了所有 AST,但循环内调用的却是:

```rust
let result = eng.execute(&q);   // ← BUG: 传的是整段原始 SQL,不是单个 stmt
```

`eng.execute()` 内部走 `parse()` 单语句解析路径,所以无论循环多少遍,**实际只跑第一句**。这是 2d882e05c 引入的 — 它在 `for` 循环内复用 `&q` 而非循环变量 `stmt` 转出的字符串,导致 multi-statement 特性实际上从未工作。

### 1.2 历史

- `6c489ce3c` (2026) 添加 parser 端 `parse_statements()` 多语句解析
- `2d882e05c` (2026-06-21) 引入 `multi_statement_test.rs` 但 handler 实现错误
- PR #3584 (2026-06-22) revert 了 2d882e05c 的编译错误部分,但 multi_statement_test 改写为"现状记录"形态(test 仍编译通过但不再验证多语句)
- 当前 fix 把多语句**真正**接通

### 1.3 协议依据

MySQL wire-protocol COM_QUERY 允许客户端在一个 packet 内发送多条 `;` 分隔的语句,server 必须按顺序执行并**为每条语句发一个 MySQL 响应 packet**(OK / ERR / result-set)。MySQL 官方实现:首个错误后停止批处理。

---

## 2. 修复方案

### 2.1 新增 `split_sql_statements` (parser crate)

`crates/parser/src/parser.rs` 新增 `pub fn split_sql_statements(sql: &str) -> Vec<String>`:
- byte-level state machine 切分原始 SQL
- 正确处理:
  - 嵌套括号 `(...)`、`[...]`
  - 字符串字面量 `'...'`(含 `''` 转义)
  - 双引号标识符 `"..."`
  - 行注释 `--` 和块注释 `/* ... */`
  - 反斜杠转义
- 返回 `Vec<String>`,每个 element 是已 trim 的非空 SQL 片段
- 输入为空 / 全注释时返回 `Vec::new()`

`crates/parser/src/lib.rs` 导出 `split_sql_statements`。

**8 个单元测试覆盖**:
- `single_statement_no_semi` — 单条无分号
- `two_statements_separated_by_semi` — 基本两语句
- `trailing_semicolon_drops_empty` — 尾随分号
- `semicolon_inside_parens_is_preserved` — 括号内分号不切分
- `semicolon_inside_string_literal_is_preserved` — 字符串内分号不切分
- `escaped_quote_does_not_close_string` — `''` 转义
- `line_comment_around_semi` — 注释中分号不切分
- `empty_input_yields_empty_vec` — 空 / 全注释输入

### 2.2 改造 COM_QUERY handler (mysql-server crate)

`crates/mysql-server/src/lib.rs` 重写 COM_QUERY 的 multi-statement 分支:

```rust
// 旧(2d882e05c 引入,bug):
match parse_statements(&q) {
    Ok(stmts) => for stmt in stmts {
        let result = eng.execute(&q);  // ← BUG: 用整段 q
        ...
    }
}

// 新:
let fragments = sqlrustgo_parser::split_sql_statements(&q);
if fragments.is_empty() { /* no-op OK */ }
else for frag in fragments {
    let is_select = /* 字符串前缀检查 SELECT/WITH/VALUES/SHOW/DESCRIBE/DESC/EXPLAIN */;
    match eng.execute(&frag) {  // ← 真正逐条独立执行
        Ok(r) if is_select => send_result_set(...)?,
        Ok(r) => make_ok_packet(...)?,
        Err(e) => { make_err_packet(...)?; break; }  // MySQL 语义:首个错误后停止
    }
}
```

**关键改动**:
- 切分从 AST 层改为字符串层,绕开 `Statement` AST 不可逆序列化的限制
- 每个 fragment 独立 `eng.execute(&frag)`,真正按 SQL 单元执行
- SELECT 判定用字符串前缀(不需要 AST 重建)
- 错误时 `break`,符合 MySQL 协议 spec

### 2.3 重写 multi_statement_test.rs

原 test 退化为"现状记录"(连续发多个独立 COM_QUERY,不验证多语句)。新 test 直接驱动 raw TCP 流,验证 server 对一次 COM_QUERY 的响应 packet 序列:

**Phase 1**: 单条 COM_QUERY 内 3 条语句 `CREATE TABLE; INSERT; SELECT` → 期望响应: OK, OK, result-set(1 行 2 列)

**Phase 2**: 两条 SELECT 在一条 COM_QUERY → 期望响应: result-set, result-set(独立 result-set 连续发回)

**Phase 3**: 多语句批处理的状态持久化 → INSERT+SELECT 在一个 COM_QUERY,后续单语句 SELECT 看到数据

**Phase 4**: 批处理中途错误 → INSERT(OK) + INSERT-no-such-table(ERR) + INSERT(不应执行) → 期望: OK, ERR, 无第三个 packet;`SELECT id, val FROM ms_t1` 看到前两条的 state(行 id=3 in),但行 id=4 不存在(server 在 ERR 后停止)

---

## 3. 验证

### 3.1 parser crate

```bash
cargo test -p sqlrustgo-parser split_sql_statements
# cargo test: 8 passed (4 suites, 487 filtered, 0.00s)
```

### 3.2 mysql-server crate

```bash
cargo test --test multi_statement_test
# cargo test: 15 passed (1 suite, 0.25s)
# 含 test_multi_statement_executes_all (4 phases 全过) + 14 个 common 模块辅助

cargo test -p sqlrustgo-mysql-server --tests
# cargo test: 126 passed (4 suites, 0.00s) — 无回归
```

### 3.3 workspace build / clippy / fmt

```bash
cargo build --workspace
# OK (只有 pre-existing warning: sqlrustgo-sql-corpus 死代码,与本 fix 无关)

cargo clippy -p sqlrustgo-parser -p sqlrustgo-mysql-server
# OK

cargo fmt --check
# 干净
```

### 3.4 改动统计

```
crates/mysql-server/src/lib.rs |  84 +++++-----
crates/parser/src/lib.rs       |   2 +-
crates/parser/src/parser.rs    | 189 +++++++++++++++++++++++
tests/multi_statement_test.rs  | 339 +++++++++++++++++++++++++----------------
4 files +445 -169
```

### 3.5 Pre-existing 不相关失败

`tests/ee_module_boundary_test.rs::ee_01_execution_engine_under_2000_lines`:
- `execution_engine.rs` 当前 2384 行,超过 2000 行硬限
- **本 fix 没碰 execution_engine.rs**
- 在 `git stash` 后的 baseline 也 fail,确认 pre-existing

---

## 4. 风险与遗留

### 4.1 已控制风险

- `split_sql_statements` 8 个单元测试覆盖主要边界(括号、字符串、注释、转义)
- handler 错误时 `break`,符合 MySQL 官方 spec 的"首错即停"语义
- DML state 持久化(在事务内的 INSERT 跨多条语句可见)被 Phase 3 验证

### 4.2 未做的工作

- **未在 server 主路径(non-test)的 `do_command_loop` 加 `tracing::info!` 日志**: 当前 2d882e05c 已加 multi-statement 分支的注释,没有加 tracing。本 fix 保留这个状态。
- **未改 `is_select_stmt` 函数本身**: 旧代码用 `is_select_stmt(&Statement)`,新代码用字符串前缀。`is_select_stmt` 函数保留但不再被 COM_QUERY handler 调用,可能在其他路径用到 — 没删。
- **未改 `parse_statements`**: 它仍存在,新代码用 `split_sql_statements` 替代。可以后续清理(若 `parse_statements` 无其他调用者,删除)。
- **未实现 prepared-statement multi-statement 路径**: COM_STMT_EXECUTE 多语句不在本 fix 范围。

### 4.3 Follow-up 建议

1. `parse_statements` 现在没有 COM_QUERY handler 调用者,检查是否还有 REPL/CLI 路径用 — 若没有,删除
2. `is_select_stmt` 同上
3. sysbench / mysql CLI 兼容性测试(本 fix 满足 MySQL wire-protocol spec,理论上兼容,但应跑一次 sysbench OLTP 多语句 workload 验证)
4. 长期(ADR-013 v3.10): 把 `eng.execute` 改为接受 `Vec<Statement>`,从源头支持多语句,handler 不用 split

---

## 5. 参考

- MySQL Protocol :: 14.6.1 "COM_QUERY" — https://dev.mysql.com/doc/dev/mysql-server/latest/page_protocol_com_query.html
- MySQL Reference Manual :: 13.2.1 "CALL Statement" - 多语句 transaction 语义
- 本 fix 涉及的 commit 链:
  - `6c489ce3c` parser multi-statement support
  - `2d882e05c` broken COM_QUERY arm (revert by #3584)
  - `d0acc4b2f` revert to restore server compile
  - `a64b42082` (unrelated) wal-recovery fix
  - 本 fix: `crates/parser/src/{lib.rs, parser.rs}` + `crates/mysql-server/src/lib.rs` + `tests/multi_statement_test.rs`
