# 升级迁移指南 (v1.0.0 → v1.1.0)

**文档版本**: 1.0.0
**适用版本**: v1.1.0
**上一版本**: v1.0.0

---

## 概述

本指南帮助用户从 v1.0.0 升级到 v1.1.0。v1.1.0 是一个向后兼容的版本，大部分现有代码无需修改即可继续工作。

---

## 变更摘要

| 变更类型 | 描述 |
|----------|------|
| 新增功能 | 性能基准测试框架 (B-001~B-010) |
| 新增功能 | Query Analyzer Phase 1 |
| 依赖更新 | tokio, bytes, regex 等安全修复 |
| 已知限制 | HashJoin 未实现 |

---

## 快速升级

### 步骤 1: 更新依赖

```bash
cargo update
```

### 步骤 2: 重新构建

```bash
cargo build --all-features
```

### 步骤 3: 运行测试

```bash
cargo test --all-features
```

---

## API 变更

### ✅ 无破坏性变更

v1.1.0 相对于 v1.0.0 **没有破坏性 API 变更**。所有现有的公共 API 保持不变。

---

## 新功能使用指南

### 性能基准测试

v1.1.0 引入了完整的性能基准测试框架。

#### 运行基准测试

```bash
# 运行所有基准测试
cargo bench --all

# 运行特定模块
cargo bench --bench lexer_bench
cargo bench --bench parser_bench
cargo bench --bench executor_bench
cargo bench --bench storage_bench
cargo bench --bench network_bench
cargo bench --bench integration_bench
```

#### 性能回归检测

```bash
# 运行基准测试并保存基线
./scripts/run_benchmarks.sh

# 与之前基线比较
cargo bench --all -- --compare-with=latest
```

### Query Analyzer (可选)

v1.1.0 引入了 Query Analyzer 模块，用于 SQL 语句分析和优化规划。

```rust
use sqlrustgo::planner::analyzer::Analyzer;

// 分析 SQL 语句
let analyzer = Analyzer::new();
let analysis_result = analyzer.analyze(&statement);

// 获取表信息
let tables = analysis_result.tables();

// 获取列信息
let columns = analysis_result.columns("table_name");
```

> **注意**: Query Analyzer 是可选功能，不影响现有查询执行流程。

---

## 配置变更

### 新增配置项

无新增必需配置项。

### 可选配置

```rust
// 启用查询分析 (可选)
use sqlrustgo::planner::config::AnalyzerConfig;

let config = AnalyzerConfig::default()
    .with_type_inference(true)
    .with_aggregate_optimization(true);
```

---

## 迁移检查清单

- [ ] 运行 `cargo update` 更新依赖
- [ ] 运行 `cargo build --all-features` 确认构建成功
- [ ] 运行 `cargo test --all-features` 确认所有测试通过
- [ ] (可选) 体验新功能：Query Analyzer

---

## 常见问题

### Q: v1.1.0 是否支持 v1.0.0 的所有功能？

**A**: 是的，v1.1.0 完全向后兼容 v1.0.0 的所有功能。

### Q: 如何回滚到 v1.0.0？

**A**: 如果遇到问题，可以回滚到 v1.0.0：

```bash
git checkout v1.0.0
cargo build --all-features
```

### Q: HashJoin 功能在哪里？

**A**: HashJoin 功能计划在 v1.2.0 中实现。当前版本不支持 HashJoin 查询。

### Q: 测试覆盖率目标是多少？

**A**: v1.1.0 的目标测试覆盖率是 90%，当前为 74.83%。这是内部质量目标，不影响用户使用。

---

## 获取帮助

- **问题报告**: https://github.com/minzuuniversity/sqlrustgo/issues
- **讨论区**: https://github.com/minzuuniversity/sqlrustgo/discussions
- **文档**: https://docs.rs/sqlrustgo/

---

## 附录：依赖版本

v1.1.0 中更新的依赖版本：

| 依赖 | v1.0.0 | v1.1.0 |
|------|---------|--------|
| tokio | 1.x | 1.49.0 |
| bytes | 1.x | 1.11.1 |
| regex | 1.x | 1.12.3 |
| serde_json | 1.x | 1.0.149 |
| thiserror | 1.x | 1.0.69 |
| anyhow | 1.x | 1.0.101 |

所有依赖更新都包含安全修复补丁。
