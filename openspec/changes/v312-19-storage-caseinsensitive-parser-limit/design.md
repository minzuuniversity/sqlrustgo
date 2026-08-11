# V312-19 #3972 Fix — Design

## Storage case-insensitive fix

### Current state (excerpt from `crates/storage/src/engine.rs`)

```rust
fn get_table_info(&self, table: &str) -> SqlResult<TableInfo> {
    self.table_infos
        .get(table)  // exact match
        .cloned()
        .ok_or_else(|| SqlError::ExecutionError(format!("Table not found: {}", table)))
}

fn has_table(&self, table: &str) -> bool {
    self.table_infos.contains_key(table)  // exact match
}
```

### Fix: case-insensitive lookup

```rust
fn get_table_info(&self, table: &str) -> SqlResult<TableInfo> {
    self.table_infos
        .get(&table.to_lowercase())  // case-insensitive
        .cloned()
        .ok_or_else(|| SqlError::ExecutionError(format!("Table not found: {}", table)))
}

fn has_table(&self, table: &str) -> bool {
    self.table_infos.contains_key(&table.to_lowercase())
}
```

Apply to all table-name key accesses: `rename_table`, `drop_column` (if it uses table key), `list_tables`, and the ALTER TABLE handling.

For `list_tables()` return the canonical (lowercased) name OR original. The fix is internal — what we store and look up is consistent. The return can be the stored (lowercased) name.

## Parser LIMIT expression fix

### Current state (excerpt from `crates/parser/src/parser.rs` lines 4735-4763)

```rust
let limit = if matches!(self.current(), Some(Token::Limit)) {
    self.next();
    match self.current() {
        Some(Token::NumberLiteral(n)) => { /* parse literal */ }
        Some(Token::identifier(ref s)) => { /* parse @var */ }
        _ => None,
    }
} else {
    None
};
```

### Fix: accept expression

```rust
let limit = if matches!(self.current(), Some(Token::Limit)) {
    self.next();
    match self.current() {
        Some(Token::NumberLiteral(n)) => {
            let val = /* existing logic */;
            self.next();
            Some(val)
        }
        Some(Token::identifier(ref s)) if s.starts_with('@') => {
            /* existing @var logic */
        }
        _ => {
            // NEW: parse as arithmetic expression, constant-fold
            let expr = self.parse_expression()?;
            let val = constant_fold_u64(&expr)
                .ok_or_else(|| format!("Invalid LIMIT: must be constant expression"))?;
            Some(val)
        }
    }
} else {
    None
};
```

`constant_fold_u64` walks the AST and returns `Some(n)` if all leaves are integer literals and the operators are arithmetic (`+ - * / %`). Returns `None` otherwise (e.g., column reference, function call).

Apply same fix to OFFSET.

## Verification

```bash
$ cargo test -p sqlrustgo-storage --test memory_storage
# All existing tests still pass

$ cargo test -p sqlrustgo-parser --test parser_test
# All existing tests still pass

$ cargo test -p sqlrustgo_sqllogictest --test sqllogictest_runner -- case_insensitive_alter
# Previously: FAIL "Table not found: MyTable"
# After fix: PASS

$ cargo test -p sqlrustgo_sqllogictest --test sqllogictest_runner -- test_limit
# Previously: FAIL "expression not supported"
# After fix: PASS (LIMIT 2-1 → 1, returns 1 row)
```
