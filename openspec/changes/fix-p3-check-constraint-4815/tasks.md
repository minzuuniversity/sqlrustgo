# Tasks — Issue #4815: CHECK constraint 真正生效

## 1. AST + Parser 验证

- [ ] 1.1 在 `crates/parser/src/ast.rs` 找到 `TableConstraint::Check`,
      确认 `name: Option<String>, expr: Expr` 字段齐全。
- [ ] 1.2 在 `crates/parser/src/parser.rs` 的 `parse_create_table` 找到
      CHECK 解析路径,确认所有 CHECK 子句进入 `table_constraints` 列表。
- [ ] 1.3 验证 `ColumnConstraint::Check` 也正确(列级 CHECK)。
- [ ] 1.4 验证 `CONSTRAINT name CHECK (expr)` 的 name 解析到
      `Check { name: Some(name), .. }`。

## 2. Storage Layer (TableInfo)

- [ ] 2.1 在 `src/storage/metadata.rs` 的 `TableInfo` 中添加:
      ```rust
      pub check_constraints: Vec<CheckConstraint>,
      ```
- [ ] 2.2 新增 `CheckConstraint { name: Option<String>, expr: Expr }`。
- [ ] 2.3 FileStorage: 把 `check_constraints` 序列化到 disk metadata。
- [ ] 2.4 MemoryStorage: 内存中保留。
- [ ] 2.5 旧表加载兼容性:如果 disk 上 metadata 没 check_constraints 字段,
      默认为空 list (向后兼容)。

## 3. Executor 修复

- [ ] 3.1 在 `src/error.rs` 新增:
      ```rust
      ConstraintViolation {
          constraint: String,
          row_data: String,
      }
      ```
- [ ] 3.2 在 `src/executor/dml.rs` 的 `execute_insert`:
      - 找到当前 INSERT 路径写入 storage 前的位置。
      - 在每行写入前,遍历 `table_info.check_constraints` 并 evaluate。
      - 任何 false → return ConstraintViolation 错误。
- [ ] 3.3 在 `src/executor/dml.rs` 的 `execute_update`:
      - 同 3.2 步骤,在每行 UPDATE 前 evaluate CHECK。
- [ ] 3.4 在 `src/execution_engine.rs` 的 `execute_create_table`:
      - 把 AST 中 `table_constraints` 的 Check 项转换为 `CheckConstraint`
        并写入 `TableInfo.check_constraints`。
- [ ] 3.5 (可选) `execute_alter_table` ADD CONSTRAINT 路径 — 推迟验证,
      当前可能没支持。

## 4. Tests (7 tests)

- [ ] 4.1 `check_constraint_blocks_negative_age` — issue anchor:
      ```sql
      CREATE TABLE t (id INT, age INT, CHECK (age >= 0));
      INSERT INTO t VALUES (1, -1);  -- must fail
      ```
- [ ] 4.2 `check_constraint_allows_valid_age` — `INSERT (age=10)` 成功。
- [ ] 4.3 `check_constraint_multiple_constraints` — 两个 CHECK 都
      evaluate。
- [ ] 4.4 `check_constraint_named_constraint` — 错误消息含
      `age_check`。
- [ ] 4.5 `check_constraint_with_expression` — `CHECK (price > cost)`
      跨列。
- [ ] 4.6 `check_constraint_blocks_update` — UPDATE 把 age 改成 -1
      必须失败。
- [ ] 4.7 `check_constraint_complex_expression` — `CHECK (status IN
      ('a','b','c'))`。

## 5. Verification

- [ ] 5.1 `cargo build --all-features` clean。
- [ ] 5.2 `cargo test --test p3_check_constraint_4815_test` 7/7 PASS。
- [ ] 5.3 `cargo test --all-features --lib` no regression。
- [ ] 5.4 `openspec validate fix-p3-check-constraint-4815 --strict` valid。

## 6. Commit + Memory

- [ ] 6.1 Commit message:
      `fix(P3 / #4815): CHECK constraint actually evaluates on INSERT/UPDATE`。
- [ ] 6.2 在 `memory/` 新增 `p3-check-constraint-4815.md` 记录 CHECK
      evaluate context、错误消息格式。
- [ ] 6.3 `openspec archive fix-p3-check-constraint-4815` after merge。

## 7. Out of Scope

- [ ] 7.1 DOMAIN types — v3.14。
- [ ] 7.2 Deferrable constraints — v3.14。
- [ ] 7.3 NOT VALID / VALIDATE CONSTRAINT — v3.14。
- [ ] 7.4 Cross-row CHECK with subquery — v3.15。
- [ ] 7.5 ALTER TABLE ADD CONSTRAINT — 验证是否已支持。