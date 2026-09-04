## Context

Issue #4670: TRUNCATE/TRUNC, HEX, MD5 return NULL. All are missing arms in `eval_fn`.

## Decisions

### D1. TRUNCATE uses f64 truncation

`TRUNCATE(x, d)` → compute `x * 10^d`, truncate toward zero, divide back. For `d < 0`, divide by `10^|d|` and truncate.

### D2. HEX uses standard library

`HEX(n)` for Integer → `format!("{:X}", n)`. For now, only Integer input.

### D3. MD5 uses md5 crate

If `md5` crate is already a dependency, use it. Otherwise use a simple pure-Rust md5 implementation.
