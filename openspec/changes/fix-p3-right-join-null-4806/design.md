## Context

Issue #4806 — RIGHT JOIN unmatched right-side 行,left columns 序列化为
空字符串而非 NULL。

v312-95 PR #4789 / commit `2dbdcaed66` 已经添加测试
`test_full_outer_join_preserves_both_sides`,验证 FULL OUTER JOIN
path 在 engine_select.rs:4408-4474 正确处理 unmatched row。

Memory 中已有记录:
> v312-95 #4639 RIGHT JOIN / FULL OUTER JOIN closure fixed-by-existing-
> code; new test_full_outer_join_preserves_both_sides (commit 
> 2dbdcaed66) confirms engine_select.rs:4408-4474 hash-join path

所以本 PR 主要工作可能是 verification 而非 new fix。

## Approach

### A1. 验证 v312-95 修复仍生效

```bash
git log --oneline --all --grep "RIGHT JOIN\|full outer"
cargo test --test <v312_95_test_name>
```

### A2. 写 issue-specific 回归测试

按 `proposal.md` 的 6 个 scenario 写测试,pin 住具体行为:
- `right_join_unmatched_returns_null` (anchor)
- `right_join_null_is_not_empty_string`
- `right_join_coalesce_returns_default`
- `right_join_count_null_works`
- `right_join_full_outer_null`
- `right_join_left_columns_typed_null`

### A3. 如果发现 regression

定位 root cause(可能是 serializer / display formatter):
- `grep -rn "unwrap_or_default\|unwrap_or(\"\")\|.unwrap_or(String::new())" src/`
- 在 RIGHT JOIN path 检查 unmatched row 构造。

### A4. 如果根因是 formatter

修复 client display,确保 NULL 显式显示为 "NULL":
- REPL output format
- 网络协议 wire format
- CSV/JSON export (out of scope)

## Files Changed (likely minimal)

| File | Lines | Purpose |
|------|-------|---------|
| `src/executor/join.rs` 或 engine_select.rs | +0 to +20 | 仅在 regression 时修复 |
| `src/display.rs` (formatter) | +5 to +15 | 仅在 formatter bug 时修复 |
| `tests/integration/sql/p3_right_join_null_4806_test.rs` | +120 | 6 regression tests |

Total: 0-35 行 fix + 120 行 test。

## Verification

| Test | Expected |
|------|----------|
| `cargo build --all-features` | clean |
| `p3_right_join_null_4806_test` | 6/6 PASS |
| `test_full_outer_join_preserves_both_sides` (v312-95) | PASS (no regression) |
| `right_join_test` (any) | PASS |
| `parser_e2e_test` | 249/249 PASS |

## Non-Goals

- LEFT JOIN NULL serialization (separate if reported)
- Display formatter changes (only if executor confirmed NULL → ''; 
  unlikely)
- CSV / JSON export NULL handling (out of scope)

## Difficulty Tier

**🟢 EASY** — Issue 极可能是 fixed-by-existing-code,本 PR 主要工作是
写 6 个 regression 测试 pin 住行为,关闭 issue。如果发现 regression,
5-20 行 fix 即可。

## Reference

- [[v312-95-4639-right-full-join-closure]] — v312-95 PR #4789 已 merged
  修复 FULL OUTER JOIN,engine_select.rs:4408-4474 hash-join path。