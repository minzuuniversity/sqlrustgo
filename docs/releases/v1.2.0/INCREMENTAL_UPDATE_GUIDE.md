# GMP-Platform v1.2.0 增量更新指南

> **版本**: v1.2.0 | **日期**: 2026-05-23
> **从**: v1.1.0 | **到**: v1.2.0

---

## 1. 更新概述

### 1.1 主要变更

| 组件 | v1.1.0 | v1.2.0 | 变更类型 |
|------|--------|--------|---------|
| 向量通道 | 部分集成 | **完全集成** | 功能增强 |
| 图查询 | 无 NL→Cypher | **NL→Cypher 转换器** | 新增功能 |
| 评估指标 | 精确匹配 | **+LLM-as-Judge** | 新增功能 |
| gmp-api | 简单连接 | **连接池优化** | 性能优化 |
| Z440 图持久化 | stub | **完整实现** | Bug 修复 |

### 1.2 数据库兼容性

v1.2.0 **完全兼容** v1.1.0 的数据库 schema，无需迁移。

---

## 2. 代码更新

### 2.1 获取更新

```bash
# 切换到 v1.2.0 分支
git fetch origin
git checkout develop/v1.2.0

# 确认分支
git log --oneline -1
# 输出: xxxxxx (HEAD -> develop/v1.2.0) v1.2.0 development
```

### 2.2 编译

```bash
# 增量编译 (推荐)
cargo build --release

# 清理后编译 (如遇编译错误)
cargo clean
cargo build --release
```

### 2.3 验证版本

```bash
./target/release/gmp-eval --version
# 输出: gmp-eval v1.2.0

./target/release/gmp-cli --version
# 输出: gmp-cli v1.2.0
```

---

## 3. 数据验证

### 3.1 现有数据检查

```bash
# 验证数据库完整性
./target/release/gmp-eval --db data/gmp.db --stats

# 预期输出:
# Documents: 1689
# Chunks: ~8700
# Embeddings: ~8700
# Graph nodes: ≥700
# Graph edges: ≥1500
```

### 3.2 索引验证

```bash
# 验证所有索引
./target/release/gmp-eval index --validate

# 验证向量索引 (v1.2.0 新增强)
./target/release/gmp-eval index --validate vector

# 验证图索引 (v1.2.0 新增强)
./target/release/gmp-eval index --validate graph
```

---

## 4. 功能验证

### 4.1 快速冒烟测试

```bash
# 1. FTS 检索
./target/release/gmp-eval --query "偏差调查" --no-hybrid
# 预期: 返回结果

# 2. 向量检索 (v1.2.0 新增强)
./target/release/gmp-eval --query "偏差管理最佳实践" --channel vector
# 预期: 返回结果 (v1.1.0 可能为空)

# 3. 图查询 (v1.2.0 新增)
./target/release/gmp-eval --query "偏差和CAPA的关系" --graph
# 预期: 返回结果 (v1.1.0 无此功能)

# 4. 混合检索
./target/release/gmp-eval --query "验证主计划" --all
# 预期: 返回结果
```

### 4.2 场景测试

```bash
# 运行 408 场景测试
./target/release/gmp-eval \
  --db data/gmp.db \
  --test-set test-data/408_scenarios.json

# 对比 v1.1.0 结果
# 预期: 向量通道和图查询场景通过率提升
```

### 4.3 LLM-as-Judge 验证 (v1.2.0)

```bash
# 启用 LLM 判断模式
./target/release/gmp-eval \
  --db data/gmp.db \
  --judge-mode \
  --query "偏差调查流程"

# 预期: 输出包含 JudgeResult (ExactMatch/SemanticMatch/NoMatch)
```

---

## 5. API 升级 (如使用)

### 5.1 重启 API 服务

```bash
# 停止旧服务
pkill -f gmp-api

# 启动新服务
./target/release/gmp-api --port 8080 --pool-size 10 &

# 验证
curl http://localhost:8080/health
# 预期: {"status": "ok", "version": "1.2.0"}
```

### 5.2 API 变更

| 端点 | v1.1.0 | v1.2.0 |
|------|--------|--------|
| `POST /api/v1/search` | 支持基本参数 | **+enable_graph 参数** |
| `GET /api/v1/stats` | 基本统计 | **+图统计** |

### 5.3 连接池配置 (v1.2.0 新增)

```bash
# 启动时配置连接池大小
./target/release/gmp-api --port 8080 --pool-size 20

# 环境变量配置
export GMP_API_POOL_SIZE=20
./target/release/gmp-api
```

---

## 6. 配置变更

### 6.1 新增配置项 (v1.2.0)

```yaml
# config.yaml (新增)
graph:
  nl_to_cypher: true          # 启用 NL→Cypher 转换
  max_depth: 3                # 图查询最大深度
  cache_enabled: true          # 图查询缓存

judge:
  enabled: true              # 启用 LLM-as-Judge
  model: "qwen2.5:14b"       # 评估模型

api:
  pool_size: 10               # 连接池大小 (新增)
  timeout_seconds: 30         # 请求超时
```

### 6.2 向后兼容

v1.1.0 的配置文件完全兼容，v1.2.0 会使用默认值填充新增项。

---

## 7. 性能验证

### 7.1 QPS 基准测试

```bash
# 运行 QPS 测试
./target/release/gmp-eval \
  --db data/gmp.db \
  --bench-qps 5

# 对比 v1.1.0:
# v1.1.0: 154.0 QPS
# v1.2.0: 预期 200+ QPS (待实测)
```

### 7.2 延迟测试

```bash
# 测试单次查询延迟
for i in {1..10}; do
  time ./target/release/gmp-eval --query "偏差调查流程" --all
done

# 预期 P95 < 5ms (v1.1.0 为 6.5ms)
```

### 7.3 内存使用

```bash
# 检查内存占用
ps aux | grep gmp-eval

# 预期: 稳定在 200-500MB (取决于数据量)
```

---

## 8. 回滚计划

### 8.1 自动回滚

如果升级后出现问题，可立即回滚到 v1.1.0：

```bash
# 停止 v1.2.0
pkill -f gmp-eval

# 切换回 v1.1.0
git checkout develop/v1.1.0
cargo build --release

# 验证版本
./target/release/gmp-eval --version
# 输出: gmp-eval v1.1.0

# 重启服务
./target/release/gmp-api &
```

### 8.2 数据回滚

v1.2.0 不修改数据库 schema，但建议在升级前备份：

```bash
# 备份
cp data/gmp.db data/gmp.db.backup.v1.1.0.$(date +%Y%m%d)

# 如需回滚数据
cp data/gmp.db.backup.v1.1.0.20260522 data/gmp.db
```

---

## 9. 升级检查清单

### 升级前

- [ ] 备份数据库: `cp data/gmp.db data/gmp.db.backup.v1.1.0.$(date +%Y%m%d)`
- [ ] 记录 v1.1.0 QPS 基线: `./target/release/gmp-eval --bench-qps 5`
- [ ] 记录 v1.1.0 场景通过率

### 升级中

- [ ] 拉取 v1.2.0 代码: `git checkout develop/v1.2.0`
- [ ] 编译: `cargo build --release`
- [ ] 验证版本: `./target/release/gmp-eval --version`

### 升级后

- [ ] 验证数据完整性: `./target/release/gmp-eval --db data/gmp.db --stats`
- [ ] 运行冒烟测试: 见 4.1 节
- [ ] 运行场景测试: `./target/release/gmp-eval --db data/gmp.db --test-set test-data/408_scenarios.json`
- [ ] 记录 v1.2.0 QPS: `./target/release/gmp-eval --bench-qps 5`
- [ ] 测试新功能: NL→Cypher, LLM-as-Judge

### 验证通过

- [ ] 场景通过率 ≥ 99% (或不低于 v1.1.0)
- [ ] QPS ≥ 200 (或不低于 v1.1.0)
- [ ] API 所有端点正常响应
- [ ] 无新增 error 日志

---

## 10. 已知问题

### 10.1 v1.2.0 已知限制

| 问题 | 影响 | 解决方案 |
|------|------|---------|
| NL→Cypher 转换依赖 LLM | 增加 500ms-1s 延迟 | 使用 `--no-graph` 禁用 |
| LLM-as-Judge 增加评估时间 | 批量评估变慢 | 使用 `--judge-only` 精确定位 |
| 图查询缓存内存占用 | 内存使用 +50MB | 使用 `--cache-size` 限制 |

### 10.2 遗留项 (v1.2.1)

| 功能 | 计划版本 | 说明 |
|------|---------|------|
| 多租户支持 | v1.2.1 | 租户隔离 |
| GPU 加速 | v1.2.1 | CUDA 加速 |

---

## 11. 支持

### 11.1 文档

- 用户手册: `docs/releases/v1.2.0/USER_MANUAL.md`
- 部署指南: `docs/releases/v1.2.0/DEPLOYMENT_GUIDE.md`
- 性能报告: `docs/releases/v1.2.0/PERFORMANCE_REPORT.md`

### 11.2 获取帮助

```bash
# 查看 CLI 帮助
./target/release/gmp-eval --help
./target/release/gmp-cli --help

# 查看详细日志
RUST_LOG=debug ./target/release/gmp-eval --query "偏差"
```
