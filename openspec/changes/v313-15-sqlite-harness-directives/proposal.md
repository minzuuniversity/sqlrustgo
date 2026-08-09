# V313-15: SQLite Harness 指令支持

## Why

V312-11 基线确立了当前 sqlrustgo_sqllogictest runner 的覆盖率：22 个 test 文件，仅 6 个通过，16 个失败。其中 3 个失败源为 harness 类问题：

```
crates/sqlrustgo_sqllogictest/testdata/duckdb_full/aggregate__quantile_fun.test  → "parse error: invalid line: 'set variable sf 0.001'"
crates/sqlrustgo_sqllogictest/testdata/duckdb_full/sql__quantile_fun.test        → "parse error: invalid line: 'set variable sf 0.001'"
crates/sqlrustgo_sqllogictest/testdata/duckdb_samples/quantile_fun.test           → "parse error: invalid line: 'set variable sf 0.001'"
```

这 3 个文件均因第 5 行的 `set variable sf 0.001` 指令导致解析失败。根本原因：sqllogictest 格式允许测试文件通过 `set` 指令向 harness 设置变量（如 scale factor），但当前 runner 基于 `sqllogictest = "0.29"` 的 `Runner::new()` API，尚未注册自定义 directive 解析器。

这些文件被记录在 `exclusions.yml` 中，category 为 `harness`，owner 为 `openclaw`，expiry 为 `2027-06-30`。本变更通过实现 harness directive 支持，消除这 3 个 exclusion，使 quantile 函数相关的 SQL 覆盖进入可测试范围。

## What Changes

- **`crates/sqlrustgo_sqllogictest/src/main.rs`**：扩展 `sqllogictest::Runner` 的 `register_directive` 机制，实现 `set variable <name> <value>` 指令解析，将变量存储于 `SltDb` 的 `variables: HashMap<String, String>` 字段
- **`exclusions.yml`**：删除 3 个 harness 类 exclusion 条目（`aggregate__quantile_fun.test`、`sql__quantile_fun.test`、`quantile_fun.test`）
- **`sqlite-corpus-manifest.json`**：将这 3 个文件状态从 `"excluded"` → `"unknown"`（待实际运行后更新为 pass/fail）

## Capabilities

### 新增能力

- `harness-set-variable`：runner 支持解析 `set variable <name> <value>` 指令，并将变量注入后续 SQL 执行上下文（若测试使用 harness 变量替换语法，变量值可供语句使用）
- `harness-directive-architecture`：建立 directive 扩展点，未来可支持 `set threads <n>`、`hash threshold` 等其他 harness 指令

### 修改能力

- `sqllogictest-corpus-coverage`：3 个 quantile 相关 fixture 从 excluded → 可测试状态，覆盖率分子 +3（若实现正确，pass rate 预期从 27.3% 提升至更高）
- `sqlite-corpus-manifest`：manifest 中 3 个文件 category=excluded → 待测试

## Impact

- **修改文件**：`crates/sqlrustgo_sqllogictest/src/main.rs`（指令解析）、`docs/releases/v3.12.0/evidence/sqllogictest/exclusions.yml`（删除 exclusion）、`docs/releases/v3.12.0/evidence/sqllogictest/sqlite-corpus-manifest.json`（状态更新）
- **风险**：`set variable` 指令目前仅做解析+存储，变量替换（如 `${sf}` 语法）属于后续任务范围；本变更仅保证文件不再报 parse error，不保证测试结果 pass
- **无新增外部 crate 依赖**：`sqllogictest = "0.29"` 已支持 `register_directive` API
