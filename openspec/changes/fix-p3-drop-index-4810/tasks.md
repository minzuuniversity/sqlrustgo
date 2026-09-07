# Tasks — Issue #4810: DROP INDEX actually drops

## 1. 现状审查 (PR #4789 验证)

- [ ] 1.1 `git log --oneline --all --grep "DROP INDEX"` 找到 PR #4789
      commit。
- [ ] 1.2 验证 PR #4789 commit 在当前 HEAD 的 ancestry:`git merge-base
      --is-ancestor <PR4789_sha> HEAD`。
- [ ] 1.3 `cargo test --test drop_index_test` 跑现有 v312-91 测试。
- [ ] 1.4 如果 1.3 全绿:写一个 minimal 复现测试,issue 报告者可能是
      binary cache 问题。结论:close as "verified"。
- [ ] 1.5 如果 1.3 有 FAIL:记下失败的测试名,定位 regression commit。

## 2. 定位 Root Cause (如果 1.5 FAIL)

- [ ] 2.1 在 `src/storage/metadata.rs` 找到 `list_all_indexes` 和
      `drop_index` 函数定义。
- [ ] 2.2 `git log -p src/storage/metadata.rs | grep -A 5 "list_all_indexes\|drop_index"`
      找最近改动。
- [ ] 2.3 `git log -p src/executor/ddl.rs | grep -A 10 "execute_drop_index"`
      找 executor 端改动。
- [ ] 2.4 跑 git bisect:
      ```bash
      git bisect start HEAD HEAD~50
      git bisect run cargo test --test drop_index_test
      ```
- [ ] 2.5 找到 root cause commit,记录 regression 类型。

## 3. 修复 (根据 root cause 类型)

- [ ] 3.1 如果 root cause = 代码丢失:重新添加 PR #4789 的核心改动。
- [ ] 3.2 如果 root cause = schema drift:unify CREATE/DROP INDEX 的
      metadata 读写字段。
- [ ] 3.3 如果 root cause = strict mode change:加回 idempotent 行为
      (或确认 strict 是预期)。
- [ ] 3.4 验证 minimal fix:anchor failing 测试现在 PASS。

## 4. Tests (6 tests)

- [ ] 4.1 `drop_index_existing_succeeds` — issue anchor:
      ```sql
      CREATE INDEX idx_a ON t(col);
      DROP INDEX idx_a;
      SELECT * FROM t WHERE col = 1;  -- 仍然正确(seq fallback)
      ```
- [ ] 4.2 `drop_index_if_exists_no_error` — `DROP INDEX IF EXISTS 
      no_idx`。
- [ ] 4.3 `drop_index_nonexistent_errors` — `DROP INDEX no_idx` 报错。
- [ ] 4.4 `drop_index_then_select_works` — DROP 后 SELECT 仍正确。
- [ ] 4.5 `drop_index_cascade_on_drop_table` — DROP TABLE 级联
      drop 索引。
- [ ] 4.6 `drop_index_with_compound_index` — `CREATE INDEX 
      idx_compound ON t(a,b); DROP INDEX idx_compound;`。

## 5. Verification

- [ ] 5.1 `cargo build --all-features` clean。
- [ ] 5.2 `cargo test --test p3_drop_index_4810_test` 6/6 PASS。
- [ ] 5.3 `cargo test --test drop_index_test` (v312-91) 5/5 PASS。
- [ ] 5.4 `cargo test --all-features --lib` no regression。
- [ ] 5.5 `openspec validate fix-p3-drop-index-4810 --strict` valid。

## 6. Commit + Memory

- [ ] 6.1 Commit message:
      `fix(P3 / #4810): DROP INDEX actually drops (regression guard)`
      或 `docs(P3 / #4810): verify PR #4789 still works`。
- [ ] 6.2 如果是 regression fix,在 `memory/` 新增 `p3-drop-index-4810.md`
      记录 regression type + 修复方法。
- [ ] 6.3 如果是 verification only,在 issue 评论说明 PR #4789 已
      修复,关闭 issue。
- [ ] 6.4 `openspec archive fix-p3-drop-index-4810` after close。

## 7. Out of Scope

- [ ] 7.1 DROP INDEX CONCURRENTLY — PostgreSQL specific, defer。
- [ ] 7.2 ALTER INDEX ... RENAME — defer。
- [ ] 7.3 DROP INDEX schema.db.idx — defer。