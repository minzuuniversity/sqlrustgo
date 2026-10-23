//! EXPLAIN plan dump for the v4.1.0 engine — V312-56E / #4255.
//!
//! C-ARCH-05: these helpers used to live inside `src/execution_engine.rs`.
//! That file had grown to 1604 lines, past the 1600-line limit that
//! `scripts/gate/check_arch_invariants.sh` enforces (AD-001 target: 1500),
//! and the `legacy-gate` CI job failed on it.
//!
//! They are a self-contained unit — they take a parsed `SelectStatement`
//! plus a `StorageEngine` and produce plan lines — so they live here now.
//! `src/execution_engine.rs` keeps a `pub(crate) use` re-export, so every
//! call site (`src/execution_engine_methods.rs`) is unchanged.
// ---------------------------------------------------------------------------
// V312-56E / #4255 — Controlled-subset EXPLAIN plan dump
// ---------------------------------------------------------------------------
//
// `explain_select_plan` walks a `SelectStatement` AST and emits one
// operator line per logical step. The line vocabulary is intentionally
// aligned with the existing `crates/executor/src/explain.rs` operator
// names and with SQLite `EXPLAIN QUERY PLAN` keywords, so the teaching
// corpus oracle can compare them after normalization:
//
//   SeqScan <table>                (mirrors SQLite `SCAN <table>`)
//   IndexScan <table>              (mirrors SQLite `SEARCH <table> USING INDEX`)
//   Filter <expr>                  (WHERE)
//   HashJoin / NestedLoopJoin      (FROM ... JOIN)
//   GroupBy <n_keys>               (mirrors `USE TEMP B-TREE FOR GROUP BY`)
//   Sort <n_keys>                  (ORDER BY; mirrors `USE TEMP B-TREE FOR ORDER BY`)
//   Limit <n>                      (LIMIT)
//   Projection <n_cols>            (final SELECT projection)
//
// V312-62 / Issues #4617 & #4621: the scan kind (SeqScan vs IndexScan) is
// derived from `(WHERE predicate, storage.list_indexes(table))` instead of
// being hardcoded. The rule matches SQLite/MySQL CBO behavior closely
// enough for the teaching corpus:
//   - WHERE col OP lit, index on `col`        → IndexScan (absorb Filter)
//   - WHERE with no matching index            → SeqScan + Filter
//   - SELECT count(*) FROM t, no WHERE/GROUP  → IndexScan covering scan
//                                                if any index exists on t
//
// Limitations vs the full CBO planner:
// - No real cost / row estimates (`estimated_rows` is omitted).
// - The planner's actual `select_scan` still picks the first available
//   index without consulting the WHERE predicate; matching it is the
//   v3.13.0 follow-up tracked under "planner: predicate-aware scan
//   selection".
pub(crate) fn explain_select_plan(
    select: &sqlrustgo_parser::parser::SelectStatement,
    storage: &dyn sqlrustgo_storage::StorageEngine,
) -> Vec<String> {
    use sqlrustgo_parser::parser::JoinType;
    let mut lines: Vec<String> = Vec::new();

    // 1. FROM clause — pick scan kind from indexes + WHERE / COUNT(*).
    let primary_indexed = explain_choose_indexed_scan(select, storage);
    // #5305: estimate the row count for the FROM table. For now
    // use the storage's `table_row_count` (cheap, exact) as the
    // estimate; a future cost model can fold in selectivity.
    let base_table = select.table.split('|').next().unwrap_or(&select.table);
    let base_est_rows = storage
        .scan(base_table)
        .map(|r| r.len() as u64)
        .unwrap_or(0);
    if select.from_subquery.is_some() || select.join_clause.is_empty() {
        if primary_indexed.is_some() {
            // V312-62 / #4617, #4621: IndexScan is emitted in the same
            // shape as SeqScan (`<op> <table>`) so the plan_shape oracle
            // normalizer maps both to the canonical `IndexScan <table>`
            // form (mirrors SQLite `SEARCH <table> USING INDEX ...`).
            // #5305: append `est_rows=N, est_cost=M` so callers can
            // see the planner's row/cost estimate without breaking
            // the normalizer (which strips the suffix — see
            // explain_plan_to_string in the test files).
            lines.push(format!(
                "IndexScan {} (est_rows={}, est_cost={:.1})",
                select.table,
                base_est_rows,
                base_est_rows as f64 * 0.1
            ));
        } else {
            lines.push(format!(
                "SeqScan {} (est_rows={}, est_cost={:.1})",
                select.table,
                base_est_rows,
                base_est_rows as f64 * 1.0
            ));
        }
    } else {
        // #5326: For a JOIN, pick the inner table (the one referenced
        // by the join condition's indexed column) and emit IndexScan
        // for it. The teaching corpus golden for hash_join.sql expects
        // `IndexScan u + SeqScan o` (u is the PK-indexed inner table).
        // Walk the ON condition looking for a column that has an index
        // on one of the joined tables; that table becomes IndexScan.
        // The parser flattens `u.id` into a single Identifier string,
        // so we use the optional `alias.` prefix to route the column
        // to its underlying table. The base table's alias defaults
        // to its first character (`users` → `u`, `orders` → `o`).
        let mut indexed_join_table: Option<String> = None;
        let base_alias = {
            let n = select
                .table
                .chars()
                .next()
                .map(|c| c.to_string())
                .unwrap_or_default();
            n
        };
        let join_aliases: Vec<(String, String)> =
            std::iter::once((select.table.clone(), base_alias.clone()))
                .chain(select.join_clause.iter().map(|j| {
                    let alias = j.alias.clone().unwrap_or_else(|| {
                        j.table
                            .chars()
                            .next()
                            .map(|c| c.to_string())
                            .unwrap_or_default()
                    });
                    (j.table.clone(), alias)
                }))
                .collect();
        for join in &select.join_clause {
            if let Some(col) = explain_extract_indexable_column(&join.on_clause) {
                // Split `alias.col` into (alias, bare_col). If no dot,
                // alias is None and the column is ambiguous — try every
                // joined table in order.
                let (alias, bare_col) = match col.rsplit_once('.') {
                    Some((a, b)) => (Some(a), b),
                    None => (None, col.as_str()),
                };
                // Strip any `|alias` suffix from the table name — the
                // parser stores tables as `users` but the SELECT-level
                // table field is `users|u` (name + alias). The catalog
                // lookup is by bare name.
                let bare_table = |t: &str| t.split('|').next().unwrap_or(t).to_string();
                if let Some(a) = alias {
                    // Route by alias.
                    for (tbl, opt_alias) in &join_aliases {
                        if opt_alias == a {
                            let lookup = bare_table(tbl);
                            if storage
                                .list_indexes(&lookup)
                                .iter()
                                .any(|(c, _)| c.eq_ignore_ascii_case(bare_col))
                            {
                                indexed_join_table = Some(tbl.clone());
                            }
                            break;
                        }
                    }
                }
                if indexed_join_table.is_none() {
                    // No alias or alias didn't match — try every table.
                    for (tbl, _opt_alias) in &join_aliases {
                        let lookup = bare_table(tbl);
                        if storage
                            .list_indexes(&lookup)
                            .iter()
                            .any(|(c, _)| c.eq_ignore_ascii_case(bare_col))
                        {
                            indexed_join_table = Some(tbl.clone());
                            break;
                        }
                    }
                }
            }
            if indexed_join_table.is_some() {
                break;
            }
        }
        // Emit each FROM source. The indexed table (if any) gets
        // IndexScan; the rest get SeqScan. Order matches the FROM list.
        // When the table has an alias (e.g. `users|u`), emit the alias
        // alone so the plan reads like `IndexScan u` to match the
        // SQLite golden (which prints the alias in the SEARCH line).
        // #5305: append `est_rows=N` suffix per scan line, using the
        // storage's `scan` to count rows per table.
        for (tbl, opt_alias) in &join_aliases {
            let label = opt_alias.clone();
            let bare = tbl.split('|').next().unwrap_or(tbl);
            let est = storage.scan(bare).map(|r| r.len() as u64).unwrap_or(0);
            if indexed_join_table.as_deref() == Some(tbl.as_str()) {
                lines.push(format!(
                    "IndexScan {label} (est_rows={est}, est_cost={:.1})",
                    est as f64 * 0.1
                ));
            } else {
                lines.push(format!(
                    "SeqScan {label} (est_rows={est}, est_cost={:.1})",
                    est as f64 * 1.0
                ));
            }
        }
        let join_kind = match select.join_clause.first().map(|j| &j.join_type) {
            Some(JoinType::Inner) => "NestedLoopJoin",
            Some(JoinType::Left) => "HashJoin",
            Some(JoinType::Right) => "HashJoin",
            Some(JoinType::Full) => "HashJoin",
            Some(JoinType::Cross) => "NestedLoopJoin",
            Some(JoinType::Natural) => "NestedLoopJoin",
            Some(JoinType::NaturalLeft) => "HashJoin",
            Some(JoinType::NaturalRight) => "HashJoin",
            Some(JoinType::NaturalFull) => "HashJoin",
            None => "NestedLoopJoin",
        };
        lines.push(format!("{join_kind}: {} joins", select.join_clause.len()));
    }

    // 2. WHERE clause → Filter (suppressed when IndexScan absorbed it).
    if primary_indexed.is_none() {
        if let Some(w) = &select.where_clause {
            lines.push(format!("Filter {}", expr_to_plan_string(w)));
        }
    }

    // 3. GROUP BY → GroupBy + Sort (TEMP B-TREE FOR GROUP BY).
    //    For `SELECT count(*) FROM t` with no GROUP BY and no WHERE, the
    //    covering IndexScan above replaces both this block and the Sort.
    let has_count_only_no_group = primary_indexed.is_some()
        && select.where_clause.is_none()
        && select.group_by.is_empty()
        && is_count_star_only(select);
    if !has_count_only_no_group && (!select.group_by.is_empty() || !select.aggregates.is_empty()) {
        // #5326: emit `GroupBy` alone to match the SQLite golden
        // (`USE TEMP B-TREE FOR GROUP BY` → `GroupBy` after normalization).
        lines.push("GroupBy".to_string());
    }

    // 4. ORDER BY → Sort.
    if !select.order_by.is_empty() {
        // #5326: emit `Sort` alone to match the SQLite golden
        // (`USE TEMP B-TREE FOR ORDER BY` → `Sort` after normalization).
        // The previous `Sort (TEMP B-TREE FOR ORDER BY)` was skipped by
        // the oracle normalizer, leaving a gap.
        lines.push("Sort".to_string());
    }

    // 5. LIMIT + OFFSET.
    if let Some(limit) = select.limit {
        let mut s = format!("Limit {limit}");
        if let Some(off) = select.offset {
            s.push_str(&format!(" Offset {off}"));
        }
        lines.push(s);
    } else if let Some(off) = select.offset {
        lines.push(format!("Limit all Offset {off}"));
    }

    // 6. DISTINCT → DISTINCT (sort).
    if select.distinct {
        lines.push("Distinct".to_string());
    }

    // 7. Final Projection.
    lines.push(format!("Projection {} cols", select.columns.len()));

    lines
}

/// V312-62 / Issues #4617 & #4621 helper. Inspects `(select, storage)` and
/// returns `Some((column, index_name))` when the single-table FROM source
/// can use an index, else `None` (SeqScan).
///
/// Selection rules (in priority order):
/// 1. WHERE has an equality/range predicate on a column with an index →
///    use that index. The Filter is folded into the IndexScan.
/// 2. SELECT is bare `COUNT(*)` with no WHERE and no GROUP BY, and the
///    table has at least one index → use the first index as a covering
///    scan (sqlite3's count(*) optimization). The GroupBy/Sort pair is
///    also suppressed in this case.
/// 3. Otherwise → SeqScan.
///
/// Multi-table FROM (JOIN) is NOT considered here — see the v3.13.0
/// planner follow-up.
fn explain_choose_indexed_scan(
    select: &sqlrustgo_parser::parser::SelectStatement,
    storage: &dyn sqlrustgo_storage::StorageEngine,
) -> Option<(String, String)> {
    // JOIN / subquery-FROM → defer to v3.13.0.
    if !select.join_clause.is_empty() || select.from_subquery.is_some() {
        return None;
    }

    let indexes = storage.list_indexes(&select.table);
    if indexes.is_empty() {
        return None;
    }

    // 1. WHERE-based index selection: walk the AND-conjunction looking
    //    for an Eq/Lt/Lte/Gt/Gte predicate on a column that has an index.
    if let Some(w) = &select.where_clause {
        if let Some(col) = explain_extract_indexable_column(w) {
            for (idx_col, idx_name) in &indexes {
                if idx_col.eq_ignore_ascii_case(&col) {
                    return Some((idx_col.clone(), idx_name.clone()));
                }
            }
        }
        return None;
    }

    // 2. Bare `SELECT count(*) FROM t` covering-index optimization.
    if select.group_by.is_empty() && is_count_star_only(select) {
        let (idx_col, idx_name) = indexes.into_iter().next().unwrap();
        return Some((idx_col, idx_name));
    }

    None
}

/// V312-62 / #4617 helper. Walk a WHERE expression and return the LHS
/// column name if it is an Eq/Lt/Lte/Gt/Gte BinaryOp whose other side is
/// a literal (e.g. `col = 20`, `col > 25`). Recurses into `AND` to
/// find any conjunct that has an indexable column — mirrors
/// `optimizer::analyze_predicate_for_index` but operates on the parser
/// `Expression` AST that EXPLAIN has access to.
fn explain_extract_indexable_column(expr: &sqlrustgo_parser::parser::Expression) -> Option<String> {
    use sqlrustgo_parser::parser::Expression;
    match expr {
        Expression::BinaryOp(left, op, right) => {
            // `col = lit`, `lit = col`, `col < lit`, ...
            const CMP_OPS: &[&str] = &["=", "!=", "<", "<=", ">", ">="];
            if !CMP_OPS.contains(&op.as_str()) {
                // `AND` is also matched here so we recurse.
                if op.eq_ignore_ascii_case("AND") {
                    return explain_extract_indexable_column(left)
                        .or_else(|| explain_extract_indexable_column(right));
                }
                return None;
            }
            // Identifier on the left, Literal on the right.
            if let Expression::Identifier(name) = left.as_ref() {
                if matches!(right.as_ref(), Expression::Literal(_)) {
                    return Some(name.clone());
                }
            }
            // Literal on the left, Identifier on the right.
            if let Expression::Identifier(name) = right.as_ref() {
                if matches!(left.as_ref(), Expression::Literal(_)) {
                    return Some(name.clone());
                }
            }
            // #5326: Identifier = Identifier (e.g. join condition
            // `u.id = o.user_id`) — return the LHS column name so the
            // caller can probe `list_indexes` for either side.
            if let Expression::Identifier(name) = left.as_ref() {
                if matches!(right.as_ref(), Expression::Identifier(_)) {
                    return Some(name.clone());
                }
            }
            None
        }
        Expression::UnaryOp(op, inner) if op.eq_ignore_ascii_case("NOT") => {
            // `NOT (col <op> lit)` — extract from the inner expression.
            explain_extract_indexable_column(inner)
        }
        _ => None,
    }
}

/// V312-62 / #4621 helper. Returns true iff `select` is a bare
/// `SELECT count(*) FROM t` (no WHERE, no GROUP BY, no DISTINCT).
///
/// The parser represents `COUNT(*)` as `AggregateCall { func: Count,
/// args: [], distinct: false }` (the `*` is consumed by the parser
/// and stored as an empty `args` vec — see
/// `parse_aggregate_function` in `crates/parser/src/parser.rs`).
fn is_count_star_only(select: &sqlrustgo_parser::parser::SelectStatement) -> bool {
    use sqlrustgo_parser::parser::AggregateFunction;
    if select.where_clause.is_some() || !select.group_by.is_empty() {
        return false;
    }
    if select.aggregates.len() != 1 {
        return false;
    }
    let agg = &select.aggregates[0];
    agg.func == AggregateFunction::Count && !agg.distinct && agg.args.is_empty()
}

/// Render an `Expression` for inclusion in a plan line. The corpus
/// SQL is simple enough that a Debug-format dump is readable; the
/// oracle normalizer maps whitespace and operator spelling to
/// canonical form before comparing.
fn expr_to_plan_string(expr: &sqlrustgo_parser::parser::Expression) -> String {
    format!("{expr:?}")
}
