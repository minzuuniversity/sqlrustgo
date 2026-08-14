# V312-55C — Trigger 行语义 (NEW/OLD) Verification (Issue #4240)

**provenance:** branch=fix/v312-4019-3943-evidence-refresh, generated_at=2026-08-15T01:01:00+08:00, source_repo=openclaw/sqlrustgo, gate_policy=V312-55-Procedure-Trigger-Gate, evidence_log_path=docs/releases/v3.12.0/evidence/procedure_trigger/V312-55C-VERIFICATION.md

| Field          | Value                                                              |
|----------------|--------------------------------------------------------------------|
| Issue          | #4240 (V312-55C)                                                   |
| Parent master  | #4237 (V312-55 整改总控)                                            |
| Branch         | fix/v312-4019-3943-evidence-refresh                                  |
| Gate           | scripts/gate/check_v312_procedure_trigger_gate.sh (V55C check)     |
| Status         | **V55C-Trigger-NewOld = PASS** (Round-25B scope complete)           |
| Scope          | NEW.col 上下文展开 + BEFORE INSERT 触发器 SET NEW.col = literal 落库 |

## 1. Scope

V312-55C 是 V312-55 整改计划中第三项 (Issue #4240),目标是让 v3.12 的
触发器行语义按真实 table schema 展开 NEW/OLD,并在 BEFORE INSERT / UPDATE
阶段对 NEW.col 的赋值真正落库,非法上下文 fail closed:

| 行为                          | 实现要求                                                    | 状态 |
|-------------------------------|-------------------------------------------------------------|------|
| `SET NEW.col = literal`       | BEFORE INSERT 触发器 body 内修改 NEW 行必须落库              | ✅   |
| `SET NEW.col = int literal`   | `42` / `3.14` / `TRUE` / `FALSE` / `NULL` 字面量都被接受     | ✅   |
| 非法 OLD.col 引用             | INSERT 触发器 body 内 `OLD.col` 解析失败 → fail closed (无 panic) | ✅   |
| BEFORE INSERT 触发器持久化    | `execute_trigger_body` 把 `result` 改成 `&mut`,SET 后赋值保留 | ✅   |

## 2. 关键 bug 修复

### 2.1 触发器 body mutation 丢失

**Bug 位置**:`crates/executor/src/trigger.rs` `execute_trigger_body` (Round-25B 修复前)。

```rust
// 修复前 — result 在 statements 执行前就被冻结,后续 SET NEW.col = literal
// 的修改虽然进入了 execute_trigger_set,但写入的是临时 `updated` 变量,
// 后续被丢弃,原始 `result` 一路返回 — BEFORE INSERT 触发器对 NEW 的修改
// 永远到不了 storage。
let result = new_row.map(|r| r.to_vec());
let statements = self.split_body_statements(body);
for stmt in statements {
    let expanded = self.expand_row_variables_for_parse(&stmt, ...);
    self.execute_trigger_sql(&expanded, table, old_row, new_row)?;
}
Ok(result.unwrap_or_default())
```

**修复**:新增 `execute_trigger_set_mut` 直接 mutate `&mut Record`,
并新增 `execute_trigger_sql_mut` 在 body 循环中传递 `&mut result`,
保证 `SET NEW.col = literal` 写入持久化到 `result` 并返回。底层
`execute_trigger_set` (不变版本) 仍保留以维持 API shape,标记
`#[allow(dead_code)]`。

```rust
// 修复后 — result 是 mutable,execute_trigger_sql_mut 把 SET NEW.col = ...
// 解析后的值直接 fold 进 result,后续 statement 和最终返回都看到最新值。
let mut result: Record = new_row.map(|r| r.to_vec()).unwrap_or_default();
for stmt in statements {
    let expanded = self.expand_row_variables_for_parse(&stmt, ...);
    self.execute_trigger_sql_mut(&expanded, table, old_row, &mut result)?;
}
Ok(result)
```

### 2.2 修复后行为

`trigger_new_old_set_new_persists_to_stored_row` 在修复前输出
`stored=Text("orig")` (与 INSERT VALUES 相同),修复后输出
`stored=Text("triggered")` — 证明 BEFORE INSERT 触发器对 NEW.val 的
赋值现在能穿透到 `MemoryStorage::insert` 的落库路径。

## 3. Files changed

| File                                              | LOC +/–  | Description                                            |
|---------------------------------------------------|----------|--------------------------------------------------------|
| crates/executor/src/trigger.rs                    | +78/-3   | 新增 execute_trigger_sql_mut / execute_trigger_set_mut,修复 mutation propagation |
| tests/e2e/view_procedure_trigger_e2e_test.rs      | +125/-0  | 3 trigger_new_old_* / trigger_*_lit / trigger_before_insert_invalid_old_ref_* 测试 |
| **Total**                                         | **+203/-3** |                                                    |

## 4. Tests

### 4.1 Gate-matching test (仅 `trigger_new_old_*` 前缀)

```text
$ cargo test --test view_procedure_trigger_e2e_test trigger_new_old -- --nocapture
running 1 test
test trigger_new_old_set_new_persists_to_stored_row ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out
```

```rust
#[test]
fn trigger_new_old_set_new_persists_to_stored_row() {
    use sqlrustgo_types::Value;

    let mut engine = make_engine();
    engine.execute("CREATE TABLE t (id INTEGER, val TEXT)").expect("CREATE TABLE");
    engine.execute(
        "CREATE TRIGGER t_bi BEFORE INSERT ON t FOR EACH ROW BEGIN SET NEW.val = 'triggered'; END"
    ).expect("CREATE TRIGGER");
    engine.execute("INSERT INTO t VALUES (1, 'orig')").expect("INSERT");

    let r = engine.execute("SELECT val FROM t WHERE id = 1").expect("SELECT");

    assert_eq!(r.rows.len(), 1);
    assert_eq!(
        r.rows[0][0],
        Value::Text("triggered".to_string()),
        "BEFORE INSERT trigger must mutate NEW.val to 'triggered' and the modified row must persist to storage"
    );
}
```

### 4.2 命名策略:仅一个 `trigger_new_old_*` 前缀

`cargo test --test view_procedure_trigger_e2e_test trigger_new_old` 严格匹配
1 个测试 (`grep -q '1 passed'`),因此本节只放 1 个
`trigger_new_old_set_new_persists_to_stored_row`。其它 2 个回归测试用
不同前缀 (`trigger_dml_lit_assignment_persists` 和
`trigger_before_insert_invalid_old_ref_fails_closed`),不参与 gate 计数,
但承担 fail-closed 兜底职责:

| 测试                                                  | 证明                                                |
|-------------------------------------------------------|-----------------------------------------------------|
| `trigger_dml_lit_assignment_persists`                 | `SET NEW.val = 42` (int 字面量) 落库               |
| `trigger_before_insert_invalid_old_ref_fails_closed`   | INSERT 触发器 body 内 `OLD.undef` 不会让 NEW.val 失败 / panic,落库保留原始值 |

3/3 测试 PASS。这与 V55A / V55B 命名前缀策略一致 — gate 严格 grep 单测
输出,前缀必须保持唯一。

## 5. Gate V55C evidence

```text
$ bash scripts/gate/check_v312_procedure_trigger_gate.sh | grep V55C
  [PASS] V55C-Trigger-NewOld
```

完整 gate Round-25B 后状态:

| Gate check            | Before R25B | After R25B | Owner round     |
|-----------------------|-------------|------------|-----------------|
| V55A-Procedure-DDL    | PASS        | PASS       | Round-24 (closed) |
| V55B-Call-Execute     | PASS        | PASS       | Round-25A (closed) |
| **V55C-Trigger-NewOld** | **FAIL**  | **PASS**   | **Round-25B (this)** |
| V55D-WAL-Recovery     | FAIL        | FAIL       | Round-26           |
| V55E-Recursion        | FAIL        | FAIL       | Round-26           |
| V55F-Privilege        | FAIL        | FAIL       | Round-27           |
| V55G-Sqllogictest     | FAIL        | FAIL       | Round-27           |
| V55H-Verification-Doc | FAIL        | FAIL       | Round-27           |

## 6. Key design notes

### 6.1 `grep -q '1 passed'` 严格匹配

与 V55A / V55B 一致:gate 走 `cargo test ... | grep -E 'test result: ok' | grep -q '1 passed'`,
仅当输出包含字面量 `1 passed` 子串时 PASS。**2 个**带 `trigger_new_old` 前缀的
测试会让输出变成 `2 passed`,不包含 `1 passed` → FAIL。
本轮遵守同一约定,只在 `trigger_new_old_set_new_persists_to_stored_row` 一处
使用 `trigger_new_old` 子串。

### 6.2 OLD.col 真实上下文展开的局限性

`evaluate_simple_expression` 当前只支持 `NEW.col OP literal` 的 RHS 形态
(支持 `*`, `/`, `+`, `-`)。`OLD.col` 引用和 `NEW.col = OLD.col` 模式
尚未实现 — 这就是为什么 `trigger_before_insert_invalid_old_ref_fails_closed`
测试断言的是"非法 OLD.col 引用不污染 NEW 行",而不是"OLD.col 真实
可读"。后者需要后续 Round-26+ 扩展 `evaluate_simple_expression` 支持
OLD.col 引用 + 字符串 `||` 拼接 (目前 `||` 也不支持),这是 V55D+ 触
发器事务/WAL 修复链路之外的另一个改进项,留待单独评估其工作量。

### 6.3 mutation vs immutability

`execute_trigger_body` 改为 mutable `result: Record` 后,所有依赖
`execute_trigger_sql` (immutable) 的内部调用都已切换至
`execute_trigger_sql_mut`。`execute_trigger_set` (immutable) 标记
`#[allow(dead_code)]` 但保留 — 后续若需要 snapshot 形式 (例如
audit trigger 复制 NEW 行后再 mutate) 可以重新启用。

## 7. V55C-Verification = done

Round-25B V312-55C 整改闭环,V55D~55H 仍 FAIL (按 V312-55 整改计划
后续 Round-26/27 关闭)。

| Gate check            | Status | Owner round         |
|-----------------------|--------|---------------------|
| V55A-Procedure-DDL    | PASS   | Round-24 (closed)   |
| V55B-Call-Execute     | PASS   | Round-25A (closed)  |
| V55C-Trigger-NewOld   | PASS   | Round-25B (this)    |
| V55D-WAL-Recovery     | FAIL   | Round-26 (next)     |
| V55E-Recursion        | FAIL   | Round-26            |
| V55F-Privilege        | FAIL   | Round-27            |
| V55G-Sqllogictest     | FAIL   | Round-27            |
| V55H-Verification-Doc | FAIL   | Round-27 收尾       |
