# Tasks — Issue #4809: INDEXED BY hint honored

## 1. 现状审查

- [ ] 1.1 在 `crates/parser/src/ast.rs` 找到 `TableReference` struct,
      验证 `indexed_by: Option<String>` 和 `not_indexed: Option<()>` 
      字段(v312-90 PR #4788 已加)。
- [ ] 1.2 在 `crates/parser/src/parser.rs` 验证 INDEXED BY 解析路径
      (FROM t INDEXED BY idx ...)。
- [ ] 1.3 验证 storage 层有 `IndexInfo` + `get_index(name)` API。
- [ ] 1.4 写一个 anchor failing 测试:`CREATE INDEX idx_a ON t(col);
      SELECT * FROM t INDEXED BY idx_a WHERE col = 5` 返回正确结果
      且走索引(用 instrumentation 或 timing 验证)。

## 2. AST → Executor wire-up

- [ ] 2.1 在 `src/executor/from.rs` (或等价文件) 的
      `resolve_table_reference`:
      - 如果 `indexed_by` 是 `Some(idx)` → 走 indexed scan 路径。
      - 如果 `not_indexed` 是 `Some(())` → 强制 sequential scan。
      - 否则 → 现有 planner 路径。
- [ ] 2.2 在 `src/storage/metadata.rs` 增加 `IndexInfo` 含
      `table_name: String` 字段用于 ownership check。
- [ ] 2.3 在 `src/error.rs` 新增:
      ```rust
      IndexNotFound(String),
      IndexNotOwnedByTable {
          index: String,
          index_table: String,
          query_table: String,
      },
      ```
- [ ] 2.4 在 resolve_table 中,检查 INDEXED BY 的 idx 是否存在 + 是否
      属于当前 table。失败立即报错。

## 3. Scan Mode Dispatch

- [ ] 3.1 定义 `enum ScanMode { Auto, Index(IndexId), Sequential }`。
- [ ] 3.2 在 table scan loop,根据 scan_mode 分派:
      - `Index(idx_id)` → 用 `storage.index_keys(idx_id, pred)` 取
        keys,然后 for each key `get_row_by_key`,做 filter。
      - `Sequential` → 现有 seq scan。
      - `Auto` → 现有 planner choice。
- [ ] 3.3 验证 indexed scan 的结果与 seq scan 一致(只影响 plan)。

## 4. Tests (7 tests)

- [ ] 4.1 `indexed_by_uses_named_index` — anchor:走索引路径,row 输
      出正确。
- [ ] 4.2 `indexed_by_nonexistent_index_errors` — `INDEXED BY 
      no_such_idx` 报错。
- [ ] 4.3 `indexed_by_table_mismatch_errors` — `FROM t INDEXED BY 
      idx_for_other_table` 报错。
- [ ] 4.4 `indexed_by_with_composite_index` — 多列 index 的等值
      查询。
- [ ] 4.5 `not_indexed_forces_sequential` — 即使有索引也走 seqscan。
- [ ] 4.6 `indexed_by_returns_same_rows` — INDEXED BY 与无 hint
      输出相同 row。
- [ ] 4.7 `indexed_by_with_join` — `FROM t INDEXED BY idx JOIN u 
      ON ...`,hint 只作用于 t。

## 5. Verification

- [ ] 5.1 `cargo build --all-features` clean。
- [ ] 5.2 `cargo test --test p3_indexed_by_4809_test` 7/7 PASS。
- [ ] 5.3 `cargo test --all-features --lib` no regression。
- [ ] 5.4 `cargo test --test parser_e2e_test` 249/249 PASS。
- [ ] 5.5 `openspec validate fix-p3-indexed-by-4809 --strict` valid。

## 6. Commit + Memory

- [ ] 6.1 Commit message:
      `fix(P3 / #4809): INDEXED BY hint actually applied to scan plan`。
- [ ] 6.2 在 `memory/` 新增 `p3-indexed-by-4809.md` 记录 INDEXED BY
      与 v312-90 PR #4788 parser fix 关系、SQLite 语义对照。
- [ ] 6.3 `openspec archive fix-p3-indexed-by-4809` after merge。

## 7. Out of Scope

- [ ] 7.1 MySQL FORCE INDEX / IGNORE INDEX — v3.14。
- [ ] 7.2 ANALYZE-driven hint selection — 不相关。
- [ ] 7.3 Cross-schema INDEXED BY — v3.14。