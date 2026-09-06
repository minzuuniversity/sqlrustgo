//! V312-64d / Issue #4664: re-formatting CREATE DDL AST nodes back into
//! the original `CREATE ...` SQL string so storage metadata can expose the
//! statement through `sqlite_master.sql`.
//!
//! These formatters are intentionally narrow — they only need to produce
//! strings that round-trip through sqlite_master introspection; they do
//! not have to be a full SQL pretty-printer. When a sub-clause is too
//! complex to round-trip cleanly (e.g. multi-line CHECK expressions,
//! multi-statement trigger bodies), we fall back to a debug-style dump
//! prefixed with the `CREATE` header so the column never reads as empty.

use sqlrustgo_parser::parser::{
    ColumnDefinition, CreateIndexStatement, CreateTableStatement, CreateTriggerStatement,
    CreateViewStatement, ForeignKeyRef, ReferentialAction, TableConstraint,
};

/// Format a `CREATE TABLE` AST back to a single-line SQL string suitable
/// for `sqlite_master.sql`. Collapses columns and table-level constraints
/// into a compact comma-separated list.
pub fn format_create_table_sql(create: &CreateTableStatement) -> String {
    let mut out = String::new();
    out.push_str("CREATE ");
    if create.or_replace {
        out.push_str("OR REPLACE TABLE ");
    } else {
        out.push_str("TABLE ");
    }
    if create.if_not_exists {
        out.push_str("IF NOT EXISTS ");
    }
    out.push_str(&create.name);
    out.push_str(" (");

    let mut parts: Vec<String> =
        Vec::with_capacity(create.columns.len() + create.constraints.len());
    for col in &create.columns {
        parts.push(format_column(col));
    }
    for c in &create.constraints {
        parts.push(format_table_constraint(c));
    }
    out.push_str(&parts.join(", "));
    out.push(')');

    if let Some(spec) = &create.compress {
        out.push_str(" COMPRESS (ALGORITHM=");
        out.push_str(match spec.algorithm {
            sqlrustgo_parser::parser::CompressionAlgorithm::Lz4 => "LZ4",
            sqlrustgo_parser::parser::CompressionAlgorithm::Zstd => "ZSTD",
            sqlrustgo_parser::parser::CompressionAlgorithm::Zlib => "ZLIB",
        });
        out.push(')');
    }

    if let Some(sel) = &create.select {
        out.push_str(" AS ");
        out.push_str(&format_select_for_create_as(sel));
        if create.with_data == Some(false) {
            out.push_str(" WITH NO DATA");
        } else {
            out.push_str(" WITH DATA");
        }
    }

    out
}

fn format_column(c: &ColumnDefinition) -> String {
    let mut out = String::new();
    out.push_str(&c.name);
    out.push(' ');
    out.push_str(&c.data_type);
    if !c.nullable {
        out.push_str(" NOT NULL");
    }
    if c.primary_key {
        out.push_str(" PRIMARY KEY");
    }
    if c.auto_increment {
        out.push_str(" AUTO_INCREMENT");
    }
    if let Some(d) = &c.default_value {
        out.push_str(" DEFAULT ");
        out.push_str(d);
    }
    if let Some(col) = &c.collation {
        out.push_str(" COLLATE ");
        out.push_str(col);
    }
    if let Some(fk) = &c.references {
        out.push_str(&format_references_inline(fk));
    }
    out
}

fn format_table_constraint(c: &TableConstraint) -> String {
    match c {
        TableConstraint::PrimaryKey { columns, name } => {
            let n = name
                .as_ref()
                .map(|n| format!(" CONSTRAINT {}", n))
                .unwrap_or_default();
            format!("{}PRIMARY KEY ({})", n, columns.join(", "))
        }
        TableConstraint::Unique { columns, name } => {
            let n = name
                .as_ref()
                .map(|n| format!(" CONSTRAINT {}", n))
                .unwrap_or_default();
            format!("{}UNIQUE ({})", n, columns.join(", "))
        }
        TableConstraint::Check { expression, name } => {
            let n = name
                .as_ref()
                .map(|n| format!("CONSTRAINT {} ", n))
                .unwrap_or_default();
            format!("{}CHECK ({:?})", n, expression)
        }
        TableConstraint::ForeignKey {
            columns,
            referenced_table,
            referenced_columns,
            on_delete,
            on_update,
            name,
        } => {
            let n = name
                .as_ref()
                .map(|n| format!("CONSTRAINT {} ", n))
                .unwrap_or_default();
            format!(
                "{}FOREIGN KEY ({}) REFERENCES {}({}){}{}",
                n,
                columns.join(", "),
                referenced_table,
                referenced_columns.join(", "),
                format_ref_action("ON DELETE", on_delete),
                format_ref_action("ON UPDATE", on_update)
            )
        }
    }
}

fn format_ref_action(prefix: &str, action: &Option<ReferentialAction>) -> String {
    match action {
        Some(ReferentialAction::Cascade) => format!(" {} CASCADE", prefix),
        Some(ReferentialAction::SetNull) => format!(" {} SET NULL", prefix),
        Some(ReferentialAction::Restrict) => format!(" {} RESTRICT", prefix),
        Some(ReferentialAction::NoAction) => format!(" {} NO ACTION", prefix),
        None => String::new(),
    }
}

fn format_references_inline(fk: &ForeignKeyRef) -> String {
    let mut out = format!(
        " REFERENCES {}({})",
        fk.referenced_table,
        fk.referenced_columns.join(", ")
    );
    out.push_str(&format_ref_action("ON DELETE", &fk.on_delete));
    out.push_str(&format_ref_action("ON UPDATE", &fk.on_update));
    out
}

fn format_select_for_create_as(sel: &sqlrustgo_parser::SelectStatement) -> String {
    format_select_inner(sel)
}

/// Public wrapper used by `execute_create_view` to populate the inner
/// `query_sql` field of `ViewInfo` — semantically equivalent to the
/// CTAS inner-select formatter but exposed so the executor can call it.
pub fn format_create_view_inner_select(sel: &sqlrustgo_parser::SelectStatement) -> String {
    format_select_inner(sel)
}

fn format_select_inner(sel: &sqlrustgo_parser::SelectStatement) -> String {
    // CTAS just needs a recognizable rendering for sqlite_master.sql —
    // produce a compact canonical form using only the SELECT projection
    // columns and the from-relation list. We don't need to model every
    // SELECT feature (joins, CTEs, etc.) for introspection purposes; the
    // result is human-readable metadata, not executable SQL.
    let mut out = String::from("SELECT ");
    let cols: Vec<String> = sel
        .columns
        .iter()
        .map(|c| {
            let base = if c.name.is_empty() || c.name == "*" {
                "*".to_string()
            } else {
                c.name.clone()
            };
            match &c.alias {
                Some(a) => format!("{} AS {}", base, a),
                None => base,
            }
        })
        .collect();
    out.push_str(&cols.join(", "));
    if !sel.table.is_empty() {
        out.push_str(" FROM ");
        out.push_str(&sel.table);
    }
    if let Some(where_clause) = &sel.where_clause {
        out.push_str(" WHERE ");
        out.push_str(&format!("{:?}", where_clause));
    }
    out
}

/// Format a `CREATE VIEW` AST back to a single-line SQL string. Falls back
/// to the parser's debug dump for the inner SELECT when its shape is too
/// exotic for a clean round-trip — sqlite_master.sql only needs to be
/// human-readable.
pub fn format_create_view_sql(view: &CreateViewStatement) -> String {
    let mut out = String::from("CREATE VIEW ");
    out.push_str(&view.name);
    if !view.columns.is_empty() {
        out.push_str(" (");
        out.push_str(&view.columns.join(", "));
        out.push(')');
    }
    out.push_str(" AS ");
    match view.query.as_ref() {
        sqlrustgo_parser::Statement::Select(sel) => {
            out.push_str(&format_select_for_create_as(sel));
        }
        _ => {
            // Non-SELECT defining query (extremely rare for views).
            out.push_str(&format!("{:?}", view.query));
        }
    }
    out
}

/// Format a `CREATE [UNIQUE] INDEX` AST back to a single-line SQL string.
pub fn format_create_index_sql(idx: &CreateIndexStatement) -> String {
    let mut out = String::from("CREATE ");
    if idx.unique {
        out.push_str("UNIQUE INDEX ");
    } else {
        out.push_str("INDEX ");
    }
    out.push_str(&idx.name);
    out.push_str(" ON ");
    out.push_str(&idx.table);
    out.push_str(" (");
    out.push_str(
        &idx
            .columns
            .iter()
            .map(|c| match &c.name {
                Some(n) => n.clone(),
                None => format!("{:?}", c.expression),
            })
            .collect::<Vec<_>>()
            .join(", "),
    );
    out.push(')');
    out
}

/// Format a `CREATE TRIGGER` AST back to a single-line SQL string.
/// V312-64d: triggers may have multi-event / multi-statement bodies, so
/// we keep the format minimal and fall back to the body verbatim when
/// the AST shape is non-standard.
pub fn format_create_trigger_sql(stmt: &CreateTriggerStatement) -> String {
    let mut out = String::from("CREATE TRIGGER ");
    out.push_str(&stmt.name);
    out.push(' ');
    out.push_str(&stmt.timing);
    if !stmt.events.is_empty() {
        out.push(' ');
        out.push_str(&stmt.events.join(" OR "));
    }
    out.push_str(" ON ");
    out.push_str(&stmt.table);
    out.push_str(" FOR EACH ROW ");
    out.push_str(&stmt.body);
    out
}
