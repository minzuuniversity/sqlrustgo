# Issue #2627: Query Processing Chain 独立开发链路

## 目标

建立不依赖 executor 的 Query Processing 独立开发链路，覆盖 Parser → Optimizer → Planner → 执行计划验证的完整数据流。

## 为什么独立

当前状态：
- `sqlrustgo-parser`: ✅ 编译通过
- `sqlrustgo-optimizer`: ✅ 编译通过
- `sqlrustgo-planner`: ✅ 编译通过
- `sqlrustgo-sql-corpus`: ✅ 编译通过
- `sqlrustgo-executor`: ❌ 无法编译（#2625）

此链路可以在 #2625 修复期间完全并行开发。

---

## 阶段 P1: Parser 增强

**目标:** 提升 Parser 覆盖率，完善 SQL 语法支持

### P1.1 测试覆盖补全

- [ ] `test_parser_select` — SELECT 语句
- [ ] `test_parser_insert` — INSERT 语句
- [ ] `test_parser_update` — UPDATE 语句
- [ ] `test_parser_delete` — DELETE 语句
- [ ] `test_parser_create_table` — CREATE TABLE
- [ ] `test_parser_alter_table` — ALTER TABLE
- [ ] `test_parser_drop_table` — DROP TABLE

### P1.2 边界情况

- [ ] `test_parser_string_escaping` — 字符串转义
- [ ] `test_parser_numeric_literals` — 数值字面量
- [ ] `test_parser_datetime_literals` — 日期时间字面量
- [ ] `test_parser_null_handling` — NULL 处理
- [ ] `test_parser_comment_stripping` — 注释剥离

---

## 阶段 P2: Optimizer 独立测试

**目标:** 验证查询优化器的核心优化规则

### P2.1 规则优化测试

- [ ] `test_predicate_pushdown` — 谓词下推
- [ ] `test_projection_pruning` — 投影剪枝
- [ ] `test_constant_folding` — 常量折叠
- [ ] `test_subquery_flattening` — 子查询扁平化

### P2.2 代价模型测试

- [ ] `test_join_ordering` — 连接顺序优化
- [ ] `test_index_selection` — 索引选择
- [ ] `test_sort_elimination` — 排序消除

---

## 阶段 P3: Planner 集成测试

**目标:** 完整的 Parser → Planner 数据流验证

### P3.1 端到端解析

- [ ] `test_plan_insert` — INSERT 计划生成
- [ ] `test_plan_update` — UPDATE 计划生成
- [ ] `test_plan_delete` — DELETE 计划生成
- [ ] `test_plan_select_aggregation` — 聚合查询计划

### P3.2 计划验证

- [ ] `test_physical_plan_generation` — 物理计划生成
- [ ] `test_plan_cache_effectiveness` — 计划缓存
- [ ] `test_plan_hash_stability` — 相同 SQL 生成相同计划哈希

---

## 测试文件位置

```
crates/parser/tests/
  ├── sql_parse_test.rs
  ├── dml_parse_test.rs
  └── ddl_parse_test.rs

crates/optimizer/tests/
  ├── predicate_pushdown_test.rs
  ├── projection_pruning_test.rs
  └── join_ordering_test.rs

crates/planner/tests/
  ├── plan_generation_test.rs
  └── plan_cache_test.rs
```

---

## 验收标准

1. Parser 覆盖率 > 85%
2. Optimizer 核心规则有独立测试
3. Planner 生成有效的物理计划

---

## 责任人

- Hermes A: 可选（主要精力在 #2625）
- 可分配给 Hermes B 或其他 Agent

---

## 与其他 Issue 的关系

| Issue | 关系 |
|-------|------|
| #2625 | 执行层修复（依赖此链路的输出） |
| #2626 | 存储层测试（无依赖，可并行） |
| #2627 | 本 Issue，独立链路 |

---

## 并行价值

- Parser/Optimizer/Planner 团队可以独立迭代
- 提前验证 SQL 语法覆盖完整性
- 为 #2625 提供可测试的数据流入口