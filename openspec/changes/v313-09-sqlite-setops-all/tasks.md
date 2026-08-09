# V313-09 Tasks: 实现 EXCEPT ALL / INTERSECT ALL 多重集合语义

## 1. 分析当前实现

- [ ] 1.1 读取 `crates/executor/src/stored_proc.rs` 中 `execute_cte_subquery` 方法的 `INTERSECT ALL` 分支（第 1464-1471 行），确认占位符实现
- [ ] 1.2 读取 `EXCEPT ALL` 分支（第 1486-1494 行），确认占位符实现
- [ ] 1.3 运行 `setops__test_setops.test`，确认当前 `EXCEPT ALL / INTERSECT ALL` 查询失败及错误信息

## 2. 创建 multiset.rs 模块

- [ ] 2.1 在 `crates/executor/src/` 下新建 `multiset.rs`
- [ ] 2.2 实现 `multiset_intersect(left: &[Vec<Value>], right: &[Vec<Value>]) -> Vec<Vec<Value>>`
  - 使用 `HashMap<&Vec<Value>, usize>` 统计 right 中每行出现次数
  - 遍历 left，按 `min(count_left, right_count)` 保留
- [ ] 2.3 实现 `multiset_except(left: &[Vec<Value>], right: &[Vec<Value>]) -> Vec<Vec<Value>>`
  - 使用 `HashMap<&Vec<Value>, usize>` 统计 right 中每行出现次数
  - 遍历 left，每匹配一次 right 抵消一次，剩余保留
- [ ] 2.4 在 `crates/executor/src/lib.rs` 中添加 `pub mod multiset;`

## 3. 编写单元测试

- [ ] 3.1 在 `crates/executor/src/multiset.rs` 中添加 `#[cfg(test)]` 模块
- [ ] 3.2 测试 `multiset_intersect_basic`：基础交集 `[1,1,2] ∩ [1,2,2,3] = [1,2,2]`
- [ ] 3.3 测试 `multiset_intersect_no_overlap`：无重叠应返回空
- [ ] 3.4 测试 `multiset_except_basic`：基础差集 `[1,1,1,2] - [1,2] = [1,1]`
- [ ] 3.5 测试 `multiset_except_all_removed`：完全抵消应返回空
- [ ] 3.6 测试 NULL 值处理：`[NULL, NULL, 1] ∩ [NULL, 1] = [NULL, 1]`
- [ ] 3.7 测试 `cargo test -p sqlrustgo-executor multiset` 通过

## 4. 更新 stored_proc.rs

- [ ] 4.1 在 `stored_proc.rs` 顶部添加 `mod multiset;`（或确认已通过 `lib.rs` 导出）
- [ ] 4.2 替换 `INTERSECT ALL` 分支的占位符代码为 `Ok(multiset::multiset_intersect(&left_records, &right_records))`
- [ ] 4.3 替换 `EXCEPT ALL` 分支的占位符代码为 `Ok(multiset::multiset_except(&left_records, &right_records))`
- [ ] 4.4 保留原有的非 ALL（dedup）分支不变

## 5. 运行 sqllogictest 验证

- [ ] 5.1 构建：`cargo build -p sqlrustgo_sqllogictest`
- [ ] 5.2 运行 `setops__test_setops.test` 中的 `EXCEPT ALL / INTERSECT ALL` 查询
- [ ] 5.3 确认输出为：
  ```
  2  2
  3  1
  4  1
  ```
- [ ] 5.4 运行 `setops__test_except.test`（如有涉及）
- [ ] 5.5 运行 `cargo clippy --all-features -- -D warnings` 确认 lint 通过
- [ ] 5.6 运行 `cargo fmt --check` 确认代码格式正确

## 6. 更新 exclusions.yml

- [ ] 6.1 更新 `docs/releases/v3.12.0/evidence/sqllogictest/exclusions.yml`：
  - 将 `setops__test_except.test` 的 `DEFERRED: Query result mismatch` 改为 `PASS`
  - 将 `setops__test_setops.test` 的 `DEFERRED: EXCEPT ALL / INTERSECT ALL` 改为 `PASS`
- [ ] 6.2 记录 evidence hash 变更

## 7. PR 与合并

- [ ] 7.1 提交所有变更到特性分支
- [ ] 7.2 打开 PR 指向 `develop/v3.13.0`
- [ ] 7.3 获得至少 1 个 reviewer 批准
- [ ] 7.4 合并到 develop/v3.13.0
- [ ] 7.5 更新 ISSUE（如有），记录 PR 链接
