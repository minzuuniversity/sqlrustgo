## Context

Issue #4675: `POSITION(substr IN str)` and `LOCATE(substr, str[, pos])` return NULL. The `eval_fn` function in `crates/executor/src/expr/mod.rs` defaults to `Value::Null` for unknown function names. Both are standard SQL / MySQL string functions that need position calculation.

## Decisions

### D1. 1-based indexing, 0 = not found

Both functions return 1-based position (matching SQL standard and MySQL). `POSITION` returns 0 if not found (SQL standard). `LOCATE` also returns 0 if not found (MySQL compatibility). This is consistent with SQLite's `instr()` which returns 0 for not found.

### D2. Case-sensitive matching

Both functions use case-sensitive comparison. This matches SQL standard and MySQL behavior.

### D3. Byte indexing (not UTF-8 codepoint)

SQLite's `instr()` and MySQL's `LOCATE` both use byte-based indexing. We use Rust's `find()` on the raw `&str` bytes which operates on byte indices. For ASCII this is codepoint-indexed; for multi-byte UTF-8, byte index may differ from codepoint index. This matches SQLite/MySQL behavior.

### D4. LOCATE optional third argument

MySQL `LOCATE(substr, str, pos)` starts searching from byte position `pos` (1-based). We implement this by slicing `str` from `pos-1` onwards, then adjusting the result back to 1-based absolute position.

## Risks

- Byte vs codepoint: matches SQLite/MySQL actual behavior. Acceptable.
- No regex support: not needed for v3.12 scope.
