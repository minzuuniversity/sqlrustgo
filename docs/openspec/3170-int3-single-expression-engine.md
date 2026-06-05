# openspec/3170 - INT-3 Single Expression Engine (P0-2 最小修复)

> **Issue**: #3170
> **作者**: Hermes Agent
> **日期**: 2026-06-05
> **Phase**: 1 (W1-2)
> **工作量**: 32h (总) → 本次最小修复 4h (聚焦 alpha1 准入识别的 6 个 parser dead_code)

## 一、问题分析

### 1.1 INT-3 历史背景

按 `docs/releases/v3.8.0/archived/INT_DEBT_REMEDIATION_PLAN.md §3`:
- INT-3 跨 v1.2.0+ 遗留债 (3 版本受影响)
- 原始范围: `crates/expr/` (有独立 expr) vs `executor/expression.rs` (并行)
- 32h 完整修复: 弃用 executor/expression + 迁移到 expr::Expression + 删重复

### 1.2 v3.9.0-alpha1 准入发现的实际范围

实际状态 (2026-06-05):
- `crates/expr/` 缺失 (历史计划但未实现)
- `executor/src/expr/UnifiedExpr` 是单一表达式实现 (已存在)
- 主表达式入口: `src/expr_utils.rs::evaluate_expression` (统一)
- 触发器专用: `executor/src/trigger_eval/expression.rs` (隔离)
- 谓词: `storage/predicate.rs` + `executor/predicate_compiler.rs` (分离)

### 1.3 v3.9.0-alpha1 准入识别的剩余问题

`crates/parser/src/parser.rs` 有 6 个 dead_code 警告 (alpha1 报告已记录):
```
warning: methods `parse_json_path_expression`, `parse_or_expression_until_close`,
  `parse_and_expression_until_close`, `parse_additive_expression_until_close`,
  `parse_multiplicative_expression_until_close`, and
  `parse_primary_expression_until_close` are never used
```

这些方法在 v3.8.0+ 中已实现但未被主路径调用, 是 P0-2 的"待消费"功能.

## 二、本次实施范围 (P0-2 最小修复)

### 2.1 范围限定

按 alpha1 准入检查 + 治理最小修改原则:

**本次 PR 范围**:
1. 修复 6 个 parser.rs dead_code 警告 (在 alpha1 加了 `#[allow(dead_code)]`)
   - 移除 `#[allow(dead_code)]` 属性
   - 让方法在主路径中被调用
2. 新增 G3 gate (类似 G4): `scripts/gate/check_int3_single_expr.sh`
3. 新增测试: `tests/int3_single_expression_test.rs`
4. 验证: L1 unit + wal_tx_contract + TPC-H 22/22 + G3 gate 全 PASS

**延后 (推 v3.9.0+ 后续或 v3.10)**:
- 完整 `crates/expr/` 创建 (32h 工作)
- executor/expression.rs 弃用
- UnifiedExpr vs parser::Expression 整合
- 14 委托分支全面合并

### 2.2 6 个 dead_code 方法的处理

#### 2.2.1 parse_json_path_expression (line 3589)

**当前**: 已实现, 未被主路径调用
**修复**: 在 `parse_primary_expression` 中检测 `Token::JsonArrow/JsonArrowText` 时调用此方法
**预期**: 死代码警告消失, JSON path 表达式通过主路径处理

#### 2.2.2 parse_or_expression_until_close (line 3806)

**当前**: 已实现, 未被主路径调用
**修复**: 在 `parse_expression_in_parens` (line 2510) 已有 `parse_or_expression` 调用, 修改为调用 `_until_close` 变体
**预期**: 正确处理括号内 OR 表达式 (避免泄漏到外层)

#### 2.2.3 parse_and_expression_until_close (line 3816)

**当前**: 已实现, 未被主路径调用
**修复**: 同上, 在 `parse_or_expression_until_close` 内调用 `parse_and_expression_until_close`
**预期**: 嵌套表达式正确停止于 RParen

#### 2.2.4 parse_additive_expression_until_close (line 3826)

**当前**: 已实现, 未被主路径调用
**修复**: 在 `parse_and_expression_until_close` 内调用
**预期**: 嵌套表达式加法正确

#### 2.2.5 parse_multiplicative_expression_until_close (line 3841)

**当前**: 已实现, 未被主路径调用
**修复**: 在 `parse_additive_expression_until_close` 内调用
**预期**: 嵌套表达式乘法正确

#### 2.2.6 parse_primary_expression_until_close (line 3871)

**当前**: 已实现, 未被主路径调用
**修复**: 在 `parse_multiplicative_expression_until_close` 内调用
**预期**: 嵌套表达式基础元素正确

## 三、变更设计

### 3.1 parser.rs 修改

最小修改 (按 governance §2.1):
- 删除 6 个 `#[allow(dead_code)]` 属性
- 修改调用点使其在主路径中被消费
- 不删除方法 (保留功能, 仅启用)

### 3.2 G3 gate 脚本

`scripts/gate/check_int3_single_expr.sh`:
- 检查 6 个 dead_code 警告消失 (clippy 0 warnings in parser)
- 检查主路径含 parse_*_until_close 调用链
- 检查 parse_json_path_expression 被调用

### 3.3 测试

`tests/int3_single_expression_test.rs`:
- JSON path 表达式 (`col -> '$.path'`)
- 嵌套括号表达式 (含 OR/AND/+/-)
- 混合算术与比较的嵌套

## 四、风险评估

| 风险 | 影响 | 缓解 |
|------|------|------|
| 调用链修改破坏现有解析 | TPC-H 22/22 失败 | 严格保留原方法签名 |
| JSON path 引入歧义 | 解析错误 | 现有实现已通过 corpus 测试 |
| 嵌套表达式停止点错误 | 解析提前终止 | 完整 TPC-H 回归 |

## 五、验收标准 (G3 门禁 - 本次范围)

```
✅ cargo check RC=0
✅ cargo test -p sqlrustgo-parser --lib: 110/110 PASS
✅ L1 unit tests 全 PASS (871/871)
✅ wal_tx_contract: 26/26 PASS
✅ TPC-H 22/22 (G1 维持)
✅ Clippy parser: 0 dead_code 警告 (6 个修复)
✅ G3 gate: PASS
✅ int3_single_expression_test: 4 tests PASS
```

## 六、Subsumed Issues

完成后, 关闭:
- **#3108 (部分)**: INT-2/INT-3 集成债务 (本任务关闭 INT-3 部分)
- **#3146 (部分)**: INT-3 expr 完整合并 (本任务关闭 dead_code 部分)

INT-2 完整集成推 P0-3 (#3171) 处理.

## 七、回滚计划

如 TPC-H 回归:
1. Revert commit
2. 恢复 `#[allow(dead_code)]` 属性
3. 重新分析调用链 (可能需要更大重构)

## 八、依赖

**上游**: 无
**下游**: P0-3 (INT-2 集成) 阻塞至 G3 PASS

## 九、参考资料

- Issue #3170
- docs/releases/v3.8.0/archived/INT_DEBT_REMEDIATION_PLAN.md §3
- docs/openspec/3169-arch3-vtu-main-path.md (P0-1 经验)
- docs/releases/v3.8.0/alpha/ALPHA_GATE_REPORT_V390.md (准入检查)
- V390_DEVELOPMENT_PLAN.md §P0-2
