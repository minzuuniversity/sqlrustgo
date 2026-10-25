# 批次 46 补充证据 —— CI lint 预算门禁红（自修）

- 日期：2026-10-25
- Agent：`mcode-macmini`（Gitea id=32，`is_admin=false`）
- 关联 Issue：#5525
- 关联 PR：openclaw/sqlrustgo#5587（批次 46 逻辑备份/恢复）
- 分支：`mcode-macmini/logical-backup-restore-5525`
- 基线 commit：`b0563cf727e96d3abd35dd3da6860594ff8da5ed`

> 这份证据回答一个问题：**CI 上把 #5587 判红的那个 `lint-build` job，到底是代码红还是基础设施红？**
> 结论：**代码红，而且是批次 46 自己引入的**，已定位、修掉、复验。

---

## 1. CI 上的原始现象

| 项 | 值 |
|---|---|
| action run | `9972`（workflow `ci.yml` / "Hermes Pipeline"） |
| 触发 commit | `b0563cf727`（批次 46 的 HEAD） |
| 判红的 job | `21413` / `lint-build`，`completed / failure`，20:22:17Z → 20:35:35Z |
| 判红原因 | `scripts/gate/check_test_target_lint_budget.sh --scope workspace` 退出码 1 |

门禁原始输出（job 21413 日志）：

```
=== Test-Target Lint Budget Gate ===
  scope:    workspace
  baseline: tests/baseline/workspace_lint_budget.json
  running: cargo clippy --workspace --all-features --all-targets --message-format=json
  compile: OK (no errors)
  lint warnings: 706 across 69 lint name(s)
  [FAIL] 1 lint budget violation(s):
         clippy::cloned_ref_to_slice_refs: 3 warning(s) — this lint is NOT in the baseline
  [OK] the baseline was not modified (compare mode)
  ❌  Failure - Main Workspace lint budget (issue
exitcode '1': failure
```

注意 **root scope 在同一次 run 里是 PASS 的**（`lint warnings: 0 across 0 lint name(s)`），
红的是 workspace scope。

## 2. 归因（本地复现，不是转述）

在同一个 commit 上跑全 workspace clippy：

```
cargo clippy --workspace --all-features --all-targets --message-format=json
→ lint names: 68  total warnings: 705
→ cloned_ref_to_slice_refs: 3
```

与 CI 报的「3 条」逐条对上。3 条的位置全部落在**批次 46 重写过的测试文件**里：

| 位置 | 代码 | 门禁给的说明 |
|---|---|---|
| `crates/tools/tests/tools_backup_manager_tests.rs:154` | `mgr.create_backup("db1", &[table.clone()]).unwrap();` | unnecessary use of `clone` to create a slice from a reference |
| `crates/tools/tests/tools_backup_manager_tests.rs:169` | `let m1 = mgr.create_backup("mydb", &[table.clone()]).unwrap();` | 同上 |
| `crates/tools/tests/tools_backup_manager_tests.rs:171` | `let m2 = mgr.create_backup("mydb", &[table.clone()]).unwrap();` | 同上 |

**为什么基线里没有它**：`tests/baseline/workspace_lint_budget.json` 里从来没有
`cloned_ref_to_slice_refs` 这个 lint 名。门禁的规则是
「**基线里没有的 lint 名一律判红**」（`scripts/gate/check_test_target_lint_budget.sh`
第 27 行与 61–64 行的说明），所以哪怕只有 3 条也会 FAIL。

**为什么是批次 46 引入的**：旧的备份 API 是 `HashMap` 猜类型，批次 46 删掉它、
改成 `create_backup(database, &[BackupTable])`（按引用接收）之后，这三个用例为了
「同一张表备份两次、两次都要用」而写成了 `&[table.clone()]`。
`table.clone()` 在这里既多余又踩线 —— **clone 出来的值和原值一模一样**，
门禁 于是判它是「用 clone 造一个 slice」。

## 3. 修法

`crates/tools/tests/tools_backup_manager_tests.rs` 三处改成 `std::slice::from_ref(&table)`：

```rust
let table = BackupTable::new("t1", vec![BackupColumn::new("id", "INTEGER")]);
mgr.create_backup("db1", std::slice::from_ref(&table)).unwrap();
std::thread::sleep(std::time::Duration::from_secs(1));
mgr.create_backup("db2", std::slice::from_ref(&table)).unwrap();
```

`std::slice::from_ref(&table)` 直接把 `&BackupTable` 变成 `&[BackupTable]`，
**不产生 clone**，同时 `table` 仍然 owned、可被下一次 `create_backup` 复用 ——
语义与修前完全一致（三个用例断言的 id 唯一性、备份条数都没变）。

## 4. 验证（都是实跑）

| 检查 | 命令 | 结果 |
|---|---|---|
| 改动文件的用例 | `cargo test -p sqlrustgo-tools --test tools_backup_manager_tests` | **16 passed / 0 failed** |
| tools 全套件 | `cargo test -p sqlrustgo-tools --no-fail-fast` | **505 passed / 0 failed / 1 ignored**（与修前同） |
| 门禁 root scope | `bash scripts/gate/check_test_target_lint_budget.sh` | **PASS**，exit 0 |
| 门禁 workspace scope | `bash scripts/gate/check_test_target_lint_budget.sh --scope workspace` | **PASS**，exit 0，`lint warnings: 702 across 67 lint name(s)`，`[OK] no lint exceeds the baseline (total 702 <= 708, comparison=union)` |

修前是 `705 across 68`，修后是 `702 across 67` —— 差的正好是
**3 条警告 + 1 个 lint 名**，就是 `cloned_ref_to_slice_refs` 这一个名字。
其余 67 个 lint 名的条数没有变化。

本地 cargo 是 `1.97.0`，基线里只记了 `1.91.0` / `1.95.0`，所以本地走的是
`comparison=union`（与所有已记录 toolchain 的并集比），CI 走的是严格比较。
两种口径下该 lint 名都已不存在，因此**两种口径都绿**。

## 5. NOT RUN（明确没跑的）

| 项 | 原因 |
|---|---|
| `cargo test --workspace` 全量 | 会覆写 `docs/releases/v4.1.0/evidence/a04-5226/wire/*.txt` 这批归档证据；本次改动只碰一个测试文件，tools 全套件已覆盖 |
| CI 上的绿 run | 修复需要推送才会触发新 run，推送后需等 CI 给出终态；本文件写下时**尚无新的 run 终态** |
| `cargo test -p sqlrustgo-cli` / `-p sqlrustgo-mysql-server` / `-p sqlrustgo-storage --features fault-injection` | 本次改动只涉及 `crates/tools/tests/tools_backup_manager_tests.rs`，不触达这些 crate；批次 46 主证据里已有它们的数字 |

## 6. 结论

* 这是**代码红**，不是基础设施红 —— 与 #5578 run 9918 的「容器中途消失 / never ran」
  是两回事，不能混为一谈。
* 归属明确：**批次 46 自己引入**，修法是去掉三个多余的 clone。
* 修完之后，#5587 上这条 lint 预算门禁**本地两个 scope 都绿**；
  CI 上的绿证据要等推送后的新 run 终态才算数。
* 这条修复**没有**改动任何被门禁盯着的基线文件（`tests/baseline/*.json` 未修改，
  门禁自身的 compare mode 自检也确认了这一点）。
