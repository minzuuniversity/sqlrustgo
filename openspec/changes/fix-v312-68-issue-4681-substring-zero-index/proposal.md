## Why

`sqlrustgo-cli sqlite` (HEAD `a359e1d80`, develop/v3.12.0) returns the entire string for `SUBSTRING('hello', 0)` instead of an empty string. Issue #4685 (and a related report in #4681) note that `SUBSTRING`/`SUBSTR` should follow 1-based indexing per the SQL standard — a 0 index refers to "before position 1" and produces an empty substring.

Repro:
```
$ printf "SELECT SUBSTRING('hello', 0), SUBSTRING('hello', 1), SUBSTRING('hello', 5);" \
    | sqlrustgo-cli sqlite --batch --mode csv /tmp/db
hello,hello,o
```

Expected:
```
,hello,o
```

Root cause: in `crates/executor/src/expr/mod.rs:1748`, the `start` argument is converted to a 0-based byte index via `(*i).saturating_sub(1).max(0) as usize`. For `i=0`, `saturating_sub(1)` is 0 (since 0-1 saturates to 0), so the 0-based index is also 0 — the function then slices from index 0 to end, returning the whole string. The `i=1` case is also wrong: it should slice from byte 0 to end (i.e., the whole string), which happens to be correct, but the conversion logic is broken in spirit.

## Fix

Treat `i == 0` as "before the start of the string" — return an empty string. For `i >= 1`, compute `i - 1` as the 0-based start index and slice from there. Clamp to `text.len()` if the start is past the end (also returns empty).

The fix lives entirely in the `SUBSTR` / `SUBSTRING` arm of `eval_fn` (`crates/executor/src/expr/mod.rs:1744`). No AST change, no new public API, no storage/WAL change.

## Capabilities

### New Capabilities

- `executor-substring-zero`: `sqlrustgo` MUST return an empty string for `SUBSTRING(s, 0)` (and `SUBSTR(s, 0)`) per the SQL/SQLite/PostgreSQL convention. The function's 1-based start position is unchanged for all other inputs.

### Modified Capabilities

- None.
