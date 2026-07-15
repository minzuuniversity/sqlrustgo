## Context

PERF-5 was discovered during v3.10.0 168h SOAK (Issue #3434, 2026-07-15 06:32 UTC).
Original report claimed "Lost connection" errors during concurrent INSERTs. After thorough
investigation on 2026-07-15, the real root cause was identified:

1. **PRIMARY**: v3.10.0 parser does NOT support `INSERT IGNORE` syntax
   - The lexer keyword map (lexer.rs) and Token enum (token.rs) lacked `Token::Ignore`
   - `INSERT IGNORE INTO ...` returned `ERROR 1064: Parse error: Expected Into, got Identifier("IGNORE")`
   - All 8 OLTP threads failed at INSERT step with parse error

2. **SECONDARY**: 8 threads used overlapping ID ranges (RANDOM % 10000 + 1000000)
   - With 10000 unique IDs and 8 threads × multiple ops, PK collision was inevitable
   - Even if INSERT IGNORE worked, the "10000 IDs" range would still cause conflicts

3. **NEGATIVE FINDING**: No actual "Lost connection" errors were found
   - The 0% INSERT rate was from parse errors and duplicate key errors
   - Server was stable and handling connections correctly
   - The original report confused the symptom (parse errors) with network errors

## Goals / Non-Goals

**Goals:**
- Add `INSERT IGNORE` support to v3.10.0 parser
- Update executor to handle `is_ignore` flag
- Add regression test for concurrent INSERTs
- Improve OLTP orchestrator to use thread-unique ID ranges

**Non-Goals:**
- No changes to wire protocol
- No changes to storage layer
- No changes to non-INSERT DML

## Decisions

### 1. Add `Token::Ignore` to Token enum

Add `Ignore` variant to the Token enum in `crates/parser/src/token.rs` and add its
Display impl, lexer keyword map, and test assertion. Position: alphabetically
near `Insert` and `Index` keywords.

### 2. Add lexer keyword mapping

Add `"IGNORE" => Token::Ignore` to the lexer keyword map in
`crates/parser/src/lexer.rs:277` so the lexer recognizes the keyword.

### 3. Add `is_ignore` field to `InsertStatement`

Add `pub is_ignore: bool` to `InsertStatement` struct in
`crates/parser/src/parser.rs:507`. This flag is set when the parser sees
`INSERT IGNORE` and propagates to the executor.

### 4. Update `parse_insert` to consume `Token::Ignore`

In `parse_insert`, after the optional REPLACE handling but before `Into`:
```rust
let is_ignore = if !is_replace && matches!(self.current(), Some(Token::Ignore)) {
    self.next(); // consume Ignore
    true
} else {
    false
};
```

### 5. Update executor to skip duplicates when is_ignore

In `src/engine_dml.rs`, the existing duplicate check returns an error. When
`is_ignore` is true, the record should be silently skipped (not error):
```rust
if matched && insert.on_duplicate_key_update.is_none() {
    if insert.is_ignore {
        odku_handled_indices.insert(new_idx); // Mark as "handled" to skip
        continue;
    }
    return Err(...);
}
```

### 6. Improve OLTP orchestrator for thread-unique IDs

In `orchestrator_v2.sh`, the OLTP workload script uses thread ID as offset:
```bash
THREAD_ID=$(basename "$0" | sed 's/oltp_workload_//; s/.sh//')
THREAD_OFFSET=$((THREAD_ID * 10000000))
ID=$((RANDOM % 1000000 + THREAD_OFFSET))
```

Plus per-thread script copy (so basename differs per thread):
```bash
for i in 1 2 3 4 5 6 7 8; do
    cp "$RESULTS_DIR/oltp_workload.sh" "$RESULTS_DIR/oltp_workload_$i.sh"
done
```

### 7. Add regression test suite

6 new tests in `tests/integration/stress/concurrent_insert_test.rs`:
- `test_insert_ignore_parses` - basic INSERT IGNORE syntax works
- `test_insert_ignore_concurrent` - 4 concurrent processes with INSERT IGNORE
- `test_insert_ignore_concurrent_10x` - 10 concurrent processes
- `test_on_duplicate_key_update_works` - existing ODKU still works
- `test_replace_into_works` - existing REPLACE still works
- `test_plain_insert_duplicate_errors` - plain INSERT still errors on duplicate

## Risks

| Risk | Mitigation |
|------|------------|
| Backward compat with existing INSERT syntax | No change to default behavior; only INSERT IGNORE path is new |
| Performance of is_ignore check | O(1) per record, negligible overhead |
| Test stability on concurrent runs | Use separate process per test (not threads) for clean state |

## Implementation Results

- **Build**: `cargo build --release` - 0 errors
- **Tests**: 6/6 concurrent_insert_test PASS
- **Test duration**: <1s total
- **Other tests**: alter_table_test 9/9 PASS, storage 35/35 PASS
