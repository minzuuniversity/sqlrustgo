## Context

Issue #4810 — `DROP INDEX idx` 在索引存在时报 "index does not exist"。
PR #4789 (v312-91 #4669) 在 v3.12.0 已经 merged 实现 DROP INDEX
executor。Issue #4810 出现在更晚的 commit,可能由于:

1. **Rebase 后代码丢失**: PR #4789 的 drop_index 在某个 rebase 中
   被覆盖或被其他 commit 改动破坏。
2. **Catalog schema drift**: PR #4789 用 `list_all_indexes` 列出索引,
   后续 commit 把索引 metadata 字段重命名,导致 `list_all_indexes` 
   找不到任何索引。
3. **Behavior change**: 某个 commit 把 drop_index 改为 strict mode
   (default error),破坏了之前 idempotent 行为。

## Approach

### A1. 先验证 PR #4789 是否仍生效

```bash
git log --oneline --all --grep "DROP INDEX"
# 找到 PR #4789 commit,验证仍然在当前 HEAD 的 ancestry
```

跑现有的 `drop_index_test` 看是否 5/5 PASS。如果 PASS,issue 是
misreport (issue 报告者用了旧 binary)。如果 FAIL,定位 regression。

### A2. 定位 regression

```bash
git bisect HEAD~50..HEAD cargo test --test drop_index_test
```

或手动 `git log -p src/storage/metadata.rs` 看哪个 commit 改了
`list_all_indexes` / `drop_index` 函数。

### A3. 修复

取决于 regression 类型:
- **代码丢失**: 重做 PR #4789 的核心改动 (5-10 行)。
- **Schema drift**: unify CREATE INDEX 和 DROP INDEX 的 metadata 读写。
- **Strict mode change**: 加回 idempotent 行为 (if index 不存在 → silent)。

### A4. 测试覆盖

写 6 个回归测试 pin 住正确行为:
- `drop_index_existing_succeeds` — 验证 drop 真发生。
- `drop_index_idempotent_with_if_exists` — IF EXISTS 不报错。
- `drop_index_strict_no_if_exists` — 不存在时报错。
- `drop_index_cascade_on_drop_table` — 级联。
- `drop_index_compound` — 多列索引。
- `drop_index_then_select_works` — DROP 后查询仍正确。

## Files Changed (likely minimal)

| File | Lines | Purpose |
|------|-------|---------|
| `src/execution_engine.rs` (或 executor/ddl.rs) | +20/-20 | 修复 drop_index 路径(取决于 root cause) |
| `tests/integration/sql/p3_drop_index_4810_test.rs` | +150 | 6 regression tests |

Total: 假设 5-10 行 fix + 150 行 test。

## Verification

| Test | Expected |
|------|----------|
| `cargo build --all-features` | clean |
| `p3_drop_index_4810_test` | 6/6 PASS |
| `drop_index_test` (v312-91) | 5/5 PASS (no regression) |
| `parser_e2e_test` | 249/249 PASS |

## Non-Goals

- DROP INDEX CONCURRENTLY
- ALTER INDEX ... RENAME
- DROP INDEX schema.db.idx form (deferred)

## Difficulty Tier

**🟢 EASY** — Issue 极可能是 fixed-by-existing-code regression,
不需要新算法,只需定位 root cause + 1-2 行 fix + 6 个 regression
test pinning 住行为。

If after Phase 1.1 (PR #4789 验证) all existing tests are green,
close as "verified, working as expected, no fix needed" 而不写
code。