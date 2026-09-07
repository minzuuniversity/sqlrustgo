## Context

Issue #4809 — SQLite-style `INDEXED BY idx_name` hint 被 silently ignored。
PR #4788 (v312-90) 已经把 hint 解析到 `TableReference` 上,executor
只需消费这个字段。

`TableReference` 现有 fields(v312-90 PR #4788 已经扩展):
```rust
pub struct TableReference {
    pub name: String,
    pub alias: Option<String>,
    pub indexed_by: Option<String>,    // SQLite INDEXED BY
    pub not_indexed: Option<()>,        // SQLite NOT INDEXED
}
```

(具体字段名以 codebase 实际为准;核心是两个 Option 字段。)

## Approach

### A1. Table resolution 路径加 hook

在 `src/executor/select_from.rs` 或 `src/engine_select.rs` 的
`resolve_table_reference` 路径中:

```rust
fn resolve_table_reference(table_ref: &TableReference) -> ResolvedTable {
    let table = storage.get_table(&table_ref.name)?;
    
    // INDEXED BY check
    if let Some(idx_name) = &table_ref.indexed_by {
        let index = storage.get_index(idx_name)
            .ok_or_else(|| SqlError::IndexNotFound(idx_name.clone()))?;
        if index.table_name != table.name {
            return Err(SqlError::IndexNotOwnedByTable {
                index: idx_name.clone(),
                index_table: index.table_name.clone(),
                query_table: table.name.clone(),
            });
        }
        Ok(ResolvedTable {
            table,
            scan_mode: ScanMode::Index(index.id),
        })
    } else if table_ref.not_indexed.is_some() {
        Ok(ResolvedTable {
            table,
            scan_mode: ScanMode::Sequential,
        })
    } else {
        Ok(ResolvedTable {
            table,
            scan_mode: ScanMode::Auto,  // planner choice
        })
    }
}
```

### A2. Scan execution dispatch

```rust
match scan_mode {
    ScanMode::Index(idx_id) => {
        let keys = storage.index_keys(idx_id, &query_predicate)?;
        for key in keys {
            let row = storage.get_row_by_key(table_id, &key)?;
            if row_matches_filter(&row, &where_clause)? {
                output_rows.push(row);
            }
        }
    }
    ScanMode::Sequential => {
        for row in storage.scan_all(table_id) {
            if row_matches_filter(&row, &where_clause)? {
                output_rows.push(row);
            }
        }
    }
    ScanMode::Auto => {
        // existing planner path
    }
}
```

### A3. 错误类型

新增:
- `SqlError::IndexNotFound(String)` — 索引不存在。
- `SqlError::IndexNotOwnedByTable { index: String, index_table: String, query_table: String }`。

### A4. NOT INDEXED

如果 `not_indexed = Some(())`,强制 `ScanMode::Sequential` 跳过所有
索引。

## Files Changed

| File | Lines | Purpose |
|------|-------|---------|
| `src/executor/scan.rs` (或 select.rs) | +60 | INDEXED BY / NOT INDEXED dispatch |
| `src/storage/metadata.rs` | +30 | IndexBy/NotIndexed 字段 + ownership check |
| `src/error.rs` | +15 | IndexNotFound / IndexNotOwnedByTable |
| `src/executor/from.rs` | +40 | table resolution 加 hint 路径 |
| `tests/integration/sql/p3_indexed_by_4809_test.rs` | +180 | 7 tests |

Total: ~325 lines, ~5 files touched.

## Verification

| Test | Expected |
|------|----------|
| `cargo build --all-features` | clean |
| `p3_indexed_by_4809_test` | 7/7 PASS |
| `indexed_by_*` (v312-90 相关) | no regression |
| `parser_e2e_test` | 249/249 PASS |
| `cte_materialization_test` | 9/9 PASS |

## Non-Goals

- MySQL FORCE INDEX / IGNORE INDEX
- ANALYZE statistics-driven hint selection
- Cross-schema INDEXED BY

## Difficulty Tier

**🟡 MEDIUM** — 改动小但需要 plumbing 多个 layer:parser AST → catalog →
executor dispatch → scan mode。已有 indexed scan 基础(PR #4789, PR
#4789-amend),需要的是 wire-up,不是算法。