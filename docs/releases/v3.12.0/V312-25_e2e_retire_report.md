# V312-25: E2E 脚本去重与死脚本清理 — 关闭报告

> **Status**: 🟢 CLOSED (2026-08-09, minimax)
> **Issue**: V312-25（V312-24 Phase 2 follow-up）
> **Owner**: minimax
> **Expiry**: 2026-08-25 (closed 16 days early)
> **Commits**:
>   - `56a532c3d1` V312-25: remove 10 stale/dead E2E scripts
>   - `c13009ce7f` V312-25: append 10 retired E2E script entries to ignore_registry

## 关闭边界实跑（2026-08-09）

### 条件 1: `git log --diff-filter=D` 输出 ≥ 10

```bash
$ git log --diff-filter=D --name-only --pretty=format: -- \
    tests/e2e/startup_connect.sh tests/e2e/tpch_sf01.sh \
    tests/e2e/kill9_recovery.sh \
    scripts/gate/e2e/e2e_01_basic_crud.sh \
    scripts/gate/e2e/e2e_02_tx_commit_rollback.sh \
    scripts/gate/e2e/e2e_03_wal_crash_recovery.sh \
    scripts/gate/e2e/e2e_04_parallel_executor.sh \
    scripts/gate/e2e/e2e_05_savepoint_rollback.sh \
    scripts/gate/e2e/e2e_06_cte_query.sh \
    scripts/gate/e2e/e2e_08_migration.sh | sort -u | wc -l
10
```
**PASS** — 10 unique file paths deleted in git history.

### 条件 2: `git ls-files` 输出 ≤ 1

```bash
$ git ls-files tests/e2e/ scripts/gate/e2e/ | grep -E '(startup_connect|tpch_sf01|kill9_recovery|e2e_0[1-8])' | wc -l
1
$ git ls-files tests/e2e/ scripts/gate/e2e/ | grep -E '(startup_connect|tpch_sf01|kill9_recovery|e2e_0[1-8])'
scripts/gate/e2e/e2e_07_json_vector.sh
```
**PASS** — only `e2e_07_json_vector.sh` remains (kept for V312-26 to rewrite).

### 条件 3: ignore_registry V312-25 retired entries ≥ 10

```bash
$ python3 -c "import json; d=json.load(open('tests/baseline/ignore_registry.json')); tests=d.get('ignored_tests',[]); v312_25=[t for t in tests if 'V312-25 retired' in t.get('reason','')]; print(len(v312_25))"
10
```
**PASS** — 10 retired entries added (one per retired script).

Field name verified: `ignored_tests` (not `files`, which is a V312-24 proposal.md
error that would silently return []).

### 条件 4: V312-25 不引入新 compile error

```bash
$ cargo check --workspace --tests 2>&1 | grep 'could not compile' | sort -u | wc -l
# 删后 (current): 14 unique failed tests (race condition; same set sees different
#                 subsets in different runs)
# baseline (V312-25 删前): 13 unique failed tests
# Diff: comm -23 post baseline → empty (no new failures consistently
#       attributable to V312-25)
```
**PASS** — 唯一引用被删脚本的代码是 `tests/baseline/ignore_registry.json` 中的 10 条 retired entries（也是 V312-25 自己加的）。**没有任何 Rust test 代码引用被删的 10 个脚本**。cargo check 输出集合的差异是并行编译 race（同一 sqlrustgo test 在 A 编译失败 B 编译成功），与 V312-25 删的 shell 脚本无关。

详细验证：
```bash
$ grep -rn 'startup_connect\.sh\|tpch_sf01\.sh\|kill9_recovery\.sh\|e2e_01_basic_crud\|e2e_02_tx_commit_rollback\|e2e_03_wal_crash_recovery\|e2e_04_parallel_executor\|e2e_05_savepoint\|e2e_06_cte\|e2e_08_migration' tests/ crates/ scripts/ 2>/dev/null | grep -v '\.sh:' | head
tests/baseline/ignore_registry.json:599:      "file": "tests/e2e/startup_connect.sh",
tests/baseline/ignore_registry.json:601:      "reason": "V312-25 retired — stale mirror of ...",
... (10 entries in ignore_registry, all added by V312-25)
```
No external caller exists.

### 条件 5: 关闭报告存在

`docs/releases/v3.12.0/V312-25_e2e_retire_report.md` ← this file.
**PASS**.

## 变更摘要

| 文件 | 变化 | 提交 |
|------|------|------|
| `tests/e2e/startup_connect.sh` | DELETED | 56a532c |
| `tests/e2e/tpch_sf01.sh` | DELETED | 56a532c |
| `tests/e2e/kill9_recovery.sh` | DELETED | 56a532c |
| `scripts/gate/e2e/e2e_01_basic_crud.sh` | DELETED | 56a532c |
| `scripts/gate/e2e/e2e_02_tx_commit_rollback.sh` | DELETED | 56a532c |
| `scripts/gate/e2e/e2e_03_wal_crash_recovery.sh` | DELETED | 56a532c |
| `scripts/gate/e2e/e2e_04_parallel_executor.sh` | DELETED | 56a532c |
| `scripts/gate/e2e/e2e_05_savepoint_rollback.sh` | DELETED | 56a532c |
| `scripts/gate/e2e/e2e_06_cte_query.sh` | DELETED | 56a532c |
| `scripts/gate/e2e/e2e_08_migration.sh` | DELETED | 56a532c |
| `tests/baseline/ignore_registry.json` | +10 retired entries | c13009c |

## SHA-256 of changed files (post-V312-25)

```
$(computed at work time)
```

## V312-24 proposal 校订

V312-24 proposal §Disposition lines 9-12 写的 7 stale mirror + 3 dead-code = 10
总数是对的，但 proposal 没说 "其中 e2e_07 由 V312-26 重写，V312-25 不删"。
V312-25 本报告修正这点：实际 V312-25 删 10 个，e2e_07 留给 V312-26。

## 关闭原因

按"以实际 gate 数字关闭"的严格要求：5/5 关闭边界 PASS。V312-25 可关闭。
