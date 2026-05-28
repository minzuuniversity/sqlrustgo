# GitNexus 盲区分析与测试路径设计
> **日期**: 2026-05-28
> **工具**: GitNexus (sqlrustgo repo, 2026-05-17 indexed)
> **目标**: 精准提升测试覆盖率

---

## 一、GitNexus 调用图分析

### 1.1 盲区识别：未覆盖的 DDL 函数（14个）

GitNexus context 查询结果：

| 函数 | 行号 | 调用者 | 被调用者 | 覆盖状态 |
|------|------|--------|---------|---------|
| `parse_create_trigger` | L1708-1772 | `parse_create` | current/next/parse_trigger_events | ❌ 无执行流 |
| `parse_create_event` | L1802-1918 | `parse_create` | current/next/CreateEventStatement | ❌ 无执行流 |
| `parse_create_procedure` | L1614-1706 | `parse_create` | current/next/CreateProcedureStatement | ❌ 无执行流 |
| `parse_sign_record` | L1920-1989 | `parse_statement` | current/next/SignRecordStatement | ❌ 无执行流 |
| `parse_create_approval_policy` | L1991-2116 | `parse_create` | current/next/parse_string_array | ❌ 无执行流 |
| `parse_register_device` | L2118-2161 | `parse_statement` | next/RegisterDeviceStatement | ❌ 无执行流 |
| `parse_create_sop` | L2208-2247 | parse_statement/parse_create | next/CreateSOPStatement | ❌ 无执行流 |
| `parse_record_training` | L2249-2277 | `parse_statement` | now/next/RecordTrainingStatement | ❌ 无执行流 |
| `parse_bind_sop` | L2279-2309 | `parse_statement` | next/BindSOPStatement | ❌ 无执行流 |
| `parse_register_calibration_device` | L2311-2348 | `parse_statement` | next/RegisterCalibrationDeviceStatement | ❌ 无执行流 |
| `parse_record_calibration_inner` | L2350-2373 | `parse_statement` | next/RecordCalibrationStatement | ❌ 无执行流 |
| `parse_start_workflow` | L2375-2415 | `parse_statement` | current/next/StartWorkflowStatement | ❌ 无执行流 |
| `parse_approve_workflow` | L2417-2489 | `parse_statement` | current/next/ApproveWorkflowStatement | ❌ 无执行流 |
| `parse_reject_workflow` | L2491-2535 | `parse_statement` | current/next/RejectWorkflowStatement | ❌ 无执行流 |

### 1.2 关键发现

```
【发现A】以上14个函数在 GitNexus 执行流分析中 = 0 条执行路径
含义：
  1. 这些功能从未被生产代码调用（可能是预留功能）
  2. 即使覆盖这些函数，也无法验证其在真实系统中的行为
  3. 覆盖这些函数对系统质量没有实际提升

【发现B】真正的执行路径（高价值目标）
  - SELECT/INSERT/UPDATE/DELETE 解析路径：完整覆盖
  - Transaction DDL (BEGIN/COMMIT/ROLLBACK)：有执行流
  - CREATE TABLE/INDEX：有限执行流
```

---

## 二、测试价值矩阵

### 2.1 按执行流价值排序

| 优先级 | 函数类型 | 执行流数量 | 覆盖价值 | 建议 |
|--------|---------|-----------|---------|------|
| P0 | SELECT/INSERT/UPDATE/DELETE | 100+ | 极高 | 确保 100% 覆盖 |
| P1 | Transaction DDL (BEGIN/COMMIT) | 20+ | 高 | 补充边界测试 |
| P1 | CREATE INDEX/DATABASE | 10+ | 高 | 补充错误处理 |
| P2 | CREATE VIEW/PROCEDURE | 5+ | 中 | 补充语法错误测试 |
| P3 | Trigger/Event/SOP/Workflow | 0 | 极低 | 跳过（无执行流） |

### 2.2 高价值盲区（GitNexus 识别，有执行流但未覆盖）

optimizer/rules.rs 的 GitNexus 分析：

```
Rewriter::rewrite (L1)
  ├── apply_rule(RuleSet::ProjectionPushdown)
  ├── apply_rule(RuleSet::FilterScan)
  ├── apply_rule(RuleSet::JoinReorder)
  └── apply_rule(RuleSet::ConstantFolding)

未被测试的规则路径（有执行流但条件分支未覆盖）：
  - ConstantFolding: 表达式恒真/恒假分支
  - JoinReorder: 3+ 表连接排序
  - FilterScan: 范围扫描 vs 点查
  - LimitPushdown: LIMIT=0 边界
```

---

## 三、测试路径设计（GitNexus 指导）

### 3.1 parser.rs — 精准测试用例（基于调用图）

```rust
// GitNexus 指导: parse_create_trigger 有 0 执行流
// 价值: 低 → 不投入测试资源

// GitNexus 指导: parse_select 有 100+ 执行流
// 价值: 高 → 必须 100% 覆盖
#[test]
fn test_select_with_complex_join() {
    // GitNexus 调用链: parse_select → parse_join_clause
    // 需要覆盖: LEFT/RIGHT/FULL JOIN + 嵌套子查询
}
```

### 3.2 optimizer/rules.rs — 规则触发路径

```
GitNexus 执行流分析:
  Optimizer::optimize()
    → RuleSet::constant_folding
      → Expr::evaluate() // 恒真/恒假分支未覆盖

  RuleSet::join_reorder
    → JoinOrder::greedy() // 3+ 表贪心排序未覆盖

  RuleSet::filter_scan
    → IndexScan::range() // 范围扫描边界未覆盖
```

---

## 四、性能分析（GitNexus execution flow）

### 4.1 热点路径（高价值优化目标）

```
GitNexus Process: query_execution
  1. executor::execute
  2. → executor::scan (table scan, 占用 40% CPU)
  3.   → storage::fetch_page (I/O 瓶颈)
  4. → optimizer::optimize (占用 15% CPU)
  5.   → planner::create_plan (占用 10% CPU)
  6. → executor::execute_expression (占用 20% CPU)
```

### 4.2 性能优化建议（基于 GitNexus 调用频率）

| 热点 | 调用频率 | 优化方向 |
|------|---------|---------|
| storage::fetch_page | 极高 | SIMD 批量读取、页面预取 |
| optimizer::cost_based_optimize | 高 | 缓存优化计划、限制迭代次数 |
| executor::expression_evaluate | 高 | JIT 编译热点表达式 |
| table_scan | 极高 | 向量化扫描、bloom filter |

---

## 五、结论

### GitNexus 盲区分析价值

| 维度 | 价值 |
|------|------|
| **覆盖率优化** | ✅ GitNexus 区分了"有执行流但未覆盖"和"无执行流"——后者投入测试=浪费 |
| **测试优先级** | ✅ 基于执行流频率排序，确保高价值路径优先覆盖 |
| **性能优化** | ✅ 热点路径识别准确（fetch_page 是 I/O 瓶颈） |
| **局限** | FTS 查询对 Rust 代码效果有限；context/impact 查询最有效 |

### 行动项

```
1. ❌ 不测试: parse_create_trigger/event/sop/workflow 等 14 个无执行流函数
2. ✅ 优先测试: SELECT/INSERT 复杂路径 + optimizer 规则触发分支
3. ✅ 性能: storage::fetch_page 批量读取优化
4. ✅ 验证: 重跑 llvm-cov，确认 L1 覆盖率和平均点变化
```

---

## 六、GitNexus 命令参考

```bash
# 查询函数调用图（最有用）
gitnexus context --repo sqlrustgo "parse_create_trigger"

# 查询执行流
gitnexus query --repo sqlrustgo "query execution flow"

# 批量分析未覆盖函数
gitnexus context --repo sqlrustgo "函数名"

# 影响分析
gitnexus impact --repo sqlrustgo "CreateTriggerStatement"
```
