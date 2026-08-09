# V313-06 Tasks: MEDIAN 聚合函数实现

## 1. 分析当前 MEDIAN 实现状态

- [ ] 1.1 读取 `crates/planner/src/lib.rs` 的 `AggregateFunction` 枚举，确认当前变体列表
- [ ] 1.2 读取 `crates/executor/src/expr/mod.rs` 的 `eval_fn` 函数，确认 MEDIAN 分发位置
- [ ] 1.3 读取 `tests/compat/mysql_v3_12/median_unsupported.sql` 和 `.out`，确认 deferred fixture 期望行为
- [ ] 1.4 读取 `crates/executor/src/parallel_group_by.rs` 的 `AggregateCall::update` 方法，确认聚合函数更新框架

## 2. 在 AggregateFunction 枚举中添加 Median 变体

- [ ] 2.1 在 `crates/planner/src/lib.rs` 的 `AggregateFunction` 枚举中添加 `Median` 变体
- [ ] 2.2 更新 `AggregateFunction` 的 `Display` 实现，添加 `"MEDIAN"` 分支
- [ ] 2.3 运行 `cargo test -p sqlrustgo-planner AggregateFunction` 确认枚举测试通过

## 3. 实现 median_aggregate 函数

- [ ] 3.1 在 `crates/executor/src/expr/mod.rs` 中实现 `median_aggregate(args: &[Value]) -> Value` 函数：
  - 收集所有非 NULL 数值到 Vec
  - 对 Vec 排序
  - 根据元素个数返回中位数（奇数取中间，偶数取平均）
- [ ] 3.2 在 `eval_fn` 匹配分支中添加 `"MEDIAN" => median_aggregate(args)` 条目
- [ ] 3.3 处理类型边界：全 NULL → Null，空集合 → Null，非数值类型 → Null
- [ ] 3.4 添加 `#[test] fn test_median_odd_elements()` 测试：验证奇数元素 [1,2,3,4,5] → 3
- [ ] 3.5 添加 `#[test] fn test_median_even_elements()` 测试：验证偶数元素 [1,2,3,4] → 2.5
- [ ] 3.6 添加 `#[test] fn test_median_empty()` 测试：验证空输入 → Null
- [ ] 3.7 添加 `#[test] fn test_median_with_nulls()` 测试：验证 NULL 跳过逻辑
- [ ] 3.8 运行 `cargo test -p sqlrustgo-executor median` 确认所有 MEDIAN 测试通过

## 4. 创建基础 MEDIAN fixture

- [ ] 4.1 新建 `tests/compat/mysql_v3_13/median_basic.sql`：
  - 验证奇数元素 MEDIAN
  - 验证偶数元素 MEDIAN（中间两值平均）
  - 验证全 NULL 输入
  - 验证空表
- [ ] 4.2 新建 `tests/compat/mysql_v3_13/median_basic.out`，记录期望输出
- [ ] 4.3 将 fixture 路径加入 compat-runner 扫描范围

## 5. 创建分组 MEDIAN fixture

- [ ] 5.1 新建 `tests/compat/mysql_v3_13/median_grouped.sql`：
  - 验证 GROUP BY 场景下的 MEDIAN
  - 多分组分别计算中位数
- [ ] 5.2 新建 `tests/compat/mysql_v3_13/median_grouped.out`，记录期望输出

## 6. 更新 deferred fixture

- [ ] 6.1 将 `tests/compat/mysql_v3_12/median_unsupported.sql` 的 `# expect: DEFERRED: returns NULL instead of error` 改为 `# expect: PASS`
- [ ] 6.2 更新对应的 `median_unsupported.out`，记录实际输出的中位数值
- [ ] 6.3 若有 deferred 证据目录，删除旧的 deferred 日志

## 7. 运行 Runner 验证

- [ ] 7.1 运行 `cargo test -p sqlrustgo-executor median` 确认所有 MEDIAN 单元测试通过
- [ ] 7.2 运行 `cargo test --all-features` 确认全量测试通过（无回归）
- [ ] 7.3 运行 `cargo clippy --all-features -- -D warnings` 确认 lint 通过
- [ ] 7.4 运行 `cargo fmt --check` 确认代码格式正确
- [ ] 7.5 运行 `./scripts/gate/run_compat_tests.sh`（或等价命令），确认 `median_basic`、`median_grouped`、`median_unsupported` fixture 全部 PASS

## 8. PR 与合并

- [ ] 8.1 提交所有变更到特性分支
- [ ] 8.2 打开 PR 指向 `develop/v3.13.0`
- [ ] 8.3 获得至少 1 个 reviewer 批准
- [ ] 8.4 合并到 develop/v3.13.0
