## Context

GA 门禁要求 80-85% 覆盖率。当前 workspace 69.9%，主要缺口在 main crate (14.54%, 7148 missed) 和 executor (64.95%, 5661 missed)。现有测试覆盖了部分路径，但核心执行引擎和算子有大量未覆盖代码。

约束：
- 测试必须通过 `cargo test --lib` 和 `cargo test --all-targets`
- parser crate 有 8000+ "cannot test inner items" 警告（无法测试内部函数），这是 lalrpop 生成代码的限制
- `test_benchmark_run_short` 必须跳过（60s 超时）
- `test_sharded_vector_insert_and_search` 已修复（cosine 相同得分导致排序不稳定）

## Goals / Non-Goals

**Goals:**
- 编写集成测试覆盖 main crate engine_select.rs、execution_engine.rs、engine_builder.rs 核心路径
- 编写 executor trigger、aggregate、join、scan 算子测试
- 编写 parser SELECT/INSERT/UPDATE/DELETE 解析测试
- workspace 总体覆盖率从 69.9% 提升到 80-85%

**Non-Goals:**
- 不修改生产代码逻辑（只写测试）
- 不为 sql-corpus crate 编写测试（0% 覆盖可接受，数据驱动搜索 crate）
- 不覆盖 parser 的 lalrpop 内部函数（无法测试）
- 不修改集成测试文件（tpch_22_mysql_cli_wire_test 等已有集成测试）

## Decisions

### Decision 1: 测试位置策略

**选择**: 在各 crate 内的 `mod tests {}` 块编写单元测试，在 `tests/` 目录编写集成测试

**理由**: Rust 惯例是 `#[cfg(test)] mod tests { ... }` 内联测试用于模块内部 API 测试，`tests/*.rs` 用于集成测试。覆盖率工具对两种都支持。

**替代方案**:
- 全部放在 `tests/` 目录 → 缺点：集成测试无法访问私有函数
- 全部放在 `mod tests {}` → 缺点：无法测试跨模块集成路径

### Decision 2: Main crate 覆盖策略

**选择**: 优先测试 engine_select.rs 的路径选择和 execution_engine.rs 的执行方法

**理由**: 这两个文件贡献了 main crate 7148 未覆盖 regions 中的 5550 个。覆盖它们可带来最大收益。

**替代方案**:
- 测试 engine_utils.rs → 贡献 988 miss，覆盖难度低但收益也低
- 测试 engine_builder.rs → 贡献 187 miss，但需要完整的 catalog/storage 初始化

### Decision 3: Executor 覆盖策略

**选择**: 优先测试 trigger、aggregate、join 的核心执行路径

**理由**: 这些是 SQL 执行的核心算子，覆盖它们对整体覆盖率贡献最大。

**替代方案**:
- 测试 filter/scan → 贡献 miss 较少
- 测试 parallel_executor → 需要多线程测试基础设施

## Risks / Trade-offs

**[Risk]** 某些核心函数依赖完整系统初始化（catalog、storage、transaction）
→ **缓解**: 使用 mock harness 或简化初始化路径

**[Risk]** parser 有 8000+ "cannot test inner items" 警告
→ **缓解**: 使用 `RUSTFLAGS="-A warnings"` 抑制，预期行为

**[Risk]** 测试编写后发现函数设计问题（难以构造测试输入）
→ **缓解**: 先读函数签名和文档，确认可测试后再写

**[Risk]** 达到 80% 需要大量测试用例（可能需要 200+ 个测试）
→ **缓解**: 优先覆盖高权重文件，其余逐步补充

## Migration Plan

1. **Phase 1**: main crate engine 集成测试（2-3 小时）
2. **Phase 2**: executor 算子测试（2-3 小时）
3. **Phase 3**: parser 和 mysql-server 测试补充（1-2 小时）
4. **Phase 4**: 验证 `cargo llvm-cov test --workspace --lib` 达到 80%+
5. **Phase 5**: 验证 `cargo test --all-targets --workspace` 全部通过

## Open Questions

- engine_select.rs 中的 cost estimation 逻辑需要 mock catalog，是否可行？
- executor 的 parallel_executor 是否需要测试（多线程复杂）？
- 是否需要为 sql-corpus crate 编写占位测试（用户接受 0% 覆盖）？
