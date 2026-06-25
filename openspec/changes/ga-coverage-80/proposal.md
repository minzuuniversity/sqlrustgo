## Why

GA 门禁要求代码覆盖率达到 80-85%，但当前 workspace 总体覆盖率仅 69.9%。有 5 个关键 crate 严重落后：main crate (14.54%)、parser (37.53%)、mysql-server (51.10%)、executor (64.95%)、sql-corpus (0%)。这些 crate 贡献了约 4 万+ 未覆盖 regions，是达到 GA 门禁的主要障碍。

## What Changes

- 为 `sqlrustgo` 主 crate 的 `execution_engine`、`engine_select`、`engine_builder` 编写集成测试（+5,468 regions 覆盖）
- 为 `sqlrustgo-executor` 的 `trigger`、`aggregate`、`join`、`scan` 模块编写测试用例（+2,423 regions 覆盖）
- 为 `sqlrustgo-parser` 的 SELECT/INSERT/UPDATE/DELETE 解析路径编写测试（+2,400 regions 覆盖）
- 为 `sqlrustgo-mysql-server` MySQL 协议解析补充测试（+1,236 regions 覆盖）
- 确认 `sqlrustgo-sql-corpus` 无需测试（数据驱动的搜索 crate，0% 覆盖可接受）
- 确保所有 crate 的 lib tests 和 integration tests 均能通过 `--all-targets`

## Capabilities

### New Capabilities
- `main-engine-integration-tests`: 覆盖 main crate engine_select.rs、execution_engine.rs、engine_builder.rs 的核心执行路径
- `executor-operator-tests`: 覆盖 executor trigger/aggregate/join/scan/filter 算子的核心路径
- `parser-statement-tests`: 覆盖 parser 对 SELECT/INSERT/UPDATE/DELETE 的解析覆盖
- `mysql-protocol-tests`: 覆盖 mysql-server MySQL 协议 handshake、COM_QUERY、COM_STMT_PREPARE 执行路径

### Modified Capabilities
- (无 spec 级行为变更，仅是测试覆盖率提升)

## Impact

- **代码**: 新增测试文件于 `tests/` 和各 crate 的 `mod tests {}`
- **CI/CD**: `cargo llvm-cov test --workspace --lib --all-targets` 必须 PASS，覆盖率 ≥ 80%
- **覆盖率**: workspace 总体从 69.9% → 80-85%
