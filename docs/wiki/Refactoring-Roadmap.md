# SQLRustGo v3.1.0 重构路线图

> **版本**: v3.1.0
> **目标**: CBO 完整激活 + SQL 覆盖增强
> **日期**: 2026-05-11

---

## 一、重构原则

### 1.1 重构准则

1. **保持向后兼容** - 不破坏现有 API，渐进式重构
2. **测试驱动重构** - 先写测试，再改实现
3. **文档同步更新** - 重构同步更新 OO 文档
4. **性能回归检测** - 重构前后性能对比

---

## 二、P0 重构任务

### 2.1 实现 MERGE 语句

**问题**: DML 不完整

**执行链路**:
```
MERGE → Parser → Planner → Executor
```

**实现计划**:

| 阶段 | 任务 | 文件 |
|------|------|------|
| 1 | Parser 支持 MERGE | `parser/src/merge.rs` |
| 2 | Planner 生成执行计划 | `planner/src/merge_planner.rs` |
| 3 | Executor 实现 | `executor/src/merge_executor.rs` |
| 4 | 测试覆盖 | `executor/tests/merge_test.rs` |

**工作量**: 3-4 周

---

### 2.2 补全窗口函数

**问题**: LEAD/LAG/NTILE 等未实现

**缺失函数**:
| 函数 | 状态 | 实现难度 |
|------|------|----------|
| LEAD | ❌ | 中 |
| LAG | ❌ | 中 |
| NTILE | ❌ | 中 |
| FIRST_VALUE | ❌ | 低 |
| LAST_VALUE | ❌ | 低 |
| NTH_VALUE | ❌ | 中 |

**工作量**: 2-3 周

---

## 三、P1 重构任务

### 3.1 激活 CBO CostModel

**问题**: CostModel 存在但未接入 planner

**重构计划**:

| 阶段 | 任务 | 目标 |
|------|------|------|
| 1 | 重构 StatsCollector | 从存储层收集统计信息 |
| 2 | 重构 CostModel | 完善代价计算公式 |
| 3 | 重构 PlanEnumerator | 使用 CostModel 选择计划 |
| 4 | 重构 IndexSelector | 基于代价选择索引 |

**工作量**: 4-6 周

---

### 3.2 实现 MERGE JOIN

**问题**: 无 Sort-Merge Join 算法

**执行链路**:
```
SortMergeJoin
    ├── Phase 1: 对两侧数据排序
    ├── Phase 2: 双路归并
    └── Phase 3: 输出结果
```

**工作量**: 2-3 周

---

### 3.3 实现 BNL JOIN

**问题**: 大表 JOIN 性能差

**Block Nested Loop Join 原理**:

外表: t1 (小表) + 内表: t2 (大表)

1. 读取外表到一个块 (e.g., 1000 rows)
2. 读取内表全部数据，构建 hash 表
3. 匹配块内行与 hash 表
4. 重复直到外表处理完成

**工作量**: 1-2 周

---

### 3.4 增强递归 CTE

**问题**: UNION (去重) 不支持

**改进**: 添加 `SetOp::Union` 支持，使用 HashSet 跟踪已见过的行

**工作量**: 1 周

---

## 四、重构验证

### 4.1 重构检查清单

```bash
# 1. 测试通过
cargo test --all-features

# 2. 覆盖率检查
cargo llvm-cov report --show-missing
# 目标: 重构模块覆盖率 ≥ +10%

# 3. 性能回归检测
cargo bench --all-features
# 目标: 无性能下降 > 5%

# 4. Clippy 检查
cargo clippy --all-features -- -D warnings

# 5. 格式化检查
cargo fmt --check --all
```

### 4.2 重构里程碑

| 版本 | 里程碑 | 验收标准 |
|------|--------|----------|
| v3.1.0-alpha | MERGE 语句完成 | 测试通过 |
| v3.1.0-alpha | 窗口函数补全 | LEAD/LAG 可用 |
| v3.1.0-beta | CBO 激活 | 查询计划正确 |
| v3.1.0-beta | Join 算法完整 | 三种 Join 可用 |
| v3.1.0-rc | 集成测试通过 | 所有门禁通过 |
| v3.1.0-ga | 发布 | MySQL 兼容性 ≥ 70 |

---

## 五、风险与对策

| 风险 | 影响 | 概率 | 对策 |
|------|------|------|------|
| CBO 激活影响性能 | 高 | 中 | 充分测试，设定性能回归阈值 |
| 重构破坏现有功能 | 高 | 低 | 测试驱动，保持向后兼容 |
| 工作量超估 | 中 | 中 | 分阶段交付，设置缓冲 |
| 依赖未就绪 | 中 | 低 | 提前规划依赖关系 |

---

## 六、GitNexus 分析支持

详见: [GitNexus-Analysis-Plan.md](GitNexus-Analysis-Plan.md)

---

*文档版本: v3.1.0*
*创建日期: 2026-05-11*