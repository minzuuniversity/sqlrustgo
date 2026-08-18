# V312-56B: SQL 教学语料库 + 多 Oracle 对比

> **Issue**: #4252
> **Status**: COMPLETED
> **Branch**: develop/v3.12.0
> **HEAD at last refresh**: `6d1b1fe9c6f786319e81c37e7cd15bf0143e53cf`
> **Policy**: Anti-Fabrication-Policy-v1.0
> **PR (merge)**: #4327 @ `5c6e640edb`

## §1 Scope

闭合 v3.12.0 BETA 准入要求"SQL 教学语料库 + 多 Oracle 对比"任务,提供可执行 SQL 教学
fixture,覆盖 SELECT/JOIN/GROUP/ORDER/DDL/DML/EXPLAIN/prepared/transaction/error 11 个分类,
并接入多 oracle (SQLite + PostgreSQL + sqlrustgo) 行级 cross-engine diff。

## §2 Fixture 矩阵 (27+ entries in manifest.yml)

**位置**: `tests/compat/teaching_sql_v3_12/manifest.yml` (6195 bytes, 27 entries)

| 子目录 | fixture 数 | 主题 |
|--------|-----------|------|
| `select/` | 4 | basic / where / distinct / alias |
| `join/` | 2 | inner_join / left_join |
| `group/` | 3 | group_by / having / aggregate |
| `order_limit/` | 2 | order_by / limit_offset |
| `null/` | 2 | is_null / coalesce |
| `ddl/` | 2 | create_table / alter_table |
| `dml/` | 3 | insert / update / delete |
| `transaction/` | 1 | basic_tx |
| `prepared/` | 3 | basic / param_binding / multiple_execute |
| `explain/` | 5 | seq_scan / index_scan / hash_join / aggregate / sort_limit |
| `error/` | 1 | division_by_zero (expected FAIL) |
| **总计** | **28 .sql 文件 + 27 manifest entries** | — |

### §2.1 error/division_by_zero (诚实披露)

**预期状态**: FAIL — sqlrustgo 触发 division-by-zero error,manifest.yml 标记为
`expected: ERROR`。这是设计性 negative-path fixture,不是 bug。

## §3 Multi-oracle 对比

### §3.1 已接入 oracle

| Oracle | 来源 | 行级 diff 支持 |
|--------|------|---------------|
| **SQLite** | 本机 sandbox (in-process) | ✅ |
| **PostgreSQL** | sandbox `127.0.0.1:5432` | ✅ |
| **sqlrustgo** | `cargo test` 路径 | ✅ |
| **MySQL** | sandbox `127.0.0.1:3306` | ⚠️ 部分 — 见 §3.2 |

### §3.2 MySQL oracle 限制 (诚实披露)

**当前**: `tests/compat/teaching_sql_v3_12/` 的 compat-runner 仅跑 SQLite + PG + sqlrustgo
三向 oracle,MySQL oracle 在 `tests/integration/mysql_compat_test.rs` 中仅 spot-check
若干 fixture,未进入 cross-engine diff 全量对比。

**影响**: MySQL 兼容性仅在 PR-review smoke 阶段验证,不在 BETA gate 全量验证中。
**关闭路径**: 已有计划在 v3.13.0 S2 (TPC-H cross-engine) 同步推进 MySQL oracle
接入,见 `docs/superpowers/plans/2026-08-17-v313-master-scope.md` Task 6。

## §4 测试结果 (PR #4327 @ merge commit `5c6e640edb`)

| 检查项 | 结果 |
|--------|------|
| `tests/compat/teaching_sql_v3_12/` directory | PRESENT (28 .sql fixtures across 11 sub-dirs) |
| `tests/compat/teaching_sql_v3_12/manifest.yml` | PRESENT (27 entries: 26 expected PASS + 1 expected FAIL) |
| `tests/compat/teaching_sql_v3_12/transaction/basic_tx.sql` | PRESENT (教学 fixture) |
| `tests/compat/teaching_sql_v3_12/explain/seq_scan.sql` | PRESENT (教学 fixture) |
| `tests/compat/teaching_sql_v3_12/error/division_by_zero.sql` | PRESENT (negative-path) |
| `B6_V312_56_TEACHING_CORPUS` Beta gate check | PASS (per V312-56-VERIFICATION.md §10 行 199-200) |
| `B6_V312_56_EXPLAIN_FIXTURES` Beta gate check | PASS (5 EXPLAIN fixtures verified) |

## §5 Verification Commands (re-runnable)

```bash
# 1. 教学语料目录存在性
test -d tests/compat/teaching_sql_v3_12
# 期望:exit=0

# 2. fixture 计数
find tests/compat/teaching_sql_v3_12 -name "*.sql" | wc -l
# 期望:28

# 3. manifest 计数
grep -c "^  - id:" tests/compat/teaching_sql_v3_12/manifest.yml
# 期望:27 entries

# 4. EXPLAIN fixture 计数
find tests/compat/teaching_sql_v3_12/explain/ -name "*.sql" | wc -l
# 期望:5 (gate B6_V312_56_EXPLAIN_FIXTURES 要求)

# 5. 跑教学 corpus 集成测试 (compat runner)
cargo test --test teaching_corpus_runner_test --all-features
# 期望:26 passed (passing fixtures), 1 expected-error fixture correctly rejected

# 6. 跑 Beta gate (V312-56 子项)
bash scripts/gate/check_beta_v3.12.0.sh 2>&1 | grep -E "V312_56"
# 期望:[PASS] B6_V312_56_TEACHING_CORPUS
# 期望:[PASS] B6_V312_56_EXPLAIN_FIXTURES

# 7. 跑 sqllogictest gate (覆盖部分 56B fixtures)
bash scripts/gate/check_sqllogictest_v312.sh
# 期望:exit=0
```

## §6 Honest Disclosure (per Anti-Fabrication-Policy-v1.0)

1. **MySQL oracle 未进入全量 diff** — 仅 spot-check,见 §3.2。v3.13.0 S2 推进。
2. **`expected: ERROR` fixture = 1 个** — `error/division_by_zero.sql` 是设计性 negative
   fixture,manifest 显式标记,不是回归。
3. **fixture 覆盖度** — 当前覆盖 11 子目录 (SELECT/JOIN/GROUP/ORDER/DDL/DML/EXPLAIN/
   prepared/transaction/error/null),**不覆盖**: window function / CTE / subquery /
   recursive CTE / MERGE / UPSERT / UUID / JSON。这些均在 v3.13 RC1 roadmap。
4. **fixture locale = EN-only** — 所有 fixture 使用 ASCII SQL identifier + 英文 column
   name。Unicode collation / locale-dependent 排序尚未覆盖,留待 i18n sprint。

## §7 Round-24 Codex Evidence Compliance

| Round-24 要求 | 实际产出 |
|--------------|---------|
| 目标分支 | develop/v3.12.0 @ `6d1b1fe9c6` |
| 关联 commit SHA | `5c6e640edb` (PR #4327 merge),`0b429a85cd` (V312-56 master plan merge) |
| 关联 PR# + merge commit | PR #4327 @ `5c6e640edb` |
| Run cmd + exit | `bash scripts/gate/check_beta_v3.12.0.sh` exit=0,`B6_V312_56_TEACHING_CORPUS` PASS,`B6_V312_56_EXPLAIN_FIXTURES` PASS |
| 输出摘要 | 28 fixture files / 27 manifest entries / 2 oracles full-diff (SQLite+PG+sqlrustgo) + MySQL spot-check |
| Evidence hash(64-hex) | `sha256=1425168b6e0d8de714ae76c612384e295028d6fae9caa13c8eff56ce198f0acb` (computed 2026-08-18, content pre-append) |
| Remaining risk | MySQL oracle 未全量 diff (deferred to v3.13 S2); fixture locale coverage 待扩展 |

## §8 Provenance

- **Generated at**: 2026-08-18T14:35:00Z
- **Source repo**: openclaw/sqlrustgo
- **Branch**: develop/v3.12.0
- **HEAD commit**: `6d1b1fe9c6f786319e81c37e7cd15bf0143e53cf`
- **Policy**: Anti-Fabrication-Policy-v1.0
- **Source issue**: #4252
- **Supersedes**: 无 (新增 lab,沿用 V312-56C template 模式)