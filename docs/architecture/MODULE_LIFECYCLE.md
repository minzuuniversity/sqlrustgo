# Module Lifecycle

> v3.7.0 Core Integrity Release - 模块生命周期定义
> 定义 crate 状态及其转换规则

## 状态定义

### 状态矩阵

| 状态 | 代码质量 | 稳定性 | 主路径 | GA 可用 | 新开发 | 示例 |
|------|---------|-------|-------|--------|-------|------|
| **production** | 高 | GA | ✅ | ✅ | ✅ | parser, executor |
| **migrating** | 中 | Beta | 部分 | ❌ | 收敛性 | mysql-server |
| **isolated** | 不确定 | 不确定 | ❌ | ❌ | ❌ | parallel_executor |
| **deprecated** | 差 | 不推荐 | ❌ | ❌ | ❌ | expr-legacy |
| **experimental** | 变化 | 不稳定 | ❌ | ❌ | ✅ | vec_simd |
| **frozen** | 当前状态 | 冻结 | ❌ | ❌ | ❌ | distributed |

### 状态详细说明

#### production

```yaml
定义: "主路径组件，经过充分测试"
特性:
  - 稳定 API
  - 完整测试覆盖
  - 已被主路径使用
  - GA 质量
允许:
  - 新功能开发
  - 性能优化
  - API 扩展
禁止:
  - 已知 critical bug
  - 破坏性变更
示例:
  - parser (SQL 解析)
  - executor (查询执行)
  - storage (存储引擎)
  - transaction (事务管理)
  - network (网络协议)
```

#### migrating

```yaml
定义: "正在收敛到主路径"
特性:
  - 有明确的目标状态
  - 正在消除孤岛特性
  - 已有集成计划
  - 预计 1-2 个版本内完成
允许:
  - 向目标状态的重构
  - 与目标模块的合并
禁止:
  - 规模扩张
  - 新孤岛 feature
  - 独立 API 增长
示例:
  - mysql-server (合并到 network)
  - expr (合并表达式系统)
```

#### isolated

```yaml
定义: "孤岛模块，未接入主路径"
特性:
  - 存在但未被主路径使用
  - 可能是实验性代码
  - 可能与主路径重复
  - 需决策：收敛或删除
允许:
  - 测试中使用
  - Bugfix（最小）
禁止:
  - 新 feature
  - 主路径依赖
  - 生产环境使用
决策:
  - 要么收敛到 migrating
  - 要么标记为 deprecated
  - 要么删除
示例:
  - parallel_executor (无主路径调用)
  - local_executor_dml (PLACEHOLDER)
```

#### deprecated

```yaml
定义: "已废弃，等待删除"
特性:
  - 存在是为了兼容性
  - 将在未来版本删除
  - 不推荐新使用
允许:
  - 编译通过
  - 基本测试通过
禁止:
  - 新 usage
  - 新 feature
  - 被 production 依赖
计划:
  - 1-2 个版本内删除
示例:
  - expr-legacy (被 expr 替代)
```

#### experimental

```yaml
定义: "实验功能，feature-gated"
特性:
  - 不稳定 API
  - 需要 feature flag
  - 可能完全改变
  - 不建议生产使用
允许:
  - 新功能开发
  - API 变化
  - 性能实验
禁止:
  - 无 feature flag
  - 生产环境使用
  - 被 production 依赖
示例:
  - vec_simd (SIMD 优化)
```

#### frozen

```yaml
定义: "冻结状态，暂停开发"
特性:
  - 当前代码冻结
  - 只修复 critical bug
  - 不进行新开发
  - 保留存档
允许:
  - 编译通过
  - Critical bug fix
  - 安全修复
禁止:
  - 新 feature 开发
  - 性能优化
  - 规模扩张
  - 依赖非 frozen 模块更新
示例:
  - distributed (分布式，暂停)
  - graph (图存储，暂停)
```

## 状态转换

### 转换图

```
                    ┌──────────────┐
                    │  experimental │
                    └──────┬───────┘
                           │
               ┌───────────┼───────────┐
               ↓                       ↓
        ┌──────────┐            ┌───────────┐
        │ frozen   │            │ migrating │
        └──────────┘            └─────┬─────┘
                                       │
                    ┌──────────────────┼──────────────────┐
                    ↓                  ↓                  ↓
             ┌──────────┐      ┌────────────┐     ┌──────────┐
             │isolated  │      │production   │     │deprecated│
             └────┬─────┘      └────────────┘     └──────────┘
                  │
     ┌────────────┼────────────┐
     ↓            ↓            ↓
┌────────┐  ┌──────────┐  ┌──────────┐
│frozen  │  │deprecated│  │ deleting │
└────────┘  └──────────┘  └──────────┘
```

### 转换规则

| 转换 | 条件 | 审批 |
|------|------|------|
| experimental → migrating | 有明确收敛计划 | Arch Review |
| experimental → frozen | 放弃实验 | Arch Review |
| isolated → migrating | 有集成计划 | Arch Review |
| isolated → deprecated | 无主路径价值 | Arch Review |
| isolated → deleting | 决定删除 | Arch Review |
| migrating → production | 收敛完成，测试通过 | Arch Review |
| migrating → deprecated | 收敛失败 | Arch Review |
| deprecated → deleting | 版本过期 | Auto |
| frozen → deleted | 多个版本后 | Arch Review |

## 模块清单

### Production (主路径)

| Crate | 版本 | 依赖 | 主路径调用 |
|-------|------|------|----------|
| parser | v2.8.0 | common | planner → optimizer → executor |
| planner | v2.8.0 | parser, types | optimizer → executor |
| optimizer | v2.8.0 | planner, types | executor |
| executor | v2.8.0 | storage, transaction, types | server, network |
| transaction | v2.8.0 | storage | executor → storage |
| storage | v2.8.0 | buffer_pool, file_storage | transaction |
| catalog | v2.8.0 | types | executor, server |
| network | v2.8.0 | parser, executor | server |
| types | v2.8.0 | - | all |
| common | v2.8.0 | - | all |
| information-schema | v2.8.0 | catalog | server |

### Migrating (收敛中)

| Crate | 目标 | 状态 | 相关 Issue |
|-------|------|------|-----------|
| mysql-server | 合并到 network | 开始收敛 | #2591, #2605 |
| expr | 合并表达式系统 | 开始收敛 | #2590, #2604 |

### Isolated (孤岛)

| Crate | 问题 | 建议 | 相关 Issue |
|-------|------|------|-----------|
| parallel_executor | 63KB，无主路径调用 | 删除/合并 | #2603 |
| parallel_vector_executor | 无主路径调用 | 删除 | #2603 |
| local_executor_dml | PLACEHOLDER | 必须实现 | #2602 |
| expression | 与 expr 重叠 | 删除 | #2604 |
| qmd-bridge | 不确定 | 评估 | - |

### Deprecated (待删除)

| Crate | 替代 | 删除版本 |
|-------|------|---------|
| expr-legacy | expr | v3.8.0 |

### Experimental (实验)

| Crate | Feature Flag | 说明 |
|-------|-------------|------|
| vec_simd | simd | SIMD 优化 |

### Frozen (冻结)

| Crate | 冻结原因 |
|-------|---------|
| distributed | 分布式不是 P0 |
| graph | 图存储不是 P0 |
| vector | 向量存储不是 P0 |

### Production Support (生产支持工具)

| Crate | 说明 |
|-------|------|
| wal-verification | Verification tooling，不是主执行路径 |
| wal-verification | verification tooling，不是主路径 |

## 维护者规则

### 添加新 Crate

1. 必须定义状态（production/migrating/isolated/deprecated/experimental/frozen）
2. 必须定义状态理由
3. 必须定义生命周期计划
4. 必须更新 MAINLINE_COMPONENTS.md

### 状态变更

1. 必须经过 Architecture Review
2. 必须更新所有相关文档
3. 必须更新 MAINLINE_COMPONENTS.md
4. 必须更新 ISOLATED_MODULES.md（如适用）
5. 必须更新本文件

### 删除 Crate

1. 必须经过 Architecture Review
2. 必须确认无 production 依赖
3. 必须有 1 个版本的 deprecation period
4. 删除后更新所有文档

## 验证命令

```bash
# 检查所有 crate 状态定义
cargo xtask dead-modules

# 检查 production 无孤岛依赖
cargo tree -p sqlrustgo-executor -e normal | grep -E "isolated|experimental"

# 检查 deprecated 无新 usage
grep -rn "use.*expr-legacy" crates/*/src/*.rs
```

## 相关文档

- `MAINLINE_COMPONENTS.md` - 主路径组件
- `ISOLATED_MODULES.md` - 孤岛模块
- `ARCHITECTURE_RULES.yaml` - 架构规则