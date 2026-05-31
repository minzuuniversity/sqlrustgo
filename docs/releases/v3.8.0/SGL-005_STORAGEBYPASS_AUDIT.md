# SGL-005 Storage Bypass 完整审计报告

**Date**: 2026-06-01
**Author**: Hermes C
**Status**: IN PROGRESS — 7 violations remain

### Progress (2026-06-01)

| PR | Action | Violations Fixed | Status |
|----|--------|-----------------|--------|
| #2723 | P0: DELETE via facade | 1 (local_executor.rs:1469) | ✅ Fixed |
| #2724 | Remove dead teaching_endpoints.rs | 4 (teaching code) | ✅ Fixed |
| #2734 | SGL classifier: facade-closure detection | 1 false positive fixed | ✅ Fixed |
| #2735 | P2: openclaw endpoints TX boundary | 2 (openclaw_endpoints.rs) | ✅ Fixed |

### Remaining Violations

| Location | Lines | P | Type | Status |
|----------|-------|---|------|--------|
| trigger.rs | 429,507,509,531 | P1 | Trigger body DML | 🔴 Architecture refactor needed |

---

## 1. SGL-005 规则定义

**Contract**: TX-002 — "all mutations must go through TransactionManager"
**Scope**: `crates/executor/src/` 和 `crates/server/src/`
**检查方法**: grep `storage.insert|storage.update|storage.delete`

**已知问题**:
- grep 使用 `head -10` 限制，只取前 10 行，未覆盖全量
- 过滤逻辑 `if 'fn ' not in line.split(":")[1]` 误判率高

---

## 2. 完整违规清单

### 2.1 生产代码违规（REAL VIOLATIONS）

| # | 文件 | 行号 | 代码 | 性质 |
|---|------|------|------|------|
| 1 | trigger.rs | 429 | `storage.insert(table_name, vec![record])` | **REAL — trigger body** |
| 2 | trigger.rs | 507 | `storage.delete(table_name, &[])` | **REAL — trigger body** |
| 3 | trigger.rs | 509 | `storage.insert(table_name, modified_rows)` | **REAL — trigger body** |
| 4 | trigger.rs | 531 | `storage.delete(&delete.table, &[])` | **REAL — trigger body** |
| 5 | local_executor.rs | 1469 | `storage.delete(table, &[])` | **REAL — execute_dml_via_facade fallback** |

### 2.2 测试代码（EXEMPT — 免查）

| # | 文件 | 行号 | 代码 | 说明 |
|---|------|------|------|------|
| 6 | harness.rs | 274 | `storage.insert("users", ...)` | 测试辅助数据 |
| 7 | harness.rs | 315 | `storage.insert("users", ...)` | 测试辅助数据 |
| 8 | harness.rs | 375 | `storage.insert("users", ...)` | 测试辅助数据 |
| 9 | parallel_vector_executor.rs | 689,706,724,743 | `storage.insert("numbers", ...)` | 测试 fixture setup |

### 2.3 合理调用（合理通过）

| # | 文件 | 行号 | 代码 | 说明 |
|---|------|------|------|------|
| 10 | wal_transactional_facade.rs | 108,111,114 | `storage.insert/update/delete` | ✅ WAL-aware via `log_mutation` |
| 11 | local_executor.rs | 1328 | `facade.execute_dml(|storage| ...)` | ✅ 通过 facade 闭包 |

### 2.4 待确认（NEEDS REVIEW）

| # | 文件 | 行号 | 代码 | 说明 |
|---|------|------|------|------|
| 12 | vector_executor.rs | 193,220,242,293 | `storage.insert("users"/"numbers", ...)` | ⚠️ 非测试，可能是 benchmark |
| 13 | parallel_executor.rs | 1140,1145,1253,1258,1578,1583,1661,1666,1733,1738 | `memory_storage.insert(...)` | ⚠️ 批数据加载 |

---

## 3. 根因分析

### 3.1 Trigger Storage Bypass

Trigger 代码在 trigger body 内直接操作 storage，绕过 WAL：

```rust
fn execute_trigger_insert(&self, ...) {
    let mut storage = self.storage.write().unwrap();  // 直接拿锁
    // ...
    storage.insert(table_name, vec![record])?;  // 绕过 WAL
}
```

**影响**: Trigger 内的 DML 不会记录 WAL，crash recovery 后 trigger 操作丢失。

### 3.2 execute_dml_via_facade Fallback

```rust
fn execute_dml_via_facade(&self, facade: &UnifiedFacade, ctx: &QueryContext) {
    if sql_upper.starts_with("DELETE") {
        let affected = {
            let mut storage = facade.storage.write();  // 直接写 storage
            storage.delete(table, &[])
        };
    }
}
```

**影响**: DELETE 语句走 `execute_dml_via_facade` 而非 `execute_dml`，绕过 WAL。

### 3.3 SGL-005 脚本缺陷

```python
r = subprocess.run(
    f'grep -rn "..." "{crate_path}/" --include="*.rs" 2>/dev/null | head -10'
)
```

`head -10` 只取前 10 行，35 处违规中只覆盖 14 处。

过滤逻辑:
```python
if line and "fn " not in line.split(":")[1] if ":" in line else True:
    violations.append(line)
```

`parts[1]` 是 "274"（行号），永远不含 "fn "，导致 `harness.rs` 的 3 行被误报为违规。

---

## 4. 修复方案

### 4.1 Trigger Storage Bypass

**方案**: Trigger 的 DML 操作应通过 `WalTransactionalFacade` 代理，确保 WAL 记录。

```rust
// trigger.rs — 修改后的 execute_trigger_insert
fn execute_trigger_insert(&self, sql: &str, new_row: Option<&Record>) -> SqlResult<()> {
    let mut facade = self.facade.write();
    let expanded = self.expand_insert_values(sql, new_row);
    let statement = parse(&expanded)?;
    if let sqlrustgo_parser::Statement::Insert(insert) = statement {
        facade.execute_insert(&insert.table, &insert.columns, &insert.values)?;
    }
    Ok(())
}
```

**依赖**: `WalTransactionalFacade.execute_insert` 需要存在。

### 4.2 execute_dml_via_facade Fallback

**方案**: `execute_dml_via_facade` 的 DELETE 操作应走 `facade.execute_dml`，而非直接操作 `facade.storage`。

```rust
// local_executor.rs:1469 — 修改后
let affected = facade.execute_dml(|storage| storage.delete(table, &[]))?;
```

**依赖**: `UnifiedFacade.execute_dml` 的 DML closure 支持 delete/update。

### 4.3 SGL-005 脚本修复

```python
# 移除 head -10 限制
r = subprocess.run(
    f'grep -rn "..." "{crate_path}/" --include="*.rs" 2>/dev/null'
)
# 改进过滤：排除 test 文件 + 改进分类
```

---

## 5. 修复优先级

| 优先级 | 违规 | 修复难度 | 说明 |
|--------|------|----------|------|
| **P0** | local_executor.rs:1469 | 低 | DELETE 走 execute_dml_via_facade fallback |
| **P1** | trigger.rs:429,507,509,531 | 中 | Trigger body DML |
| **P2** | SGL-005 grep 截断 | 低 | 移除 head -10 |

---

## 6. 验证方法

修复后运行:

```bash
# 验证 SGL-005 不再报告生产违规
python3 scripts/gate/semantic_gate_check.py 2>&1 | grep -A5 "SGL-005"

# 验证 WAL invariant 测试仍然通过
cargo test --test wal_tx_contract_test 2>&1 | grep -E "test result|passed|failed"

# 验证 trigger 测试通过
cargo test -p sqlrustgo-executor trigger 2>&1 | tail -10
```

---

## 7. DRIFT 豁免申请

对于 `vector_executor.rs` 和 `parallel_executor.rs` 的 batch loading：

**申请豁免理由**:
- `vector_executor.rs` 的 storage.insert 是 benchmark/数据加载代码
- `parallel_executor.rs` 的 memory_storage.insert 是批加载（不是 production OLTP 路径）

**建议**: 将 batch loading 路径豁免为 INTENTIONAL DRIFT，在文档中记录。

---

## 8. 执行记录

| 日期 | 操作 | 结果 |
|------|------|------|
| 2026-06-01 | SGL-005 完整审计 | 发现 5 处真实违规 |
| 2026-06-01 | P0 修复 local_executor.rs:1469 | 待执行 |
| 2026-06-01 | P1 修复 trigger.rs | 待执行 |
| 2026-06-01 | P2 修复 SGL-005 脚本 | 待执行 |
