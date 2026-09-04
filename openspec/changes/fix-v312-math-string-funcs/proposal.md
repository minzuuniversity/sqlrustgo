## Why

Issue #4670: `TRUNCATE`/`TRUNC` 报 parse error；`MD5`/`SHA1` 静默返回空字符串。两个问题都是纯 executor/parser 改动，改动范围小，可独立验证。

## What Changes

- `TRUNCATE(expr, digits)` / `TRUNC(expr, digits)`: parser 识别为函数调用，executor 实现截断逻辑
- `MD5(string)`: executor 调用 Rust 的 md5 crate 返回 32 字符十六进制串
- `SHA1(string)`: executor 调用 Rust 的 sha1 crate 返回 40 字符十六进制串

## Capabilities

### New Capabilities

- `trunc-fn`: TRUNCATE/TRUNC 数值截断函数
- `md5-sha1-fn`: MD5/SHA1 哈希函数

### Modified Capabilities

- None
