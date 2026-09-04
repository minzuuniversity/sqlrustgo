## Why

`sqlrustgo-cli sqlite` (HEAD, develop/v3.12.0) returns NULL for `TRUNCATE`/`TRUNC` (numeric truncation), `HEX` (binary-to-hex), and `MD5`/`SHA1`/`SHA2` (hash functions) because `eval_fn` has no arms for these and defaults to `Value::Null`. Issue #4670 reports all as blocking.

## What Changes

- `TRUNCATE(x, d)` / `TRUNC(x, d)` — truncate `x` to `d` decimal places (d defaults 0)
- `HEX(x)` — convert integer/blob to uppercase hex string (e.g. `HEX(255)` = `'FF'`)
- `MD5(string)` — MD5 hash as 32-char hex string (for compatibility)
- All return `Value::Text` or `Value::Float`; NULL → NULL

## Capabilities

### New Capabilities

- `executor-math-trunc`: `sqlrustgo` MUST evaluate `TRUNCATE`/`TRUNC` with numeric truncation.
- `executor-hex-hash`: `sqlrustgo` MUST evaluate `HEX` (and `MD5` as compatibility stub).

### Modified Capabilities

- None.
