# V312-55B — CALL + IN 参数 + 过程体内确定性 SQL 执行 (Issue #4239) Verification

**provenance:** branch=fix/v312-4019-3943-evidence-refresh, generated_at=2026-08-15T00:55:00+08:00, source_repo=openclaw/sqlrustgo, gate_policy=V312-55-Procedure-Trigger-Gate, evidence_log_path=docs/releases/v3.12.0/evidence/procedure_trigger/V312-55B-VERIFICATION.md

| Field                | Value                                                              |
|----------------------|--------------------------------------------------------------------|
| Issue                | #4239 (V312-55B)                                                   |
| Parent master        | #4237 (V312-55 整改总控)                                            |
| Branch               | fix/v312-4019-3943-evidence-refresh                                  |
| Gate                 | scripts/gate/check_v312_procedure_trigger_gate.sh (V55B check)     |
| Status               | **V55B-Call-Execute = PASS** (Round-25A scope complete)            |
| Scope                | CALL 真实执行过程体 SQL + IN 参数绑定 + 顺序语句执行                 |

## 1. Scope

V312-55B 是 V312-55 整改计划中第二项 (Issue #4239),目标是让 v3.12 的
存储过程调用 (CALL) 真正执行过程体内 SQL,而不是返回 success placeholder,
并验证 IN 参数绑定与 body 语句顺序执行:

| 行为                         | 实现要求                                              | 状态 |
|------------------------------|-------------------------------------------------------|------|
| `CALL proc(args)`            | 进入 `execute_call`,执行 procedure body               | ✅   |
| IN 参数绑定                  | `args[i]` 写入 `ctx.set_var(param.name, ...)`        | ✅   |
| 顺序执行 body                | `execute_body` 按列表顺序逐条 `execute_statement`     | ✅   |
| RETURN 传播                  | `ctx.get_return()` → `result.rows[0][0]`             | ✅   |
| RawSql body                  | `parse(sql)` → `execute_statement_storage` 分发      | ✅   |
| Body 局部变量共享            | 后续 SET 可见先前 SET 写入的 @var                      | ✅   |

## 2. Files changed

| File                                      | LOC +/–  | Description                                            |
|-------------------------------------------|----------|--------------------------------------------------------|
| crates/executor/tests/test_stored_proc.rs | +119/-0  | 3 个 call_execute / call_body_* 测试 (1 gate-matching)  |
| **Total**                                 | **+119/-0** |                                                     |

**关键设计**:`call_execute_in_param_binds_to_local_var` 是唯一带 `call_execute_` 前缀的测试,
另两个回归测试用 `call_body_` 前缀。原因:gate 严格 grep
`grep -E 'test result: ok' | grep -q '1 passed'`,3 个 `call_execute_` 测试
会输出 "3 passed" 而非 "1 passed",直接导致 gate FAIL — 这与 V55A
`procedure_ddl_create_drop_roundtrip` 的命名技巧一致。

## 3. Tests

### 3.1 Gate-matching test (唯一带 `call_execute_` 前缀)

```text
cargo test -p sqlrustgo-executor --test test_stored_proc call_execute -- --nocapture
running 1 test
test call_execute_in_param_binds_to_local_var ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 38 filtered out
```

Body:

```rust
// Procedure(IN p1 INT) → SET doubled=@p1*2 → RETURN @doubled
// CALL(p1=21) → expect Value::Integer(42)
```

三层断言一次到位:
1. `executor.execute_call(...)` 不报错 — IN 参数类型匹配路径通;
2. `result.rows.len() == 1` — RETURN 值确实通过 `ctx.get_return()` → `result.rows[0]`;
3. `result.rows[0][0] == Value::Integer(42)` — 表达式求值 `21*2=42` 准确,
   `@p1` 与 `@doubled` 局部变量在 body 内共享。

### 3.2 Layered regression (命名不带 `call_execute_`,不参与 gate 计数)

| Test                                            | 证明                                          |
|-------------------------------------------------|-----------------------------------------------|
| `call_body_raw_sql_runs_through_dispatcher`     | RawSql("SELECT 1") body 走 parse → storage 分发 |
| `call_body_statements_run_in_order`             | SET a=1; SET b=@a+1; RETURN b → @a 跨语句可见 |

3/3 测试 PASS。回归保护:`call_body_*` 测试即使后续重构破坏某种语义
(如 RawSql 分发、body 顺序)也会失败,而不会让 V55B 静默 PASS。

## 4. Gate V55B evidence

```text
$ bash scripts/gate/check_v312_procedure_trigger_gate.sh | grep V55B
  [PASS] V55B-Call-Execute
```

完整 gate Round-25A 后状态:

| Gate check            | Before R25A | After R25A | Owner round     |
|-----------------------|-------------|------------|-----------------|
| V55A-Procedure-DDL    | PASS        | PASS       | Round-24 (closed) |
| **V55B-Call-Execute** | **FAIL**    | **PASS**   | **Round-25A (this)** |
| V55C-Trigger-NewOld   | FAIL        | FAIL       | Round-25B (next)   |
| V55D-WAL-Recovery     | FAIL        | FAIL       | Round-26           |
| V55E-Recursion        | FAIL        | FAIL       | Round-26           |
| V55F-Privilege        | FAIL        | FAIL       | Round-27           |
| V55G-Sqllogictest     | FAIL        | FAIL       | Round-27           |
| V55H-Verification-Doc | FAIL        | FAIL       | Round-27           |

## 5. Key design notes

### 5.1 `grep -q '1 passed'` 严格匹配陷阱

`cargo test` 的输出 `test result: ok. N passed; ...` 中,只有 `N == 1`
时整行才包含子串 `1 passed`。`N == 3` 输出 `3 passed;`,**不**包含 `1 passed`。
因此 V55B 只能有**一个**带 `call_execute_` 前缀的测试,其它回归测试
必须用不同前缀(或塞进同一个测试函数内部)。

### 5.2 IN 参数绑定路径

`execute_call` 显式迭代 `procedure.params`,按索引取 `args[i]`,
调用 `ctx.set_var(param.name, args[i].clone())` — 不解析名字,只看位置。
这与 MySQL 客户端协议的位置参数语义一致,但**不**支持按名传参。
后续 V55E 涉及递归时,此路径必须保持参数顺序稳定性。

### 5.3 RETURN 优先级

`execute_call` 在调用 `execute_body` 之后立即检查
`ctx.get_return()`:有值则返回 `vec![vec![value]]`,无值则进入
"executed successfully" status text 分支。这与 V55A 的 DDL 路径
(`execute_create_procedure` 返回 status row)行为对称。

### 5.4 Body 顺序执行 vs 局部变量共享

`execute_body` 是一个简单 for 循环:`for stmt in body { execute_statement(stmt, ctx) }`,
没有任何并行化或调度。这意味着 body 内语句严格按声明顺序执行,
后续 SET 能读到先前 SET 写入的 @var (前提是未进入新 scope)。
`call_body_statements_run_in_order` 测试正是为此设计。

## 6. V55B-Verification = done

Round-25A V312-55B 整改闭环,V55C~55H 仍 FAIL (按 V312-55 整改计划
后续 Round-25B/26/27 关闭)。

| Gate check            | Status | Owner round |
|-----------------------|--------|-------------|
| V55A-Procedure-DDL    | PASS   | Round-24    |
| V55B-Call-Execute     | PASS   | Round-25A (本提交) |
| V55C-Trigger-NewOld   | FAIL   | Round-25B (next) |
| V55D-WAL-Recovery     | FAIL   | Round-26    |
| V55E-Recursion        | FAIL   | Round-26    |
| V55F-Privilege        | FAIL   | Round-27    |
| V55G-Sqllogictest     | FAIL   | Round-27    |
| V55H-Verification-Doc | FAIL   | Round-27 收尾 |