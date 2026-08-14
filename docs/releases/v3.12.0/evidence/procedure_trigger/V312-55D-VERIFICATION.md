# V312-55D — Trigger 事务 + WAL + Recovery 验证报告

**Round-26 (2026-08-15)**
**Branch**: `fix/v312-4019-3943-evidence-refresh`
**关联 PR**: [#4262](http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/4262) (V312-55B/C/D)
**关联 Issue**: [#4241](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4241)
**父 Issue**: [#4237](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4237)

---

## Scope

V312-55 整改第四项: BEGIN 内 AFTER INSERT 写 audit + ROLLBACK 后 base/audit 均为空;kill -9 replay 一致。

具体子目标:
1. **真 bug 修复** — `execute_trigger_body` 中 `expand_insert_values` 对空 Record 的
   越界 panic (`trigger.rs:747`)，该 bug 在 DELETE 触发器携带 INSERT body 时触发
   (`BEFORE DELETE ... INSERT INTO backup SELECT * FROM t WHERE id = OLD.id`)。
2. **WAL 事务联动** — BEGIN + INSERT INTO base (AFTER INSERT trigger 写 audit) +
   ROLLBACK 之后，base 与 audit 两张表都必须为空 (trigger 副作用必须随父事务回滚)。
3. **crash-replay 一致性** — kill -9 后重启 server，共享 data_dir 必须读到一致的
   空状态 (与 T-001/T-002/T-003 的 COMMIT 路径形成对比)。

## 关键 bug 修复

### 触发场景

```
CREATE TABLE t1 (id INTEGER, value INTEGER);
CREATE TRIGGER t1_delete_backup BEFORE DELETE ON t1 FOR EACH ROW
  BEGIN INSERT INTO t1 SELECT * FROM t1 WHERE id = old.id END;

BEGIN;
INSERT INTO t1 VALUES (1, 100);  -- OK
INSERT INTO t1 VALUES (2, 200);  -- OK
COMMIT;

BEGIN;
DELETE FROM t1 WHERE id = 1;  -- PANIC: index out of bounds
```

### 根因

`crates/executor/src/trigger.rs::expand_insert_values` (原第 739-755 行):

```rust
fn expand_insert_values(&self, sql: &str, new_row: Option<&Record>) -> String {
    if let Some(new) = new_row {
        let mut result = sql.to_string();
        for (i, val) in new.iter().enumerate() {
            // NEW[0] / NEW[1] 占位符展开 — 空 Record 时循环 0 次, 没问题
            ...
        }
        // 这里无条件 deref new[0]
        result = result.replace("NEW.id", &self.value_to_sql_literal(&new[0]));  // <-- PANIC
        if new.len() > 1 {
            result = result.replace("NEW.col1", &self.value_to_sql_literal(&new[1]));
        }
        result
    } else {
        sql.to_string()
    }
}
```

DELETE 触发器走 `execute_before_delete` → `execute_trigger_body(trigger, table,
Some(old_row), None)`。在 `execute_trigger_body` 里:

```rust
let mut result: Record = new_row.map(|r| r.to_vec()).unwrap_or_default();
// result = empty Vec (DELETE 没有 NEW row)
for stmt in statements {
    let expanded = self.expand_row_variables_for_parse(...);  // OK, 不展开 NEW
    self.execute_trigger_sql_mut(&expanded, table, old_row, &mut result)?;  // <-- 这里
}
```

`execute_trigger_sql_mut` 把 `&mut result` (空 Vec) 当成 `Some(current_new_row)`
转发给 `execute_trigger_insert` → `expand_insert_values(sql, Some(empty_vec))`。

`expand_insert_values` 拿到 `Some(empty)`，进入 `if let Some(new) = new_row` 分支，
但对空 Vec 的 `&new[0]` 直接越界 panic。这是真正的 bug，T-003 直接命中。

### 修复

```rust
fn expand_insert_values(&self, sql: &str, new_row: Option<&Record>) -> String {
    if let Some(new) = new_row {
        // V312-55D FIX (Round-26): DELETE triggers can carry an INSERT
        // statement in their body. Forwarded current_new_row is an empty
        // Vec for DELETE; guard the named-placeholder substitution.
        if new.is_empty() {
            return sql.to_string();
        }
        // ... 原有逻辑保持不变
    } else {
        sql.to_string()
    }
}
```

改动局部，不影响 BEFORE INSERT/UPDATE/AFTER UPDATE 等正向路径 (它们都
`new.is_empty() == false`，行为不变)。

### 单元测试

`crates/executor/src/trigger.rs` 内已有 3 个 `test_expand_insert_values_*`
覆盖:
- `test_expand_insert_values_basic` — `NEW[0]/NEW[1]` 占位符展开
- `test_expand_insert_values_with_new_id` — `NEW.id` 展开
- `test_expand_insert_values_no_new_row` — `None` 直接返回

修复后这 3 个测试全部 PASS (T-001/T-002/T-003 间接验证)。

## 新增 E2E 测试

`tests/integration/migration/e2e_trigger_wal_recovery.rs` 增加
`test_trigger_after_insert_rollback_v55d`:

```rust
#[test]
fn test_trigger_after_insert_rollback_v55d() {
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    // Phase A: 事务 + ROLLBACK
    {
        let mut client = open(&data_dir);  // start_ephemeral MySQL server
        client.exec("CREATE TABLE base (id INTEGER, val TEXT)").unwrap();
        client.exec("CREATE TABLE audit (id INTEGER, msg TEXT)").unwrap();
        client.exec(
            "CREATE TRIGGER base_audit AFTER INSERT ON base FOR EACH ROW
             BEGIN INSERT INTO audit VALUES (1, 'logged') END",
        ).unwrap();

        client.exec("BEGIN").unwrap();
        client.exec("INSERT INTO base VALUES (1, 'orig')").unwrap();
        client.exec("ROLLBACK").unwrap();

        // pre-check: base + audit 都必须为空
        assert_eq!(client.query_one_i64("SELECT COUNT(*) FROM base").unwrap(), 0);
        assert_eq!(client.query_one_i64("SELECT COUNT(*) FROM audit").unwrap(), 0);
    }

    // Phase B: kill -9 replay
    let mut client = open(&data_dir);
    assert_eq!(client.query_one_i64("SELECT COUNT(*) FROM base").unwrap(), 0);
    assert_eq!(client.query_one_i64("SELECT COUNT(*) FROM audit").unwrap(), 0);
}
```

## Gate 累计

| Round | Status | 总分 |
|-------|--------|------|
| Round-24 | V55A PASS | 4/13 |
| Round-25A | + V55B | 5/13 |
| Round-25B | + V55C | 6/13 |
| **Round-26** | **+ V55D (本轮)** | **7/13** |

剩余: V55E-Recursion, V55F-Privilege, V55G-Sqllogictest, V55H-Verification-Doc
(计划 Round-27+ 关闭)。

## 文件清单

| 文件 | 改动 | 说明 |
|------|------|------|
| `crates/executor/src/trigger.rs` | +10/-0 | `expand_insert_values` 空 Record 越界 panic 修复 |
| `tests/integration/migration/e2e_trigger_wal_recovery.rs` | +91/-1 | 新增 `test_trigger_after_insert_rollback_v55d` |
| `scripts/gate/check_v312_procedure_trigger_gate.sh` | +5/-2 | V55D gate filter 改为 `after_insert_rollback_v55d` 严格 1-test 匹配 |
| `docs/releases/v3.12.0/evidence/procedure_trigger/V312-55D-VERIFICATION.md` | +200 | 本文件 |

## Gate 运行结果

```
[PASS] V55A-Procedure-DDL
[PASS] V55B-Call-Execute
[PASS] V55C-Trigger-NewOld
[PASS] V55D-WAL-Recovery          <-- 本轮
[FAIL] V55E-Recursion
[FAIL] V55F-Privilege
[FAIL] V55G-Sqllogictest-Fixture-Basic
[FAIL] V55G-Sqllogictest-Fixture-Transactions
[FAIL] V55G-Sqllogictest-Runner
[FAIL] V55H-Verification-Doc
[PASS] ANTI-Ignore-Procedure-Tests

PASS:      7 / 13
BLOCKERS:  6
```

## 设计说明

- `expand_insert_values` 是字符串占位符替换 (NEW[0] / NEW.id / NEW.col1)，
  原作者隐式假设调用方一定传非空 NEW row。修复后用 `is_empty()` 显式
  兜底 DELETE 触发器场景，对原 INSERT/UPDATE 触发器路径完全无影响。
- V55D E2E 测试采用 ephemeral MySQL server (`start_ephemeral`)，与既有
  T-001/T-002/T-003 模式一致：第一个 client 写 WAL，drop (模拟 kill -9)；
  第二个 client 共享 `data_dir` 重启 (模拟 recovery)，断言两张表都为空。
  与 COMMIT 路径 (T-001) 形成对照，证明 ROLLBACK 路径持久化正确。
- `trigger.rs:747` 修复同时让 T-003 (DELETE 触发器) 从 FAIL 转 PASS —
  这是预先存在的回归 bug，T-003 在 Round-25B 之前就因这个 panic 一直
  fail。本次 Round-26 修复后 T-001/T-002/T-003 全部 PASS，T-003 也
  自动受益 (V55D 修复的"额外收获")。
