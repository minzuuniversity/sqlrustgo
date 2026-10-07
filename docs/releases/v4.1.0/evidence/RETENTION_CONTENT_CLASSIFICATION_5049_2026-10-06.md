# #5049 收尾：retention 配额按内容分类，而非 manifest 标签

- **日期**：2026-10-06
- **Issue**：#5049
- **分支**：`fix/5049-retention-classification`（worktree `~/workspace/dev/sqlrustgo-worktrees/wt-5055`）
- **基线**：`develop/v4.1.0` @ `4410bf68b5`（含 PR #5064）

---

## 1. 现状核实：issue 记的是漂移，漂移已消解，但没东西在守

#4938 把「导出全部表」的增量备份标签从 `Incremental` 改成 `Full`，因为它确实是全量导出；
#5048 打通了真增量。现状是**三个**生产者：

| 生产者 | manifest 标签 | 实际内容 |
|---|---|---|
| `create_full_backup_from_storage` (@517) | `Full` | 全量导出 |
| `create_incremental_backup_from_storage` (@645) | `Full` | 全量导出（虽然名字叫 incremental） |
| `create_incremental_backup_with_changeset` (@875) | `Incremental` | 真差量，写 `changes.json` |

标签今天**确实**和内容一致了 —— issue 任务 2 成立。但没有任何东西**强制**这一点，
而它已经滑过一次（#4938 之前，全量导出被标成 `Incremental`，于是 `keep_incremental`
在数全量导出、`keep_full` 形同虚设）。

由一个自己申报的字符串决定删除哪些备份，离「再写错一次就删错」只有一步之遥。

## 2. 修复

`crates/tools/src/backup.rs`

新增 `carries_changeset(path, manifest) -> bool`：一个备份是差量，当且仅当它
**同时**满足

1. 目录里有 `changes.json`（真携带变更集），且
2. manifest 的 `tables` 为空（没有导出整表）。

两个条件缺一不可：

- `changes.json` 只在有可重放变更时才写；
- 空库的全量备份 `tables` 合法地为空。

`apply_retention_policy` 的两个 bucket 改用该函数分类，不再读 `manifest.backup_type`。

### 任务 3：`keep_incremental` 对全量导出目录的行为 —— 明确为「不报错」

一个全是全量导出的目录**没有差量类**，`keep_incremental` 因此无人可施。
**这不是错误** —— 备份目录本来就可以一个差量都没有。全量导出无论 manifest 怎么写，
一律消耗 `keep_full` 配额。已由 `test_retention_on_a_full_export_only_directory_uses_keep_full_alone`
钉住（含 `keep_incremental = 0` 不得变成「全删」这一条）。

### 接线决定

**不接线**，`#[allow(dead_code)]` 保留。#5049 验收标准写的是「**若**决定接线该函数」，
这是有条件的；本 PR 不接线，并在函数文档注释里写明该属性为何仍在，
避免日后有人以为它已接线或以为它已被弃用。

## 3. 测试

替换掉原 `test_backup_retention_policy`。**替换理由**（原测试是无效证据）：

- 它对 `apply_retention_policy` 的返回值和实际删除结果**零断言** —— 函数返回什么都过；
- 它用固定路径 `temp_dir/retention_test`，并发跑会互相删对方的 fixture；
- 它构造的 5 个目录标签与内容自洽，分类逻辑改成什么都不会被发现。

新增 3 项（`crates/tools/src/backup.rs` 内 `mod tests`）：

| 用例 | 钉住什么 |
|---|---|
| `test_retention_classifies_by_content_not_by_label` | 标签与内容**故意矛盾**的目录：被误标 `Incremental` 的全量导出、被误标 `Full` 的真差量。配额必须跟内容走 |
| `test_retention_on_a_full_export_only_directory_uses_keep_full_alone` | 任务 3 的语义 |
| `test_retention_requires_both_signals_for_a_delta` | `&&` 两半都要 |

辅助 `seed_backup` 让「标签」与「磁盘内容」可独立设置，`unique_root` 让临时目录按
进程 + 线程唯一。

`cargo test -p sqlrustgo-tools --all-features`：**0 失败**（155 + 若干 target）。

## 4. 变异验证

| 变异 | 做法 | 结果 | 判定 |
|---|---|---|---|
| **M9** | 分类改回按 `manifest.backup_type` 过滤（issue 验收标准指定的那一条） | 2 项 FAILED：`test_retention_classifies_by_content_not_by_label`、`test_retention_requires_both_signals_for_a_delta` | **CAUGHT** |
| **M10** | 去掉 `tables.is_empty()` 这半个条件，只留 `changes.json` | 1 项 FAILED：`test_retention_requires_both_signals_for_a_delta` | **CAUGHT（补 fixture 后）** |

### M10 首轮存活 —— 又一处 fixture 退化

第一版 `test_retention_requires_both_signals_for_a_delta` 用了
`keep_incremental = 0`。配额为 0 时，任何被归入差量类的备份都会被**无条件删除**，
于是「它落在哪一类」这件事被彻底掩盖 —— 无论分类成差量还是全量，删除结果都一样。

改用 `keep_incremental = 1`，并把 fixture 扩到三个目录（让全量类有两个成员，
`keep_full = 1` 才会真正做取舍），M10 才被抓住。测试的注释里写明了这一条，
以免日后有人「简化」回 0。

## 5. 登记

`tests/baseline/v4.1_fix_pr_test_registry.json` 已登记 PR 5064；
`bash scripts/gate/check_fix_pr_test_registry.sh` → `[PASS] summary: v4.1_fix_pr_test_registry gate PASS (warnings=0)`。
