# V312-26: E2E 脚本 WARN-only 修复 — proposal

> **Issue**: V312-26（V312-24 Phase 3 follow-up）
> **Owner**: minimax
> **Expiry**: 2026-08-30
> **Source**: V312-24 tasks.md Phase 3

## Why

3 个 E2E 脚本以 WARN-only 形式掩盖真实失败：
- `tests/e2e/backup_restore.sh` 用 `|| true` 吞错并假 re-insert 数据
- `tests/e2e/sysbench_wired.sh` 用 `if X; then PASS; fi` 静默 fallback
- `scripts/gate/e2e/e2e_07_json_vector.sh` 用 `grep -q name` 占位断言

按 V312-24 acceptance 条款："已知 broken test binaries 不得继续靠 WARN-only 掩盖"。

## What

重写 3 个脚本为真实断言（无 `|| true`），并补 1 个验证脚本 `scripts/gate/check_no_warn_only.sh`。
重写 + 验证两个动作缺一不可。

## 关闭边界（必须全部满足，命令 + 数字 + 文件存在）

1. `grep -rn '|| true' tests/e2e/backup_restore.sh tests/e2e/sysbench_wired.sh scripts/gate/e2e/e2e_07_json_vector.sh` 输出 **0 行**。
2. `bash scripts/gate/e2e/e2e_07_json_vector.sh` 退出码 **≠ 0** 当 fixture 缺失时（验证 byte-exact 断言生效，不再静默 PASS）。
3. `bash tests/e2e/backup_restore.sh` 包含 `mysqldump` + `mysql` round-trip 证据写入 `/tmp/backup_restore_evidence.txt`，文件 size **> 100 字节** 且包含 `restore` 关键字。
4. `bash tests/e2e/sysbench_wired.sh` 在 `sysbench --version` 缺失时 exit **1** + stderr 含 `sysbench not found`。
5. 关闭报告：`docs/releases/v3.12.0/V312-26_warn_only_fix_report.md`，含 4 个脚本的 diff + 执行 log + sha256。

## 禁止关闭条件

仅以"测试通过"或"无 `|| true`"为依据，必须含**真实失败注入**测试。
每个脚本必须额外跑一次"故意制造失败输入"的命令并验证退出码 ≠ 0。
