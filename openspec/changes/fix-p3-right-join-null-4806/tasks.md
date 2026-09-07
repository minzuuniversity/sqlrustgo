# Tasks — Issue #4806: RIGHT JOIN NULL serialization

## 1. 验证 v312-95 修复

- [ ] 1.1 `git log --oneline --all --grep "RIGHT JOIN\|full outer"`
      找 v312-95 PR #4789 commit (`2dbdcaed66` per memory)。
- [ ] 1.2 跑现有 `test_full_outer_join_preserves_both_sides` 测试
      (commit `2dbdcaed66` 引入的)。
- [ ] 1.3 检查 `src/executor/engine_select.rs:4408-4474` hash-join 
      path。
- [ ] 1.4 验证 issue 报告者的具体 SQL:
      ```sql
      CREATE TABLE l (id INT, val TEXT);
      CREATE TABLE r (id INT, val TEXT);
      INSERT INTO l VALUES (1, 'a');
      INSERT INTO r VALUES (2, 'b');
      SELECT l.*, r.* FROM l RIGHT JOIN r ON l.id = r.id;
      ```
      - 期望: 1 行, l.id = NULL, l.val = NULL, r.id = 2, r.val = 'b'
      - 当前: ?

## 2. 现状评估

- [ ] 2.1 如果 1.4 输出正确 (l.id IS NULL): issue 是误报或 display 
      bug,关闭。
- [ ] 2.2 如果 1.4 输出 l.id = '' (字符串): 定位根因:
      - join executor 真的填了 ''?
      - 还是 storage 存 NULL 但 display 格式化空?
- [ ] 2.3 验证 with `SELECT l.id IS NULL FROM l RIGHT JOIN r ...`:
      如果 client 显示 `true`,storage 正确;如果 `false`,真 bug。
- [ ] 2.4 用 storage probe (e.g., `SELECT typeof(l.id) FROM l RIGHT 
      JOIN r`) 验证 column type 和 value。

## 3. 修复 (根据 2.x)

- [ ] 3.1 如果 executor bug:在 engine_select.rs:4408-4474 的 hash-join 
      unmatched row 构造中,unmapped column 填 `Value::Null`。
- [ ] 3.2 如果 formatter bug:修复 display 路径,把 `None` 显示为
      `NULL` 而不是空字符串。
- [ ] 3.3 如果 storage serialization bug:修复 `Row → Vec<Value>` 
      序列化,保留 None → NULL。
- [ ] 3.4 验证 minimal fix 后 anchor SQL 正确。

## 4. Tests (6 tests)

- [ ] 4.1 `right_join_unmatched_returns_null` — anchor。
- [ ] 4.2 `right_join_null_is_not_empty_string` — `IS NULL` 判断
      正确。
- [ ] 4.3 `right_join_coalesce_returns_default` — `COALESCE` 返回
      default (非 '')。
- [ ] 4.4 `right_join_count_null_works` — `COUNT(l.id)` 不计 NULL。
- [ ] 4.5 `right_join_full_outer_null` — FULL OUTER JOIN 一致行为。
- [ ] 4.6 `right_join_left_columns_typed_null` — column type 保持
      原类型。

## 5. Verification

- [ ] 5.1 `cargo build --all-features` clean。
- [ ] 5.2 `cargo test --test p3_right_join_null_4806_test` 6/6 PASS。
- [ ] 5.3 `cargo test --test test_full_outer_join_preserves_both_sides`
      (v312-95) PASS。
- [ ] 5.4 `cargo test --all-features --lib` no regression。
- [ ] 5.5 `openspec validate fix-p3-right-join-null-4806 --strict`
      valid。

## 6. Commit + Memory

- [ ] 6.1 如果 fixed-by-existing-code:
      - Issue 评论: "verified by v312-95 PR #4789, no fix needed"
      - 在 `memory/` 更新 [[v312-95-4639-right-full-join-closure]]
        增加 #4806 reference
      - 关闭 issue
- [ ] 6.2 如果 regression fix:
      - Commit message:
        `fix(P3 / #4806): RIGHT JOIN NULL serialization (regression)`
      - 在 `memory/` 新增 `p3-right-join-null-4806.md`
      - `openspec archive fix-p3-right-join-null-4806` after merge

## 7. Out of Scope

- [ ] 7.1 LEFT JOIN NULL — 已在现有 join path,验证一致性即可。
- [ ] 7.2 Display formatter — 仅在 executor 确认 NULL → '' 时修。
- [ ] 7.3 CSV / JSON export — defer。