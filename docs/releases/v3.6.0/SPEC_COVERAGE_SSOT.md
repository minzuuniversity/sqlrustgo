# Executor Operator-level Coverage SSOT Specification

> **版本**: 1.0  
> **状态**: Active  
> **分支**: `develop/v3.6.0`  
> **维护人**: hermes-agent  
> **创建日期**: 2026-05-29  
> **Issue**: #1468

---

## 一、概述

本文档定义了 Executor Operator-level Coverage 的 SSOT（Single Source of Truth），建立 Operator 级别coverage 的测量标准、目标阈值和数据来源。

### 1.1 背景

- **当前状态**: 整体 coverage 87.35%（combined method）
- **问题**: 无法识别具体 Operator 的覆盖缺口
- **目标**: 建立 Operator 级别的 coverage SSOT，追踪每个执行算子的覆盖情况

### 1.2 适用范围

- Executor crate (`crates/executor`)
- 所有物理执行算子（Physical Operators）
- Operator 级别的 coverage 测量和报告

---

## 二、Operator 分类

### 2.1 算子分类矩阵

| 类别 | 算子 | 目标 Coverage |
|------|------|---------------|
| **Scan Operators** | SeqScan, IndexScan, IndexOnlyScan | 90% |
| **Filter Operators** | Selection, Filter | 85% |
| **Join Operators** | HashJoin, NestedLoop, MergeJoin | 80% |
| **Aggregation Operators** | HashAgg, SortAgg, SimpleAgg | 85% |
| **Sort/Limit Operators** | Sort, Limit, TopN | 85% |
| **Write Operators** | Insert, Update, Delete | 80% |

### 2.2 Source Files → Operator 映射

```
executor/src/
├── scan.rs                    → SeqScan
├── index_scan.rs              → IndexScan, IndexOnlyScan
├── filter.rs                  → Selection, Filter
├── merge.rs                   → MergeJoin
├── parallel_executor.rs       → HashJoin, NestedLoop
├── vectorization.rs           → Vectorized Operators
├── window_executor.rs         → Window Functions
├── ddl_executor.rs            → DDL Operators
├── local_executor.rs          → Main Executor
└── [tests/]                  → Test Coverage
```

---

## 三、Coverage 测量方法

### 3.1 工具链

- **工具**: `cargo-llvm-cov`
- **输出格式**: 
  - JSON (机器解析)
  - HTML (人工审查)
  - LCOV (CI 集成)

### 3.2 测量原则

1. **Per-file coverage**: 每个 Operator 源文件的行/分支覆盖率
2. **Per-operator targets**: 每个 Operator 有独立目标
3. **Combined reporting**: 汇总报告用于整体健康度判断

### 3.3 数据来源

```
target/coverage/
├── operator_coverage.json     # 每次测量生成
├── operator_coverage.html     # 可视化报告
└── lcov/
    └── executor.lcov          # LCOV 格式
```

---

## 四、Target 定义

### 4.1 Per-Operator Coverage Targets

| Operator | Category | Target | File(s) |
|----------|----------|--------|---------|
| SeqScan | Scan | 90% | scan.rs |
| IndexScan | Scan | 90% | index_scan.rs |
| IndexOnlyScan | Scan | 90% | index_scan.rs |
| Selection | Filter | 85% | filter.rs |
| HashJoin | Join | 80% | parallel_executor.rs |
| NestedLoop | Join | 80% | parallel_executor.rs |
| MergeJoin | Join | 80% | merge.rs |
| HashAgg | Aggregation | 85% | parallel_executor.rs |
| Sort | Sort/Limit | 85% | merge.rs |
| Limit | Sort/Limit | 85% | local_executor.rs |
| Insert | Write | 80% | local_executor.rs |
| Update | Write | 80% | local_executor.rs |
| Delete | Write | 80% | local_executor.rs |

### 4.2 Overall Health Criteria

- **Green**: 所有 Operator >= target
- **Warning**: 任何 Operator < target 但 >= (target - 5%)
- **Fail**: 任何 Operator < (target - 5%)

---

## 五、文件结构

### 5.1 Documentation Files

```
docs/releases/v3.6.0/
├── SPEC_COVERAGE_SSOT.md      # 本文档 (SSOT)
├── COVERAGE_MAP.md            # Operator → Test 文件映射
└── COVERAGE_TARGETS.md       # Per-operator 目标详情
```

### 5.2 Scripts

```
scripts/coverage/
├── check_operator_coverage.sh # 主脚本
├── scan_coverage_gaps.py       # Gap 分析
└── generate_overall_chart.py  # 趋势图表
```

---

## 六、报告格式

### 6.1 JSON Report Schema

```json
{
  "timestamp": "ISO8601",
  "version": "3.6.0",
  "overall_coverage": 87.35,
  "operators": [
    {
      "name": "SeqScan",
      "category": "Scan",
      "file": "scan.rs",
      "line_coverage": 92.5,
      "branch_coverage": 85.0,
      "target": 90.0,
      "status": "warning"
    }
  ]
}
```

### 6.2 Human-readable Output

```
OPERATOR COVERAGE REPORT
================================================================================
Overall: 87.35% | Target: 80% | Status: PASS
================================================================================
Scan Operators (Target: 90%)
  SeqScan        92.5%  [PASS]
  IndexScan      88.0%  [PASS]
  IndexOnlyScan  85.0%  [PASS]
Filter Operators (Target: 85%)
  Selection      86.0%  [PASS]
Join Operators (Target: 80%)
  HashJoin       78.0%  [FAIL]  <-- Below target
  NestedLoop     82.0%  [PASS]
  MergeJoin      75.0%  [FAIL]  <-- Below target
================================================================================
```

---

## 七、维护规则

1. **每次 PR 修改 executor 源码后**: 必须运行 `check_operator_coverage.sh`
2. **Coverage 下降 >1%**: 自动 blocking PR
3. **新增 Operator**: 必须同时添加 COVERAGE_TARGETS.md 条目
4. **版本升级**: 更新 SPEC 版本号和日期

---

## 八、相关文档

- [COVERAGE_MAP.md](./COVERAGE_MAP.md) - Operator 测试文件映射
- [COVERAGE_TARGETS.md](./COVERAGE_TARGETS.md) - 详细目标定义
- [scripts/coverage/check_operator_coverage.sh](../../scripts/coverage/check_operator_coverage.sh) - 测量脚本
