# Tasks — issue-4636 关联标量子查询

## T1: from-table 标量子查询执行

- [x] T1.1 提取 `is_no_table_scalar_subq` 判定助手
- [x] T1.2 新增 `execute_subquery_for_scalar_from_table`：影子保护列收集
- [x] T1.3 `substitute_outer_refs_in_select` 外层引用改写（裸列 + 限定名）
- [x] T1.4 `execute_select` 递归执行 + 首行首列提取（空 → NULL）

## T2: 死锁规避（预计算）

- [x] T2.1 在 `storage.write()` 之前按外层行预计算（WindowCall 同款位置对齐）
- [x] T2.2 projection 循环增加 `scalar_subq_results[col_idx]` 位置查找分支
- [x] T2.3 literal 形态保持原 lazy 路径

## T3: 测试

- [x] T3.1 新增 v312_75 集成测试 6 用例（注册 Cargo.toml）
- [x] T3.2 更新 v312_67 `with_table` 用例：报错断言 → 执行断言
- [x] T3.3 二进制实跑 issue 复现（`1,10` / `2,20`）

## T4: 回归

- [x] T4.1 correlated EXISTS / quantified / literal / parser_e2e / cte 全过
- [x] T4.2 TPC-H fixture 类失败（q13/q21/q2_q17）确认为环境缺失
- [x] T4.3 v312_62 3 败确认为 develop 基线已有
- [x] T4.4 触碰区域 fmt 干净（文件内其余 fmt 差异为基线已有）
