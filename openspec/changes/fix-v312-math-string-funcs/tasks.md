## 1. TRUNCATE/TRUNC Parser 修复

- [ ] 1.1 在 `crates/parser/src/parser.rs` 的 `parse_function_call` 中找到 `TRUNCATE`/`TRUNC` token 注册位置，添加这两个关键字作为已知函数名
- [ ] 1.2 验证 `SELECT TRUNCATE(3.7, 0);` 和 `SELECT TRUNC(3.14159, 2);` 不再报 parse error

## 2. TRUNCATE/TRUNC Executor 实现

- [ ] 2.1 在 `crates/executor/src/expr/mod.rs` 的 `eval_function` 中找到 `TRUNCATE` 分支，实现截断逻辑：`f64::trunc(x * 10^digits) / 10^digits`
- [ ] 2.2 验证 `SELECT TRUNCATE(3.14159, 2)` 返回 `3.14`，`SELECT TRUNCATE(1234.5, -2)` 返回 `1200.0`

## 3. MD5 Executor 实现

- [ ] 3.1 在 `crates/executor/src/expr/mod.rs` 的 `eval_function` 中找到 `MD5` 分支，调用 `md5::compute()` 并返回十六进制字符串
- [ ] 3.2 验证 `SELECT MD5('abc')` 返回 `900150983cd24fb0d6963f7d28e17f72`

## 4. SHA1 Executor 实现

- [ ] 4.1 在 `crates/executor/src/expr/mod.rs` 的 `eval_function` 中找到 `SHA1` 分支，调用 `sha1::Sha1::digest()` 并返回十六进制字符串
- [ ] 4.2 验证 `SELECT SHA1('abc')` 返回 `a9993e364706816aba3e25717850c26c9cd0d89d`
