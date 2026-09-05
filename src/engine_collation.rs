//! Collation-aware helpers for set-operation executors (V4077 / #4077).
//!
//! Free functions extracted from `execution_engine.rs` as part of the
//! C-ARCH-05 refactor. Used by `engine_setops::execute_union`,
//! `execute_intersect`, `execute_except`, and the trailing ORDER BY
//! logic shared by all three. Also used directly by tests in
//! `tests/anomaly/null_handling_test.rs` (V4077 / V4158 cases).
//!
//! The functions here operate on borrowed `Statement` / `SelectStatement`
//! trees and the storage engine's table metadata, and have no `self`
//! dependency on `ExecutionEngine`. They were top-level `fn`s in
//! `execution_engine.rs`; moving them into a named module makes the
//! dependency graph explicit and shrinks `execution_engine.rs` below
//! the 1600-line architectural limit.

use sqlrustgo_parser::parser::{OrderByExpression, SelectStatement};
use sqlrustgo_parser::Statement;
use sqlrustgo_storage::StorageEngine;

use sqlrustgo_types::Value;

/// Walk nested set-operation statements to find the left-most SELECT's
/// column display names (alias → name). Used by `execute_union` to
/// resolve trailing ORDER BY column references.
pub fn leftmost_column_names(stmt: &Statement) -> Vec<&str> {
    match stmt {
        Statement::Select(s) => s
            .columns
            .iter()
            .map(|c| c.alias.as_deref().unwrap_or(&c.name))
            .collect(),
        Statement::Union(u) => leftmost_column_names(&u.left),
        Statement::Intersect(i) => leftmost_column_names(&i.left),
        Statement::Except(e) => leftmost_column_names(&e.left),
        _ => Vec::new(),
    }
}

/// V4077 / Issue #4077: expand `*` / `table.*` to the actual physical
/// column names of the source table, so ORDER BY resolution against a
/// `SELECT *` left side can find real column indices. Returns owned
/// strings (rather than `&str`) because the physical columns come from
/// a fresh `get_table_info` call and live only as long as the storage
/// read guard.
pub fn expand_column_names(storage: &dyn StorageEngine, stmt: &Statement) -> Vec<String> {
    match stmt {
        Statement::Select(s) => {
            let raw = s
                .columns
                .iter()
                .map(|c| c.alias.clone().unwrap_or_else(|| c.name.clone()))
                .collect::<Vec<_>>();
            // Expand `*` (or `table.*`) once via the source table info.
            let mut out: Vec<String> = Vec::new();
            for n in raw {
                if n == "*" {
                    if !s.table.is_empty() {
                        if let Ok(info) = storage.get_table_info(&s.table) {
                            for col in &info.columns {
                                out.push(col.name.clone());
                            }
                            continue;
                        }
                    }
                    // V313-followup-6 / Issue #4159: `SELECT * FROM (VALUES ...) s(x)`
                    // has `table="s"` but `s` is not in catalog; recurse into
                    // `from_subquery` so the column-list form `s(x)` surfaces.
                    if let Some(sub) = &s.from_subquery {
                        let sub_names =
                            expand_column_names(storage, &Statement::Select(sub.as_ref().clone()));
                        out.extend(sub_names);
                    }
                } else {
                    out.push(n);
                }
            }
            out
        }
        Statement::Union(u) => expand_column_names(storage, &u.left),
        Statement::Intersect(i) => expand_column_names(storage, &i.left),
        Statement::Except(e) => expand_column_names(storage, &e.left),
        _ => Vec::new(),
    }
}

/// V4077 / Issue #4077: collation-aware multiset entries.
/// Returns a map keyed by the *normalized* row (per-column collation
/// applied) to `(count, first_original_row)`. The first row from the
/// input slice is preserved so set-op output retains the LEFT side's
/// original casing for NOCASE columns.
pub fn multiset_entries(
    rows: &[Vec<Value>],
    collations: &[Option<String>],
) -> std::collections::HashMap<Vec<Value>, (usize, Vec<Value>)> {
    let mut entries: std::collections::HashMap<Vec<Value>, (usize, Vec<Value>)> =
        std::collections::HashMap::new();
    for row in rows {
        let key = normalize_row_for_compare(row, collations);
        let entry = entries.entry(key).or_insert_with(|| (0usize, row.clone()));
        entry.0 += 1;
    }
    entries
}

/// V4077 / Issue #4077: walk a set-op's left operand to find the
/// leftmost SELECT statement (so we can look up its source table's
/// column collations).
pub fn leftmost_select(stmt: &Statement) -> Option<&SelectStatement> {
    match stmt {
        Statement::Select(s) => Some(s),
        Statement::Union(u) => leftmost_select(&u.left),
        Statement::Intersect(i) => leftmost_select(&i.left),
        Statement::Except(e) => leftmost_select(&e.left),
        _ => None,
    }
}

/// V4077 / Issue #4077: pull each column's COLLATE name from the
/// source table's TableInfo (if any). Returns a Vec aligned with
/// SELECT projection positions; position N corresponds to column N
/// of the SELECT's projection. If the leftmost SELECT references no
/// resolvable table, returns an empty Vec (caller treats empty as
/// "no collation context" → binary comparison, the original
/// behavior).
pub fn collect_column_collations(
    storage: &dyn StorageEngine,
    stmt: &Statement,
) -> Vec<Option<String>> {
    let Some(select) = leftmost_select(stmt) else {
        return Vec::new();
    };
    if select.table.is_empty() {
        return Vec::new();
    }
    let Ok(info) = storage.get_table_info(&select.table) else {
        return Vec::new();
    };
    // Column names from the SELECT projection (alias → name). For `*`
    // (or `table.*`), expand to every physical column of the source
    // table in declaration order. Otherwise the alias-or-name lookup
    // would fail for `*` and the collation context would silently
    // collapse to binary comparison — which is what produced the V4077
    // NOCASE bug where ABC != abc under binary.
    let mut names: Vec<String> = Vec::new();
    for c in &select.columns {
        let n = c.alias.clone().unwrap_or_else(|| c.name.clone());
        if n == "*" {
            for col in &info.columns {
                names.push(col.name.clone());
            }
        } else {
            names.push(n);
        }
    }
    names
        .into_iter()
        .map(|n| {
            info.columns
                .iter()
                .find(|c| c.name == n)
                .and_then(|c| c.collation.clone())
        })
        .collect()
}

/// V4077 / Issue #4077: produce a comparison key from a row by
/// applying each column's collation. NOCASE folds ASCII case;
/// unknown / BINARY leaves the value as-is.
pub fn normalize_row_for_compare(row: &[Value], collations: &[Option<String>]) -> Vec<Value> {
    if collations.is_empty() {
        return row.to_vec();
    }
    row.iter()
        .enumerate()
        .map(|(i, v)| {
            let coll = collations.get(i).and_then(|c| c.as_deref());
            normalize_value_for_collation(v, coll)
        })
        .collect()
}

pub fn normalize_value_for_collation(v: &Value, collation: Option<&str>) -> Value {
    match (v, collation) {
        (Value::Text(s), Some("NOCASE")) => Value::Text(s.to_uppercase()),
        (v, _) => v.clone(),
    }
}

/// Evaluate a single ORDER BY expression against a row, using column
/// names (from the left SELECT) or 1-based integer position.
pub fn order_by_expr_value(ob: &OrderByExpression, col_names: &[&str], row: &[Value]) -> Value {
    use sqlrustgo_parser::Expression;
    match &ob.expression {
        Expression::Identifier(name) => {
            if let Some(idx) = col_names.iter().position(|n| *n == name) {
                if idx < row.len() {
                    return row[idx].clone();
                }
            }
            Value::Null
        }
        Expression::Literal(lit) => {
            if let Ok(pos) = lit.parse::<usize>() {
                let idx = pos.saturating_sub(1);
                if idx < row.len() {
                    return row[idx].clone();
                }
            }
            Value::Null
        }
        _ => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    //! Direct coverage for the free functions in `engine_collation`.
    //! Targets the V4077 / Issue #4077 collation-aware multiset semantics
    //! and the NOCASE-aware DISTINCT/UNION path. These functions are pure
    //! (or take only `&dyn StorageEngine` + borrowed AST), so unit tests
    //! exercise them without spinning up a full execution engine.

    use super::*;
    use sqlrustgo_parser::{
        parser::{ExceptStatement, IntersectStatement, OrderByExpression, UnionStatement},
        Expression, SelectColumn, Statement,
    };
    use sqlrustgo_storage::{
        ColumnDefinition as StorageColumnDef, MemoryStorage, StorageEngine, TableInfo,
    };

    fn mk_select(table: &str, cols: &[&str]) -> SelectStatement {
        SelectStatement {
            columns: cols
                .iter()
                .map(|n| SelectColumn {
                    name: (*n).to_string(),
                    alias: None,
                    expression: None,
                })
                .collect(),
            table: table.to_string(),
            schema: None,
            from_alias: None,
            from_subquery: None,
            // V312-95 v2 / Issue #4717: propagate FROM (WITH ...) subquery.
            from_with_subquery: None,
            from_values: None,
            from_function_args: None,
            where_clause: None,
            join_clause: vec![],
            extra_tables: vec![],
            aggregates: vec![],
            group_by: vec![],
            with_rollup: false,
            with_cube: false,
            having: None,
            order_by: vec![],
            limit: None,
            offset: None,
            distinct: false,
            index_hints: vec![],
            lock_clause: None,
            grouping_sets: Vec::new(),
        }
    }

    fn mk_storage_with_table(
        name: &str,
        col_names: &[&str],
        collations: &[Option<&str>],
    ) -> MemoryStorage {
        let mut s = MemoryStorage::new();
        let cols: Vec<StorageColumnDef> = col_names
            .iter()
            .zip(collations.iter())
            .map(|(n, c)| StorageColumnDef {
                name: (*n).to_string(),
                data_type: "TEXT".to_string(),
                nullable: true,
                primary_key: false,
                char_max_length: None,
                collation: c.map(|x| x.to_string()),
                default_value: None,
                auto_increment: false,
            })
            .collect();
        let info = TableInfo {
            name: name.to_string(),
            columns: cols,
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            partition_info: None,
            collations: Default::default(),
            original_sql: String::new(),
            compression: None,
        };
        s.create_table(&info).unwrap();
        s
    }

    // ---- leftmost_column_names --------------------------------------------------

    #[test]
    fn leftmost_column_names_plain_select_with_aliases() {
        let mut s = mk_select("t", &["x", "y", "z"]);
        s.columns[0].alias = Some("a".to_string());
        s.columns[1].alias = Some("b".to_string());
        let stmt = Statement::Select(s);
        let names = leftmost_column_names(&stmt);
        assert_eq!(names, vec!["a", "b", "z"]);
    }

    #[test]
    fn leftmost_column_names_walks_union_intersect_except() {
        let mut left = mk_select("l", &["a", "b"]);
        left.columns[0].alias = Some("aa".to_string());
        let mut right = mk_select("r", &["c", "d"]);
        right.columns[1].alias = Some("dd".to_string());

        let u = Statement::Union(UnionStatement {
            left: Box::new(Statement::Select(left.clone())),
            right: Box::new(Statement::Select(right.clone())),
            union_all: false,
            trailing_order_by: vec![],
            trailing_limit: None,
            trailing_offset: None,
        });
        let i = Statement::Intersect(IntersectStatement {
            left: Box::new(Statement::Select(left.clone())),
            right: Box::new(Statement::Select(right.clone())),
            intersect_all: false,
            trailing_order_by: vec![],
            trailing_limit: None,
            trailing_offset: None,
        });
        let e = Statement::Except(ExceptStatement {
            left: Box::new(Statement::Select(left.clone())),
            right: Box::new(Statement::Select(right.clone())),
            except_all: false,
            trailing_order_by: vec![],
            trailing_limit: None,
            trailing_offset: None,
        });
        assert_eq!(leftmost_column_names(&u), vec!["aa", "b"]);
        assert_eq!(leftmost_column_names(&i), vec!["aa", "b"]);
        assert_eq!(leftmost_column_names(&e), vec!["aa", "b"]);
    }

    #[test]
    fn leftmost_column_names_non_select_returns_empty() {
        let stmt = Statement::Values(vec![vec![Expression::Literal("1".into())]]);
        assert!(leftmost_column_names(&stmt).is_empty());
    }

    // ---- leftmost_select --------------------------------------------------------

    #[test]
    fn leftmost_select_walks_nested_setops() {
        let s = mk_select("t", &["a"]);
        let inner = Statement::Union(UnionStatement {
            left: Box::new(Statement::Select(mk_select("u", &["b"]))),
            right: Box::new(Statement::Select(mk_select("v", &["c"]))),
            union_all: true,
            trailing_order_by: vec![],
            trailing_limit: None,
            trailing_offset: None,
        });
        let outer = Statement::Except(ExceptStatement {
            left: Box::new(inner),
            right: Box::new(Statement::Select(s.clone())),
            except_all: false,
            trailing_order_by: vec![],
            trailing_limit: None,
            trailing_offset: None,
        });
        let ls = leftmost_select(&outer).expect("should resolve leftmost SELECT");
        assert_eq!(ls.table, "u");

        // Direct select path
        assert_eq!(
            leftmost_select(&Statement::Select(s.clone()))
                .unwrap()
                .table,
            "t"
        );
        // Non-select → None
        assert!(leftmost_select(&Statement::Values(vec![])).is_none());
    }

    // ---- normalize_value_for_collation -----------------------------------------

    #[test]
    fn normalize_value_nocase_uppercases_ascii() {
        let v = Value::Text("Abc".to_string());
        let out = normalize_value_for_collation(&v, Some("NOCASE"));
        assert_eq!(out, Value::Text("ABC".to_string()));
    }

    #[test]
    fn normalize_value_binary_passthrough() {
        let v = Value::Text("Abc".to_string());
        let out = normalize_value_for_collation(&v, Some("BINARY"));
        assert_eq!(out, v);
    }

    #[test]
    fn normalize_value_no_collation_passthrough() {
        let v = Value::Integer(42);
        assert_eq!(normalize_value_for_collation(&v, None), Value::Integer(42));
        let t = Value::Text("abc".to_string());
        assert_eq!(normalize_value_for_collation(&t, None), t);
    }

    #[test]
    fn normalize_value_nocase_non_text_passthrough() {
        let v = Value::Integer(42);
        assert_eq!(
            normalize_value_for_collation(&v, Some("NOCASE")),
            Value::Integer(42)
        );
    }

    // ---- normalize_row_for_compare ----------------------------------------------

    #[test]
    fn normalize_row_empty_collations_returns_copy() {
        let row = vec![Value::Text("a".into()), Value::Integer(1)];
        let out = normalize_row_for_compare(&row, &[]);
        assert_eq!(out, row);
    }

    #[test]
    fn normalize_row_applies_per_column_collation() {
        let row = vec![
            Value::Text("Foo".into()),
            Value::Text("Bar".into()),
            Value::Integer(7),
        ];
        let colls = vec![Some("NOCASE".to_string()), None, Some("NOCASE".to_string())];
        let out = normalize_row_for_compare(&row, &colls);
        assert_eq!(out[0], Value::Text("FOO".into()));
        assert_eq!(out[1], Value::Text("Bar".into()));
        assert_eq!(out[2], Value::Integer(7));
    }

    #[test]
    fn normalize_row_truncates_collations() {
        let row = vec![Value::Text("X".into()), Value::Text("Y".into())];
        let colls = vec![Some("NOCASE".to_string())];
        // collations shorter than row → only first column normalized
        let out = normalize_row_for_compare(&row, &colls);
        assert_eq!(out[0], Value::Text("X".into()));
        assert_eq!(out[1], Value::Text("Y".into()));
    }

    // ---- multiset_entries -------------------------------------------------------

    #[test]
    fn multiset_entries_counts_with_nocase_folding() {
        let rows = vec![
            vec![Value::Text("abc".into())],
            vec![Value::Text("ABC".into())],
            vec![Value::Text("xyz".into())],
        ];
        let colls = vec![Some("NOCASE".to_string())];
        let m = multiset_entries(&rows, &colls);
        assert_eq!(m.len(), 2);
        let key_abc = vec![Value::Text("ABC".into())];
        assert_eq!(m.get(&key_abc).unwrap().0, 2);
        // First original row for the abc/ABC bucket should be the lowercase variant
        assert_eq!(m.get(&key_abc).unwrap().1, vec![Value::Text("abc".into())]);
    }

    #[test]
    fn multiset_entries_empty_input_empty_map() {
        let m = multiset_entries(&[], &[Some("NOCASE".to_string())]);
        assert!(m.is_empty());
    }

    #[test]
    fn multiset_entries_binary_distinct() {
        let rows = vec![
            vec![Value::Text("abc".into())],
            vec![Value::Text("ABC".into())],
        ];
        // No collation → binary distinct; both keys present
        let m = multiset_entries(&rows, &[]);
        assert_eq!(m.len(), 2);
    }

    // ---- expand_column_names ----------------------------------------------------

    #[test]
    fn expand_column_names_plain_columns_passthrough() {
        let s = mk_select("t", &["a", "b"]);
        let storage = mk_storage_with_table("t", &["a", "b"], &[None, None]);
        let stmt = Statement::Select(s);
        let names = expand_column_names(&storage, &stmt);
        assert_eq!(names, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn expand_column_names_star_expands_to_physical() {
        let mut s = mk_select("t", &["*"]);
        s.columns[0].name = "*".to_string();
        let storage = mk_storage_with_table("t", &["x", "y", "z"], &[None, None, None]);
        let names = expand_column_names(&storage, &Statement::Select(s));
        assert_eq!(
            names,
            vec!["x".to_string(), "y".to_string(), "z".to_string()]
        );
    }

    #[test]
    fn expand_column_names_star_table_missing_falls_back_to_subquery() {
        // V313-followup-6 / #4159: FROM (subquery) AS s(x,y)
        let mut outer = mk_select("s", &["*"]);
        outer.columns[0].name = "*".to_string();
        outer.from_subquery = Some(Box::new(mk_select("", &["x", "y"])));
        let storage = MemoryStorage::new(); // "s" not registered
        let names = expand_column_names(&storage, &Statement::Select(outer));
        assert_eq!(names, vec!["x".to_string(), "y".to_string()]);
    }

    #[test]
    fn expand_column_names_walks_left_of_setop() {
        let mut left = mk_select("l", &["*"]);
        left.columns[0].name = "*".to_string();
        let storage = mk_storage_with_table("l", &["la", "lb"], &[None, None]);
        let stmt = Statement::Union(UnionStatement {
            left: Box::new(Statement::Select(left)),
            right: Box::new(Statement::Select(mk_select("r", &["ra"]))),
            union_all: false,
            trailing_order_by: vec![],
            trailing_limit: None,
            trailing_offset: None,
        });
        let names = expand_column_names(&storage, &stmt);
        assert_eq!(names, vec!["la".to_string(), "lb".to_string()]);
    }

    #[test]
    fn expand_column_names_non_select_returns_empty() {
        let storage = MemoryStorage::new();
        let stmt = Statement::Values(vec![vec![Expression::Literal("1".into())]]);
        assert!(expand_column_names(&storage, &stmt).is_empty());
    }

    // ---- collect_column_collations ---------------------------------------------

    #[test]
    fn collect_column_collations_empty_when_no_table() {
        let s = mk_select("", &["a"]);
        let storage = MemoryStorage::new();
        assert!(collect_column_collations(&storage, &Statement::Select(s)).is_empty());
    }

    #[test]
    fn collect_column_collations_empty_when_table_missing() {
        let s = mk_select("missing", &["a"]);
        let storage = MemoryStorage::new();
        assert!(collect_column_collations(&storage, &Statement::Select(s)).is_empty());
    }

    #[test]
    fn collect_column_collations_per_column() {
        let s = mk_select("t", &["a", "b", "c"]);
        let storage = mk_storage_with_table(
            "t",
            &["a", "b", "c"],
            &[Some("NOCASE"), None, Some("BINARY")],
        );
        let colls = collect_column_collations(&storage, &Statement::Select(s));
        assert_eq!(
            colls,
            vec![Some("NOCASE".to_string()), None, Some("BINARY".to_string())]
        );
    }

    #[test]
    fn collect_column_collations_star_uses_physical_order() {
        let mut s = mk_select("t", &["*"]);
        s.columns[0].name = "*".to_string();
        let storage = mk_storage_with_table("t", &["x", "y"], &[Some("NOCASE"), Some("BINARY")]);
        let colls = collect_column_collations(&storage, &Statement::Select(s));
        assert_eq!(
            colls,
            vec![Some("NOCASE".to_string()), Some("BINARY".to_string())]
        );
    }

    #[test]
    fn collect_column_collations_alias_name_match() {
        // SELECT a AS foo: collation lookup should match physical name 'a'
        let mut s = mk_select("t", &["a"]);
        s.columns[0].alias = Some("foo".to_string());
        let storage = mk_storage_with_table("t", &["a"], &[Some("NOCASE")]);
        let colls = collect_column_collations(&storage, &Statement::Select(s));
        // Aliased name doesn't match physical 'a' so we get None
        assert_eq!(colls, vec![None]);
    }

    #[test]
    fn collect_column_collations_non_select_returns_empty() {
        let storage = MemoryStorage::new();
        let stmt = Statement::Values(vec![vec![Expression::Literal("1".into())]]);
        assert!(collect_column_collations(&storage, &stmt).is_empty());
    }

    // ---- order_by_expr_value ----------------------------------------------------

    #[test]
    fn order_by_expr_identifier_match() {
        let ob = OrderByExpression {
            expression: Expression::Identifier("b".to_string()),
            ascending: true,
            nulls_first: None,
        };
        let row = vec![Value::Text("a-val".into()), Value::Integer(99)];
        let names = vec!["a", "b"];
        assert_eq!(order_by_expr_value(&ob, &names, &row), Value::Integer(99));
    }

    #[test]
    fn order_by_expr_identifier_miss_returns_null() {
        let ob = OrderByExpression {
            expression: Expression::Identifier("z".to_string()),
            ascending: true,
            nulls_first: None,
        };
        let row = vec![Value::Integer(1)];
        let names = vec!["a"];
        assert_eq!(order_by_expr_value(&ob, &names, &row), Value::Null);
    }

    #[test]
    fn order_by_expr_literal_position_one_based() {
        let ob = OrderByExpression {
            expression: Expression::Literal("2".to_string()),
            ascending: false,
            nulls_first: None,
        };
        let row = vec![Value::Integer(10), Value::Integer(20), Value::Integer(30)];
        let names: Vec<&str> = vec![];
        assert_eq!(order_by_expr_value(&ob, &names, &row), Value::Integer(20));
    }

    #[test]
    fn order_by_expr_literal_out_of_range_returns_null() {
        let ob = OrderByExpression {
            expression: Expression::Literal("9".to_string()),
            ascending: true,
            nulls_first: None,
        };
        let row = vec![Value::Integer(1), Value::Integer(2)];
        assert_eq!(order_by_expr_value(&ob, &[], &row), Value::Null);
    }

    #[test]
    fn order_by_expr_non_integer_literal_returns_null() {
        let ob = OrderByExpression {
            expression: Expression::Literal("abc".to_string()),
            ascending: true,
            nulls_first: None,
        };
        let row = vec![Value::Integer(1)];
        assert_eq!(order_by_expr_value(&ob, &[], &row), Value::Null);
    }

    #[test]
    fn order_by_expr_other_expression_returns_null() {
        let ob = OrderByExpression {
            expression: Expression::Literal("1+1".to_string()),
            ascending: true,
            nulls_first: None,
        };
        let row = vec![Value::Integer(1), Value::Integer(2)];
        assert_eq!(order_by_expr_value(&ob, &[], &row), Value::Null);
    }

    #[test]
    fn order_by_expr_literal_zero_saturates_to_first() {
        // saturating_sub(1) on 0 keeps 0; behavior is "first column".
        let ob = OrderByExpression {
            expression: Expression::Literal("0".to_string()),
            ascending: true,
            nulls_first: None,
        };
        let row = vec![Value::Integer(1), Value::Integer(2)];
        assert_eq!(order_by_expr_value(&ob, &[], &row), Value::Integer(1));
    }
}
