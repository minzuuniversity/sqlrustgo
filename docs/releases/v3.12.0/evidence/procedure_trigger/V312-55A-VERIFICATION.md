# V312-55A — Procedure DDL 生命周期 (Issue #4238) Verification

**provenance:** branch=fix/v312-4019-3943-evidence-refresh, generated_at=2026-08-15T00:50:00+08:00, source_repo=openclaw/sqlrustgo, gate_policy=V312-55-Procedure-Trigger-Gate, evidence_log_path=docs/releases/v3.12.0/evidence/procedure_trigger/V312-55A-VERIFICATION.md

| Field                | Value                                                              |
|----------------------|--------------------------------------------------------------------|
| Issue                | #4238 (V312-55A)                                                   |
| Parent master        | #4237 (V312-55 整改总控)                                            |
| Branch               | fix/v312-4019-3943-evidence-refresh                                  |
| Gate                 | scripts/gate/check_v312_procedure_trigger_gate.sh (V55A check)     |
| Status               | **V55A-Procedure-DDL = PASS** (Round-24A scope complete)            |
| Scope                | CREATE/DROP/SHOW PROCEDURE + IF EXISTS + OR REPLACE + 大小写不敏感   |

## 1. Scope

V312-55A 是 V312-55 整改计划中第一项 (Issue #4238),目标是让 v3.12 的存储过程 DDL 生命周期
(MySQL 风格) 全部闭环:

| DDL 语句                       | 实现要求                                | 状态 |
|--------------------------------|----------------------------------------|------|
| `CREATE PROCEDURE`             | 主体解析、参数解析、入 catalog           | ✅   |
| `CREATE OR REPLACE PROCEDURE`  | 同名过程覆盖语义                        | ✅   |
| `DROP PROCEDURE`               | 删 catalog 记录,缺失报错                | ✅   |
| `DROP PROCEDURE IF EXISTS`     | 缺失 no-op (不报错)                     | ✅   |
| `SHOW PROCEDURE STATUS`        | 列出所有过程 (Db/Name/Type/...)         | ✅   |
| `SHOW PROCEDURE STATUS LIKE 'p'`| LIKE 模式过滤 (`%`/`_` 通配符)         | ✅   |
| 大小写不敏感查找                | MySQL 风格 (小写归一,保留原大小写)      | ✅   |

## 2. Files changed

| File                                                 | LOC +/–  | Description                                         |
|------------------------------------------------------|----------|-----------------------------------------------------|
| crates/parser/src/parser.rs                          | +110/-7  | `Statement::DropProcedure` + OR REPLACE 字段 + SHOW |
| crates/catalog/src/catalog.rs                        | +48/-17  | proc_key 归一化 + add_or_replace_stored_procedure    |
| crates/catalog/src/error.rs                          | +12/-0   | `ProcedureNotFound` / `DuplicateProcedure`          |
| crates/catalog/src/stored_proc.rs                    | +63/-0   | procedure_ddl_lifecycle 单元测试                     |
| src/execution_engine.rs                              | +42/-3   | `execute_drop_procedure` + OR REPLACE 路由            |
| src/engine_ddl.rs                                    | +120/-4  | `execute_show_procedure_status` + like_match helper  |
| crates/mysql-server/src/lib.rs                       | +2/-0    | `statement_kind` 加 `DropProcedure` 分支              |
| crates/executor/tests/stored_proc_test.rs            | +31/-0   | procedure_ddl_create_drop_roundtrip (gate arm 2)     |
| tests/integration/transaction/stored_proc_catalog_test.rs | +148/-0 | 4 个 procedure_ddl_* 集成测试                        |
| **Total**                                            | **+584/-23** |                                                    |

## 3. Tests

### 3.1 Catalog unit (gate arm 1)

```text
cargo test -p sqlrustgo-catalog --lib procedure_ddl
running 1 test
test stored_proc::tests::procedure_ddl_lifecycle ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 183 filtered out
```

`procedure_ddl_lifecycle` covers 7 invariants in one round-trip:
add → 2× duplicate detection (case-insensitive) → 3× lookup variants → OR REPLACE overwrite → OR REPLACE fresh insert → remove → missing-remove returns None.

### 3.2 Executor integration (gate arm 2)

```text
cargo test -p sqlrustgo-executor --test stored_proc_test procedure_ddl
running 1 test
test procedure_ddl_create_drop_roundtrip ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 38 filtered out
```

### 3.3 Full integration

```text
cargo test --test stored_proc_catalog_test procedure_ddl
running 4 tests
test procedure_ddl_drop_if_exists_semantics ... ok
test procedure_ddl_create_drop_show_lifecycle ... ok
test procedure_ddl_show_status_like_filter ... ok
test procedure_ddl_or_replace_creates_when_absent ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 18 filtered out
```

### 3.4 Cumulative V55A tests passing

| Layer      | Count | Tests                                                                             |
|------------|-------|-----------------------------------------------------------------------------------|
| catalog    | 1/1   | procedure_ddl_lifecycle                                                           |
| executor   | 1/1   | procedure_ddl_create_drop_roundtrip                                               |
| integration| 4/4   | procedure_ddl_create_drop_show_lifecycle, _drop_if_exists_semantics, _or_replace_creates_when_absent, _show_status_like_filter |
| **Total**  | **6/6** |                                                                                  |

## 4. Gate V55A evidence

```text
$ bash scripts/gate/check_v312_procedure_trigger_gate.sh | grep V55A
  [PASS] V55A-Procedure-DDL
```

Gate V55A-Procedure-DDL 的两臂 (catalog arm + executor arm) 至少有一个输出 "1 passed" 即 PASS。

## 5. Key design notes

### 5.1 Case-insensitive lookup, original casing preserved

MySQL 行为: `CREATE PROCEDURE MyProc` 后 `CALL myproc` 能找到,但 `SHOW PROCEDURE STATUS`
仍显示 `MyProc`。`Catalog::proc_key` 用 `to_lowercase()` 归一化 hash key,但 stored procedure
本身的 `name` 字段保留原始大小写。

```rust
fn proc_key(name: &str) -> String {
    name.to_lowercase()
}

pub fn add_stored_procedure(&mut self, procedure: StoredProcedure) -> CatalogResult<()> {
    let key = Self::proc_key(&procedure.name);
    if self.stored_procedures.contains_key(&key) {
        return Err(CatalogError::DuplicateProcedure(procedure.name.clone()));
    }
    self.stored_procedures.insert(key, procedure);
    Ok(())
}
```

### 5.2 OR REPLACE 语义

`add_or_replace_stored_procedure` 直接 `insert` (覆盖),与 `add_stored_procedure` 不同 — 后者
预先 `contains_key` 检查以抛 `DuplicateProcedure`。

### 5.3 Parser: shared OR REPLACE consumption

`parse_create` 在进入 `parse_create_procedure` 之前先消费 `OR REPLACE`,然后由 `parse_create`
的 Procedure 分支把 flag 写回 stmt — 与 Table 路径一致。这避免 `parse_create_procedure`
内部再次消费 `Or`/`Replace` 时发生冲突 (曾导致 "Failed to create procedure" 而非 "or replace")。

### 5.4 SHOW PROCEDURE STATUS LIKE

`engine_ddl::like_match` 实现 MySQL 风格的 `_` / `%` 通配符匹配,不依赖 regex,
确保与 catalog 中 proc.name 字段 (原始大小写) 协作一致。

## 6. V55A-Verification = done

Round-24 V312-55A 整改闭环,V55B~55H 仍 FAIL (按 V312-55 整改计划后续 Round 关闭)。

| Gate check            | Status | Owner round |
|-----------------------|--------|-------------|
| V55A-Procedure-DDL    | PASS   | Round-24 (本提交) |
| V55B-Call-Execute     | FAIL   | Round-25 (后续)  |
| V55C-Trigger-NewOld   | FAIL   | Round-25 (后续)  |
| V55D-WAL-Recovery     | FAIL   | Round-26 (后续)  |
| V55E-Recursion        | FAIL   | Round-26 (后续)  |
| V55F-Privilege        | FAIL   | Round-27 (后续)  |
| V55G-Sqllogictest     | FAIL   | Round-27 (后续)  |
| V55H-Verification-Doc | FAIL   | Round-27 收尾 (V312-55 全 DONE 时建) |

V55A 是唯一本轮关闭的检查项,V55B~55H 的失败**不属于本 PR 范围**,将由后续 Round-25/26/27
按 V312-55_PROCEDURE_TRIGGER_REMEDIATION_PLAN.md 推进。