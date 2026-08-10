# V312-25: E2E 脚本去重与死脚本清理 — proposal

> **Issue**: V312-25（V312-24 Phase 2 follow-up）
> **Owner**: minimax
> **Expiry**: 2026-08-25
> **Source**: V312-24 proposal.md §Disposition + tasks.md Phase 2
> **Baseline evidence**: `docs/releases/v3.12.0/evidence/V312-25_baseline_evidence.txt`

## Why

V312-24 激活了 sqlancer / test-runner / test-registry 三套 `[[bin]]` 工具，
但仓库里仍保留 7 个 byte-identical stale mirror + 3 个 dead-code E2E 脚本。
这些脚本是 v3.10.0 test directory restructure（commit 3e4cd4accc）前的遗物，
留着只会让新人误以为它们是 active 测试。

**Baseline 实测**（2026-08-09）：
- `git ls-files` 命中 **11 个** 目标脚本（startup_connect / tpch_sf01 / kill9_recovery / e2e_01..e2e_08 共 11，因为 e2e_05/06/07/08 也匹配 `e2e_0[1-8]`）
- `git log --diff-filter=D` 命中 **0**（这些文件在 v3.12.0 还未被删过）
- `tests/baseline/ignore_registry.json` 当前 `ignored_tests` 数组 **74 entries**（无 V312-25 相关条目）

## What

`git rm` 10 个脚本（注意：matching 命中的 e2e_07 因为 V312-26 重写它，**不删**；e2e_05/06/08 因为是 dead-code 删），并在
`tests/baseline/ignore_registry.json` 内登记 10 条带 `owner + expiry` 的
retired 条目。

| 路径 | 类型 | 替代 |
|------|------|------|
| `tests/e2e/startup_connect.sh` | stale mirror | （无 active 替代） |
| `tests/e2e/tpch_sf01.sh` | stale mirror | `tests/e2e/union_set_ops.sh` 内含 TPCH SF=1 入口 |
| `tests/e2e/kill9_recovery.sh` | stale mirror | `scripts/gate/e2e/e2e_runner_exec.sh` 内的恢复段 |
| `scripts/gate/e2e/e2e_01_basic_crud.sh` | stale mirror | — |
| `scripts/gate/e2e/e2e_02_tx_commit_rollback.sh` | stale mirror | — |
| `scripts/gate/e2e/e2e_03_wal_crash_recovery.sh` | stale mirror | — |
| `scripts/gate/e2e/e2e_04_parallel_executor.sh` | stale mirror | — |
| `scripts/gate/e2e/e2e_05_savepoint_rollback.sh` | dead-code | — |
| `scripts/gate/e2e/e2e_06_cte_query.sh` | dead-code | — |
| `scripts/gate/e2e/e2e_08_migration.sh` | dead-code（mislabeled） | — |

## 关闭边界（必须全部满足，命令 + 数字 + 文件存在）

1. `git log --diff-filter=D --name-only --pretty=format: -- tests/e2e/startup_connect.sh tests/e2e/tpch_sf01.sh tests/e2e/kill9_recovery.sh scripts/gate/e2e/e2e_01_basic_crud.sh scripts/gate/e2e/e2e_02_tx_commit_rollback.sh scripts/gate/e2e/e2e_03_wal_crash_recovery.sh scripts/gate/e2e/e2e_04_parallel_executor.sh scripts/gate/e2e/e2e_05_savepoint_rollback.sh scripts/gate/e2e/e2e_06_cte_query.sh scripts/gate/e2e/e2e_08_migration.sh | sort -u | wc -l` 输出 **≥ 10**。
2. `git ls-files tests/e2e/ scripts/gate/e2e/ | grep -E '(startup_connect|tpch_sf01|kill9_recovery|e2e_0[1-8])' | wc -l` 输出 **≤ 1**（保留 e2e_07 给 V312-26 重写）。
3. `python3 -c "import json; d=json.load(open('tests/baseline/ignore_registry.json')); tests=d.get('ignored_tests',[]); v312_25=[t for t in tests if 'V312-25 retired' in t.get('reason','')]; print(len(v312_25))"` 输出 **≥ 10**。注意字段名是 `ignored_tests` 不是 `files`。
4. 关闭报告写到 `docs/releases/v3.12.0/V312-25_e2e_retire_report.md`，包含：
   - 实际 `git log` 输出（≥ 10 行 deletion records）
   - 实际 `git ls-files` 输出（≤ 1 行）
   - 10 个 retired 条目的 sha256
   - ignore_registry diff (before=74, after=84)
   - 文件路径 + owner + expiry
5. `cargo test --workspace --no-fail-fast 2>&1 | tail -3` 退出 **0**（删 stale 脚本后不能破坏现有 test）

## 禁止关闭条件

1. 仅以"openspec 标 done"或"报告里写已完成"为依据关闭。
2. 必须附实际 `git log` 输出 + `git ls-files` 输出 + ignore_registry diff。
3. **字段名错误**（用 `d.get('files',[])` 而不是 `d.get('ignored_tests',[])`）的 cross-check 视为未执行。
4. 不允许只删除文件不更新 ignore_registry（即只满足 1+2 不满足 3）。
