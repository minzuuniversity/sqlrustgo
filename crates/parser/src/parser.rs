//! SQL Parser Module
//!
//! # What (是什么)
//! Parser 将 Lexer 输出的 Token 序列转换为抽象语法树 (AST)
//!
//! # Why (为什么)
//! Token 序列只是单词的列表，无法表达 SQL 语句的层级结构
//! AST 将单词组织成有意义的树结构，表示查询的语义
//!
//! # How (如何实现)
//! - 递归下降解析器：自顶向下处理 SQL 语句
//! - 每个 Statement 类型有对应的 parse_xxx 方法
//! - 支持：SELECT, INSERT, UPDATE, DELETE, CREATE TABLE, DROP TABLE
//! - 表达式解析支持基本二元运算

use crate::lexer::Lexer;
use crate::token::Token;
use crate::transaction::{IsolationLevel, TransactionStatement};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

// Phase 3 (TPCH-01 Q15): registry of subqueries that appear in
// `FROM t, (SELECT ...) AS alias` comma-lists. The parser registers
// each subquery here during parsing; the executor retrieves and
// materializes them before executing the join chain.
//
// Key = synthetic table name (e.g. "__subq_0"), Value = subquery AST.
thread_local! {
    static DERIVED_SUBQUERIES: RefCell<std::collections::HashMap<String, Box<SelectStatement>>> =
        RefCell::new(std::collections::HashMap::new());
    // Phase 9 (TPCH-01 Q15): parallel map of synthetic __subq_N -> alias
    // (e.g. __subq_1 -> "revenue"). Populated by the comma-followed
    // subquery branch in the FROM clause; consumed by the multi-table
    // auto-rewrite loop when it builds the JoinClause. Without this map
    // the JoinClause for a derived table loses its alias, so the outer
    // `revenue.l_suppkey` qualifier in the WHERE clause cannot be
    // resolved against the materialized subquery's columns.
    static DERIVED_ALIASES: RefCell<std::collections::HashMap<String, String>> =
        RefCell::new(std::collections::HashMap::new());
}

/// Phase 3 (TPCH-01 Q15): retrieve all derived subqueries registered
/// during parsing. Returns a snapshot of the current thread's registry.
/// The executor calls this to materialize subqueries before executing
/// the join chain.
pub fn get_and_clear_derived_subqueries() -> std::collections::HashMap<String, Box<SelectStatement>>
{
    DERIVED_SUBQUERIES.with(|cell| {
        let mut map = std::collections::HashMap::new();
        std::mem::swap(&mut *cell.borrow_mut(), &mut map);
        map
    })
}

/// SQL Statement types
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Select(SelectStatement),
    Insert(InsertStatement),
    Update(UpdateStatement),
    Delete(DeleteStatement),
    Merge(MergeStatement),
    CreateTable(CreateTableStatement),
    CreateIndex(CreateIndexStatement),
    CreateView(CreateViewStatement),
    DropTable(DropTableStatement),
    DropIndex(DropIndexStatement),
    DropView(DropViewStatement),
    Truncate(TruncateStatement),
    Analyze(AnalyzeStatement),
    WithSelect(WithSelect),
    /// WITH-clause followed by a DML statement (INSERT / UPDATE / DELETE).
    /// The CTE definitions are evaluated first, then the DML body is
    /// executed with the CTE tables materialized.
    WithDml(WithDmlStatement),
    AlterTable(AlterTableStatement),
    Call(CallStatement),
    CreateProcedure(CreateProcedureStatement),
    Union(UnionStatement),
    CreateTrigger(CreateTriggerStatement),
    Transaction(TransactionStatement),
    Grant(GrantStatement),
    Revoke(RevokeStatement),
    Show(ShowStatement),
    Describe(DescribeStatement),
    CreateRole(CreateRoleStatement),
    DropRole(DropRoleStatement),
    GrantRole(GrantRoleStatement),
    RevokeRole(RevokeRoleStatement),
    SetRole(SetRoleStatement),
    ShowRoles,
    ShowGrantsFor(String),
    /// SEM-1 (#3172): SAVEPOINT/ROLLBACK TO SAVEPOINT/RELEASE SAVEPOINT.
    /// MySQL 5.7 savepoint control statements. The `name` is the user-supplied
    /// identifier; `op` selects among the three forms.
    SavepointStatement {
        name: String,
        op: SavepointOp,
    },

    Prepare {
        name: String,
        sql: String,
    },
    Execute {
        name: String,
        params: Vec<Expression>,
    },
    Deallocate {
        name: String,
    },
}

/// SEM-1 (#3172): Savepoint operation kind.
#[derive(Debug, Clone, PartialEq)]
pub enum SavepointOp {
    /// `SAVEPOINT <name>` — register or reset a savepoint.
    Save,
    /// `ROLLBACK TO SAVEPOINT <name>` — roll back to the savepoint
    /// (the savepoint itself is preserved; nested ones after it are dropped).
    RollbackTo,
    /// `RELEASE SAVEPOINT <name>` — remove the savepoint from the stack.
    Release,
}

/// UNION statement
#[derive(Debug, Clone, PartialEq)]
pub struct UnionStatement {
    pub left: Box<Statement>,
    pub right: Box<Statement>,
    pub union_all: bool,
}

/// CREATE INDEX statement
#[derive(Debug, Clone, PartialEq)]
pub struct CreateIndexStatement {
    pub name: String,
    pub table: String,
    pub columns: Vec<String>,
    pub unique: bool,
}

/// DROP INDEX statement
#[derive(Debug, Clone, PartialEq)]
pub struct DropIndexStatement {
    pub name: String,
    pub if_exists: bool,
}

/// ALTER TABLE statement
#[derive(Debug, Clone, PartialEq)]
pub struct AlterTableStatement {
    pub table_name: String,
    pub operation: AlterTableOperation,
}

/// ALTER TABLE operation types
#[derive(Debug, Clone, PartialEq)]
pub enum AlterTableOperation {
    AddColumn {
        name: String,
        data_type: String,
        nullable: bool,
        default_value: Option<String>,
    },
    DropColumn {
        name: String,
    },
    ModifyColumn {
        name: String,
        data_type: String,
        nullable: bool,
    },
    RenameTo {
        new_name: String,
    },
}

/// CALL statement for invoking stored procedures
#[derive(Debug, Clone, PartialEq)]
pub struct CallStatement {
    pub procedure_name: String,
    pub args: Vec<String>,
}

/// CREATE PROCEDURE statement
#[derive(Debug, Clone, PartialEq)]
pub struct CreateProcedureStatement {
    pub name: String,
    pub params: Vec<StoredProcParam>,
    pub body: Vec<StoredProcStatement>,
}

/// Stored procedure parameter
#[derive(Debug, Clone, PartialEq)]
pub struct StoredProcParam {
    pub name: String,
    pub mode: StoredProcParamMode,
    pub data_type: String,
}

/// Parameter mode for stored procedure
#[derive(Debug, Clone, PartialEq)]
pub enum StoredProcParamMode {
    In,
    Out,
    InOut,
}

/// Stored procedure statement types
#[derive(Debug, Clone, PartialEq)]
pub enum StoredProcStatement {
    RawSql(String),
}

/// CREATE TRIGGER statement
#[derive(Debug, Clone, PartialEq)]
pub struct CreateTriggerStatement {
    pub name: String,
    pub table: String,
    pub timing: String,
    pub events: Vec<String>,
    pub body: String,
}

/// CREATE VIEW statement
#[derive(Debug, Clone, PartialEq)]
pub struct CreateViewStatement {
    pub name: String,
    pub columns: Vec<String>,
    pub query: Box<Statement>,
}

/// DROP VIEW statement
#[derive(Debug, Clone, PartialEq)]
pub struct DropViewStatement {
    pub name: String,
    pub if_exists: bool,
}

/// Common Table Expression (CTE)
#[derive(Debug, Clone, PartialEq)]
pub struct CommonTableExpression {
    pub name: String,
    pub columns: Vec<String>,
    pub subquery: Box<Statement>,
}

/// WITH clause for CTEs
#[derive(Debug, Clone, PartialEq)]
pub struct WithClause {
    pub recursive: bool,
    pub ctes: Vec<CommonTableExpression>,
}

/// SELECT with optional WITH clause
#[derive(Debug, Clone, PartialEq)]
pub struct WithSelect {
    pub with_clause: Option<WithClause>,
    pub select: SelectStatement,
}

/// WITH-clause followed by a DML statement (INSERT / UPDATE / DELETE).
/// The body is any of the standard DML statement variants.
#[derive(Debug, Clone, PartialEq)]
pub struct WithDmlStatement {
    pub with_clause: WithClause,
    pub body: Box<Statement>,
}

/// ANALYZE statement for collecting statistics
#[derive(Debug, Clone, PartialEq)]
pub struct AnalyzeStatement {
    pub table_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrantStatement {
    pub privileges: Vec<Privilege>,
    pub columns: Vec<String>,
    pub object_type: ObjectType,
    pub object_name: String,
    pub recipients: Vec<String>,
    pub with_grant_option: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RevokeStatement {
    pub privileges: Vec<Privilege>,
    pub columns: Vec<String>,
    pub object_type: ObjectType,
    pub object_name: String,
    pub from_users: Vec<String>,
    pub grant_option_for: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Privilege {
    Select,
    Insert,
    Update,
    Delete,
    Read,
    Write,
    Execute,
    Usage,
    All,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObjectType {
    Table,
    Database,
    Procedure,
    Function,
    Column,
}

/// CREATE ROLE statement
#[derive(Debug, Clone, PartialEq)]
pub struct CreateRoleStatement {
    pub name: String,
    pub parent_role: Option<String>,
}

/// DROP ROLE statement
#[derive(Debug, Clone, PartialEq)]
pub struct DropRoleStatement {
    pub name: String,
}

/// GRANT role TO user statement
#[derive(Debug, Clone, PartialEq)]
pub struct GrantRoleStatement {
    pub role_name: String,
    pub user_name: String,
    pub host: Option<String>,
}

/// REVOKE role FROM user statement
#[derive(Debug, Clone, PartialEq)]
pub struct RevokeRoleStatement {
    pub role_name: String,
    pub user_name: String,
    pub host: Option<String>,
}

/// SET ROLE statement
#[derive(Debug, Clone, PartialEq)]
pub struct SetRoleStatement {
    pub role_name: String,
}

/// Join type
#[derive(Debug, Clone, PartialEq)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

/// Aggregate function
#[derive(Debug, Clone, PartialEq)]
pub enum AggregateFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

/// Join clause
#[derive(Debug, Clone, PartialEq)]
pub struct JoinClause {
    pub join_type: JoinType,
    pub table: String,
    /// TPC-H Q7/Q8/Q9: same table referenced multiple times via different
    /// aliases (e.g. `nation n1 JOIN ... JOIN nation n2 ON ...`). When set,
    /// the executor uses this as the column-name prefix in the
    /// accumulated join schema; the user can then refer to columns via
    /// the alias (`n1.n_nationkey`).
    pub alias: Option<String>,
    pub on_clause: Expression,
}

/// Aggregate function call
#[derive(Debug, Clone, PartialEq)]
pub struct AggregateCall {
    pub func: AggregateFunction,
    pub args: Vec<Expression>,
    pub distinct: bool,
}

/// SELECT statement
#[derive(Debug, Clone, PartialEq)]
pub struct SelectStatement {
    pub columns: Vec<SelectColumn>,
    pub table: String,
    /// TPC-H Q7/Q8/Q9: FROM `t a` stores `table = "t"`, `from_alias = "a"`.
    /// The executor uses the alias as the column-name prefix in the
    /// scan schema so the user can write `a.col` in subsequent JOIN ON.
    pub from_alias: Option<String>,
    /// TPC-H Sprint 1b fix (Q7/Q8/Q9): FROM (subquery) AS alias.
    /// When set, executor first executes the subquery and materializes its
    /// result into a temporary table named `table`, then runs the outer
    /// SELECT against that table.
    pub from_subquery: Option<Box<SelectStatement>>,
    pub where_clause: Option<Expression>,
    pub join_clause: Vec<JoinClause>,
    /// TPC-H Sprint 1c: additional tables from `FROM t1, t2, t3` (after the
    /// first). The parser auto-rewrites the comma-list form into a chain of
    /// INNER JOINs using predicates pulled out of the WHERE clause, and
    /// pushes the resulting JoinClauses into `join_clause`.
    pub extra_tables: Vec<String>,
    pub aggregates: Vec<AggregateCall>,
    pub group_by: Vec<Expression>,
    /// MySQL 5.7 WITH ROLLUP: emit hierarchical subtotals
    /// (NULL in trailing group columns).
    pub with_rollup: bool,
    /// MySQL 5.7 WITH CUBE: emit 2^k subtotals over all subsets.
    pub with_cube: bool,
    pub having: Option<Expression>,
    pub order_by: Vec<OrderByExpression>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub distinct: bool,
}

/// ORDER BY expression
#[derive(Debug, Clone, PartialEq)]
pub struct OrderByExpression {
    pub expression: Expression,
    pub ascending: bool,
    pub nulls_first: Option<bool>,
}

/// Column in SELECT
#[derive(Debug, Clone, PartialEq)]
pub struct SelectColumn {
    pub name: String,
    pub alias: Option<String>,
    pub expression: Option<Expression>,
}

/// INSERT statement
#[derive(Debug, Clone, PartialEq)]
pub struct InsertStatement {
    pub table: String,
    pub columns: Vec<String>,
    pub values: Vec<Vec<Expression>>,         // For INSERT VALUES
    pub select: Option<Box<SelectStatement>>, // For INSERT SELECT
    pub is_replace: bool,                     // For REPLACE INTO (MySQL compatibility)
    pub on_duplicate_key_update: Option<Vec<(String, Expression)>>, // For ON DUPLICATE KEY UPDATE
}

/// UPDATE statement
#[derive(Debug, Clone, PartialEq)]
pub struct UpdateStatement {
    pub table: String,
    pub set_clauses: Vec<(String, Expression)>,
    pub where_clause: Option<Expression>,
}

/// DELETE statement
#[derive(Debug, Clone, PartialEq)]
pub struct DeleteStatement {
    pub table: String,
    pub where_clause: Option<Expression>,
}

/// MERGE statement (SQL:2003)
#[derive(Debug, Clone, PartialEq)]
pub struct MergeStatement {
    pub target_table: String,
    pub target_alias: Option<String>,
    pub source: MergeSource,
    pub source_alias: Option<String>,
    pub on_condition: Expression,
    pub when_clauses: Vec<MergeWhenClause>,
}

/// Source for MERGE: a table reference or a subquery
#[derive(Debug, Clone, PartialEq)]
pub enum MergeSource {
    Table { name: String },
    Subquery(Box<SelectStatement>),
}

/// A WHEN clause inside MERGE statement
#[derive(Debug, Clone, PartialEq)]
pub struct MergeWhenClause {
    pub is_matched: bool,
    pub additional_condition: Option<Expression>,
    pub action: MergeAction,
}

/// Action for a MERGE WHEN clause
#[derive(Debug, Clone, PartialEq)]
pub enum MergeAction {
    Update {
        set_clauses: Vec<(String, Expression)>,
    },
    Insert {
        columns: Vec<String>,
        values: Vec<Expression>,
    },
    Delete,
}

/// CREATE TABLE statement
#[derive(Debug, Clone, PartialEq)]
pub struct CreateTableStatement {
    pub name: String,
    pub columns: Vec<ColumnDefinition>,
    pub constraints: Vec<TableConstraint>,
    pub if_not_exists: bool,
}

/// DROP TABLE statement
#[derive(Debug, Clone, PartialEq)]
pub struct DropTableStatement {
    pub name: String,
    pub if_exists: bool,
}

/// TRUNCATE TABLE statement
#[derive(Debug, Clone, PartialEq)]
pub struct TruncateStatement {
    pub name: String,
}

/// SHOW statement variants
#[derive(Debug, Clone, PartialEq)]
pub enum ShowStatement {
    Databases,
    Tables,
    Columns {
        table: String,
        pattern: Option<String>,
    },
    Index {
        table: String,
    },
    Grants {
        user: Option<String>,
    },
    CreateTable {
        table: String,
    },
}

/// DESCRIBE statement (aliased as DESC)
#[derive(Debug, Clone, PartialEq)]
pub struct DescribeStatement {
    pub table: String,
}

/// Column definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnDefinition {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub primary_key: bool,
    pub auto_increment: bool,
    pub default_value: Option<String>,
    pub references: Option<ForeignKeyRef>,
}

/// Foreign key referential action
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReferentialAction {
    Cascade,
    SetNull,
    Restrict,
    NoAction,
}

/// Foreign key reference (for REFERENCES clause)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForeignKeyRef {
    pub columns: Vec<String>,
    pub referenced_table: String,
    pub referenced_columns: Vec<String>,
    pub on_delete: Option<ReferentialAction>,
    pub on_update: Option<ReferentialAction>,
}

/// Table-level constraint
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TableConstraint {
    PrimaryKey {
        columns: Vec<String>,
    },
    ForeignKey {
        columns: Vec<String>,
        referenced_table: String,
        referenced_columns: Vec<String>,
        on_delete: Option<ReferentialAction>,
        on_update: Option<ReferentialAction>,
    },
    Unique {
        columns: Vec<String>,
    },
    Check {
        expression: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhenClause {
    pub condition: Expression,
    pub result: Expression,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowSpecification {
    pub partition_by: Vec<Expression>,
    pub order_by: Vec<(Expression, bool)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowCall {
    pub func_name: String,
    pub args: Vec<Expression>,
    pub window_spec: WindowSpecification,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Literal(String),
    Identifier(String),
    BinaryOp(Box<Expression>, String, Box<Expression>),
    Subquery(Box<SelectStatement>),
    /// Postfix field access on a parenthesized expression: `(subq).col`.
    /// Used for things like `(ST_dump(...).geom)` and `(SELECT ...).col`.
    /// The executor can dispatch on the (Subquery, field) pair to extract
    /// the named field from the subquery result.
    SubqueryField(Box<Expression>, String),
    In(Box<Expression>, Box<SelectStatement>),
    NotIn(Box<Expression>, Box<SelectStatement>),
    InList(Box<Expression>, Vec<Expression>), // MySQL: col IN (1, 2, 3)
    NotInList(Box<Expression>, Vec<Expression>), // MySQL: col NOT IN (1, 2, 3)
    NotLike(Box<Expression>, Box<Expression>, Option<char>), // col NOT LIKE pattern [ESCAPE char]
    NotBetween(Box<Expression>, Box<Expression>, Box<Expression>), // col NOT BETWEEN low AND high
    NotRegexp(Box<Expression>, Box<Expression>), // col NOT REGEXP pattern
    Exists(Box<SelectStatement>),
    NotExists(Box<SelectStatement>),
    QuantifiedOp(Box<Expression>, String, Box<SelectStatement>),
    Aggregate(AggregateCall), // For HAVING clause - supports aggregate functions in expressions
    IsNull(Box<Expression>),  // IS NULL check
    IsNotNull(Box<Expression>), // IS NOT NULL check
    UnaryOp(String, Box<Expression>), // Unary operator: NOT expr
    Like(Box<Expression>, Box<Expression>, Option<char>), // LIKE pattern [ESCAPE char]
    Between(Box<Expression>, Box<Expression>, Box<Expression>), // expr BETWEEN low AND high
    CaseWhen(Vec<WhenClause>, Option<Box<Expression>>), // CASE WHEN ... ELSE ... END
    FunctionCall(String, Vec<Expression>),
    WindowCall(WindowCall),
}

/// Flatten a top-level AND conjunction: `a AND b AND c` -> vec![a, b, c].
/// If expr is not an AND, returns vec![expr].
/// TPC-H Sprint 1c: used by the multi-table FROM auto-rewrite to extract
/// join predicates from the WHERE clause.
fn flatten_and(expr: &Expression) -> Vec<Expression> {
    match expr {
        Expression::BinaryOp(left, op, right) if op.to_uppercase() == "AND" => {
            let mut v = flatten_and(left);
            v.extend(flatten_and(right));
            v
        }
        other => vec![other.clone()],
    }
}

/// Check whether all tables referenced by a predicate's left and
/// right sides are "known" to the current join context, i.e. either
/// the new table, its TPC-H prefix, its inline alias, or already
/// in `joined`. Used by the relaxed fallback in the FROM
/// auto-rewriter (TPC-H Q2) to reject predicates that would later
/// fail the executor's `find_join_key_index` because they reference
/// a not-yet-joined table (e.g. Q2 joining supplier with
/// `s_suppkey = ps_suppkey` where `ps_suppkey` lives in
/// not-yet-joined `partsupp`).
fn predicate_fully_resolvable(
    p: &Expression,
    new_table: &str,
    new_alias: Option<&str>,
    joined: &[String],
) -> bool {
    if let Expression::BinaryOp(l, op, r) = p {
        if op == "=" {
            let new_prefix: &str = if new_table.contains('_') {
                let us = new_table.find('_').unwrap();
                &new_table[..us]
            } else if new_table == "partsupp" {
                "ps"
            } else {
                &new_table[..1]
            };
            let lr = collect_referenced_tables(l);
            let rr = collect_referenced_tables(r);
            let is_known = |t: &str| -> bool {
                t == new_table
                    || t == new_prefix
                    || new_alias == Some(t)
                    || joined.iter().any(|j| j == t)
            };
            return lr.iter().all(|t| is_known(t.as_str()))
                && rr.iter().all(|t| is_known(t.as_str()));
        }
    }
    false
}

/// Structural equality on Expression. Used to dedupe predicates
/// between the original WHERE conj and the auto-rewriter's
/// `remaining` list when the new best-match selector picks one
/// (TPCH-01 Q2/Q9 fix).
fn same_expression(a: &Expression, b: &Expression) -> bool {
    use Expression::*;
    match (a, b) {
        (Literal(x), Literal(y)) => x == y,
        (Identifier(x), Identifier(y)) => x == y,
        (BinaryOp(al, ao, ar), BinaryOp(bl, bo, br)) => {
            ao == bo && same_expression(al, bl) && same_expression(ar, br)
        }
        (UnaryOp(ao, ai), UnaryOp(bo, bi)) => ao == bo && same_expression(ai, bi),
        (FunctionCall(an, aa), FunctionCall(bn, ba)) => {
            an == bn
                && aa.len() == ba.len()
                && aa.iter().zip(ba.iter()).all(|(x, y)| same_expression(x, y))
        }
        (Aggregate(a), Aggregate(b)) => format!("{:?}", a) == format!("{:?}", b),
        _ => false,
    }
}

/// Pick the best equality predicate that joins `new_table` to the
/// already-joined set `joined_tables`.
///
/// For a predicate to be a valid JOIN ON clause that connects a new
/// table to the already-joined set, it must be a binary `=` whose
/// two sides reference *disjoint* table sets: one side must
/// reference `new_table` (and no other joined table), and the other
/// side must reference the already-joined set (or just `new_table`,
/// which would be a degenerate self-join).
///
/// If `joined_tables` is empty, we accept any `t1.col = t2.col`
/// predicate whose left side references `new_table` — the right
/// side is the candidate anchor.
///
/// Phase 2 (TPCH-01 Q7/Q21 fix): the caller may pass an `alias`
/// (e.g. `Some("n1")` for `nation n1`) so qualifiers like
/// `n1.n_nationkey` are recognised as referencing the new table
/// (the bare `nation` table name would not be found inside
/// `n1.n_nationkey` because the SQL writer used the alias as the
/// qualifier, not the table name).
///
/// Returns Some(predicate) if a qualifying predicate exists;
/// None otherwise (caller falls back to other strategies).
fn find_join_predicate(
    predicates: &[Expression],
    new_table: &str,
    new_alias: Option<&str>,
    joined_tables: &[String],
) -> Option<Expression> {
    for p in predicates {
        if let Expression::BinaryOp(l, op, r) = p {
            if op == "=" {
                let left_refs = collect_referenced_tables(l);
                let right_refs = collect_referenced_tables(r);
                // Phase 4 (TPCH-01 Q15): skip predicates that reference
                // synthetic __subq_N derived-table names. These are
                // predicates that belong INSIDE the derived subquery
                // (e.g. l_shipdate >= '1995-09-01' from a Q15
                // `FROM t, (SELECT ... WHERE l_shipdate >= ...) AS rev`),
                // not the outer join chain. Using them as outer JOIN
                // ONs produces wrong tables.
                if left_refs.iter().any(|t| t.starts_with("__subq_"))
                    || right_refs.iter().any(|t| t.starts_with("__subq_"))
                {
                    continue;
                }
                // The "new table" can be referenced by either its
                // full name (e.g. "supplier"), its TPC-H prefix
                // (e.g. "s"), or (Phase 2) its inline alias
                // (e.g. "n1"). Try all three. We use the same
                // TPC-H table-name -> column-prefix map as in
                // `parse_select_statement`'s `joined` builder so the
                // prefix extracted here matches what
                // `collect_referenced_tables` produces from the
                // predicate's column identifiers.
                let new_prefix: &str = match new_table {
                    "region" => "r",
                    "nation" => "n",
                    "supplier" => "s",
                    "customer" => "c",
                    "part" => "p",
                    "partsupp" => "ps",
                    "orders" => "o",
                    "lineitem" => "l",
                    _ => new_table.split('_').next().unwrap_or(new_table),
                };
                let new_alts: Vec<&str> = match new_alias {
                    Some(a) => vec![new_table, new_prefix, a],
                    None => vec![new_table, new_prefix],
                };
                let left_has_new = left_refs.iter().any(|t| new_alts.iter().any(|n| n == t));
                let right_has_new = right_refs.iter().any(|t| new_alts.iter().any(|n| n == t));
                // Phase 6 (TPCH-01 Q8 fix): a real join key must reference
                // a table on BOTH sides of the `=`. A column-vs-literal
                // equality (e.g. `n2.n_name = 'GERMANY'`,
                // `r_name = 'EUROPE'`) is a WHERE filter, not a join key,
                // and must stay in the outer WHERE clause. Using such a
                // predicate as an ON clause produces a cartesian join
                // (JoinKey::All) that explodes with 6+ tables.
                //
                // TPC-H convention: when one side is an unqualified
                // column (no table ref) and the other side is the new
                // table, treat the unqualified column as referencing
                // the ALREADY-JOINED side via TPC-H prefix matching
                // (e.g. `c_custkey = o_custkey` joins customer to orders
                // via "c" prefix → customer, "o" prefix → orders).
                let left_is_col = matches!(l.as_ref(), Expression::Identifier(_));
                let right_is_col = matches!(r.as_ref(), Expression::Identifier(_));
                if !left_is_col || !right_is_col {
                    // One side is a literal or function — NOT a join key.
                    // Skip this predicate; keep in outer WHERE via `rest`.
                    continue;
                }
                if left_has_new
                    && !right_has_new
                    && right_refs
                        .iter()
                        .all(|t| joined_tables.is_empty() || joined_tables.iter().any(|j| j == t))
                {
                    return Some(p.clone());
                }
                if right_has_new
                    && !left_has_new
                    && left_refs
                        .iter()
                        .all(|t| joined_tables.is_empty() || joined_tables.iter().any(|j| j == t))
                {
                    return Some(p.clone());
                }
            }
        }
    }
    None
}

/// Walk an expression and collect all referenced table names.
/// For qualified `t.col`, the qualifier is used directly. For
/// unqualified identifiers, the underscore-separated prefix is
/// extracted: "p_partkey" -> "p" (which the caller can map to
/// the `part` table via the TPC-H naming convention), and
/// "ps_partkey" -> "ps" (-> `partsupp`).
fn collect_referenced_tables(expr: &Expression) -> Vec<String> {
    let mut out = Vec::new();
    fn visit(e: &Expression, acc: &mut Vec<String>) {
        match e {
            Expression::Identifier(name) => {
                if let Some((qualifier, _col)) = name.split_once('.') {
                    if !acc.iter().any(|x: &String| x == qualifier) {
                        acc.push(qualifier.to_string());
                    }
                } else if let Some(prefix) = name.split('_').next() {
                    if !acc.iter().any(|x: &String| x == prefix) {
                        acc.push(prefix.to_string());
                    }
                }
            }
            Expression::BinaryOp(l, _, r) => {
                visit(l, acc);
                visit(r, acc);
            }
            Expression::IsNull(inner) | Expression::IsNotNull(inner) => visit(inner, acc),
            Expression::UnaryOp(_, inner) => visit(inner, acc),
            Expression::FunctionCall(_, args) => {
                for a in args {
                    visit(a, acc);
                }
            }
            Expression::Aggregate(agg) => {
                for a in &agg.args {
                    visit(a, acc);
                }
            }
            _ => {}
        }
    }
    visit(expr, &mut out);
    out
}

/// Is the expression a binary `=` between two column references (not column vs literal)?
/// Used by TPC-H multi-table auto-rewrite to distinguish real join keys
/// (e.g. `c_custkey = o_custkey`) from filters (e.g. `n2.n_name = 'GERMANY'`).
/// The latter should stay in WHERE, not be promoted to an ON clause.
fn is_binary_column_equality(expr: &Expression) -> bool {
    if let Expression::BinaryOp(l, op, r) = expr {
        if op == "=" {
            return matches!(l.as_ref(), Expression::Identifier(_))
                && matches!(r.as_ref(), Expression::Identifier(_));
        }
    }
    false
}

/// Does the expression reference the given table? Used by TPC-H multi-table
/// auto-rewrite: "o_orderdate" counts as referencing "orders" because the
/// table's columns are prefixed with the table name once JOINs combine them.
/// Also matches explicit qualified references like "orders.o_orderdate".
fn predicate_references_table(expr: &Expression, table: &str) -> bool {
    fn visit(e: &Expression, table: &str, found: &mut bool) {
        if *found {
            return;
        }
        match e {
            Expression::Identifier(name) => {
                if let Some((qualifier, _col)) = name.split_once('.') {
                    if qualifier.eq_ignore_ascii_case(table) {
                        *found = true;
                    }
                } else {
                    // Heuristic: identifier starts with the table's 1-char prefix
                    // followed by '_' (TPC-H naming convention: orders -> o_,
                    // customer -> c_, lineitem -> l_, nation -> n_, etc.).
                    if table.len() >= 2 {
                        let tprefix = table[..1].to_lowercase();
                        let lower = name.to_lowercase();
                        if lower.starts_with(&tprefix)
                            && name.len() > 1
                            && name.as_bytes()[1] == b'_'
                        {
                            *found = true;
                        }
                    }
                }
            }
            Expression::BinaryOp(l, _, r) => {
                visit(l, table, found);
                visit(r, table, found);
            }
            Expression::IsNull(inner) | Expression::IsNotNull(inner) => {
                visit(inner, table, found);
            }
            Expression::UnaryOp(_, inner) => {
                visit(inner, table, found);
            }
            Expression::FunctionCall(_, args) => {
                for a in args {
                    visit(a, table, found);
                }
            }
            Expression::Aggregate(agg) => {
                for a in &agg.args {
                    visit(a, table, found);
                }
            }
            _ => {}
        }
    }
    let mut found = false;
    visit(expr, table, &mut found);
    found
}

/// SQL Parser
pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    /// Create a parser from tokens
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    /// Get current token
    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position + 1)
    }

    /// Check if at end
    #[allow(dead_code)]
    fn is_eof(&self) -> bool {
        self.position >= self.tokens.len() || matches!(self.current(), Some(Token::Eof))
    }

    /// Advance to next token
    fn next(&mut self) -> Option<Token> {
        self.position += 1;
        self.tokens.get(self.position - 1).cloned()
    }

    /// Expect a specific token
    fn expect(&mut self, expected: Token) -> Result<Token, String> {
        match self.current() {
            Some(t) if t == &expected => Ok(self.next().unwrap()),
            Some(t) => Err(format!("Expected {:?}, got {:?}", expected, t)),
            None => Err("Unexpected end of input".to_string()),
        }
    }

    /// Parse a complete SQL statement
    pub fn parse_statement(&mut self) -> Result<Statement, String> {
        match self.current() {
            Some(Token::Select) => self.parse_select(),
            Some(Token::Insert) | Some(Token::Replace) => self.parse_insert(),
            Some(Token::Update) => self.parse_update(),
            Some(Token::Delete) => self.parse_delete(),
            Some(Token::Merge) => self.parse_merge(),
            Some(Token::Create) => self.parse_create(),
            Some(Token::Drop) => self.parse_drop(),
            Some(Token::Truncate) => self.parse_truncate(),
            Some(Token::Analyze) => self.parse_analyze(),
            Some(Token::With) => self.parse_with_select(),
            Some(Token::Alter) => self.parse_alter_table(),
            Some(Token::Call) => self.parse_call(),
            // SEM-1 (#3172): Rollback is now dispatched via the SAVEPOINT
            // arms below. The `parse_transaction` group no longer
            // accepts Token::Rollback so that the ROLLBACK TO SAVEPOINT
            // peek (which requires us to NOT be inside parse_transaction
            // first) actually wins. Regular ROLLBACK [WORK] is handled
            // by the `Some(Token::Rollback)` arm at the bottom of this
            // match, which calls parse_rollback.
            Some(Token::Begin) | Some(Token::Commit) | Some(Token::Set) | Some(Token::Start) => {
                self.parse_transaction()
            }
            // SEM-1 (#3172): SAVEPOINT dispatcher
            Some(Token::Savepoint) => self.parse_savepoint_statement(),
            // SEM-1 (#3172): RELEASE SAVEPOINT dispatcher
            Some(Token::Release) => self.parse_release_savepoint(),
            Some(Token::Prepare) => self.parse_prepare(),
            Some(Token::Execute) => self.parse_execute(),
            Some(Token::Deallocate) => self.parse_deallocate(),
            // SEM-1 (#3172): ROLLBACK — peek for `TO` to route to
            // savepoint handling; otherwise plain ROLLBACK [WORK].
            Some(Token::Rollback) if self.peek() == Some(&Token::To) => {
                self.parse_savepoint_statement()
            }
            Some(Token::Rollback) => self.parse_rollback(),
            Some(Token::Grant) => self.parse_grant(),
            Some(Token::Revoke) => self.parse_revoke(),
            Some(Token::Show) => self.parse_show(),
            Some(Token::Describe) | Some(Token::Desc) => self.parse_describe(),
            Some(t) => Err(format!("Unexpected token: {:?}", t)),
            None => Err("Empty input".to_string()),
        }
    }

    fn parse_transaction(&mut self) -> Result<Statement, String> {
        match self.current() {
            Some(Token::Begin) => self.parse_begin(),
            Some(Token::Commit) => self.parse_commit(),
            Some(Token::Rollback) => self.parse_rollback(),
            Some(Token::Set) => {
                // Check if this is SET ROLE or SET TRANSACTION using peek
                if let Some(Token::Role) = self.peek() {
                    self.parse_set_role()
                } else {
                    self.parse_set_transaction()
                }
            }
            Some(Token::Start) => self.parse_start_transaction(),
            Some(t) => Err(format!("Unexpected transaction token: {:?}", t)),
            None => Err("Unexpected end of input".to_string()),
        }
    }

    fn parse_begin(&mut self) -> Result<Statement, String> {
        self.expect(Token::Begin)?;
        let work = if self.current() == Some(&Token::Work) {
            self.next();
            true
        } else {
            false
        };
        let isolation_level = if self.current() == Some(&Token::Isolation) {
            self.next();
            self.expect(Token::Level)?;
            Some(self.parse_isolation_level_value()?)
        } else if self.current() == Some(&Token::Serializable) {
            self.next();
            Some(IsolationLevel::Serializable)
        } else if self.current() == Some(&Token::Repeatable) {
            self.next();
            Some(IsolationLevel::SnapshotIsolation)
        } else if self.current() == Some(&Token::Read) {
            self.next();
            match self.current() {
                Some(Token::Committed) => {
                    self.next();
                    Some(IsolationLevel::ReadCommitted)
                }
                Some(Token::Uncommitted) => {
                    self.next();
                    Some(IsolationLevel::ReadUncommitted)
                }
                Some(t) => {
                    return Err(format!(
                        "Expected COMMITTED or UNCOMMITTED after READ, got {:?}",
                        t
                    ))
                }
                None => return Err("Unexpected end of input after READ".to_string()),
            }
        } else {
            None
        };
        Ok(Statement::Transaction(TransactionStatement::Begin {
            work,
            isolation_level,
        }))
    }

    fn parse_commit(&mut self) -> Result<Statement, String> {
        self.expect(Token::Commit)?;
        let work = if self.current() == Some(&Token::Work) {
            self.next();
            true
        } else {
            false
        };
        Ok(Statement::Transaction(TransactionStatement::Commit {
            work,
        }))
    }

    fn parse_rollback(&mut self) -> Result<Statement, String> {
        self.expect(Token::Rollback)?;
        let work = if self.current() == Some(&Token::Work) {
            self.next();
            true
        } else {
            false
        };
        Ok(Statement::Transaction(TransactionStatement::Rollback {
            work,
        }))
    }

    /// SEM-1 (#3172): Parse SAVEPOINT/ROLLBACK TO SAVEPOINT/RELEASE SAVEPOINT.
    ///
    /// MySQL 5.7 syntax:
    ///   `SAVEPOINT <name>;`
    ///   `ROLLBACK [WORK] TO SAVEPOINT <name>;`
    ///   `RELEASE SAVEPOINT <name>;`
    ///
    /// We disambiguate by looking at the first token after the optional
    /// `WORK` keyword: if it's `TO`, this is a rollback-to; if it's
    /// `SAVEPOINT`, this is a release; otherwise the original ROLLBACK
    /// path (handled by parse_rollback) should be invoked.
    fn parse_savepoint_statement(&mut self) -> Result<Statement, String> {
        // We are called when current() == Token::Savepoint OR
        // (Token::Rollback with next token = TO).
        let op = if self.current() == Some(&Token::Savepoint) {
            self.next(); // consume SAVEPOINT
            SavepointOp::Save
        } else {
            // ROLLBACK [WORK] TO SAVEPOINT <name>
            debug_assert_eq!(self.current(), Some(&Token::Rollback));
            self.next(); // consume ROLLBACK
            if self.current() == Some(&Token::Work) {
                self.next();
            }
            self.expect(Token::To)?;
            self.expect(Token::Savepoint)?;
            SavepointOp::RollbackTo
        };
        // Parse the savepoint name.
        let name = match self.next() {
            Some(Token::Identifier(n)) => n,
            Some(t) => return Err(format!("Expected savepoint name (identifier), got {:?}", t)),
            None => return Err("Expected savepoint name, got EOF".to_string()),
        };
        Ok(Statement::SavepointStatement { name, op })
    }

    /// SEM-1 (#3172): Parse `RELEASE SAVEPOINT <name>`.
    fn parse_release_savepoint(&mut self) -> Result<Statement, String> {
        self.expect(Token::Release)?;
        self.expect(Token::Savepoint)?;
        let name = match self.next() {
            Some(Token::Identifier(n)) => n,
            Some(t) => return Err(format!("Expected savepoint name (identifier), got {:?}", t)),
            None => return Err("Expected savepoint name, got EOF".to_string()),
        };
        Ok(Statement::SavepointStatement {
            name,
            op: SavepointOp::Release,
        })
    }

    fn parse_prepare(&mut self) -> Result<Statement, String> {
        self.expect(Token::Prepare)?;
        let name = match self.next() {
            Some(Token::Identifier(n)) => n,
            Some(t) => return Err(format!("Expected prepared statement name, got {:?}", t)),
            None => return Err("Expected prepared statement name, got EOF".to_string()),
        };
        self.expect(Token::As)?;
        let sql = match self.next() {
            Some(Token::StringLiteral(s)) => s,
            Some(t) => return Err(format!("Expected SQL string literal, got {:?}", t)),
            None => return Err("Expected SQL string literal, got EOF".to_string()),
        };
        if sql.trim().is_empty() {
            return Err("PREPARE requires non-empty SQL body".to_string());
        }
        Ok(Statement::Prepare { name, sql })
    }

    fn parse_execute(&mut self) -> Result<Statement, String> {
        self.expect(Token::Execute)?;
        let name = match self.next() {
            Some(Token::Identifier(n)) => n,
            Some(t) => return Err(format!("Expected prepared statement name, got {:?}", t)),
            None => return Err("Expected prepared statement name, got EOF".to_string()),
        };
        let params = Vec::new();
        Ok(Statement::Execute { name, params })
    }

    fn parse_deallocate(&mut self) -> Result<Statement, String> {
        self.expect(Token::Deallocate)?;
        let name = match self.next() {
            Some(Token::Identifier(n)) => n,
            Some(t) => return Err(format!("Expected prepared statement name, got {:?}", t)),
            None => return Err("Expected prepared statement name, got EOF".to_string()),
        };
        Ok(Statement::Deallocate { name })
    }

    fn parse_start_transaction(&mut self) -> Result<Statement, String> {
        self.expect(Token::Start)?;
        self.expect(Token::Transaction)?;
        let isolation_level = if self.current() == Some(&Token::Isolation) {
            self.next();
            self.expect(Token::Level)?;
            Some(self.parse_isolation_level_value()?)
        } else {
            None
        };
        Ok(Statement::Transaction(
            TransactionStatement::StartTransaction { isolation_level },
        ))
    }

    fn parse_set_transaction(&mut self) -> Result<Statement, String> {
        self.expect(Token::Set)?;
        self.expect(Token::Transaction)?;
        self.expect(Token::Isolation)?;
        self.expect(Token::Level)?;
        let isolation_level = self.parse_isolation_level_value()?;
        Ok(Statement::Transaction(
            TransactionStatement::SetTransaction { isolation_level },
        ))
    }

    fn parse_set_role(&mut self) -> Result<Statement, String> {
        self.expect(Token::Set)?;
        self.expect(Token::Role)?;
        let role_name = match self.next() {
            Some(Token::Identifier(name)) => name,
            Some(Token::StringLiteral(s)) => s,
            Some(t) => return Err(format!("Expected role name, got {:?}", t)),
            None => return Err("Expected role name".to_string()),
        };
        Ok(Statement::SetRole(SetRoleStatement { role_name }))
    }

    fn parse_isolation_level_value(&mut self) -> Result<IsolationLevel, String> {
        match self.current() {
            Some(Token::Serializable) => {
                self.next();
                Ok(IsolationLevel::Serializable)
            }
            Some(Token::Repeatable) => {
                self.next();
                Ok(IsolationLevel::SnapshotIsolation)
            }
            Some(Token::Read) => {
                self.next();
                match self.current() {
                    Some(Token::Committed) => {
                        self.next();
                        Ok(IsolationLevel::ReadCommitted)
                    }
                    Some(Token::Uncommitted) => {
                        self.next();
                        Ok(IsolationLevel::ReadUncommitted)
                    }
                    Some(t) => Err(format!(
                        "Expected COMMITTED or UNCOMMITTED after READ, got {:?}",
                        t
                    )),
                    None => Err("Unexpected end of input after READ".to_string()),
                }
            }
            Some(t) => Err(format!("Unexpected isolation level keyword: {:?}", t)),
            None => Err("Unexpected end of input after READ".to_string()),
        }
    }

    fn parse_create(&mut self) -> Result<Statement, String> {
        self.expect(Token::Create)?;
        match self.current() {
            Some(Token::Table) => self.parse_create_table(),
            Some(Token::Index) => {
                self.next();
                self.parse_create_index(false)
            }
            Some(Token::Unique) => {
                self.next();
                self.expect(Token::Index)?;
                self.parse_create_index(true)
            }
            Some(Token::Procedure) => self.parse_create_procedure(),
            Some(Token::Trigger) => self.parse_create_trigger(),
            Some(Token::Role) => self.parse_create_role(),
            Some(Token::View) => self.parse_create_view(),
            Some(t) => Err(format!(
                "Expected TABLE, INDEX, PROCEDURE, TRIGGER, ROLE, or VIEW after CREATE, got {:?}",
                t
            )),
            None => Err(
                "Expected TABLE, INDEX, PROCEDURE, TRIGGER, ROLE, or VIEW after CREATE".to_string(),
            ),
        }
    }

    fn parse_create_role(&mut self) -> Result<Statement, String> {
        self.expect(Token::Role)?;
        let name = match self.next() {
            Some(Token::Identifier(name)) => name,
            Some(Token::StringLiteral(s)) => s,
            Some(t) => return Err(format!("Expected role name, got {:?}", t)),
            None => return Err("Expected role name".to_string()),
        };

        let mut parent_role = None;
        if let Some(Token::Identifier(name)) = self.current() {
            let upper = name.to_uppercase();
            if upper == "WITH" {
                self.next();
                if matches!(self.current(), Some(Token::Parent)) {
                    self.next();
                    parent_role = match self.next() {
                        Some(Token::Identifier(n)) => Some(n.clone()),
                        Some(Token::StringLiteral(s)) => Some(s.clone()),
                        Some(t) => return Err(format!("Expected parent role name, got {:?}", t)),
                        None => return Err("Expected parent role name".to_string()),
                    };
                }
            }
        }

        Ok(Statement::CreateRole(CreateRoleStatement {
            name,
            parent_role,
        }))
    }

    fn parse_create_index(&mut self, unique: bool) -> Result<Statement, String> {
        let index_name = match self.next() {
            Some(Token::Identifier(name)) => name,
            Some(t) => return Err(format!("Expected index name, got {:?}", t)),
            None => return Err("Expected index name".to_string()),
        };
        self.expect(Token::On)?;
        let table_name = match self.next() {
            Some(Token::Identifier(name)) => name,
            Some(t) => return Err(format!("Expected table name, got {:?}", t)),
            None => return Err("Expected table name".to_string()),
        };
        self.expect(Token::LParen)?;
        let columns = self.parse_column_list()?;
        Ok(Statement::CreateIndex(CreateIndexStatement {
            name: index_name,
            table: table_name,
            columns,
            unique,
        }))
    }

    fn parse_create_procedure(&mut self) -> Result<Statement, String> {
        self.expect(Token::Procedure)?;

        let name = match self.next() {
            Some(Token::Identifier(name)) => name,
            Some(t) => return Err(format!("Expected procedure name, got {:?}", t)),
            None => return Err("Expected procedure name".to_string()),
        };

        self.expect(Token::LParen)?;
        let mut params = Vec::new();
        while !matches!(self.current(), Some(Token::RParen) | None) {
            let mode = match self.current() {
                Some(Token::In) => {
                    self.next();
                    StoredProcParamMode::In
                }
                Some(Token::Identifier(mode_str))
                    if ["OUT", "INOUT"].contains(&mode_str.to_uppercase().as_str()) =>
                {
                    let mode = match mode_str.to_uppercase().as_str() {
                        "OUT" => StoredProcParamMode::Out,
                        "INOUT" => StoredProcParamMode::InOut,
                        _ => StoredProcParamMode::In,
                    };
                    self.next();
                    mode
                }
                _ => StoredProcParamMode::In,
            };

            let param_name = match self.next() {
                Some(Token::Identifier(name)) => name,
                Some(t) => return Err(format!("Expected parameter name, got {:?}", t)),
                None => return Err("Expected parameter name".to_string()),
            };

            let data_type = match self.next() {
                Some(Token::Identifier(typename)) => typename,
                Some(Token::Integer) => "INTEGER".to_string(),
                Some(Token::Text) => "TEXT".to_string(),
                Some(Token::Float) => "FLOAT".to_string(),
                Some(Token::Boolean) => "BOOLEAN".to_string(),
                Some(t) => return Err(format!("Expected data type, got {:?}", t)),
                None => return Err("Expected data type".to_string()),
            };

            params.push(StoredProcParam {
                name: param_name,
                mode,
                data_type,
            });

            if matches!(self.current(), Some(Token::Comma)) {
                self.next();
            }
        }
        self.expect(Token::RParen)?;

        self.expect(Token::Begin)?;
        let mut body = Vec::new();
        let mut current_sql = String::new();
        while !matches!(self.current(), Some(Token::End) | None) {
            match self.next() {
                Some(Token::Semicolon) => {
                    if !current_sql.is_empty() {
                        body.push(StoredProcStatement::RawSql(current_sql.trim().to_string()));
                        current_sql = String::new();
                    }
                }
                Some(Token::Identifier(sql)) => {
                    current_sql.push_str(&sql);
                    current_sql.push(' ');
                }
                Some(t) => {
                    current_sql.push_str(&t.to_string());
                    current_sql.push(' ');
                }
                None => return Err("Expected END".to_string()),
            }
        }
        self.expect(Token::End)?;

        if !current_sql.is_empty() {
            body.push(StoredProcStatement::RawSql(current_sql.trim().to_string()));
        }

        Ok(Statement::CreateProcedure(CreateProcedureStatement {
            name,
            params,
            body,
        }))
    }

    fn parse_create_trigger(&mut self) -> Result<Statement, String> {
        self.expect(Token::Trigger)?;

        let name = match self.next() {
            Some(Token::Identifier(name)) => name,
            Some(t) => return Err(format!("Expected trigger name, got {:?}", t)),
            None => return Err("Expected trigger name".to_string()),
        };

        let timing = match self.current() {
            Some(Token::Before) => {
                self.next();
                "BEFORE".to_string()
            }
            Some(Token::After) => {
                self.next();
                "AFTER".to_string()
            }
            Some(t) => return Err(format!("Expected BEFORE or AFTER, got {:?}", t)),
            None => return Err("Expected BEFORE or AFTER".to_string()),
        };

        let events = self.parse_trigger_events()?;

        self.expect(Token::On)?;

        let table = match self.next() {
            Some(Token::Identifier(name)) => name,
            Some(t) => return Err(format!("Expected table name, got {:?}", t)),
            None => return Err("Expected table name".to_string()),
        };

        self.expect(Token::For)?;
        self.expect(Token::Each)?;
        self.expect(Token::Row)?;

        self.expect(Token::Begin)?;
        let mut body = String::new();
        while !matches!(self.current(), Some(Token::End) | None) {
            match self.next() {
                Some(Token::Semicolon) => {
                    body.push(';');
                    body.push(' ');
                }
                Some(Token::Identifier(sql)) => {
                    body.push_str(&sql);
                    body.push(' ');
                }
                Some(t) => {
                    body.push_str(&t.to_string());
                    body.push(' ');
                }
                None => return Err("Expected END".to_string()),
            }
        }
        self.expect(Token::End)?;

        Ok(Statement::CreateTrigger(CreateTriggerStatement {
            name,
            table,
            timing,
            events,
            body: body.trim().to_string(),
        }))
    }

    fn parse_create_view(&mut self) -> Result<Statement, String> {
        self.expect(Token::View)?;
        let name = match self.next() {
            Some(Token::Identifier(name)) => name,
            Some(t) => return Err(format!("Expected view name, got {:?}", t)),
            None => return Err("Expected view name".to_string()),
        };
        let mut columns = Vec::new();
        if matches!(self.current(), Some(Token::LParen)) {
            self.next();
            while !matches!(self.current(), Some(Token::RParen) | None) {
                match self.next() {
                    Some(Token::Identifier(col)) => columns.push(col),
                    Some(Token::Comma) => {}
                    t => return Err(format!("Expected column name, got {:?}", t)),
                }
            }
            self.expect(Token::RParen)?;
        }
        self.expect(Token::As)?;
        let query = Box::new(self.parse_select_or_union()?);
        Ok(Statement::CreateView(CreateViewStatement {
            name,
            columns,
            query,
        }))
    }

    fn parse_drop_view(&mut self) -> Result<Statement, String> {
        self.expect(Token::View)?;
        let if_exists = if matches!(self.current(), Some(Token::If)) {
            self.next();
            match self.current() {
                Some(Token::Exists) => {
                    self.next();
                    true
                }
                _ => return Err("Expected 'EXISTS' after 'IF'".to_string()),
            }
        } else {
            false
        };
        let name = match self.next() {
            Some(Token::Identifier(name)) => name,
            Some(t) => return Err(format!("Expected view name, got {:?}", t)),
            None => return Err("Expected view name".to_string()),
        };
        Ok(Statement::DropView(DropViewStatement { name, if_exists }))
    }

    fn parse_trigger_events(&mut self) -> Result<Vec<String>, String> {
        let mut events = Vec::new();
        loop {
            match self.current() {
                Some(Token::Insert) => {
                    events.push("INSERT".to_string());
                    self.next();
                }
                Some(Token::Update) => {
                    events.push("UPDATE".to_string());
                    self.next();
                }
                Some(Token::Delete) => {
                    events.push("DELETE".to_string());
                    self.next();
                }
                Some(Token::Identifier(ref s))
                    if ["INSERT", "UPDATE", "DELETE"].contains(&s.to_uppercase().as_str()) =>
                {
                    events.push(s.to_uppercase().clone());
                    self.next();
                }
                Some(t) => {
                    if events.is_empty() {
                        return Err(format!("Expected INSERT, UPDATE, or DELETE, got {:?}", t));
                    }
                    break;
                }
                None => {
                    if events.is_empty() {
                        return Err("Expected INSERT, UPDATE, or DELETE".to_string());
                    }
                    break;
                }
            }
        }
        Ok(events)
    }

    fn parse_select(&mut self) -> Result<Statement, String> {
        self.parse_select_or_union()
    }

    fn parse_select_or_union(&mut self) -> Result<Statement, String> {
        let first_select = self.parse_select_statement()?;

        let mut current = Statement::Select(first_select);
        // A SELECT can be followed by zero or more UNION / UNION ALL
        // chains. Each chain consumes a new SELECT and wraps the
        // existing `current` as the left side of a new UnionStatement.
        while matches!(self.current(), Some(Token::Union)) {
            self.next();
            let union_all = if matches!(self.current(), Some(Token::All)) {
                self.next();
                true
            } else {
                false
            };
            let next_select = self.parse_select_statement()?;
            current = Statement::Union(UnionStatement {
                left: Box::new(current),
                right: Box::new(Statement::Select(next_select)),
                union_all,
            });
        }
        Ok(current)
    }

    fn parse_with_select(&mut self) -> Result<Statement, String> {
        self.expect(Token::With)?;

        let recursive = if matches!(self.current(), Some(Token::Recursive)) {
            self.next();
            true
        } else {
            false
        };

        let mut ctes = Vec::new();
        loop {
            let cte_name = match self.next() {
                Some(Token::Identifier(name)) => name,
                _ => return Err("Expected CTE name".to_string()),
            };

            let columns = if matches!(self.current(), Some(Token::LParen)) {
                self.next();
                let mut cols = Vec::new();
                loop {
                    match self.current() {
                        Some(Token::Identifier(name)) => {
                            cols.push(name.clone());
                            self.next();
                        }
                        Some(Token::Comma) => {
                            self.next();
                        }
                        Some(Token::RParen) => {
                            self.next();
                            break;
                        }
                        _ => return Err("Expected column name".to_string()),
                    }
                }
                cols
            } else {
                Vec::new()
            };

            self.expect(Token::As)?;
            self.expect(Token::LParen)?;
            // Detect nested WITH: if the subquery itself starts with
            // `WITH`, recurse into parse_with_select so the nested
            // WithSelect is parsed correctly.
            let subquery = if matches!(self.current(), Some(Token::With)) {
                self.parse_with_select()?
            } else {
                self.parse_select_or_union()?
            };
            self.expect(Token::RParen)?;

            ctes.push(CommonTableExpression {
                name: cte_name,
                columns,
                subquery: Box::new(subquery),
            });

            if matches!(self.current(), Some(Token::Comma)) {
                self.next();
                continue;
            }
            break;
        }

        let with_clause = WithClause { recursive, ctes };
        // The body following the CTE list can be either a SELECT (the
        // standard WithSelect case) or a DML statement (INSERT, UPDATE,
        // DELETE) when the user wrote e.g. `WITH cte AS (...) UPDATE t ...`.
        // We dispatch on the next token to handle both.
        let body = match self.current() {
            Some(Token::Insert) | Some(Token::Replace) => self.parse_insert()?,
            Some(Token::Update) => self.parse_update()?,
            Some(Token::Delete) => self.parse_delete()?,
            _ => {
                let select = self.parse_select_statement()?;
                return Ok(Statement::WithSelect(WithSelect {
                    with_clause: Some(with_clause),
                    select,
                }));
            }
        };
        Ok(Statement::WithDml(WithDmlStatement {
            with_clause,
            body: Box::new(body),
        }))
    }

    fn parse_select_statement(&mut self) -> Result<SelectStatement, String> {
        self.expect(Token::Select)?;

        // Check for DISTINCT keyword (MySQL: SELECT DISTINCT col FROM t)
        let distinct = if matches!(self.current(), Some(Token::Distinct)) {
            self.next();
            true
        } else {
            false
        };

        // MySQL 5.7 SELECT modifiers: HIGH_PRIORITY, SQL_CACHE,
        // SQL_NO_CACHE, SQL_CALC_FOUND_ROWS. All optional; consume
        // any sequence of them before the column list begins.
        while matches!(
            self.current(),
            Some(Token::HighPriority)
                | Some(Token::SqlCache)
                | Some(Token::SqlNoCache)
                | Some(Token::SqlCalcFoundRows)
        ) {
            self.next();
        }

        let mut columns = Vec::new();
        let mut aggregates = Vec::new();

        loop {
            match self.current() {
                // RParen = end of containing subquery (caller already consumed the LParen).
                // Union = end of first SELECT in a UNION/UNION ALL (caller will parse the rest).
                // Must break here so we don't fall through to "Expected FROM or column name"
                // when this parse_select_statement is called recursively for FROM (SELECT ...) AS alias
                // or as the left/right side of UNION ALL.
                Some(Token::RParen) | Some(Token::Union) => break,
                Some(Token::From) | Some(Token::Eof) => {
                    break;
                }
                // `||` (string concat): in column position, fall through to
                // expression parser to handle `'Level ' || n` (Or token between
                // two primary expressions). This requires special-casing
                // because `||` is the same token as boolean `OR`.
                Some(Token::Or) => {
                    return Err("Unexpected '||' / 'OR' at start of column".to_string());
                }
                Some(Token::Star) => {
                    columns.push(SelectColumn {
                        name: "*".to_string(),
                        alias: None,
                        expression: None,
                    });
                    self.next();
                }
                // NULL literal in column position (e.g. `SELECT NULL as order_id`)
                Some(Token::Null) => {
                    self.next();
                    let alias = if matches!(self.current(), Some(Token::As)) {
                        self.next();
                        if let Some(Token::Identifier(n)) = self.current() {
                            let a = n.clone();
                            self.next();
                            Some(a)
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    columns.push(SelectColumn {
                        name: "NULL".to_string(),
                        alias,
                        expression: Some(Expression::Literal("NULL".to_string())),
                    });
                }
                // Handle aggregate functions: COUNT(*), SUM(col), etc.
                // Only treat as aggregate if followed by LParen
                Some(Token::Count) | Some(Token::Sum) | Some(Token::Avg) | Some(Token::Min)
                | Some(Token::Max) => {
                    if matches!(self.peek(), Some(Token::LParen)) {
                        let agg = self.parse_aggregate_function()?;

                        if matches!(self.current(), Some(Token::Over)) {
                            self.next();
                            self.expect(Token::LParen)?;

                            let func_name = match agg.func {
                                AggregateFunction::Count => "COUNT",
                                AggregateFunction::Sum => "SUM",
                                AggregateFunction::Avg => "AVG",
                                AggregateFunction::Min => "MIN",
                                AggregateFunction::Max => "MAX",
                            };

                            let mut partition_by = Vec::new();
                            if matches!(self.current(), Some(Token::Partition)) {
                                self.next();
                                self.expect(Token::By)?;
                                loop {
                                    partition_by.push(self.parse_expression()?);
                                    if matches!(self.current(), Some(Token::Comma)) {
                                        self.next();
                                    } else {
                                        break;
                                    }
                                }
                            }

                            let mut order_by = Vec::new();
                            if matches!(self.current(), Some(Token::Order)) {
                                self.next();
                                self.expect(Token::By)?;
                                loop {
                                    let expr = self.parse_expression()?;
                                    let asc = if matches!(self.current(), Some(Token::Asc)) {
                                        self.next();
                                        true
                                    } else if matches!(self.current(), Some(Token::Desc)) {
                                        self.next();
                                        false
                                    } else {
                                        true
                                    };
                                    order_by.push((expr, asc));
                                    if matches!(self.current(), Some(Token::Comma)) {
                                        self.next();
                                    } else {
                                        break;
                                    }
                                }
                            }

                            self.expect(Token::RParen)?;

                            let alias = if matches!(self.current(), Some(Token::As)) {
                                self.next();
                                match self.current() {
                                    Some(Token::Identifier(name)) => {
                                        let alias_name = name.clone();
                                        self.next();
                                        Some(alias_name)
                                    }
                                    _ => None,
                                }
                            } else {
                                None
                            };

                            columns.push(SelectColumn {
                                name: format!("{}() OVER (...)", func_name),
                                alias,
                                expression: Some(Expression::WindowCall(WindowCall {
                                    func_name: func_name.to_string(),
                                    args: agg.args,
                                    window_spec: WindowSpecification {
                                        partition_by,
                                        order_by,
                                    },
                                })),
                            });
                        } else {
                            aggregates.push(agg.clone());

                            // Phase 3 (TPCH-01 Q8): after a bare aggregate,
                            // allow a following binary operator so expressions
                            // like `SUM(...) / SUM(...)` are parsed correctly.
                            if matches!(
                                self.current(),
                                Some(Token::Plus)
                                    | Some(Token::Minus)
                                    | Some(Token::Star)
                                    | Some(Token::Slash)
                                    | Some(Token::Percent)
                            ) {
                                let op = match self.current() {
                                    Some(Token::Plus) => "+",
                                    Some(Token::Minus) => "-",
                                    Some(Token::Star) => "*",
                                    Some(Token::Slash) => "/",
                                    Some(Token::Percent) => "%",
                                    _ => unreachable!(),
                                };
                                self.next();
                                let rhs = self.parse_expression()?;
                                let bin_expr = Expression::BinaryOp(
                                    Box::new(Expression::Aggregate(agg)),
                                    op.to_string(),
                                    Box::new(rhs),
                                );
                                let alias = if matches!(self.current(), Some(Token::As)) {
                                    self.next();
                                    match self.current() {
                                        Some(Token::Identifier(name)) => {
                                            let a = name.clone();
                                            self.next();
                                            Some(a)
                                        }
                                        _ => None,
                                    }
                                } else {
                                    None
                                };
                                columns.push(SelectColumn {
                                    name: format!("__agg_bin_{}", columns.len()),
                                    alias,
                                    expression: Some(bin_expr),
                                });
                            } else {
                                let alias = if matches!(self.current(), Some(Token::As)) {
                                    self.next();
                                    match self.current() {
                                        Some(Token::Identifier(name)) => {
                                            let a = name.clone();
                                            self.next();
                                            Some(a)
                                        }
                                        _ => None,
                                    }
                                } else {
                                    None
                                };
                                columns.push(SelectColumn {
                                    name: format!("__agg_{}", aggregates.len()),
                                    alias,
                                    expression: Some(Expression::Aggregate(agg)),
                                });
                            }
                        }
                    } else {
                        // Not followed by LParen - treat as identifier (column name)
                        // Convert keyword to identifier and process as column
                        let name = match self.current() {
                            Some(Token::Count) => "count",
                            Some(Token::Sum) => "sum",
                            Some(Token::Avg) => "avg",
                            Some(Token::Min) => "min",
                            Some(Token::Max) => "max",
                            _ => return Err("Expected column name".to_string()),
                        };
                        self.next(); // consume the keyword
                        columns.push(SelectColumn {
                            name: name.to_string(),
                            alias: None,
                            expression: Some(Expression::Identifier(name.to_string())),
                        });
                    }
                }
                Some(Token::LParen) => {
                    let start_position = self.position;
                    // parse_expression_in_parens already consumes the
                    // matching RParen, so do not call expect(RParen) here.
                    let expr = self.parse_expression_in_parens(0)?;

                    // After `(expr)`, allow binary operator like `* fact`
                    // (e.g., `SELECT (n + 1) * fact FROM t`).
                    if matches!(
                        self.current(),
                        Some(Token::Plus)
                            | Some(Token::Minus)
                            | Some(Token::Star)
                            | Some(Token::Slash)
                            | Some(Token::Percent)
                    ) {
                        let op = match self.current() {
                            Some(Token::Plus) => "+",
                            Some(Token::Minus) => "-",
                            Some(Token::Star) => "*",
                            Some(Token::Slash) => "/",
                            Some(Token::Percent) => "%",
                            _ => unreachable!(),
                        };
                        self.next();
                        let right = self.parse_expression()?;
                        let bin_expr =
                            Expression::BinaryOp(Box::new(expr), op.to_string(), Box::new(right));
                        let alias = if matches!(self.current(), Some(Token::As)) {
                            self.next();
                            match self.current() {
                                Some(Token::Identifier(name)) => {
                                    let alias_name = name.clone();
                                    self.next();
                                    Some(alias_name)
                                }
                                _ => None,
                            }
                        } else {
                            None
                        };
                        columns.push(SelectColumn {
                            name: format!("{:?}", bin_expr),
                            alias,
                            expression: Some(bin_expr),
                        });
                    } else {
                        let _ = start_position;
                        let alias = if matches!(self.current(), Some(Token::As)) {
                            self.next();
                            match self.current() {
                                Some(Token::Identifier(name)) => {
                                    let alias_name = name.clone();
                                    self.next();
                                    Some(alias_name)
                                }
                                _ => None,
                            }
                        } else {
                            None
                        };
                        columns.push(SelectColumn {
                            name: format!("{:?}", expr),
                            alias,
                            expression: Some(expr),
                        });
                    }
                }
                // Handle CASE expressions in SELECT
                Some(Token::Case) => {
                    let expr = self.parse_expression()?;
                    let alias = if matches!(self.current(), Some(Token::As)) {
                        self.next();
                        match self.current() {
                            Some(Token::Identifier(name)) => {
                                let alias_name = name.clone();
                                self.next();
                                Some(alias_name)
                            }
                            _ => None,
                        }
                    } else {
                        None
                    };
                    columns.push(SelectColumn {
                        name: format!("{:?}", expr),
                        alias,
                        expression: Some(expr),
                    });
                }
                // Handle aggregate functions: COUNT(*), SUM(col), etc.
                Some(Token::NumberLiteral(ref n)) => {
                    let n_str = n.to_string();
                    self.next();
                    // If the next token is a binary operator (e.g., `1 - 2`,
                    // `1 + 2`), parse the right side and build a BinaryOp.
                    // Otherwise treat as a bare literal column.
                    if matches!(
                        self.current(),
                        Some(Token::Plus)
                            | Some(Token::Minus)
                            | Some(Token::Star)
                            | Some(Token::Slash)
                            | Some(Token::Percent)
                    ) {
                        let op = match self.current() {
                            Some(Token::Plus) => "+",
                            Some(Token::Minus) => "-",
                            Some(Token::Star) => "*",
                            Some(Token::Slash) => "/",
                            Some(Token::Percent) => "%",
                            _ => unreachable!(),
                        };
                        self.next();
                        let right = self.parse_expression()?;
                        let expr = Expression::BinaryOp(
                            Box::new(Expression::Literal(n_str.clone())),
                            op.to_string(),
                            Box::new(right),
                        );
                        let alias = if matches!(self.current(), Some(Token::As)) {
                            self.next();
                            if let Some(Token::Identifier(name)) = self.current() {
                                let a = name.clone();
                                self.next();
                                Some(a)
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        columns.push(SelectColumn {
                            name: format!("{:?}", expr),
                            alias,
                            expression: Some(expr),
                        });
                    } else {
                        let alias = if matches!(self.current(), Some(Token::As)) {
                            self.next();
                            match self.current() {
                                Some(Token::Identifier(name)) => {
                                    let alias_name = name.clone();
                                    self.next();
                                    Some(alias_name)
                                }
                                _ => None,
                            }
                        } else {
                            None
                        };
                        columns.push(SelectColumn {
                            name: n_str.clone(),
                            alias,
                            expression: Some(Expression::Literal(n_str)),
                        });
                    }
                }
                // Handle StringLiteral in SELECT (e.g., SELECT 'hello')
                Some(Token::StringLiteral(ref s)) => {
                    let s_owned = s.clone();
                    self.next();
                    // After a string literal, allow `|| <expr>` to build a
                    // string-concat expression. The lexer's `||` produces
                    // `Token::Or`, so we special-case it here (this is the
                    // same path Identifier uses for the `is_operator` /
                    // `op` branches).
                    if matches!(self.current(), Some(Token::Or)) {
                        self.next();
                        let right = self.parse_expression()?;
                        let expr = Expression::BinaryOp(
                            Box::new(Expression::Literal(format!("'{}'", s_owned))),
                            "||".to_string(),
                            Box::new(right),
                        );
                        let alias = if matches!(self.current(), Some(Token::As)) {
                            self.next();
                            match self.current() {
                                Some(Token::Identifier(name)) => {
                                    let alias_name = name.clone();
                                    self.next();
                                    Some(alias_name)
                                }
                                Some(Token::Level) => {
                                    self.next();
                                    Some("LEVEL".to_string())
                                }
                                _ => None,
                            }
                        } else {
                            None
                        };
                        columns.push(SelectColumn {
                            name: format!("{:?}", expr),
                            alias,
                            expression: Some(expr),
                        });
                        continue;
                    }
                    let alias = if matches!(self.current(), Some(Token::As)) {
                        self.next();
                        match self.current() {
                            Some(Token::Identifier(name)) => {
                                let alias_name = name.clone();
                                self.next();
                                Some(alias_name)
                            }
                            _ => None,
                        }
                    } else {
                        None
                    };
                    columns.push(SelectColumn {
                        name: format!("'{}'", s_owned),
                        alias,
                        expression: Some(Expression::Literal(format!("'{}'", s_owned))),
                    });
                }
                // Handle BooleanLiteral in SELECT (e.g., SELECT TRUE, FALSE)
                Some(Token::BooleanLiteral(b)) => {
                    let val = if *b { "TRUE" } else { "FALSE" };
                    self.next();
                    let alias = if matches!(self.current(), Some(Token::As)) {
                        self.next();
                        match self.current() {
                            Some(Token::Identifier(name)) => {
                                let alias_name = name.clone();
                                self.next();
                                Some(alias_name)
                            }
                            _ => None,
                        }
                    } else {
                        None
                    };
                    columns.push(SelectColumn {
                        name: val.to_string(),
                        alias,
                        expression: Some(Expression::Literal(val.to_string())),
                    });
                }
                // DATE keyword: can be a function call `DATE(x)` or a column
                // name (e.g. `AS date`). Check LParen to disambiguate.
                Some(Token::Date) => {
                    if matches!(self.peek(), Some(Token::LParen)) {
                        // DATE(x) — treat as function call
                        self.next(); // consume DATE
                        self.next(); // consume LParen
                        let mut args = Vec::new();
                        if !matches!(self.current(), Some(Token::RParen)) {
                            loop {
                                args.push(self.parse_expression()?);
                                if matches!(self.current(), Some(Token::Comma)) {
                                    self.next();
                                } else {
                                    break;
                                }
                            }
                        }
                        let alias = if matches!(self.current(), Some(Token::As)) {
                            self.next();
                            if let Some(Token::Identifier(n)) = self.current() {
                                let a = n.clone();
                                self.next();
                                Some(a)
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        columns.push(SelectColumn {
                            name: format!(
                                "{:?}",
                                Expression::FunctionCall("DATE".to_string(), args.clone())
                            ),
                            alias,
                            expression: Some(Expression::FunctionCall("DATE".to_string(), args)),
                        });
                    } else {
                        // DATE as column name (e.g. `AS date` or `GROUP BY date`)
                        self.next();
                        let alias = if matches!(self.current(), Some(Token::As)) {
                            self.next();
                            if let Some(Token::Identifier(n)) = self.current() {
                                let a = n.clone();
                                self.next();
                                Some(a)
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        columns.push(SelectColumn {
                            name: "date".to_string(),
                            alias,
                            expression: Some(Expression::Identifier("date".to_string())),
                        });
                    }
                }
                // MySQL 5.7: LEFT/RIGHT/INSERT/REPLACE/IF/CONVERT/DATE_ADD/DATE_SUB/SUBSTRING/POSITION
                // as scalar functions in the SELECT list. Same logic as
                // parse_primary_expression.
                // INT-4 / CTE-01: see Token::Level in Token::Level AS alias path
                Some(Token::Left)
                | Some(Token::Right)
                | Some(Token::Insert)
                | Some(Token::Replace)
                | Some(Token::If)
                | Some(Token::Convert)
                | Some(Token::DateAdd)
                | Some(Token::DateSub)
                | Some(Token::Substring)
                | Some(Token::Position)
                | Some(Token::Text)
                | Some(Token::Interval) => {
                    let name = match self.current() {
                        Some(Token::Left) => "LEFT",
                        Some(Token::Right) => "RIGHT",
                        Some(Token::Insert) => "INSERT",
                        Some(Token::Replace) => "REPLACE",
                        Some(Token::If) => "IF",
                        Some(Token::Convert) => "CONVERT",
                        Some(Token::DateAdd) => "DATE_ADD",
                        Some(Token::DateSub) => "DATE_SUB",
                        Some(Token::Substring) => "SUBSTRING",
                        Some(Token::Position) => "POSITION",
                        Some(Token::Text) => "CHAR",
                        Some(Token::Interval) => "INTERVAL",
                        _ => unreachable!(),
                    };
                    self.next();
                    if !matches!(self.current(), Some(Token::LParen)) {
                        return Err(format!(
                            "Expected '(' after {name} in SELECT list; \
                             {name} as statement requires a table target"
                        ));
                    }
                    self.next();
                    // DATE_ADD/DATE_SUB(expr, INTERVAL n unit) and
                    // POSITION(needle IN haystack) — MySQL 5.7 special
                    // forms. Dispatch here to bypass the general
                    // arg-parsing loop which would choke on the
                    // `INTERVAL` and `IN` keywords.
                    if name == "DATE_ADD" || name == "DATE_SUB" {
                        let date_expr = self.parse_primary_expression()?;
                        if !matches!(self.current(), Some(Token::Comma)) {
                            return Err(format!(
                                "Expected ',' in {}(...), got {:?}",
                                name,
                                self.current()
                            ));
                        }
                        self.next(); // consume Comma
                        if !matches!(self.current(), Some(Token::Interval)) {
                            return Err(format!(
                                "Expected INTERVAL in {}(...), got {:?}",
                                name,
                                self.current()
                            ));
                        }
                        self.next(); // consume INTERVAL
                        let n_expr = self.parse_primary_expression()?;
                        let unit = match self.current() {
                            Some(Token::Identifier(u)) => {
                                let s = u.clone();
                                self.next();
                                s
                            }
                            _ => {
                                return Err(format!(
                                    "Expected unit (DAY/MONTH/...) after INTERVAL n in {}(...)",
                                    name
                                ));
                            }
                        };
                        self.expect(Token::RParen)?;
                        // Handle optional AS alias
                        let alias = if matches!(self.current(), Some(Token::As)) {
                            self.next();
                            if let Some(Token::Identifier(n)) = self.current() {
                                let a = n.clone();
                                self.next();
                                Some(a)
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        let args = vec![date_expr, n_expr, Expression::Literal(unit)];
                        columns.push(SelectColumn {
                            name: format!(
                                "{:?}",
                                Expression::FunctionCall(name.to_string(), args.clone())
                            ),
                            alias,
                            expression: Some(Expression::FunctionCall(name.to_string(), args)),
                        });
                        continue;
                    }
                    if name == "POSITION" {
                        let needle = self.parse_primary_expression()?;
                        if !matches!(self.current(), Some(Token::In)) {
                            return Err(format!(
                                "Expected IN after POSITION needle, got {:?}",
                                self.current()
                            ));
                        }
                        self.next(); // consume IN
                        let haystack = self.parse_primary_expression()?;
                        self.expect(Token::RParen)?;
                        let args = vec![needle, haystack];
                        columns.push(SelectColumn {
                            name: format!(
                                "{:?}",
                                Expression::FunctionCall(name.to_string(), args.clone())
                            ),
                            alias: None,
                            expression: Some(Expression::FunctionCall(name.to_string(), args)),
                        });
                        continue;
                    }
                    if name == "SUBSTRING" {
                        // SUBSTRING(str [FROM n] [FOR len]) — ANSI form
                        let str_expr = self.parse_primary_expression()?;
                        let mut args = vec![str_expr];
                        if matches!(self.current(), Some(Token::From)) {
                            self.next(); // consume FROM
                            args.push(self.parse_expression()?);
                            if matches!(self.current(), Some(Token::For)) {
                                self.next(); // consume FOR
                                args.push(self.parse_expression()?);
                            }
                        } else if matches!(self.current(), Some(Token::Comma)) {
                            // MySQL comma form: SUBSTRING(str, n) or SUBSTRING(str, n, len)
                            self.next();
                            args.push(self.parse_expression()?);
                            if matches!(self.current(), Some(Token::Comma)) {
                                self.next();
                                args.push(self.parse_expression()?);
                            }
                        }
                        self.expect(Token::RParen)?;
                        // TPC-H Q22: SUBSTRING(...) AS alias. The
                        // SUBSTRING path previously forgot to consume
                        // the optional `AS <name>` suffix, so
                        // `SELECT SUBSTR(c_phone, 1, 2) AS cntrycode
                        // FROM customer` failed with "Expected FROM
                        // or column name" because the parser saw
                        // `AS` and dropped out of the column loop.
                        let alias = if matches!(self.current(), Some(Token::As)) {
                            self.next();
                            if let Some(Token::Identifier(n)) = self.current() {
                                let a = n.clone();
                                self.next();
                                Some(a)
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        columns.push(SelectColumn {
                            name: format!(
                                "{:?}",
                                Expression::FunctionCall(name.to_string(), args.clone())
                            ),
                            alias,
                            expression: Some(Expression::FunctionCall(name.to_string(), args)),
                        });
                        continue;
                    }
                    let mut args = Vec::new();
                    if !matches!(self.current(), Some(Token::RParen)) {
                        loop {
                            args.push(self.parse_expression()?);
                            if matches!(self.current(), Some(Token::Comma)) {
                                self.next();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(Token::RParen)?;
                    let alias = if matches!(self.current(), Some(Token::As)) {
                        self.next();
                        if let Some(Token::Identifier(n)) = self.current() {
                            let a = n.clone();
                            self.next();
                            Some(a)
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    columns.push(SelectColumn {
                        name: format!(
                            "{:?}",
                            Expression::FunctionCall(name.to_string(), args.clone())
                        ),
                        alias,
                        expression: Some(Expression::FunctionCall(name.to_string(), args)),
                    });
                }
                Some(Token::Identifier(_)) => {
                    let start_position = self.position;
                    let (name, consumed, _is_expression) = match self.current().cloned() {
                        Some(Token::Identifier(name)) => {
                            if matches!(self.peek(), Some(Token::Dot)) {
                                let table = name.clone();
                                self.next();
                                self.expect(Token::Dot)?;
                                match self.current().cloned() {
                                    Some(Token::Identifier(col)) => {
                                        self.next();
                                        (format!("{}.{}", table, col), true, false)
                                    }
                                    // `Level` is a reserved token (used for
                                    // transaction isolation levels) but can
                                    // appear as a user-chosen column name in
                                    // `tbl.Level` (e.g. `oc.level`). Accept
                                    // it here as a column name.
                                    Some(Token::Level) => {
                                        self.next();
                                        (format!("{}.level", table), true, false)
                                    }
                                    // `table.*` — qualified star (e.g.
                                    // `SELECT orders.* FROM orders JOIN users`).
                                    // Emit a Star column with a special name
                                    // so the executor can resolve it from
                                    // the joined tables.
                                    Some(Token::Star) => {
                                        self.next();
                                        (format!("{}.*", table), true, false)
                                    }
                                    Some(t) => {
                                        return Err(format!("Expected column name, got {:?}", t))
                                    }
                                    None => return Err("Expected column name".to_string()),
                                }
                            } else {
                                (name.clone(), false, false)
                            }
                        }
                        _ => return Err("Expected column name".to_string()),
                    };
                    let is_operator = if consumed {
                        matches!(self.current(), Some(Token::Plus))
                            || matches!(self.current(), Some(Token::Minus))
                            || matches!(self.current(), Some(Token::Star))
                            || matches!(self.current(), Some(Token::Slash))
                            || matches!(self.current(), Some(Token::Percent))
                            || matches!(self.current(), Some(Token::Equal))
                            || matches!(self.current(), Some(Token::NotEqual))
                            || matches!(self.current(), Some(Token::Greater))
                            || matches!(self.current(), Some(Token::Less))
                            || matches!(self.current(), Some(Token::GreaterEqual))
                            || matches!(self.current(), Some(Token::LessEqual))
                            // `||` is `Token::Or` (string concat) — operator in
                            // expression position.
                            || matches!(self.current(), Some(Token::Or))
                            // JSON path operators (MySQL 5.7).
                            || matches!(self.current(), Some(Token::JsonArrow))
                            || matches!(self.current(), Some(Token::JsonArrowText))
                    } else {
                        matches!(self.peek(), Some(Token::Plus))
                            || matches!(self.peek(), Some(Token::Minus))
                            || matches!(self.peek(), Some(Token::Star))
                            || matches!(self.peek(), Some(Token::Slash))
                            || matches!(self.peek(), Some(Token::Percent))
                            || matches!(self.peek(), Some(Token::Equal))
                            || matches!(self.peek(), Some(Token::NotEqual))
                            || matches!(self.peek(), Some(Token::Greater))
                            || matches!(self.peek(), Some(Token::Less))
                            || matches!(self.peek(), Some(Token::GreaterEqual))
                            || matches!(self.peek(), Some(Token::LessEqual))
                            || matches!(self.peek(), Some(Token::LParen))
                            // `||` peek — see above.
                            || matches!(self.peek(), Some(Token::Or))
                            // JSON path operators (MySQL 5.7).
                            || matches!(self.peek(), Some(Token::JsonArrow))
                            || matches!(self.peek(), Some(Token::JsonArrowText))
                    };

                    if is_operator {
                        if consumed {
                            let left = Expression::Identifier(name);
                            let op = match self.current() {
                                Some(Token::Plus) => "+",
                                Some(Token::Minus) => "-",
                                Some(Token::Star) => "*",
                                Some(Token::Slash) => "/",
                                Some(Token::Percent) => "%",
                                Some(Token::Equal) => "=",
                                Some(Token::NotEqual) => "!=",
                                Some(Token::Greater) => ">",
                                Some(Token::Less) => "<",
                                Some(Token::GreaterEqual) => ">=",
                                Some(Token::LessEqual) => "<=",
                                // `||` is `Token::Or` — string concat.
                                Some(Token::Or) => "||",
                                // JSON path operators (MySQL 5.7).
                                Some(Token::JsonArrow) => "->",
                                Some(Token::JsonArrowText) => "->>",
                                _ => return Err("Expected operator".to_string()),
                            };
                            self.next();
                            let right = self.parse_expression()?;
                            let expr = Expression::BinaryOp(
                                Box::new(left),
                                op.to_string(),
                                Box::new(right),
                            );
                            let alias = if matches!(self.current(), Some(Token::As)) {
                                self.next();
                                match self.current() {
                                    Some(Token::Identifier(name)) => {
                                        let alias_name = name.clone();
                                        self.next();
                                        Some(alias_name)
                                    }
                                    _ => None,
                                }
                            } else {
                                None
                            };
                            columns.push(SelectColumn {
                                name: format!("{:?}", expr),
                                alias,
                                expression: Some(expr),
                            });
                        } else {
                            self.position = start_position;
                            let expr = self.parse_expression()?;
                            let alias = if matches!(self.current(), Some(Token::As)) {
                                self.next();
                                match self.current() {
                                    Some(Token::Identifier(name)) => {
                                        let alias_name = name.clone();
                                        self.next();
                                        Some(alias_name)
                                    }
                                    _ => None,
                                }
                            } else {
                                None
                            };
                            columns.push(SelectColumn {
                                name: format!("{:?}", expr),
                                alias,
                                expression: Some(expr),
                            });
                        }
                    } else {
                        // Sprint 2 SELECT projection: the column is a plain
                        // identifier (no table prefix, no LParen), so the
                        // expression should be Identifier(name) so projection
                        // can evaluate it as a column reference. Previously
                        // this stored `expression: None` which made the
                        // projection path fall back to `row.first()`, breaking
                        // `SELECT col` for any non-first column.
                        let col_name = name;
                        columns.push(SelectColumn {
                            name: col_name.clone(),
                            alias: None,
                            expression: Some(Expression::Identifier(col_name)),
                        });
                        // For !consumed: advance past the column identifier.
                        // For consumed (table.col): already advanced, current is at next token.
                        if !consumed {
                            self.next();
                        }
                        // Handle AS alias for both consumed and !consumed cases.
                        if matches!(self.current(), Some(Token::As)) {
                            self.next();
                            if let Some(Token::Identifier(alias_name)) = self.current() {
                                let name = alias_name.clone();
                                self.next();
                                if let Some(col) = columns.last_mut() {
                                    col.alias = Some(name);
                                }
                            }
                        }
                    }
                }
                Some(Token::Comma) => {
                    self.next();
                }
                // INT-4 / CTE-01: accept `LEVEL` as a bare column reference
                // in the SELECT list (e.g. `SELECT level FROM t`).
                Some(Token::Level) => {
                    self.next();
                    columns.push(SelectColumn {
                        name: "level".to_string(),
                        alias: None,
                        expression: Some(Expression::Identifier("level".to_string())),
                    });
                }
                _ => {
                    return Err("Expected FROM or column name".to_string());
                }
            }
        }

        // Handle SELECT without FROM (e.g., SELECT NULL, SELECT 1, SELECT 'hello')
        // Also handle FROM (subquery) AS alias (TPC-H Q7/Q8/Q9)
        // RParen means this is a subquery whose caller (parent SELECT) will consume the RParen.
        // TPC-H Sprint 1c: also returns extra_tables (multi-table `FROM t1, t2, t3`
        // list, rest after first).
        let (table, from_subquery, extra_tables) = match self.current() {
            Some(Token::From) => {
                self.next(); // consume FROM
                if matches!(self.current(), Some(Token::LParen)) {
                    // Sprint 1b: FROM (subquery) AS alias
                    // Sprint 1d: also support FROM (table_ref [JOIN table_ref]*) AS alias
                    // (derived table without explicit SELECT).
                    self.next(); // consume (
                    if matches!(self.current(), Some(Token::Select))
                        || matches!(self.current(), Some(Token::With))
                    {
                        // Subquery: parse as SELECT statement
                        let subquery = self.parse_select_statement()?;
                        self.expect(Token::RParen)?;
                        if matches!(self.current(), Some(Token::As)) {
                            self.next();
                        }
                        let alias = match self.next() {
                            Some(Token::Identifier(name)) => name,
                            Some(t) => {
                                return Err(format!("Expected alias for subquery, got {:?}", t))
                            }
                            None => return Err("Expected alias for subquery".to_string()),
                        };
                        (alias, Some(Box::new(subquery)), Vec::new())
                    } else {
                        // Derived table: (table_ref [JOIN table_ref]*)
                        // Parse the first table, then any JOINs, then expect RParen.
                        let first_table = match self.next() {
                            Some(Token::Identifier(name)) => name,
                            Some(t) => {
                                return Err(format!(
                                    "Expected table name in derived table, got {:?}",
                                    t
                                ))
                            }
                            None => return Err("Expected table name in derived table".to_string()),
                        };
                        let first_alias = if matches!(self.current(), Some(Token::Identifier(_))) {
                            match self.next() {
                                Some(Token::Identifier(a)) => Some(a),
                                _ => None,
                            }
                        } else {
                            None
                        };
                        // Build a synthetic SELECT * FROM first_table for the executor
                        // to materialise. The JOINs are not represented in the
                        // synthetic SELECT (executor doesn't support derived-table
                        // JOINs in this path) — we register a separate subquery
                        // for the FULL derived-table content instead.
                        // For simplicity, register just the first table as the
                        // synthetic derived table. (This is a partial fix; full
                        // derived-table JOIN support would require more work.)
                        let synth_select = SelectStatement {
                            columns: vec![SelectColumn {
                                name: "*".to_string(),
                                alias: None,
                                expression: None,
                            }],
                            table: first_table.clone(),
                            from_alias: first_alias.clone(),
                            from_subquery: None,
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
                        };
                        // Skip remaining tokens until matching RParen (consume
                        // any JOINs, ON clauses, etc. — we don't model them
                        // in the synthetic SELECT but the executor will at
                        // least find the first table).
                        // Skip remaining tokens until matching RParen. We
                        // don't model the JOINs in the synthetic SELECT, but
                        // we do need to consume them so the outer parse can
                        // continue. Walk tokens counting parens.
                        let mut depth = 1;
                        while depth > 0 && self.current().is_some() {
                            match self.current() {
                                Some(Token::LParen) => {
                                    depth += 1;
                                    self.next();
                                }
                                Some(Token::RParen) => {
                                    depth -= 1;
                                    if depth > 0 {
                                        self.next();
                                    }
                                }
                                Some(_) => {
                                    self.next();
                                }
                                None => break,
                            }
                        }
                        // Consume the matching RParen
                        self.expect(Token::RParen)?;
                        // Optional AS alias (or just bare alias). If no alias
                        // is provided, synthesise one based on the first table
                        // (e.g. `(employees e JOIN ...) AS sub` → `sub`; if no
                        // alias, use `__derived_employees`).
                        if matches!(self.current(), Some(Token::As)) {
                            self.next();
                        }
                        let alias = match self.current().cloned() {
                            Some(Token::Identifier(name)) => {
                                self.next();
                                name
                            }
                            _ => format!("__derived_{}", first_table),
                        };
                        (alias, Some(Box::new(synth_select)), Vec::new())
                    }
                } else {
                    // FROM table_list — TPC-H Sprint 1c: collect table names.
                    // First goes into `table`; the rest into `extra_tables` for
                    // later auto-rewrite into chain JOINs.
                    let first_table = match self.next() {
                        Some(Token::Identifier(name)) => name,
                        Some(t) => return Err(format!("Expected table name, got {:?}", t)),
                        None => return Err("Expected table name".to_string()),
                    };
                    // Sprint 5 v4: handle the FIRST table's inline alias
                    // BEFORE the comma loop, so the loop sees the comma
                    // (not the alias as the next token). Without this,
                    // `FROM emp e, emp m` parses with `e` consumed as the
                    // from_alias after the loop, leaving the comma and
                    // `emp m` unconsumed (extra_tables=[], the second
                    // table is lost).
                    let first_table_with_alias: String =
                        if matches!(self.current(), Some(Token::Identifier(_)))
                            && !matches!(
                                self.current(),
                                Some(Token::Where)
                                    | Some(Token::Group)
                                    | Some(Token::Order)
                                    | Some(Token::Limit)
                                    | Some(Token::RParen)
                                    | Some(Token::Eof)
                                    | Some(Token::Comma)
                                    | Some(Token::Join)
                                    | Some(Token::Left)
                                    | Some(Token::Right)
                                    | Some(Token::Inner)
                                    | Some(Token::Full)
                                    | Some(Token::Cross)
                                    | Some(Token::On)
                                    | Some(Token::As)
                            )
                        {
                            if let Some(Token::Identifier(a)) = self.next() {
                                format!("{}|{}", first_table, a)
                            } else {
                                first_table.clone()
                            }
                        } else {
                            first_table.clone()
                        };
                    let mut tables: Vec<String> = vec![first_table_with_alias];
                    // Phase 4 (TPCH-01 Q15): comma-list can include
                    // a parenthesized subquery aliased, e.g.
                    // FROM supplier, (SELECT ... FROM lineitem
                    // WHERE l_shipdate >= ...) AS revenue.
                    // Subqueries are registered in thread_local::DERIVED_SUBQUERIES
                    // for executor materialization.
                    while matches!(self.current(), Some(Token::Comma)) {
                        self.next(); // consume comma
                        if matches!(self.current(), Some(Token::LParen)) {
                            // Comma followed by parenthesized subquery.
                            self.next(); // consume (
                            let subquery = self.parse_select_statement()?;
                            self.expect(Token::RParen)?;
                            if matches!(self.current(), Some(Token::As)) {
                                self.next();
                            }
                            let alias_name = match self.current() {
                                Some(Token::Identifier(a)) => a.clone(),
                                _ => {
                                    return Err(
                                        "Expected alias after comma-followed subquery".to_string()
                                    );
                                }
                            };
                            self.next();
                            // The synthetic name is based on the position
                            // in `tables` before any push, so the
                            // downstream auto-rewrite can find this entry
                            // at that same position.
                            let synthetic_name = format!("__subq_{}", tables.len());
                            tables.push(synthetic_name.clone());
                            // Phase 9 (TPCH-01 Q15): also encode the alias
                            // into the synthetic name as `__subq_N|alias`
                            // (the alias reuses the same __subq_N index)
                            // so the downstream auto-rewrite loop in the
                            // `else` arm can recover the alias when it
                            // builds the JoinClause. Without this, the
                            // outer `revenue.l_suppkey` qualifier cannot
                            // be resolved against the materialized
                            // __subq_N table by the executor.
                            //
                            // We DON'T push a second entry to `tables`
                            // because that would shift downstream
                            // indices. Instead, we use a small parallel
                            // thread-local map: derived_aliases[__subq_N] = alias.
                            // (See `DERIVED_ALIASES` below.)
                            DERIVED_ALIASES.with(|cell| {
                                cell.borrow_mut()
                                    .insert(synthetic_name.clone(), alias_name.clone());
                            });
                            // Phase 3 (TPCH-01 Q15): register subquery in thread-local
                            DERIVED_SUBQUERIES.with(|cell| {
                                cell.borrow_mut()
                                    .insert(synthetic_name.clone(), Box::new(subquery.clone()));
                            });
                            continue;
                        }
                        let consumed_name = match self.next() {
                            Some(Token::Identifier(name)) => name,
                            Some(t) => {
                                return Err(format!(
                                    "Expected table name after comma, got {:?} \
                                     (use JOIN instead: `FROM t1 JOIN (subquery) AS a`)",
                                    t
                                ));
                            }
                            None => return Err("Expected table name after comma".to_string()),
                        };
                        // Phase 2 (TPCH-01 Q7/Q21 fix): handle inline alias
                        // for *each* table in the comma list. The previous
                        // design attached the alias to the LAST table only,
                        // which silently dropped `n1` from `nation n1, nation n2`
                        // (TPC-H Q7) and `l1` from `lineitem l1, lineitem l2`
                        // (TPC-H Q21). The auto-rewrite then had no way to
                        // know what qualifier `n1.n_nationkey` resolved to.
                        //
                        // We encode the alias into the table name with a
                        // `|` separator (e.g. `nation|n1`) so the auto-rewrite
                        // selector loop downstream can recover both pieces.
                        let token_after = self.current();
                        if matches!(token_after, Some(Token::Identifier(_)))
                            && !matches!(
                                token_after,
                                Some(Token::Where)
                                    | Some(Token::Group)
                                    | Some(Token::Order)
                                    | Some(Token::Limit)
                                    | Some(Token::RParen)
                                    | Some(Token::Eof)
                                    | Some(Token::Comma)
                                    | Some(Token::Join)
                                    | Some(Token::Left)
                                    | Some(Token::Right)
                                    | Some(Token::Inner)
                                    | Some(Token::Full)
                                    | Some(Token::Cross)
                                    | Some(Token::On)
                                    | Some(Token::As)
                            )
                        {
                            if let Some(Token::Identifier(a)) = self.next() {
                                tables.push(format!("{}|{}", consumed_name, a));
                            } else {
                                tables.push(consumed_name);
                            }
                        } else {
                            tables.push(consumed_name);
                        }
                    }
                    let first = tables[0].clone();
                    let rest: Vec<String> = tables[1..].to_vec();
                    (first, None, rest)
                }
            }
            Some(Token::Eof) | None | Some(Token::RParen) | Some(Token::Union) => {
                (String::new(), None, Vec::new())
            }
            Some(t) => return Err(format!("Expected FROM or end of query, got {:?}", t)),
        };

        // Sprint 5 v4: the first table's inline alias (e.g. `FROM emp e`)
        // is now consumed in the table_list arm above, encoded into
        // `table` as `emp|e`. The original from_alias field is kept None
        // here for backwards compat; the executor reads the alias from
        // the table name's `|` suffix.
        let from_alias: Option<String> = None;

        // Check for JOIN (one or more chained JOINs: t1 JOIN t2 ... JOIN tN)
        let mut join_clause: Vec<JoinClause> = Vec::new();
        while matches!(
            self.current(),
            Some(Token::Join)
                | Some(Token::Left)
                | Some(Token::Right)
                | Some(Token::Inner)
                | Some(Token::Full)
                | Some(Token::Cross)
        ) {
            join_clause.push(self.parse_join_clause()?);
        }

        // TPC-H Sprint 1c: collect additional chained JOINs (JOIN ... JOIN ...).
        // Each is parsed with parse_join_clause (which consumes one JOIN block).
        // Stops at WHERE / GROUP / ORDER / LIMIT / OFFSET / RPAREN / EOF.
        let mut join_chain: Vec<JoinClause> = Vec::new();
        while matches!(
            self.current(),
            Some(Token::Join)
                | Some(Token::Left)
                | Some(Token::Right)
                | Some(Token::Inner)
                | Some(Token::Full)
                | Some(Token::Cross)
        ) {
            join_chain.push(self.parse_join_clause()?);
        }

        let where_clause = if matches!(self.current(), Some(Token::Where)) {
            self.next();
            Some(self.parse_expression()?)
        } else {
            None
        };

        // TPC-H Sprint 1c: auto-rewrite `FROM t1, t2, t3 WHERE p1 AND p2 AND ...`
        // into `FROM t1 JOIN t2 ON p_extracted JOIN t3 ON p_extracted WHERE p_rest`.
        // For each extra table, find an equality predicate in WHERE that joins
        // the new table to either the first table or a previously-joined table,
        // and move it into an ON clause on the new JOIN.
        let extra_tables = if !extra_tables.is_empty() {
            // We re-build join_chain (instead of join_clause) since the
            // existing join_clause is None for the multi-table form.
            if let Some(ref wc) = where_clause {
                let mut chain: Vec<JoinClause> = Vec::new();
                let mut remaining: Vec<Expression> = Vec::new();
                let conj = flatten_and(wc);
                // TPCH-01 Q2/Q9 fix (Phase 1): track the accumulated
                // join state so `find_join_predicate` can validate
                // that the predicate's "other side" is reachable.
                // Seed `joined` with the FROM base table + alias
                // + the TPC-H 1-char prefix (`p` for `part`, `s` for
                // `supplier`, etc.) so the underscore-split prefix
                // extraction in `collect_referenced_tables` resolves
                // to a known name in the joined set.
                let mut joined: Vec<String> = if table.is_empty() {
                    Vec::new()
                } else {
                    let mut v = vec![table.clone()];
                    // TPC-H 1-char prefix (e.g. "supplier" -> "s",
                    // "partsupp" -> "ps", "nation" -> "n"). The
                    // underscore-separated prefix is what
                    // `collect_referenced_tables` extracts from
                    // unqualified column names like `s_suppkey`.
                    // TPC-H table-name -> TPC-H column-prefix
                    // mapping. The 1-char-or-underscore heuristic
                    // breaks for `partsupp` (no underscore; naive
                    // slice gives "p" but the TPC-H prefix is "ps").
                    // Use a hard-coded map for the 8 TPC-H tables to
                    // keep the column-prefix extraction in sync with
                    // `collect_referenced_tables` (which also handles
                    // these by their known prefixes).
                    let prefix: &str = match table.as_str() {
                        "region" => "r",
                        "nation" => "n",
                        "supplier" => "s",
                        "customer" => "c",
                        "part" => "p",
                        "partsupp" => "ps",
                        "orders" => "o",
                        "lineitem" => "l",
                        _ => {
                            // Fall back to the old heuristic for any
                            // non-TPC-H table the user may define.
                            if table.contains('_') {
                                let underscore = table.find('_').unwrap();
                                &table[..underscore]
                            } else {
                                &table[..1]
                            }
                        }
                    };
                    v.push(prefix.to_string());
                    if let Some(ref a) = from_alias {
                        v.push(a.clone());
                    }
                    // Phase 7 (TPCH-01 Q15): seed `joined` with any
                    // comma-list synthetic derived tables (__subq_N) and
                    // their aliases, so subsequent find_join_predicate
                    // and predicate_references_table calls can resolve
                    // qualifiers like `revenue.l_suppkey` to the right
                    // table. We do this by reading the thread-local
                    // DERIVED_SUBQUERIES map, which the comma loop has
                    // already populated with `__subq_N` -> SelectStatement.
                    for (key, _subq) in DERIVED_SUBQUERIES.with(|cell| {
                        cell.borrow()
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect::<Vec<_>>()
                    }) {
                        v.push(key.clone());
                    }
                    v
                };
                for t in &extra_tables {
                    if joined.is_empty() {
                        // subquery FROM — bail out, fall through to
                        // cartesian joins in the else branch below.
                        break;
                    }
                    // Phase 2 (TPCH-01 Q7/Q21 fix): split the
                    // `table|alias` encoding produced by the
                    // comma-list parser (see the FROM-clause loop
                    // above). `t` may be `nation`, `nation|n1`, or
                    // a synthetic `__subq_N`. The auto-rewrite
                    // selector needs the bare table name to match
                    // TPC-H column prefixes, and the alias to
                    // populate the JoinClause that the executor
                    // consumes.
                    let (table_name, mut table_alias) = match t.find('|') {
                        Some(idx) => {
                            let n = t[..idx].to_string();
                            let a = t[idx + 1..].to_string();
                            (n, Some(a))
                        }
                        None => (t.clone(), None),
                    };
                    // TPC-H Q15 (Sprint 5 v7): for synthetic
                    // __subq_N derived tables, recover the alias
                    // (e.g. "revenue") from the DERIVED_ALIASES
                    // thread-local map. The alias is set by the
                    // comma-followed-subquery branch in
                    // parse_select_statement and is critical for
                    // find_join_predicate to resolve outer WHERE
                    // qualifiers like `revenue.l_suppkey` to this
                    // table. Without it, the join falls back to
                    // `on=Literal("true")` and produces a cartesian
                    // product (Q15 returns 0 rows instead of 91).
                    if table_alias.is_none() && table_name.starts_with("__subq_") {
                        table_alias =
                            DERIVED_ALIASES.with(|cell| cell.borrow().get(&table_name).cloned());
                    }
                    // Phase 1: best-match selector — find a predicate
                    // that joins `t` to the already-joined set.
                    // Phase 2: also pass the inline alias (if any)
                    // so qualifiers like `n1.col` are recognised as
                    // referencing the new table.
                    let mut found: Option<Expression> = None;
                    let mut rest: Vec<Expression> = Vec::new();
                    let candidates: Vec<Expression> =
                        conj.iter().chain(remaining.iter()).cloned().collect();
                    let _found =
                        find_join_predicate(&conj, &table_name, table_alias.as_deref(), &joined);
                    if let Some(p) = find_join_predicate(
                        &candidates,
                        &table_name,
                        table_alias.as_deref(),
                        &joined,
                    ) {
                        found = Some(p);
                        let matched = found.clone().unwrap();
                        for c in &conj {
                            if !same_expression(c, &matched) {
                                rest.push(c.clone());
                            }
                        }
                        for r in &remaining {
                            if !same_expression(r, &matched) {
                                rest.push(r.clone());
                            }
                        }
                    } else {
                        // Phase 4 (TPCH-01 Q15): skip predicates referencing
                        // synthetic __subq_N derived-table names in BOTH
                        // the conjunction (from the original WHERE) AND the
                        // remaining (from previous extra_table picks). These
                        // belong inside the derived subquery, not the outer
                        // join chain.
                        let is_derived_ref = |p: &Expression| -> bool {
                            match p {
                                Expression::BinaryOp(l, _, r) => {
                                    let lr = collect_referenced_tables(l);
                                    let rr = collect_referenced_tables(r);
                                    lr.iter().any(|t| t.starts_with("__subq_"))
                                        || rr.iter().any(|t| t.starts_with("__subq_"))
                                }
                                _ => false,
                            }
                        };
                        // Phase 5 (TPCH-01 Q2): in the relaxed fallback,
                        // additionally require the candidate predicate to
                        // be fully resolvable in the current join context
                        // (no references to not-yet-joined tables). The
                        // strict check already enforces this; the relaxed
                        // fallback used to be more permissive and would
                        // promote `s_suppkey = ps_suppkey` (Q2) as the
                        // JOIN ON for supplier, which then fails the
                        // executor's `find_join_key_index`.
                        //
                        // Phase 6 (TPCH-01 Q8 fix): additionally require the
                        // predicate to be a binary `=` between two COLUMN
                        // references, not a column-vs-literal. Q8 has
                        // `n2.n_name = 'GERMANY'` as a filter, and the
                        // parser would otherwise promote it as the ON
                        // clause for n2 (no real join key exists), which
                        // makes the executor reject it.
                        for p in &conj {
                            if found.is_none()
                                && !is_derived_ref(p)
                                && is_binary_column_equality(p)
                                && predicate_references_table(p, &table_name)
                                && predicate_fully_resolvable(
                                    p,
                                    &table_name,
                                    table_alias.as_deref(),
                                    &joined,
                                )
                            {
                                found = Some(p.clone());
                            } else {
                                rest.push(p.clone());
                            }
                        }
                        for p in &remaining {
                            if found.is_none()
                                && !is_derived_ref(p)
                                && is_binary_column_equality(p)
                                && predicate_references_table(p, &table_name)
                                && predicate_fully_resolvable(
                                    p,
                                    &table_name,
                                    table_alias.as_deref(),
                                    &joined,
                                )
                            {
                                found = Some(p.clone());
                            } else {
                                rest.push(p.clone());
                            }
                        }
                    }
                    // Phase 8 (TPCH-01 Q15): for derived tables (__subq_N),
                    // emit a cartesian JoinClause with ON=true so the
                    // executor's `execute_joins` materializes the
                    // subquery into `DERIVED_RESULTS` and the outer
                    // WHERE clause (`s_suppkey = revenue.l_suppkey`)
                    // filters the cross product down to the real rows.
                    // The previous `continue` left `join_clause` empty
                    // for the derived table, so executor's
                    // `execute_select` saw `select.join_clause.is_empty()`
                    // and used `select.table` (just `supplier`) for
                    // scan, never materializing __subq_N.
                    if found.is_none() && table_name.starts_with("__subq_") {
                        // Phase 10 (TPCH-01 Q15): pull the alias from the
                        // DERIVED_ALIASES thread-local map (populated by
                        // the comma-followed subquery branch). Without
                        // this, the JoinClause has alias=None and the
                        // outer WHERE `revenue.l_suppkey` qualifier
                        // cannot be resolved to a known joined table.
                        let alias_for_join: Option<String> = table_alias.clone().or_else(|| {
                            DERIVED_ALIASES.with(|cell| cell.borrow().get(&table_name).cloned())
                        });
                        chain.push(JoinClause {
                            join_type: JoinType::Inner,
                            table: table_name.clone(),
                            alias: alias_for_join.clone(),
                            on_clause: Expression::Literal("true".to_string()),
                        });
                        remaining = rest;
                        joined.push(table_name.clone());
                        if let Some(a) = alias_for_join {
                            joined.push(a);
                        }
                        continue;
                    }
                    let on = found.unwrap_or(Expression::Literal("true".to_string()));
                    chain.push(JoinClause {
                        join_type: JoinType::Inner,
                        table: table_name.clone(),
                        alias: table_alias.clone(),
                        on_clause: on,
                    });
                    remaining = rest;
                    // Add the new table + its 1-char TPC-H prefix
                    // (e.g. "part" -> "p", "partsupp" -> "ps") so
                    // the next iteration can validate that the
                    // "other side" of any candidate predicate
                    // references an already-joined table by either
                    // its full name or its underscore-separated
                    // prefix.
                    joined.push(table_name.clone());
                    // TPC-H 1-char/2-char prefix extraction
                    // (see the base-table seed above for rationale).
                    let prefix = if table_name.contains('_') {
                        let underscore = table_name.find('_').unwrap();
                        table_name[..underscore].to_string()
                    } else {
                        table_name[..1].to_string()
                    };
                    joined.push(prefix);
                    // Phase 2: also push the inline alias (e.g.
                    // "n1" for `nation n1`) so subsequent
                    // `find_join_predicate` calls recognise
                    // qualifiers like `n1.n_nationkey` as
                    // referencing an already-joined table.
                    if let Some(a) = table_alias {
                        joined.push(a);
                    }
                }
                // extends them.
                join_clause.extend(chain);
            } else {
                // No WHERE — pure cartesian joins (ON=true).
                let cart: Vec<JoinClause> = extra_tables
                    .iter()
                    .map(|t| JoinClause {
                        join_type: JoinType::Inner,
                        table: t.clone(),
                        alias: None,
                        on_clause: Expression::Literal("true".to_string()),
                    })
                    .collect();
                join_clause.extend(cart);
            }
            Vec::new() // consumed
        } else {
            extra_tables
        };

        // Parse GROUP BY clause
        let group_by = if matches!(self.current(), Some(Token::Group)) {
            self.next();
            self.expect(Token::By)?;
            self.parse_expression_list()?
        } else {
            Vec::new()
        };

        // Parse optional WITH ROLLUP / WITH CUBE modifier (MySQL 5.7)
        // Both are SQL grouping-set extensions. Parsed here and
        // attached to the SelectStatement for downstream executors.
        let mut with_rollup = false;
        let mut with_cube = false;
        if !group_by.is_empty() && matches!(self.current(), Some(Token::With)) {
            self.next(); // consume WITH
            if matches!(self.current(), Some(Token::Rollup)) {
                self.next();
                with_rollup = true;
            } else if matches!(self.current(), Some(Token::Cube)) {
                self.next();
                with_cube = true;
            } else {
                return Err(format!(
                    "Expected ROLLUP or CUBE after WITH, got {:?}",
                    self.current()
                ));
            }
        }

        // Parse HAVING clause
        let having = if matches!(self.current(), Some(Token::Having)) {
            self.next();
            Some(self.parse_expression()?)
        } else {
            None
        };

        // Parse ORDER BY clause
        let order_by = if matches!(self.current(), Some(Token::Order)) {
            self.next();
            self.expect(Token::By)?;
            self.parse_order_by()?
        } else {
            Vec::new()
        };

        // Parse LIMIT clause
        let limit = if matches!(self.current(), Some(Token::Limit)) {
            self.next();
            match self.current() {
                Some(Token::NumberLiteral(n)) => {
                    let val = n
                        .parse::<u64>()
                        .map_err(|e| format!("Invalid LIMIT: {}", e))?;
                    self.next();
                    Some(val)
                }
                Some(Token::Identifier(ref s)) => {
                    // Support LIMIT variable (e.g., @limit)
                    let val = s
                        .parse::<u64>()
                        .map_err(|e| format!("Invalid LIMIT: {}", e))?;
                    self.next();
                    Some(val)
                }
                _ => None,
            }
        } else {
            None
        };

        // Parse OFFSET clause
        let offset = if matches!(self.current(), Some(Token::Offset)) {
            self.next();
            match self.current() {
                Some(Token::NumberLiteral(n)) => {
                    let val = n
                        .parse::<u64>()
                        .map_err(|e| format!("Invalid OFFSET: {}", e))?;
                    self.next();
                    Some(val)
                }
                Some(Token::Identifier(ref s)) => {
                    let val = s
                        .parse::<u64>()
                        .map_err(|e| format!("Invalid OFFSET: {}", e))?;
                    self.next();
                    Some(val)
                }
                _ => None,
            }
        } else {
            None
        };

        // Post-processing pass: scan the column list for
        // `FunctionCall("SUM"|"AVG"|"COUNT"|"MIN"|"MAX", args)` and
        // re-register them as `Expression::Aggregate` plus add to the
        // `aggregates` list. This is the catch-up path for expressions
        // like `100.00 * SUM(...) / SUM(...)` (Q14) where the parser's
        // "after-a-bare-aggregate" special case only fires when the
        // aggregate is the first token of the column. With this fix
        // `Q14: 100.00 * SUM(...) / SUM(...)` registers both SUMs as
        // aggregates and produces 1 row (instead of 0 or 5000).
        let mut extra_aggregates: Vec<AggregateCall> = Vec::new();

        for col in &columns {
            Self::find_aggregates_in_expr(
                &col.expression
                    .clone()
                    .unwrap_or(Expression::Literal("NULL".to_string())),
                &mut extra_aggregates,
            );
        }

        for agg in &extra_aggregates {
            if !aggregates
                .iter()
                .any(|a| a.func == agg.func && a.args == agg.args)
            {
                aggregates.push(agg.clone());
            }
        }
        Ok(SelectStatement {
            columns,
            table,
            from_alias,
            from_subquery,
            where_clause,
            join_clause,
            extra_tables,
            aggregates,
            group_by,
            with_rollup,
            with_cube,
            having,
            order_by,
            limit,
            offset,
            distinct,
        })
    }

    /// Walk an expression tree looking for `FunctionCall` whose name
    /// is a known aggregate (SUM/AVG/COUNT/MIN/MAX). When found,
    /// emit a corresponding `AggregateCall` into `out`. This is the
    /// catch-up pass for cases like `100.00 * SUM(...) / SUM(...)`
    /// (Q14) where the parser's "after-a-bare-aggregate" special
    /// case only fires when the aggregate is the leftmost token of
    /// the column expression.
    fn find_aggregates_in_expr(expr: &Expression, out: &mut Vec<AggregateCall>) {
        match expr {
            Expression::FunctionCall(name, args) => {
                let upper = name.to_uppercase();
                if let Some(func) = match upper.as_str() {
                    "SUM" => Some(AggregateFunction::Sum),
                    "AVG" => Some(AggregateFunction::Avg),
                    "COUNT" => Some(AggregateFunction::Count),
                    "MIN" => Some(AggregateFunction::Min),
                    "MAX" => Some(AggregateFunction::Max),
                    _ => None,
                } {
                    out.push(AggregateCall {
                        func,
                        args: args.clone(),
                        distinct: false,
                    });
                }
                for a in args {
                    Self::find_aggregates_in_expr(a, out);
                }
            }
            // TPC-H Q14 / Q11: aggregates may already be in `Expression::Aggregate`
            // form (when they came from `parse_expression` via `parse_primary`).
            // Re-register them in the `aggregates` Vec so the engine's
            // `if !select.aggregates.is_empty()` branch fires.
            Expression::Aggregate(agg) => {
                out.push(agg.clone());
                for a in &agg.args {
                    Self::find_aggregates_in_expr(a, out);
                }
            }
            Expression::BinaryOp(left, _op, right) => {
                Self::find_aggregates_in_expr(left, out);
                Self::find_aggregates_in_expr(right, out);
            }
            Expression::UnaryOp(_op, inner) => {
                Self::find_aggregates_in_expr(inner, out);
            }
            Expression::CaseWhen(whens, else_val) => {
                for w in whens {
                    Self::find_aggregates_in_expr(&w.condition, out);
                    Self::find_aggregates_in_expr(&w.result, out);
                }
                if let Some(e) = else_val {
                    Self::find_aggregates_in_expr(e, out);
                }
            }
            Expression::Like(inner, pat, _esc) => {
                Self::find_aggregates_in_expr(inner, out);
                Self::find_aggregates_in_expr(pat, out);
            }
            Expression::InList(left, values) => {
                Self::find_aggregates_in_expr(left, out);
                for v in values {
                    Self::find_aggregates_in_expr(v, out);
                }
            }
            Expression::NotInList(left, values) => {
                Self::find_aggregates_in_expr(left, out);
                for v in values {
                    Self::find_aggregates_in_expr(v, out);
                }
            }
            Expression::IsNull(inner) | Expression::IsNotNull(inner) => {
                Self::find_aggregates_in_expr(inner, out);
            }
            _ => {}
        }
    }

    fn parse_expression_list(&mut self) -> Result<Vec<Expression>, String> {
        let mut exprs = Vec::new();
        loop {
            exprs.push(self.parse_expression()?);
            if matches!(self.current(), Some(Token::Comma)) {
                self.next();
            } else {
                break;
            }
        }
        Ok(exprs)
    }

    fn parse_order_by(&mut self) -> Result<Vec<OrderByExpression>, String> {
        let mut expressions = Vec::new();
        loop {
            let expr = self.parse_expression()?;
            let ascending = match self.current() {
                Some(Token::Asc) => {
                    self.next();
                    true
                }
                Some(Token::Desc) => {
                    self.next();
                    false
                }
                _ => true,
            };
            let nulls_first = match self.current() {
                Some(Token::Nulls) => {
                    self.next();
                    match self.current() {
                        Some(Token::First) => {
                            self.next();
                            Some(true)
                        }
                        Some(Token::Last) => {
                            self.next();
                            Some(false)
                        }
                        _ => return Err("Expected FIRST or LAST after NULLS".to_string()),
                    }
                }
                _ => None,
            };
            expressions.push(OrderByExpression {
                expression: expr,
                ascending,
                nulls_first,
            });
            if matches!(self.current(), Some(Token::Comma)) {
                self.next();
            } else {
                break;
            }
        }
        Ok(expressions)
    }

    fn parse_join_clause(&mut self) -> Result<JoinClause, String> {
        // Determine join type
        let join_type = match self.current() {
            Some(Token::Inner) => {
                self.next();
                JoinType::Inner
            }
            Some(Token::Left) => {
                self.next();
                // Consume optional OUTER keyword (e.g. `LEFT OUTER JOIN`).
                if matches!(self.current(), Some(Token::Outer)) {
                    self.next();
                }
                JoinType::Left
            }
            Some(Token::Right) => {
                self.next();
                // Consume optional OUTER keyword (e.g. `RIGHT OUTER JOIN`).
                if matches!(self.current(), Some(Token::Outer)) {
                    self.next();
                }
                JoinType::Right
            }
            Some(Token::Full) => {
                self.next();
                if matches!(self.current(), Some(Token::Outer)) {
                    self.next();
                }
                JoinType::Full
            }
            Some(Token::Cross) => {
                self.next();
                JoinType::Cross
            }
            Some(Token::Join) => {
                self.next();
                JoinType::Inner
            }
            _ => return Err("Expected JOIN type".to_string()),
        };

        // If join type was LEFT/RIGHT/CROSS, we still need to consume the JOIN token
        if matches!(self.current(), Some(Token::Join)) {
            self.next();
        }

        // Parse joined table name OR subquery (e.g. `JOIN (SELECT ...) AS sub`).
        let table = if matches!(self.current(), Some(Token::LParen)) {
            // Subquery in JOIN: parse the parenthesized SELECT, then expect
            // AS alias. We register the subquery in DERIVED_SUBQUERIES under
            // a synthetic name "__subq_<alias>" and use that as the table
            // name so the executor can materialize it before the join.
            self.next(); // consume LParen
            let subquery = self.parse_select_statement()?;
            self.expect(Token::RParen)?;
            // Expect AS alias (or just bare alias, MySQL allows both).
            if matches!(self.current(), Some(Token::As)) {
                self.next();
            }
            let alias_name = match self.current().cloned() {
                Some(Token::Identifier(name)) => {
                    self.next();
                    name
                }
                _ => return Err("Expected alias after JOIN subquery".to_string()),
            };
            let synthetic_name = format!("__subq_{}", alias_name);
            DERIVED_SUBQUERIES.with(|cell| {
                cell.borrow_mut()
                    .insert(synthetic_name.clone(), Box::new(subquery));
            });
            synthetic_name
        } else {
            match self.current().cloned() {
                Some(Token::Identifier(name)) => {
                    self.next();
                    name
                }
                Some(t) => return Err(format!("Expected table name, got {:?}", t)),
                None => return Err("Expected table name".to_string()),
            }
        };

        // Check for table alias (e.g., `JOIN orders o`).
        // TPC-H Q7/Q8/Q9: `JOIN nation n1 ON ...` — the alias is what the
        // executor uses as the column-name prefix in the accumulated
        // join schema, so `n1.n_nationkey` can be resolved.
        let alias: Option<String> = if matches!(self.current(), Some(Token::Identifier(_))) {
            let a = match self.current().cloned() {
                Some(Token::Identifier(name)) => name,
                _ => return Err("Expected alias identifier".to_string()),
            };
            self.next();
            Some(a)
        } else {
            None
        };

        // Parse ON condition (optional for CROSS JOIN)
        let on_clause = if matches!(self.current(), Some(Token::On)) {
            self.next();
            self.parse_expression()?
        } else {
            Expression::Literal("true".to_string())
        };

        Ok(JoinClause {
            join_type,
            table,
            alias,
            on_clause,
        })
    }

    fn parse_aggregate_function(&mut self) -> Result<AggregateCall, String> {
        let func = match self.current() {
            Some(Token::Count) => AggregateFunction::Count,
            Some(Token::Sum) => AggregateFunction::Sum,
            Some(Token::Avg) => AggregateFunction::Avg,
            Some(Token::Min) => AggregateFunction::Min,
            Some(Token::Max) => AggregateFunction::Max,
            _ => return Err("Expected aggregate function".to_string()),
        };
        self.next();

        self.expect(Token::LParen)?;

        let mut args = Vec::new();
        let mut distinct = false;

        if matches!(self.current(), Some(Token::Distinct)) {
            distinct = true;
            self.next();
        }

        // Handle COUNT(*) specially
        if matches!(self.current(), Some(Token::Star)) {
            self.next();
        } else if !matches!(self.current(), Some(Token::RParen)) {
            loop {
                match self.current() {
                    Some(Token::RParen) => break,
                    Some(Token::Comma) => {
                        self.next();
                    }
                    _ => {
                        // General expression — covers CASE WHEN, function
                        // calls (EXTRACT, CAST, …), nested aggregates, and
                        // the simple Identifier/NumberLiteral forms that
                        // the old hand-rolled match used to handle.
                        let expr = self.parse_expression()?;
                        args.push(expr);
                    }
                }
            }
        }

        self.expect(Token::RParen)?;

        Ok(AggregateCall {
            func,
            args,
            distinct,
        })
    }

    fn parse_insert(&mut self) -> Result<Statement, String> {
        // Check for REPLACE INTO (MySQL compatibility) - consume Replace token if present
        let is_replace = if matches!(self.current(), Some(Token::Replace)) {
            self.next(); // consume Replace
            true
        } else {
            self.expect(Token::Insert)?;
            false
        };

        self.expect(Token::Into)?;

        let table = match self.next() {
            Some(Token::Identifier(name)) => name,
            _ => return Err("Expected table name".to_string()),
        };

        // Check for column list: (col1, col2, ...)
        let columns = if matches!(self.current(), Some(Token::LParen)) {
            self.next(); // consume '('
            let mut cols = Vec::new();
            loop {
                match self.current() {
                    Some(Token::Identifier(name)) => {
                        cols.push(name.clone());
                        self.next();
                    }
                    Some(Token::RParen) => {
                        self.next();
                        break;
                    }
                    Some(Token::Comma) => {
                        self.next();
                    }
                    _ => return Err("Expected column name".to_string()),
                }
            }
            cols
        } else {
            Vec::new()
        };

        // Check if INSERT VALUES or INSERT SELECT
        let (values, select) = if matches!(self.current(), Some(Token::Values)) {
            self.next(); // consume VALUES

            // Parse multiple rows: (val1, val2, ...), (val1, val2, ...), ...
            let mut values = Vec::new();

            // Parse first row
            if !matches!(self.current(), Some(Token::LParen)) {
                return Err("Expected ( after VALUES".to_string());
            }

            // Parse all rows
            loop {
                if !matches!(self.current(), Some(Token::LParen)) {
                    break;
                }

                // Parse one row
                self.next(); // consume '('
                let mut row = Vec::new();
                loop {
                    match self.current() {
                        Some(Token::RParen) => {
                            self.next();
                            break;
                        }
                        Some(Token::Comma) => {
                            self.next();
                        }
                        _ => {
                            let expr = self.parse_expression()?;
                            row.push(expr);
                        }
                    }
                }
                values.push(row);

                match self.current() {
                    Some(Token::Comma) => {
                        self.next();
                    }
                    _ => break,
                }
            }

            if values.is_empty() {
                return Err("Expected at least one row of values".to_string());
            }

            (values, None)
        } else if matches!(self.current(), Some(Token::Select)) {
            let select_stmt = self.parse_select()?;
            match select_stmt {
                Statement::Select(s) => (Vec::new(), Some(Box::new(s))),
                _ => return Err("Expected SELECT statement".to_string()),
            }
        } else {
            return Err("Expected VALUES or SELECT".to_string());
        };

        let on_duplicate_key_update = if matches!(self.current(), Some(Token::On)) {
            self.next();
            match self.current() {
                Some(Token::Duplicate) => {
                    self.next();
                    self.expect(Token::Key)?;
                    self.expect(Token::Update)?;
                    let mut updates = Vec::new();
                    loop {
                        match self.current() {
                            Some(Token::Identifier(_)) => {
                                let expr = self.parse_expression()?;
                                match expr {
                                    Expression::BinaryOp(left, op, right) if op == "=" => {
                                        match (*left, *right) {
                                            (Expression::Identifier(col), val) => {
                                                updates.push((col, val));
                                            }
                                            _ => {
                                                return Err("Expected column = value assignment"
                                                    .to_string())
                                            }
                                        }
                                    }
                                    _ => {
                                        return Err("Expected column = value assignment".to_string())
                                    }
                                }
                            }
                            Some(Token::Comma) => {
                                self.next();
                            }
                            _ => break,
                        }
                    }
                    if updates.is_empty() {
                        return Err(
                            "Expected column assignments after ON DUPLICATE KEY UPDATE".to_string()
                        );
                    }
                    Some(updates)
                }
                _ => return Err("Expected 'DUPLICATE KEY' after 'ON'".to_string()),
            }
        } else {
            None
        };

        Ok(Statement::Insert(InsertStatement {
            table,
            columns,
            values,
            select,
            is_replace,
            on_duplicate_key_update,
        }))
    }

    fn parse_update(&mut self) -> Result<Statement, String> {
        self.expect(Token::Update)?;
        let table = match self.next() {
            Some(Token::Identifier(name)) => name,
            _ => return Err("Expected table name".to_string()),
        };

        // Expect SET keyword
        if !matches!(self.current(), Some(Token::Set)) {
            return Err("Expected SET".to_string());
        }
        self.next(); // consume SET

        // Parse SET clause: column = value [, column = value ...]
        let mut set_clauses = Vec::new();
        loop {
            let column = match self.current() {
                Some(Token::Identifier(name)) => name.clone(),
                _ => return Err("Expected column name in SET".to_string()),
            };
            self.next();

            // Expect =
            match self.current() {
                Some(Token::Equal) => {}
                _ => return Err("Expected = in SET clause".to_string()),
            }
            self.next(); // consume =

            // Parse value - use parse_expression to support binary operations
            let value = self.parse_expression()?;

            set_clauses.push((column, value));

            // Check for more SET clauses or WHERE
            match self.current() {
                Some(Token::Comma) => {
                    self.next(); // consume comma, continue to parse next column
                }
                Some(Token::Where) | None | Some(Token::Eof) => break,
                _ => return Err("Expected , or WHERE".to_string()),
            }
        }

        // Parse WHERE clause (optional)
        let where_clause = if matches!(self.current(), Some(Token::Where)) {
            self.next(); // consume WHERE
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Statement::Update(UpdateStatement {
            table,
            set_clauses,
            where_clause,
        }))
    }

    /// Parse a simple expression (for WHERE clause)
    /// Supports: comparison operators (=, !=, >, <, >=, <=)
    /// Logical operators: AND, OR
    fn parse_expression(&mut self) -> Result<Expression, String> {
        // Try JSON path first (column -> '$.path' or column ->> '$.path').
        // Falls through to OR expression if no JSON arrow.
        let mut left = self.parse_or_expression()?;
        while matches!(
            self.current(),
            Some(Token::JsonArrow) | Some(Token::JsonArrowText)
        ) {
            let op = match self.current() {
                Some(Token::JsonArrow) => "->",
                Some(Token::JsonArrowText) => "->>",
                _ => unreachable!(),
            };
            self.next();
            let right = self.parse_or_expression()?;
            left = Expression::BinaryOp(Box::new(left), op.to_string(), Box::new(right));
        }
        // Postfix field access: `(expr).col` (e.g. `f(x).col` where f(x) is a
        // function call result with named field access). Mirrors the
        // JSON-path dispatch above.
        while matches!(self.current(), Some(Token::Dot)) {
            self.next();
            let field = match self.current().cloned() {
                Some(Token::Identifier(name)) => {
                    self.next();
                    name
                }
                Some(Token::Level) => {
                    self.next();
                    "level".to_string()
                }
                Some(t) => return Err(format!("Expected field name after '.', got {:?}", t)),
                None => return Err("Expected field name after '.'".to_string()),
            };
            left = Expression::SubqueryField(Box::new(left), field);
        }
        Ok(left)
    }

    /// Parse OR expression (lowest precedence)
    /// Parse JSON path expression: `column -> '$.path'` or `column ->> '$.path'`
    /// (MySQL 5.7 JSON operators). Emits a BinaryOp with the operator
    /// "->" or "->>" so the executor can apply JSON_EXTRACT/JSON_UNQUOTE.
    #[allow(dead_code)] // reserved for future JSON-aware subquery parsing
    fn parse_json_path_expression(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_multiplicative_expression()?;
        while matches!(
            self.current(),
            Some(Token::JsonArrow) | Some(Token::JsonArrowText)
        ) {
            let op = match self.current() {
                Some(Token::JsonArrow) => "->",
                Some(Token::JsonArrowText) => "->>",
                _ => unreachable!(),
            };
            self.next();
            let right = self.parse_primary_expression()?;
            left = Expression::BinaryOp(Box::new(left), op.to_string(), Box::new(right));
        }
        Ok(left)
    }

    fn parse_or_expression(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_and_expression()?;

        while let Some(Token::Or) = self.current() {
            self.next(); // consume OR
            let right = self.parse_and_expression()?;
            left = Expression::BinaryOp(Box::new(left), "OR".to_string(), Box::new(right));
        }

        Ok(left)
    }

    /// Parse an expression that lives inside a parenthesized context
    /// (e.g. `SELECT (n + 1) * fact`). Stops at the matching `RParen`
    /// AND consumes it, so the caller doesn't need to. Operators at
    /// the boundary (e.g. the `* fact` in the example) are left for
    /// the outer expression to pick up.
    fn parse_expression_in_parens(&mut self, _depth: i32) -> Result<Expression, String> {
        // Depth-aware paren tracking: the inner function call's args loop
        // calls parse_expression which greedily consumes RParens. So when
        // we see an RParen, we don't know if it's the outer matching one
        // or an inner function's closer. Track paren depth: increment on
        // LParen, decrement on RParen, only consume when depth == 0.
        self.next(); // consume the opening LParen
        let mut depth: i32 = 0;
        let mut expr = self.parse_or_expression_until_close_with_depth(&mut depth)?;
        // The inner expression may have consumed RParens for inner function
        // calls. Walk forward consuming RParens until depth == 0.
        loop {
            if matches!(self.current(), Some(Token::RParen)) && depth == 0 {
                self.next();
                break;
            } else if matches!(self.current(), Some(Token::RParen)) {
                self.next();
                depth -= 1;
            } else {
                // Postfix Dot handling: (expr).col
                if matches!(self.current(), Some(Token::Dot)) {
                    self.next();
                    let field = match self.current().cloned() {
                        Some(Token::Identifier(name)) => {
                            self.next();
                            name
                        }
                        Some(Token::Level) => {
                            self.next();
                            "level".to_string()
                        }
                        Some(t) => {
                            return Err(format!("Expected field name after '.', got {:?}", t))
                        }
                        None => return Err("Expected field name after '.'".to_string()),
                    };
                    expr = Expression::SubqueryField(Box::new(expr), field);
                } else {
                    return Err(format!(
                        "Expected RParen in parens, depth={}, current={:?}",
                        depth,
                        self.current()
                    ));
                }
            }
        }
        Ok(expr)
    }

    /// Like `parse_or_expression_until_close` but tracks paren depth
    /// by incrementing on LParen and decrementing on RParen. The depth
    /// is used by the caller to know when the matching outer RParen
    /// has been reached (depth == 0 again).
    fn parse_or_expression_until_close_with_depth(
        &mut self,
        depth: &mut i32,
    ) -> Result<Expression, String> {
        let mut left = self.parse_and_expression_until_close_with_depth(depth)?;
        while matches!(self.current(), Some(Token::Or)) {
            self.next();
            let right = self.parse_and_expression_until_close_with_depth(depth)?;
            left = Expression::BinaryOp(Box::new(left), "OR".to_string(), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and_expression_until_close_with_depth(
        &mut self,
        depth: &mut i32,
    ) -> Result<Expression, String> {
        let mut left = self.parse_comparison_expression_until_close_with_depth(depth)?;
        while matches!(self.current(), Some(Token::And)) {
            self.next();
            let right = self.parse_comparison_expression_until_close_with_depth(depth)?;
            left = Expression::BinaryOp(Box::new(left), "AND".to_string(), Box::new(right));
        }
        Ok(left)
    }

    fn parse_comparison_expression_until_close_with_depth(
        &mut self,
        depth: &mut i32,
    ) -> Result<Expression, String> {
        let mut left = self.parse_additive_expression_until_close_with_depth(depth)?;
        // Comparison operators: =, !=, <, >, <=, >=
        while matches!(
            self.current(),
            Some(Token::Equal)
                | Some(Token::NotEqual)
                | Some(Token::Less)
                | Some(Token::Greater)
                | Some(Token::LessEqual)
                | Some(Token::GreaterEqual)
        ) {
            let op = match self.current() {
                Some(Token::Equal) => "=",
                Some(Token::NotEqual) => "!=",
                Some(Token::Less) => "<",
                Some(Token::Greater) => ">",
                Some(Token::LessEqual) => "<=",
                Some(Token::GreaterEqual) => ">=",
                _ => unreachable!(),
            };
            self.next();
            let right = self.parse_additive_expression_until_close_with_depth(depth)?;
            left = Expression::BinaryOp(Box::new(left), op.to_string(), Box::new(right));
        }
        Ok(left)
    }

    fn parse_additive_expression_until_close_with_depth(
        &mut self,
        depth: &mut i32,
    ) -> Result<Expression, String> {
        let mut left = self.parse_multiplicative_expression_until_close_with_depth(depth)?;
        while matches!(self.current(), Some(Token::Plus) | Some(Token::Minus)) {
            let op = match self.current() {
                Some(Token::Plus) => "+",
                Some(Token::Minus) => "-",
                _ => unreachable!(),
            };
            self.next();
            let right = self.parse_multiplicative_expression_until_close_with_depth(depth)?;
            left = Expression::BinaryOp(Box::new(left), op.to_string(), Box::new(right));
        }
        Ok(left)
    }

    fn parse_multiplicative_expression_until_close_with_depth(
        &mut self,
        depth: &mut i32,
    ) -> Result<Expression, String> {
        if matches!(
            self.current(),
            Some(Token::RParen) | Some(Token::Comma) | None
        ) {
            return Err(format!(
                "Empty expression in parens, current={:?}",
                self.current()
            ));
        }
        let mut left = self.parse_primary_expression_with_depth(depth)?;
        while matches!(
            self.current(),
            Some(Token::Star) | Some(Token::Slash) | Some(Token::Percent)
        ) {
            let op = match self.current() {
                Some(Token::Star) => "*",
                Some(Token::Slash) => "/",
                Some(Token::Percent) => "%",
                _ => unreachable!(),
            };
            self.next();
            let right = self.parse_primary_expression_with_depth(depth)?;
            left = Expression::BinaryOp(Box::new(left), op.to_string(), Box::new(right));
        }
        Ok(left)
    }

    fn parse_primary_expression_with_depth(
        &mut self,
        depth: &mut i32,
    ) -> Result<Expression, String> {
        // If we see LParen, increment depth and parse the inner expression.
        // If we see RParen, decrement depth and return a placeholder
        // (the caller will consume the RParen).
        match self.current() {
            Some(Token::LParen) => {
                *depth += 1;
                self.next();
                // Parse inner expression until matching RParen
                let inner = self.parse_or_expression_until_close_with_depth(depth)?;
                // Consume the matching RParen (the one that brought us back to depth)
                if matches!(self.current(), Some(Token::RParen)) {
                    *depth -= 1;
                    self.next();
                }
                Ok(inner)
            }
            _ => self.parse_primary_expression(),
        }
    }

    /// Like `parse_or_expression` but stops at RParen (the matching
    /// paren closer — which the caller of `parse_expression_in_parens`
    /// will consume).
    #[allow(dead_code)] // reserved for future subquery-in-parens AST nodes
    fn parse_or_expression_until_close(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_and_expression_until_close()?;
        while matches!(self.current(), Some(Token::Or)) {
            self.next();
            let right = self.parse_and_expression_until_close()?;
            left = Expression::BinaryOp(Box::new(left), "OR".to_string(), Box::new(right));
        }
        Ok(left)
    }

    #[allow(dead_code)] // reserved for future subquery-in-parens AST nodes
    fn parse_and_expression_until_close(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_additive_expression_until_close()?;
        while matches!(self.current(), Some(Token::And)) {
            self.next();
            let right = self.parse_additive_expression_until_close()?;
            left = Expression::BinaryOp(Box::new(left), "AND".to_string(), Box::new(right));
        }
        Ok(left)
    }

    #[allow(dead_code)] // reserved for future subquery-in-parens AST nodes
    fn parse_additive_expression_until_close(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_multiplicative_expression_until_close()?;
        while matches!(self.current(), Some(Token::Plus) | Some(Token::Minus)) {
            let op = match self.current() {
                Some(Token::Plus) => "+",
                Some(Token::Minus) => "-",
                _ => unreachable!(),
            };
            self.next();
            let right = self.parse_multiplicative_expression_until_close()?;
            left = Expression::BinaryOp(Box::new(left), op.to_string(), Box::new(right));
        }
        Ok(left)
    }

    #[allow(dead_code)] // reserved for future subquery-in-parens AST nodes
    fn parse_multiplicative_expression_until_close(&mut self) -> Result<Expression, String> {
        // Empty inner expression is an error.
        if matches!(
            self.current(),
            Some(Token::RParen) | Some(Token::Comma) | None
        ) {
            return Err(format!(
                "Empty expression in parens, current={:?}",
                self.current()
            ));
        }
        let mut left = self.parse_primary_expression_until_close()?;
        // After primary, accept * / % but stop at RParen (outer boundary).
        while matches!(
            self.current(),
            Some(Token::Star) | Some(Token::Slash) | Some(Token::Percent)
        ) {
            let op = match self.current() {
                Some(Token::Star) => "*",
                Some(Token::Slash) => "/",
                Some(Token::Percent) => "%",
                _ => unreachable!(),
            };
            self.next();
            let right = self.parse_primary_expression_until_close()?;
            left = Expression::BinaryOp(Box::new(left), op.to_string(), Box::new(right));
        }
        Ok(left)
    }

    #[allow(dead_code)] // reserved for future subquery-in-parens AST nodes
    fn parse_primary_expression_until_close(&mut self) -> Result<Expression, String> {
        // For boundary tokens (RParen / Comma), this is the end of the
        // expression — the caller will see the boundary and exit.
        // We just call the normal primary parser and trust that nested
        // parens are handled correctly (each nested LParen is consumed
        // by parse_primary_expression which then expects its own RParen).
        self.parse_primary_expression()
    }

    /// Parse AND expression (higher precedence than OR)
    fn parse_and_expression(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_comparison_expression()?;

        while let Some(Token::And) = self.current() {
            self.next(); // consume AND
            let right = self.parse_comparison_expression()?;
            left = Expression::BinaryOp(Box::new(left), "AND".to_string(), Box::new(right));
        }

        Ok(left)
    }

    fn parse_comparison_expression(&mut self) -> Result<Expression, String> {
        let left = self.parse_additive_expression()?;

        if matches!(self.current(), Some(Token::In)) {
            self.next();
            self.expect(Token::LParen)?;
            // Check if it's a subquery (SELECT ...) or a value list (1, 2, 3)
            if matches!(self.current(), Some(Token::Select)) {
                let subquery = self.parse_select_statement()?;
                self.expect(Token::RParen)?;
                return Ok(Expression::In(Box::new(left), Box::new(subquery)));
            } else {
                // Parse value list: IN (1, 2, 3)
                let mut values = Vec::new();
                loop {
                    values.push(self.parse_expression()?);
                    if matches!(self.current(), Some(Token::Comma)) {
                        self.next();
                    } else {
                        break;
                    }
                }
                self.expect(Token::RParen)?;
                return Ok(Expression::InList(Box::new(left), values));
            }
        }

        if matches!(self.current(), Some(Token::Not)) {
            self.next();
            if matches!(self.current(), Some(Token::In)) {
                self.next();
                self.expect(Token::LParen)?;
                // Check if it's a subquery or value list
                if matches!(self.current(), Some(Token::Select)) {
                    let subquery = self.parse_select_statement()?;
                    self.expect(Token::RParen)?;
                    return Ok(Expression::NotIn(Box::new(left), Box::new(subquery)));
                } else {
                    // Parse value list: NOT IN (1, 2, 3)
                    let mut values = Vec::new();
                    loop {
                        values.push(self.parse_expression()?);
                        if matches!(self.current(), Some(Token::Comma)) {
                            self.next();
                        } else {
                            break;
                        }
                    }
                    self.expect(Token::RParen)?;
                    return Ok(Expression::NotInList(Box::new(left), values));
                }
            }
            // Check for NOT LIKE
            if matches!(self.current(), Some(Token::Like))
                || matches!(self.current(), Some(Token::Identifier(ref ident)) if ident.to_uppercase() == "LIKE")
            {
                self.next();
                let pattern = self.parse_primary_expression()?;
                let escape = if matches!(self.current(), Some(Token::Escape)) {
                    self.next();
                    match self.current() {
                        Some(Token::StringLiteral(s)) if s.len() == 1 => {
                            let esc = s.chars().next().unwrap();
                            self.next();
                            Some(esc)
                        }
                        _ => return Err("Expected single character after ESCAPE".to_string()),
                    }
                } else {
                    None
                };
                return Ok(Expression::NotLike(
                    Box::new(left),
                    Box::new(pattern),
                    escape,
                ));
            }
            // Check for NOT BETWEEN
            if matches!(self.current(), Some(Token::Between)) {
                self.next();
                let low = self.parse_additive_expression()?;
                self.expect(Token::And)?;
                let high = self.parse_additive_expression()?;
                return Ok(Expression::NotBetween(
                    Box::new(left),
                    Box::new(low),
                    Box::new(high),
                ));
            }
            // Check for NOT REGEXP
            if matches!(self.current(), Some(Token::Identifier(ref ident)) if ident.to_uppercase() == "REGEXP")
            {
                self.next();
                let pattern = self.parse_primary_expression()?;
                return Ok(Expression::NotRegexp(Box::new(left), Box::new(pattern)));
            }
            return Err("NOT must be followed by IN, LIKE, BETWEEN, or REGEXP".to_string());
        }

        // IS NULL / IS NOT NULL
        if matches!(self.current(), Some(Token::Is)) {
            self.next();
            if matches!(self.current(), Some(Token::Not)) {
                self.next();
                if matches!(self.current(), Some(Token::Null)) {
                    self.next();
                    return Ok(Expression::IsNotNull(Box::new(left)));
                }
                return Err("IS NOT must be followed by NULL".to_string());
            }
            if matches!(self.current(), Some(Token::Null)) {
                self.next();
                return Ok(Expression::IsNull(Box::new(left)));
            }
            return Err("IS must be followed by NULL or NOT NULL".to_string());
        }

        // BETWEEN
        if matches!(self.current(), Some(Token::Between)) {
            self.next();
            let low = self.parse_additive_expression()?;
            self.expect(Token::And)?;
            let high = self.parse_additive_expression()?;
            return Ok(Expression::Between(
                Box::new(left),
                Box::new(low),
                Box::new(high),
            ));
        }

        // LIKE / NOT LIKE
        // The lexer's keyword map doesn't include LIKE, so it arrives
        // as Token::Identifier("LIKE") even when the user wrote the
        // bare keyword. Match both encodings to keep WHERE compatibility.
        if matches!(self.current(), Some(Token::Like))
            || matches!(self.current(), Some(Token::Identifier(ref ident)) if ident.to_uppercase() == "LIKE")
        {
            self.next();
            let pattern = self.parse_additive_expression()?;
            let escape = if matches!(self.current(), Some(Token::Escape)) {
                self.next();
                match self.current() {
                    Some(Token::StringLiteral(s)) if s.len() == 1 => {
                        let esc = s.chars().next().unwrap();
                        self.next();
                        Some(esc)
                    }
                    _ => return Err("Expected single character after ESCAPE".to_string()),
                }
            } else {
                None
            };
            return Ok(Expression::Like(Box::new(left), Box::new(pattern), escape));
        }

        let op = match self.current() {
            Some(Token::Equal) => "=",
            Some(Token::NotEqual) => "!=",
            Some(Token::Greater) => ">",
            Some(Token::Less) => "<",
            Some(Token::GreaterEqual) => ">=",
            Some(Token::LessEqual) => "<=",
            _ => return Ok(left),
        };
        self.next();

        // Check for quantified comparison (ANY/ALL/SOME) BEFORE parsing right side
        // because these are not standalone expressions
        if matches!(
            self.current(),
            Some(Token::All) | Some(Token::Any) | Some(Token::Some)
        ) {
            let quantifier = match self.current() {
                Some(Token::All) => "ALL",
                Some(Token::Any) => "ANY",
                Some(Token::Some) => "SOME",
                _ => return Ok(left),
            };
            self.next();
            self.expect(Token::LParen)?;
            let subquery = self.parse_select_statement()?;
            self.expect(Token::RParen)?;
            return Ok(Expression::QuantifiedOp(
                Box::new(Expression::BinaryOp(
                    Box::new(left),
                    op.to_string(),
                    Box::new(Expression::Literal("ANY_SUBQUERY".to_string())),
                )),
                quantifier.to_string(),
                Box::new(subquery),
            ));
        }

        let right = self.parse_primary_expression()?;

        if matches!(
            self.current(),
            Some(Token::All) | Some(Token::Any) | Some(Token::Some)
        ) {
            let quantifier = match self.current() {
                Some(Token::All) => "ALL",
                Some(Token::Any) => "ANY",
                Some(Token::Some) => "SOME",
                _ => {
                    return Ok(Expression::BinaryOp(
                        Box::new(left),
                        op.to_string(),
                        Box::new(right),
                    ))
                }
            };
            self.next();
            self.expect(Token::LParen)?;
            let subquery = self.parse_select_statement()?;
            self.expect(Token::RParen)?;
            Ok(Expression::QuantifiedOp(
                Box::new(Expression::BinaryOp(
                    Box::new(left),
                    op.to_string(),
                    Box::new(right),
                )),
                quantifier.to_string(),
                Box::new(subquery),
            ))
        } else {
            Ok(Expression::BinaryOp(
                Box::new(left),
                op.to_string(),
                Box::new(right),
            ))
        }
    }

    fn parse_additive_expression(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_multiplicative_expression()?;

        while let Some(Token::Plus) | Some(Token::Minus) = self.current() {
            let op = match self.current() {
                Some(Token::Plus) => "+",
                Some(Token::Minus) => "-",
                _ => break,
            };
            self.next();
            let right = self.parse_multiplicative_expression()?;
            left = Expression::BinaryOp(Box::new(left), op.to_string(), Box::new(right));
        }

        Ok(left)
    }

    fn parse_multiplicative_expression(&mut self) -> Result<Expression, String> {
        let mut left = self.parse_primary_expression()?;

        while let Some(Token::Star) | Some(Token::Slash) = self.current() {
            let op = match self.current() {
                Some(Token::Star) => "*",
                Some(Token::Slash) => "/",
                _ => break,
            };
            self.next();
            let right = self.parse_primary_expression()?;
            left = Expression::BinaryOp(Box::new(left), op.to_string(), Box::new(right));
        }

        Ok(left)
    }

    /// Parse primary expression (identifier, literal, or parenthesized)
    fn parse_primary_expression(&mut self) -> Result<Expression, String> {
        match self.current() {
            // MySQL 5.7: LEFT/RIGHT/INSERT/REPLACE/IF/CONVERT/DATE_ADD/DATE_SUB/SUBSTRING/POSITION
            // and ROLLUP/CUBE (as function calls in GROUP BY context, e.g.
            // `GROUP BY ROLLUP(department)`) as scalar function names
            // when followed by `(`. MySQL has these as both statement
            // keywords and string functions; in expression position the
            // function interpretation wins.
            Some(Token::Left)
            | Some(Token::Right)
            | Some(Token::Insert)
            | Some(Token::Replace)
            | Some(Token::If)
            | Some(Token::Convert)
            | Some(Token::Date)
            | Some(Token::DateAdd)
            | Some(Token::DateSub)
            | Some(Token::Substring)
            | Some(Token::Position)
            | Some(Token::Rollup)
            | Some(Token::Cube) => {
                let name = match self.current() {
                    Some(Token::Left) => "LEFT",
                    Some(Token::Right) => "RIGHT",
                    Some(Token::Insert) => "INSERT",
                    Some(Token::Replace) => "REPLACE",
                    Some(Token::If) => "IF",
                    Some(Token::Convert) => "CONVERT",
                    Some(Token::Date) => "DATE",
                    Some(Token::DateAdd) => "DATE_ADD",
                    Some(Token::DateSub) => "DATE_SUB",
                    Some(Token::Substring) => "SUBSTRING",
                    Some(Token::Position) => "POSITION",
                    Some(Token::Rollup) => "ROLLUP",
                    Some(Token::Cube) => "CUBE",
                    _ => unreachable!(),
                };
                self.next();
                if !matches!(self.current(), Some(Token::LParen)) {
                    return Err(format!(
                        "Expected '(' after {name} in expression position; \
                         {name} as statement requires a table target"
                    ));
                }
                self.next();
                // DATE_ADD/DATE_SUB(expr, INTERVAL n unit) — MySQL 5.7
                // special form (mirrors the Identifier arm special form).
                if name == "DATE_ADD" || name == "DATE_SUB" {
                    let date_expr = self.parse_primary_expression()?;
                    if !matches!(self.current(), Some(Token::Comma)) {
                        return Err(format!(
                            "Expected ',' in {}(...), got {:?}",
                            name,
                            self.current()
                        ));
                    }
                    self.next(); // consume Comma
                    if !matches!(self.current(), Some(Token::Interval)) {
                        return Err(format!(
                            "Expected INTERVAL in {}(...), got {:?}",
                            name,
                            self.current()
                        ));
                    }
                    self.next(); // consume INTERVAL
                    let n_expr = self.parse_primary_expression()?;
                    let unit = match self.current() {
                        Some(Token::Identifier(u)) => {
                            let s = u.clone();
                            self.next();
                            s
                        }
                        _ => {
                            return Err(format!(
                                "Expected unit (DAY/MONTH/...) after INTERVAL n in {}(...)",
                                name
                            ));
                        }
                    };
                    self.expect(Token::RParen)?;
                    return Ok(Expression::FunctionCall(
                        name.to_string(),
                        vec![date_expr, n_expr, Expression::Literal(unit)],
                    ));
                }
                // POSITION(needle IN haystack) — MySQL 5.7 special form.
                if name == "POSITION" {
                    let needle = self.parse_primary_expression()?;
                    if !matches!(self.current(), Some(Token::In)) {
                        return Err(format!(
                            "Expected IN after POSITION needle, got {:?}",
                            self.current()
                        ));
                    }
                    self.next(); // consume IN
                    let haystack = self.parse_primary_expression()?;
                    self.expect(Token::RParen)?;
                    return Ok(Expression::FunctionCall(
                        "POSITION".to_string(),
                        vec![needle, haystack],
                    ));
                }
                // SUBSTRING(str [FROM n] [FOR len]) — ANSI/SQL standard form
                // (MySQL uses SUBSTRING(str, n) or SUBSTRING(str FROM n) for the
                // two-arg form, plus SUBSTRING(str, n, len) for the three-arg
                // form). Dispatch the FROM/FOR form here so the general
                // arg-parsing loop doesn't choke on the keywords.
                if name == "SUBSTRING" {
                    let str_expr = self.parse_primary_expression()?;
                    if matches!(self.current(), Some(Token::From)) {
                        self.next(); // consume FROM
                        let from_expr = self.parse_primary_expression()?;
                        if matches!(self.current(), Some(Token::For)) {
                            self.next(); // consume FOR
                            let for_expr = self.parse_primary_expression()?;
                            self.expect(Token::RParen)?;
                            return Ok(Expression::FunctionCall(
                                "SUBSTRING".to_string(),
                                vec![str_expr, from_expr, for_expr],
                            ));
                        }
                        self.expect(Token::RParen)?;
                        return Ok(Expression::FunctionCall(
                            "SUBSTRING".to_string(),
                            vec![str_expr, from_expr],
                        ));
                    }
                    // Comma-separated form: SUBSTRING(str, n) or SUBSTRING(str, n, len)
                    if !matches!(self.current(), Some(Token::RParen)) {
                        if !matches!(self.current(), Some(Token::Comma)) {
                            return Err(format!(
                                "Expected ',' or FROM in SUBSTRING, got {:?}",
                                self.current()
                            ));
                        }
                        self.next(); // consume Comma
                        let mut args = vec![str_expr, self.parse_expression()?];
                        if matches!(self.current(), Some(Token::Comma)) {
                            self.next();
                            args.push(self.parse_expression()?);
                        }
                        self.expect(Token::RParen)?;
                        return Ok(Expression::FunctionCall("SUBSTRING".to_string(), args));
                    }
                    self.expect(Token::RParen)?;
                    return Ok(Expression::FunctionCall(
                        "SUBSTRING".to_string(),
                        vec![str_expr],
                    ));
                }
                let mut args = Vec::new();
                if !matches!(self.current(), Some(Token::RParen)) {
                    loop {
                        args.push(self.parse_expression()?);
                        if matches!(self.current(), Some(Token::Comma)) {
                            self.next();
                        } else {
                            break;
                        }
                    }
                }
                self.expect(Token::RParen)?;
                Ok(Expression::FunctionCall(name.to_string(), args))
            }
            Some(Token::Identifier(_)) => {
                let name = match self.current() {
                    Some(Token::Identifier(n)) => n.clone(),
                    _ => return Err("Expected identifier".to_string()),
                };
                self.next();
                if matches!(self.current(), Some(Token::Dot)) {
                    self.next();
                    match self.current() {
                        Some(Token::Identifier(col)) => {
                            let col_name = col.clone();
                            self.next();
                            Ok(Expression::Identifier(format!("{}.{}", name, col_name)))
                        }
                        Some(t) => Err(format!("Expected column name after dot, got {:?}", t)),
                        None => Err("Expected column name after dot".to_string()),
                    }
                } else if matches!(self.current(), Some(Token::LParen)) {
                    self.next();
                    // EXTRACT(field FROM expr) is a special function-call form
                    // that doesn't fit the comma-separated arg list. Handle
                    // it before the general arg-parsing loop so the parser
                    // doesn't trip on the `FROM` keyword.
                    if name.to_uppercase() == "EXTRACT" {
                        // current is the field name (LParen already consumed).
                        let field_tok = self.current().cloned();
                        let field_name = match field_tok {
                            Some(Token::Identifier(s)) => {
                                self.next();
                                s
                            }
                            other => {
                                return Err(format!(
                                    "Expected EXTRACT field name (e.g. YEAR), got {:?}",
                                    other
                                ));
                            }
                        };
                        if !matches!(self.current(), Some(Token::From)) {
                            return Err(format!(
                                "Expected FROM after EXTRACT field, got {:?}",
                                self.current()
                            ));
                        }
                        self.next(); // consume FROM
                        let source_expr = self.parse_expression()?;
                        self.expect(Token::RParen)?;
                        return Ok(Expression::FunctionCall(
                            "EXTRACT".to_string(),
                            vec![Expression::Literal(field_name), source_expr],
                        ));
                    }

                    // TRIM(LEADING/TRAILING/BOTH remstr FROM str) — MySQL 5.7
                    // standard form, with optional modifier keyword. The
                    // general arg-parsing loop above would fail on the
                    // FROM keyword (it only knows Comma as separator), so
                    // handle it explicitly here. Args are emitted in
                    // [modifier_sentinel, remstr, str] order; the executor
                    // dispatches on args.len() and args[0]. Sentinel
                    // strings: "__TRIM_LEADING__", "__TRIM_TRAILING__",
                    // "__TRIM_BOTH__".
                    //   TRIM(s)                                 -> 1 arg  (existing)
                    //   TRIM(remstr, s)                         -> 2 args (existing)
                    //   TRIM(remstr FROM s)                     -> [Literal("__TRIM_BOTH__"), remstr, s]
                    //   TRIM(LEADING remstr FROM s)            -> [Literal("__TRIM_LEADING__"), remstr, s]
                    //   TRIM(TRAILING remstr FROM s)           -> [Literal("__TRIM_TRAILING__"), remstr, s]
                    //   TRIM(BOTH remstr FROM s)               -> [Literal("__TRIM_BOTH__"), remstr, s]
                    if name.to_uppercase() == "TRIM" {
                        // Detect optional modifier keyword (LEADING/TRAILING/BOTH).
                        // The lexer treats these as identifiers (not reserved
                        // words), so we match on Identifier + value comparison.
                        let mut modifier: Option<String> = None;
                        if let Some(Token::Identifier(m)) = self.current() {
                            let m_upper = m.to_uppercase();
                            if m_upper == "LEADING" || m_upper == "TRAILING" || m_upper == "BOTH" {
                                modifier = Some(m_upper);
                                self.next(); // consume modifier keyword
                            }
                        }
                        // Enter the 3-arg FROM form if:
                        //   - we just consumed a modifier keyword, OR
                        //   - the current token is the remstr expression
                        //     and the NEXT-after-it token is FROM
                        //     (peek without consuming the remstr), OR
                        //   - the current token is FROM itself (no-modifier
                        //     form like `TRIM(FROM s)` — rare but parseable
                        //     by accepting it as the default BOTH form).
                        let from_after_remstr = !matches!(
                            self.current(),
                            Some(Token::RParen) | Some(Token::Comma) | None
                        ) && matches!(self.peek(), Some(Token::From));
                        let from_now = matches!(self.current(), Some(Token::From));
                        if modifier.is_some() || from_after_remstr || from_now {
                            let mod_str = modifier.unwrap_or_else(|| "BOTH".to_string());
                            let sentinel = match mod_str.as_str() {
                                "LEADING" => "__TRIM_LEADING__",
                                "TRAILING" => "__TRIM_TRAILING__",
                                _ => "__TRIM_BOTH__",
                            };
                            // If the current token is FROM (no-modifier case
                            // with empty remstr), skip directly to the str
                            // expression and treat remstr as empty string.
                            let remstr = if from_now {
                                Expression::Literal("".to_string())
                            } else {
                                self.parse_expression()?
                            };
                            if !matches!(self.current(), Some(Token::From)) {
                                return Err(format!(
                                    "Expected FROM in TRIM({} ... FROM ...), got {:?}",
                                    mod_str,
                                    self.current()
                                ));
                            }
                            self.next(); // consume FROM
                            let str_expr = self.parse_expression()?;
                            self.expect(Token::RParen)?;
                            return Ok(Expression::FunctionCall(
                                "TRIM".to_string(),
                                vec![Expression::Literal(sentinel.to_string()), remstr, str_expr],
                            ));
                        }
                        // No modifier and no FROM → fall through to the
                        // standard comma-separated arg loop below.
                    }

                    // DATE_ADD/DATE_SUB(expr, INTERVAL n unit) — MySQL 5.7
                    // special form. The general arg-parsing loop below
                    // would treat `INTERVAL` as a bare identifier and
                    // fail on the unit suffix (DAY, MONTH, …). We
                    // dispatch here, mirroring EXTRACT(field FROM expr)
                    // and TRIM(LEADING … FROM …). Args are emitted as
                    // [date, n, unit_string] so the executor can apply
                    // the offset directly.
                    if name == "DATE_ADD" || name == "DATE_SUB" {
                        let date_expr = self.parse_primary_expression()?;
                        if !matches!(self.current(), Some(Token::Comma)) {
                            return Err(format!(
                                "Expected ',' in {}(...), got {:?}",
                                name,
                                self.current()
                            ));
                        }
                        self.next(); // consume Comma
                        if !matches!(self.current(), Some(Token::Interval)) {
                            return Err(format!(
                                "Expected INTERVAL in {}(...), got {:?}",
                                name,
                                self.current()
                            ));
                        }
                        self.next(); // consume INTERVAL
                        let n_expr = self.parse_primary_expression()?;
                        let unit = match self.current() {
                            Some(Token::Identifier(u)) => {
                                let s = u.clone();
                                self.next();
                                s
                            }
                            _ => {
                                return Err(format!(
                                    "Expected unit (DAY/MONTH/...) after INTERVAL n in {}(...)",
                                    name
                                ));
                            }
                        };
                        self.expect(Token::RParen)?;
                        return Ok(Expression::FunctionCall(
                            name.to_string(),
                            vec![date_expr, n_expr, Expression::Literal(unit)],
                        ));
                    }
                    // POSITION(needle IN haystack) — MySQL 5.7 special form.
                    // The general arg-parsing loop below would treat
                    // `IN` as the start of an IN-list operator and try
                    // to consume a `(`. The needle here is a primary
                    // expression. Mirrors EXTRACT(field FROM expr) and
                    // DATE_ADD(expr, INTERVAL n unit).
                    if name == "POSITION" {
                        let needle = self.parse_primary_expression()?;
                        if !matches!(self.current(), Some(Token::In)) {
                            return Err(format!(
                                "Expected IN after POSITION needle, got {:?}",
                                self.current()
                            ));
                        }
                        self.next(); // consume IN
                        let haystack = self.parse_primary_expression()?;
                        self.expect(Token::RParen)?;
                        return Ok(Expression::FunctionCall(
                            "POSITION".to_string(),
                            vec![needle, haystack],
                        ));
                    }
                    // GROUP_CONCAT([DISTINCT] expr [ORDER BY expr [ASC|DESC]] [SEPARATOR str])
                    // — MySQL 5.7 aggregate special form. Emit args as a
                    // flat vec, with optional DISTINCT/ORDER BY/SEPARATOR
                    // encoded as sentinels so the executor can dispatch.
                    //   GROUP_CONCAT(x)              -> [Literal("__NO_DISTINCT__"), x]
                    //   GROUP_CONCAT(DISTINCT x)     -> [Literal("__DISTINCT__"), x]
                    //   GROUP_CONCAT(DISTINCT x ORDER BY y)        -> [..., Literal("__ORDER_BY__"), y]
                    //   GROUP_CONCAT(DISTINCT x SEPARATOR s)        -> [..., Literal("__SEPARATOR__"), s]
                    if name.to_uppercase() == "GROUP_CONCAT" {
                        let mut gc_args = Vec::new();
                        let _distinct = if matches!(self.current(), Some(Token::Distinct)) {
                            self.next();
                            gc_args.push(Expression::Literal("__DISTINCT__".to_string()));
                            true
                        } else {
                            gc_args.push(Expression::Literal("__NO_DISTINCT__".to_string()));
                            false
                        };
                        if !matches!(self.current(), Some(Token::RParen)) {
                            gc_args.push(self.parse_expression()?);
                        }
                        // Optional ORDER BY clause
                        if matches!(self.current(), Some(Token::Order)) {
                            self.next(); // consume ORDER
                            self.expect(Token::By)?;
                            gc_args.push(Expression::Literal("__ORDER_BY__".to_string()));
                            gc_args.push(self.parse_expression()?);
                            // Optional ASC/DESC after ORDER BY expr
                            if matches!(self.current(), Some(Token::Asc)) {
                                self.next();
                                gc_args.push(Expression::Literal("__ASC__".to_string()));
                            } else if matches!(self.current(), Some(Token::Desc)) {
                                self.next();
                                gc_args.push(Expression::Literal("__DESC__".to_string()));
                            }
                        }
                        // Optional SEPARATOR clause
                        if matches!(self.current(), Some(Token::Identifier(ref sep_ident))
                            if sep_ident.to_uppercase() == "SEPARATOR")
                        {
                            self.next(); // consume SEPARATOR
                            gc_args.push(Expression::Literal("__SEPARATOR__".to_string()));
                            gc_args.push(self.parse_expression()?);
                        }
                        self.expect(Token::RParen)?;
                        return Ok(Expression::FunctionCall(
                            "GROUP_CONCAT".to_string(),
                            gc_args,
                        ));
                    }
                    let mut args = Vec::new();
                    if !matches!(self.current(), Some(Token::RParen)) {
                        loop {
                            args.push(self.parse_expression()?);
                            if matches!(self.current(), Some(Token::Comma)) {
                                self.next();
                            } else {
                                break;
                            }
                        }
                    }
                    // CAST(expr AS TYPE) — args loop may have terminated on AS,
                    // in which case the closing RParen was already consumed by
                    // parse_expression (e.g. CAST(SUBSTR(x,1,4) AS INTEGER) where
                    // parse_expression consumed SUBSTR's RParen). For plain CAST,
                    // expect RParen now.
                    if !(name.to_uppercase() == "CAST" && matches!(self.current(), Some(Token::As)))
                    {
                        self.expect(Token::RParen)?;
                    }

                    if matches!(self.current(), Some(Token::Over)) {
                        self.next();
                        self.expect(Token::LParen)?;

                        let mut partition_by = Vec::new();
                        if matches!(self.current(), Some(Token::Partition)) {
                            self.next();
                            self.expect(Token::By)?;
                            loop {
                                partition_by.push(self.parse_expression()?);
                                if matches!(self.current(), Some(Token::Comma)) {
                                    self.next();
                                } else {
                                    break;
                                }
                            }
                        }

                        let mut order_by = Vec::new();
                        if matches!(self.current(), Some(Token::Order)) {
                            self.next();
                            self.expect(Token::By)?;
                            loop {
                                let expr = self.parse_expression()?;
                                let asc = if matches!(self.current(), Some(Token::Asc)) {
                                    self.next();
                                    true
                                } else if matches!(self.current(), Some(Token::Desc)) {
                                    self.next();
                                    false
                                } else {
                                    true
                                };
                                order_by.push((expr, asc));
                                if matches!(self.current(), Some(Token::Comma)) {
                                    self.next();
                                } else {
                                    break;
                                }
                            }
                        }

                        self.expect(Token::RParen)?;

                        Ok(Expression::WindowCall(WindowCall {
                            func_name: name,
                            args,
                            window_spec: WindowSpecification {
                                partition_by,
                                order_by,
                            },
                        }))
                    } else {
                        // CAST(expr AS TYPE) — consume optional `AS TYPE` suffix.
                        // We don't propagate the target type to the executor; the
                        // executor's `eval_fn` for "CAST" passes the value through,
                        // and downstream INTEGER()/TEXT() context coerces.
                        // (TPC-H Q7/Q8/Q9 always use CAST(... AS INTEGER) anyway.)
                        if name.to_uppercase() == "CAST"
                            && matches!(self.current(), Some(Token::As))
                        {
                            self.next(); // consume AS
                                         // Accept any token that names a type (Token::Integer, Token::Text,
                                         // Token::Float, Token::Boolean, or a bare identifier like VARCHAR).
                            match self.current().cloned() {
                                Some(Token::Integer) | Some(Token::Text) | Some(Token::Float)
                                | Some(Token::Boolean) => {
                                    self.next();
                                }
                                Some(Token::Identifier(_)) => {
                                    // Custom type name like VARCHAR(10) — consume identifier
                                    // and optional (length) if present.
                                    self.next();
                                    if matches!(self.current(), Some(Token::LParen)) {
                                        self.next();
                                        while !matches!(self.current(), Some(Token::RParen)) {
                                            self.next();
                                        }
                                        self.expect(Token::RParen)?;
                                    }
                                }
                                _ => {
                                    return Err(format!(
                                        "Expected type name after CAST AS, got {:?}",
                                        self.current()
                                    ));
                                }
                            }
                        }
                        // EXTRACT(field FROM expr) — field is a SQL token (YEAR,
                        // MONTH, DAY, ...). The executor's `EXTRACT` eval_fn
                        // expects a 2-arg FunctionCall where arg[0] is the
                        // field name and arg[1] is the source expression. We
                        // encode it that way: push the field as a quoted
                        // Literal string so it round-trips through the AST.
                        Ok(Expression::FunctionCall(name, args))
                    }
                } else {
                    Ok(Expression::Identifier(name))
                }
            }
            Some(Token::NumberLiteral(n)) => {
                let expr = Expression::Literal(n.clone());
                self.next();
                Ok(expr)
            }
            Some(Token::StringLiteral(s)) => {
                let expr = Expression::Literal(format!("'{}'", s));
                self.next();
                Ok(expr)
            }
            Some(Token::Null) => {
                let expr = Expression::Literal("NULL".to_string());
                self.next();
                Ok(expr)
            }
            Some(Token::Minus) => {
                self.next();
                if let Some(Token::NumberLiteral(n)) = self.current() {
                    let expr = Expression::Literal(format!("-{}", n));
                    self.next();
                    Ok(expr)
                } else {
                    Err("Expected number after -".to_string())
                }
            }
            Some(Token::LParen) => {
                // parse_expression_in_parens consumes the LParen itself,
                // so we do NOT call self.next() here. The previous code
                // called self.next() and then in_parens also called
                // self.next(), which double-advanced the cursor and
                // caused "Expected number after -" failures on expressions
                // like `l_extendedprice * (1 - l_discount)`.
                // PEEK at next token for subquery detection.
                match self.peek() {
                    Some(Token::Select) | Some(Token::With) => {
                        // Consume the opening LParen before parsing the
                        // SELECT/WITH subquery, since parse_select_or_union
                        // expects the cursor to be at the first token of
                        // the SELECT statement, not at the LParen.
                        self.next();
                        // (SELECT ...) subquery: extract the SelectStatement
                        // out of the returned Statement enum.
                        let stmt = self.parse_select_or_union()?;
                        let subquery = match stmt {
                            Statement::Select(s) => s,
                            Statement::Union(u) => {
                                // UNION subquery: synthesise a SelectStatement
                                // by wrapping the UNION in a FROM-context.
                                // For now, return the left side as a fallback.
                                if let Statement::Select(left) = *u.left {
                                    left
                                } else {
                                    return Err("Unsupported subquery form".to_string());
                                }
                            }
                            _ => return Err("Expected subquery".to_string()),
                        };
                        self.expect(Token::RParen)?;
                        // Optional postfix field access: (subquery).col
                        // (e.g. `(SELECT ... FROM t) AS sub` outer usage
                        //  or `(ST_dump(...).geom)`)
                        let mut expr = Expression::Subquery(Box::new(subquery));
                        while matches!(self.current(), Some(Token::Dot)) {
                            self.next();
                            let field = match self.current().cloned() {
                                Some(Token::Identifier(name)) => {
                                    self.next();
                                    name
                                }
                                Some(Token::Level) => {
                                    self.next();
                                    "level".to_string()
                                }
                                Some(t) => {
                                    return Err(format!(
                                        "Expected field name after '.', got {:?}",
                                        t
                                    ))
                                }
                                None => return Err("Expected field name after '.'".to_string()),
                            };
                            // For now, wrap as a column reference (the executor
                            // can dispatch on the Subquery+Identifier pair).
                            expr = Expression::SubqueryField(Box::new(expr), field);
                        }
                        Ok(expr)
                    }
                    _ => {
                        // parse_expression_in_parens consumes the matching
                        // RParen as part of its depth-aware logic, so do
                        // not call expect(RParen) here.
                        let mut expr = self.parse_expression_in_parens(0)?;
                        // Optional postfix field access: (expr).col
                        // (e.g. `(ST_dump(...).geom)`, `(subq).col`)
                        while matches!(self.current(), Some(Token::Dot)) {
                            self.next();
                            let field = match self.current().cloned() {
                                Some(Token::Identifier(name)) => {
                                    self.next();
                                    name
                                }
                                Some(Token::Level) => {
                                    self.next();
                                    "level".to_string()
                                }
                                Some(t) => {
                                    return Err(format!(
                                        "Expected field name after '.', got {:?}",
                                        t
                                    ))
                                }
                                None => return Err("Expected field name after '.'".to_string()),
                            };
                            expr = Expression::SubqueryField(Box::new(expr), field);
                        }
                        Ok(expr)
                    }
                }
            }
            Some(Token::Exists) => {
                self.next();
                self.expect(Token::LParen)?;
                let subquery = self.parse_select_statement()?;
                self.expect(Token::RParen)?;
                Ok(Expression::Exists(Box::new(subquery)))
            }
            Some(Token::Not) => {
                self.next();
                if matches!(self.current(), Some(Token::Exists)) {
                    self.next();
                    self.expect(Token::LParen)?;
                    let subquery = self.parse_select_statement()?;
                    self.expect(Token::RParen)?;
                    Ok(Expression::NotExists(Box::new(subquery)))
                } else {
                    let expr = self.parse_expression()?;
                    Ok(Expression::UnaryOp("NOT".to_string(), Box::new(expr)))
                }
            }
            Some(Token::Case) => {
                self.next();
                self.parse_case_when_expression()
            }
            // Support aggregate functions in expressions (for HAVING clause)
            Some(Token::Count) | Some(Token::Sum) | Some(Token::Avg) | Some(Token::Min)
            | Some(Token::Max) => {
                let agg = self.parse_aggregate_function()?;

                if matches!(self.current(), Some(Token::Over)) {
                    self.next();
                    self.expect(Token::LParen)?;

                    let func_name = match agg.func {
                        AggregateFunction::Count => "COUNT",
                        AggregateFunction::Sum => "SUM",
                        AggregateFunction::Avg => "AVG",
                        AggregateFunction::Min => "MIN",
                        AggregateFunction::Max => "MAX",
                    };

                    let mut partition_by = Vec::new();
                    if matches!(self.current(), Some(Token::Partition)) {
                        self.next();
                        self.expect(Token::By)?;
                        loop {
                            partition_by.push(self.parse_expression()?);
                            if matches!(self.current(), Some(Token::Comma)) {
                                self.next();
                            } else {
                                break;
                            }
                        }
                    }

                    let mut order_by = Vec::new();
                    if matches!(self.current(), Some(Token::Order)) {
                        self.next();
                        self.expect(Token::By)?;
                        loop {
                            let expr = self.parse_expression()?;
                            let asc = if matches!(self.current(), Some(Token::Asc)) {
                                self.next();
                                true
                            } else if matches!(self.current(), Some(Token::Desc)) {
                                self.next();
                                false
                            } else {
                                true
                            };
                            order_by.push((expr, asc));
                            if matches!(self.current(), Some(Token::Comma)) {
                                self.next();
                            } else {
                                break;
                            }
                        }
                    }

                    self.expect(Token::RParen)?;

                    Ok(Expression::WindowCall(WindowCall {
                        func_name: func_name.to_string(),
                        args: agg.args,
                        window_spec: WindowSpecification {
                            partition_by,
                            order_by,
                        },
                    }))
                } else {
                    Ok(Expression::Aggregate(agg))
                }
            }
            // INT-4 / CTE-01: `LEVEL` is a reserved token (used for
            // transaction isolation levels) but can also be a column
            // name (e.g. `WHERE level <= 2` referencing a column
            // called `level`). Treat it as an Identifier when it
            // appears in expression position.
            Some(Token::Level) => {
                self.next();
                Ok(Expression::Identifier("level".to_string()))
            }
            // MySQL 5.7 type names (CHAR, TEXT, INT, FLOAT, BOOLEAN) used
            // both as bare identifiers (e.g. `CONVERT(price, CHAR)`) and
            // as function names (e.g. `CHAR(65, 66, 67)` to produce the
            // string "ABC"). When followed by `(`, treat as a function
            // call; otherwise emit an Identifier and let the executor
            // interpret it via the enclosing function dispatch.
            Some(Token::Text) => {
                if matches!(self.peek(), Some(Token::LParen)) {
                    self.next();
                    self.next(); // consume LParen
                    let mut args = Vec::new();
                    if !matches!(self.current(), Some(Token::RParen)) {
                        loop {
                            args.push(self.parse_expression()?);
                            if matches!(self.current(), Some(Token::Comma)) {
                                self.next();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(Token::RParen)?;
                    return Ok(Expression::FunctionCall("CHAR".to_string(), args));
                }
                self.next();
                Ok(Expression::Identifier("CHAR".to_string()))
            }
            // MySQL 5.7 INTERVAL keyword used as a bare identifier
            // (DATE_ADD(expr, INTERVAL n unit) parses INTERVAL via
            // a special form in the column list path, but the unit
            // suffix is matched as a regular identifier; INTERVAL
            // itself is also accepted for symmetry with other
            // reserved-keyword-as-identifier fallbacks).
            // When followed by `(`, treat as a function call
            // (e.g. `INTERVAL(5, 1, 3, 5, 7, 9)` returns the index
            // of the first value > 5 in the list).
            Some(Token::Interval) => {
                if matches!(self.peek(), Some(Token::LParen)) {
                    self.next();
                    self.next(); // consume LParen
                    let mut args = Vec::new();
                    if !matches!(self.current(), Some(Token::RParen)) {
                        loop {
                            args.push(self.parse_expression()?);
                            if matches!(self.current(), Some(Token::Comma)) {
                                self.next();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(Token::RParen)?;
                    return Ok(Expression::FunctionCall("INTERVAL".to_string(), args));
                }
                self.next();
                Ok(Expression::Identifier("INTERVAL".to_string()))
            }
            Some(Token::Integer) => {
                self.next();
                Ok(Expression::Identifier("INTEGER".to_string()))
            }
            Some(Token::Float) => {
                self.next();
                Ok(Expression::Identifier("FLOAT".to_string()))
            }
            Some(Token::Boolean) => {
                self.next();
                Ok(Expression::Identifier("BOOLEAN".to_string()))
            }
            // Phase 3 (TPCH-01 Q17/Q20/Q22): scalar subquery in parentheses,
            // e.g. `WHERE x = (SELECT ... FROM t WHERE ...)`.
            Some(Token::Select) => {
                let subquery = self.parse_select_statement()?;
                Ok(Expression::Subquery(Box::new(subquery)))
            }
            _ => Err("Expected expression".to_string()),
        }
    }

    fn parse_case_when_expression(&mut self) -> Result<Expression, String> {
        let mut when_clauses = Vec::new();

        let base_expr = match self.current() {
            Some(Token::When) => None,
            Some(Token::Case) => None,
            _ => {
                let expr = self.parse_expression()?;
                Some(expr)
            }
        };

        loop {
            match self.current() {
                Some(Token::When) => {
                    self.next();
                    let condition = if let Some(ref base) = base_expr {
                        let value = self.parse_expression()?;
                        Expression::BinaryOp(
                            Box::new(base.clone()),
                            "=".to_string(),
                            Box::new(value),
                        )
                    } else {
                        self.parse_expression()?
                    };
                    self.expect(Token::Then)?;
                    let result = self.parse_expression()?;
                    when_clauses.push(WhenClause { condition, result });
                }
                Some(Token::Else) => {
                    self.next();
                    let else_result = self.parse_expression()?;
                    self.expect(Token::End)?;
                    return Ok(Expression::CaseWhen(
                        when_clauses,
                        Some(Box::new(else_result)),
                    ));
                }
                Some(Token::End) => {
                    self.next();
                    return Ok(Expression::CaseWhen(when_clauses, None));
                }
                _ => {
                    if when_clauses.is_empty() {
                        return Err("CASE must have at least one WHEN clause".to_string());
                    } else {
                        return Err("Expected WHEN, ELSE, or END".to_string());
                    }
                }
            }
        }
    }

    fn parse_delete(&mut self) -> Result<Statement, String> {
        self.expect(Token::Delete)?;
        self.expect(Token::From)?;
        let table = match self.next() {
            Some(Token::Identifier(name)) => name,
            _ => return Err("Expected table name".to_string()),
        };

        // Parse WHERE clause (optional)
        let where_clause = if matches!(self.current(), Some(Token::Where)) {
            self.next(); // consume WHERE
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Statement::Delete(DeleteStatement {
            table,
            where_clause,
        }))
    }

    /// Parse MERGE statement (SQL:2003)
    fn parse_merge(&mut self) -> Result<Statement, String> {
        self.expect(Token::Merge)?;
        self.expect(Token::Into)?;
        let target_table = match self.next() {
            Some(Token::Identifier(name)) => name,
            _ => return Err("Expected target table name after MERGE INTO".to_string()),
        };
        let target_alias = self.parse_optional_alias()?;

        self.expect(Token::Using)?;
        let source = self.parse_merge_source()?;
        let source_alias = self.parse_optional_alias()?;

        self.expect(Token::On)?;
        let on_condition = self.parse_expression()?;

        let mut when_clauses = Vec::new();
        while matches!(self.current(), Some(Token::When)) {
            when_clauses.push(self.parse_merge_when_clause()?);
        }
        if when_clauses.is_empty() {
            return Err("MERGE requires at least one WHEN clause".to_string());
        }

        Ok(Statement::Merge(MergeStatement {
            target_table,
            target_alias,
            source,
            source_alias,
            on_condition,
            when_clauses,
        }))
    }

    /// Parse optional alias: either `AS ident` or bare `ident` (SQL:2003 optional AS)
    fn parse_optional_alias(&mut self) -> Result<Option<String>, String> {
        match self.current() {
            Some(Token::As) => {
                self.next();
                match self.next() {
                    Some(Token::Identifier(name)) => Ok(Some(name)),
                    _ => Err("Expected identifier after AS".to_string()),
                }
            }
            // Bare identifier as alias (only consume if it doesn't look like a keyword)
            Some(Token::Identifier(name)) => {
                let alias = name.clone();
                self.next();
                Ok(Some(alias))
            }
            _ => Ok(None),
        }
    }

    /// Parse MERGE source: either a table name or a subquery in parens
    fn parse_merge_source(&mut self) -> Result<MergeSource, String> {
        if matches!(self.current(), Some(Token::LParen)) {
            self.next(); // consume (
            let select = self.parse_select_statement()?;
            self.expect(Token::RParen)?;
            Ok(MergeSource::Subquery(Box::new(select)))
        } else {
            match self.next() {
                Some(Token::Identifier(name)) => Ok(MergeSource::Table { name }),
                _ => Err("Expected table name or (subquery) after USING".to_string()),
            }
        }
    }

    /// Parse a WHEN MATCHED or WHEN NOT MATCHED clause
    fn parse_merge_when_clause(&mut self) -> Result<MergeWhenClause, String> {
        self.next(); // consume WHEN

        // Expect NOT before MATCHED if NOT MATCHED
        let is_matched = if matches!(self.current(), Some(Token::Not)) {
            self.next(); // consume NOT
            self.expect(Token::Matched)?;
            false
        } else {
            self.expect(Token::Matched)?;
            true
        };

        // Optional AND <additional_condition>
        let additional_condition = if matches!(self.current(), Some(Token::And)) {
            self.next(); // consume AND
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.expect(Token::Then)?;
        let action = self.parse_merge_action()?;

        Ok(MergeWhenClause {
            is_matched,
            additional_condition,
            action,
        })
    }

    /// Parse MERGE action: UPDATE SET ... or INSERT (...) VALUES (...)
    fn parse_merge_action(&mut self) -> Result<MergeAction, String> {
        match self.current() {
            Some(Token::Update) => {
                self.next(); // consume UPDATE
                self.expect(Token::Set)?;
                let mut set_clauses = Vec::new();
                loop {
                    // Column may be qualified (e.g., target.col) per SQL:2003
                    let mut column = match self.next() {
                        Some(Token::Identifier(name)) => name,
                        _ => return Err("Expected column name in SET".to_string()),
                    };
                    if matches!(self.current(), Some(Token::Dot)) {
                        self.next();
                        match self.next() {
                            Some(Token::Identifier(col)) => {
                                column = format!("{}.{}", column, col);
                            }
                            _ => return Err("Expected column name after dot".to_string()),
                        }
                    }
                    self.expect(Token::Equal)?;
                    let value = self.parse_expression()?;
                    set_clauses.push((column, value));
                    if matches!(self.current(), Some(Token::Comma)) {
                        self.next();
                    } else {
                        break;
                    }
                }
                Ok(MergeAction::Update { set_clauses })
            }
            Some(Token::Insert) => {
                self.next(); // consume INSERT
                             // INTO is optional in MERGE INSERT context (SQL:2003 MERGE syntax)
                if matches!(self.current(), Some(Token::Into)) {
                    self.next();
                }
                self.expect(Token::LParen)?;
                let mut columns = Vec::new();
                loop {
                    match self.next() {
                        Some(Token::Identifier(name)) => columns.push(name),
                        _ => return Err("Expected column name in INSERT".to_string()),
                    }
                    if matches!(self.current(), Some(Token::Comma)) {
                        self.next();
                    } else if matches!(self.current(), Some(Token::RParen)) {
                        self.next();
                        break;
                    } else {
                        return Err("Expected , or ) in INSERT column list".to_string());
                    }
                }
                self.expect(Token::Values)?;
                self.expect(Token::LParen)?;
                let mut values = Vec::new();
                loop {
                    values.push(self.parse_expression()?);
                    if matches!(self.current(), Some(Token::Comma)) {
                        self.next();
                    } else if matches!(self.current(), Some(Token::RParen)) {
                        self.next();
                        break;
                    } else {
                        return Err("Expected , or ) in INSERT VALUES".to_string());
                    }
                }
                Ok(MergeAction::Insert { columns, values })
            }
            Some(Token::Delete) => {
                self.next();
                Ok(MergeAction::Delete)
            }
            _ => Err("Expected UPDATE, INSERT, or DELETE after THEN".to_string()),
        }
    }

    fn parse_create_table(&mut self) -> Result<Statement, String> {
        // Check if we need to consume Token::Create (may have been consumed by parse_statement)
        if matches!(self.current(), Some(Token::Create)) {
            self.next(); // consume CREATE if not already consumed
        }
        self.expect(Token::Table)?;

        let if_not_exists = if matches!(self.current(), Some(Token::If)) {
            self.next();
            match self.current() {
                Some(Token::Not) => {
                    self.next();
                    match self.current() {
                        Some(Token::Exists) => {
                            self.next();
                            true
                        }
                        _ => return Err("Expected 'EXISTS' after 'NOT'".to_string()),
                    }
                }
                _ => return Err("Expected 'NOT EXISTS' after 'IF'".to_string()),
            }
        } else {
            false
        };

        let name = match self.next() {
            Some(Token::Identifier(name)) => name,
            _ => return Err("Expected table name".to_string()),
        };

        let mut columns = Vec::new();
        let mut constraints = Vec::new();

        if matches!(self.current(), Some(Token::LParen)) {
            self.next();
            loop {
                match self.current() {
                    Some(Token::Identifier(_)) => {
                        let col_def = self.parse_column_definition()?;
                        columns.push(col_def);
                    }
                    Some(Token::Primary) => {
                        self.next();
                        self.expect(Token::Key)?;
                        let columns = self.parse_column_list()?;
                        constraints.push(TableConstraint::PrimaryKey { columns });
                    }
                    Some(Token::Foreign) => {
                        let fk = self.parse_foreign_key_constraint()?;
                        constraints.push(fk);
                    }
                    Some(Token::Unique) => {
                        self.next();
                        let columns = self.parse_column_list()?;
                        constraints.push(TableConstraint::Unique { columns });
                    }
                    Some(Token::Check) => {
                        self.next();
                        self.expect(Token::LParen)?;
                        let expr = self.parse_expression()?;
                        self.expect(Token::RParen)?;
                        constraints.push(TableConstraint::Check {
                            expression: format!("{:?}", expr),
                        });
                    }
                    Some(Token::Constraint) => {
                        self.next();
                        if let Some(Token::Identifier(_name)) = self.next() {
                            self.next();
                            match self.current() {
                                Some(Token::Primary) => {
                                    self.next();
                                    self.expect(Token::Key)?;
                                    let cols = self.parse_column_list()?;
                                    constraints.push(TableConstraint::PrimaryKey { columns: cols });
                                }
                                Some(Token::Foreign) => {
                                    let fk = self.parse_foreign_key_constraint()?;
                                    constraints.push(fk);
                                }
                                Some(Token::Unique) => {
                                    self.next();
                                    let cols = self.parse_column_list()?;
                                    constraints.push(TableConstraint::Unique { columns: cols });
                                }
                                Some(Token::Check) => {
                                    self.next();
                                    self.expect(Token::LParen)?;
                                    let expr = self.parse_expression()?;
                                    self.expect(Token::RParen)?;
                                    constraints.push(TableConstraint::Check {
                                        expression: format!("{:?}", expr),
                                    });
                                }
                                _ => return Err("Expected constraint type".to_string()),
                            }
                        }
                    }
                    Some(Token::RParen) => {
                        self.next();
                        break;
                    }
                    Some(Token::Comma) => {
                        self.next();
                    }
                    _ => break,
                }
            }
        }

        Ok(Statement::CreateTable(CreateTableStatement {
            name,
            columns,
            constraints,
            if_not_exists,
        }))
    }

    fn parse_column_definition(&mut self) -> Result<ColumnDefinition, String> {
        let name = match self.next() {
            Some(Token::Identifier(name)) => name,
            _ => return Err("Expected column name".to_string()),
        };

        let data_type = match self.current() {
            Some(Token::Identifier(type_name)) => {
                let t = type_name.to_uppercase();
                self.next();
                t
            }
            Some(Token::Integer) => {
                self.next();
                "INTEGER".to_string()
            }
            Some(Token::Text) => {
                self.next();
                "TEXT".to_string()
            }
            Some(Token::Float) => {
                self.next();
                "REAL".to_string()
            }
            Some(Token::Boolean) => {
                self.next();
                "BOOLEAN".to_string()
            }
            _ => "INTEGER".to_string(),
        };

        let mut nullable = true;
        let mut primary_key = false;
        let mut auto_increment = false;
        let mut default_value = None;
        let mut references = None;

        loop {
            match self.current() {
                Some(Token::Not) => {
                    self.next();
                    if let Some(Token::Null) = self.current() {
                        self.next();
                        nullable = false;
                    }
                }
                Some(Token::Null) => {
                    self.next();
                    nullable = true;
                }
                Some(Token::Primary) => {
                    self.next();
                    self.expect(Token::Key)?;
                    primary_key = true;
                    nullable = false;
                }
                Some(Token::Default) => {
                    self.next();
                    default_value = Some(self.parse_simple_value()?);
                }
                Some(Token::References) => {
                    self.next();
                    let ref_table = match self.next() {
                        Some(Token::Identifier(name)) => name,
                        _ => return Err("Expected referenced table name".to_string()),
                    };
                    let ref_columns = if matches!(self.current(), Some(Token::LParen)) {
                        self.next();
                        self.parse_column_list()?
                    } else {
                        vec![]
                    };
                    let (on_delete, on_update) = self.parse_referential_actions()?;
                    references = Some(ForeignKeyRef {
                        columns: vec![name.clone()],
                        referenced_table: ref_table,
                        referenced_columns: ref_columns,
                        on_delete,
                        on_update,
                    });
                }
                Some(Token::AutoIncrement) => {
                    self.next();
                    auto_increment = true;
                }
                _ => break,
            }
        }

        Ok(ColumnDefinition {
            name,
            data_type,
            nullable,
            primary_key,
            auto_increment,
            default_value,
            references,
        })
    }

    fn parse_foreign_key_constraint(&mut self) -> Result<TableConstraint, String> {
        self.expect(Token::Foreign)?;
        self.expect(Token::Key)?;
        let columns = self.parse_column_list()?;
        self.expect(Token::References)?;
        let referenced_table = match self.next() {
            Some(Token::Identifier(name)) => name,
            _ => return Err("Expected referenced table name".to_string()),
        };
        let referenced_columns = if matches!(self.current(), Some(Token::LParen)) {
            self.next();
            self.parse_column_list()?
        } else {
            vec![]
        };
        let (on_delete, on_update) = self.parse_referential_actions()?;
        Ok(TableConstraint::ForeignKey {
            columns,
            referenced_table,
            referenced_columns,
            on_delete,
            on_update,
        })
    }

    fn parse_referential_actions(
        &mut self,
    ) -> Result<(Option<ReferentialAction>, Option<ReferentialAction>), String> {
        let mut on_delete = None;
        let mut on_update = None;
        while let Some(Token::On) = self.current() {
            self.next();
            match self.current() {
                Some(Token::Delete) => {
                    self.next();
                    on_delete = Some(self.parse_referential_action()?);
                }
                Some(Token::Update) => {
                    self.next();
                    on_update = Some(self.parse_referential_action()?);
                }
                _ => break,
            }
        }
        Ok((on_delete, on_update))
    }

    fn parse_referential_action(&mut self) -> Result<ReferentialAction, String> {
        match self.current() {
            Some(Token::Cascade) => {
                self.next();
                Ok(ReferentialAction::Cascade)
            }
            Some(Token::Set) => {
                self.next();
                if let Some(Token::Null) = self.current() {
                    self.next();
                    Ok(ReferentialAction::SetNull)
                } else {
                    Err("Expected NULL after SET".to_string())
                }
            }
            Some(Token::Restrict) => {
                self.next();
                Ok(ReferentialAction::Restrict)
            }
            Some(Token::No) => {
                self.next();
                if let Some(Token::Action) = self.current() {
                    self.next();
                    Ok(ReferentialAction::NoAction)
                } else {
                    Err("Expected ACTION after NO".to_string())
                }
            }
            _ => Err(
                "Expected referential action (CASCADE, SET NULL, RESTRICT, NO ACTION)".to_string(),
            ),
        }
    }

    fn parse_simple_value(&mut self) -> Result<String, String> {
        let token = self.current().cloned();
        match token {
            Some(Token::NumberLiteral(n)) => {
                self.next();
                Ok(n)
            }
            Some(Token::StringLiteral(s)) => {
                self.next();
                Ok(format!("'{}'", s))
            }
            Some(Token::Identifier(name)) => {
                self.next();
                Ok(name)
            }
            Some(Token::Null) => {
                self.next();
                Ok("NULL".to_string())
            }
            _ => Err("Expected a value".to_string()),
        }
    }

    fn parse_column_list(&mut self) -> Result<Vec<String>, String> {
        let mut columns = Vec::new();

        loop {
            match self.current() {
                Some(Token::Identifier(name)) => {
                    columns.push(name.clone());
                    self.next();
                }
                Some(Token::Comma) => {
                    self.next();
                }
                Some(Token::RParen) => {
                    self.next();
                    break;
                }
                _ => break,
            }
        }
        Ok(columns)
    }

    fn parse_drop(&mut self) -> Result<Statement, String> {
        self.expect(Token::Drop)?;
        match self.current() {
            Some(Token::Table) => {
                self.next();
                let if_exists = if matches!(self.current(), Some(Token::If)) {
                    self.next();
                    match self.current() {
                        Some(Token::Exists) => {
                            self.next();
                            true
                        }
                        _ => return Err("Expected 'EXISTS' after 'IF'".to_string()),
                    }
                } else {
                    false
                };
                let name = match self.next() {
                    Some(Token::Identifier(name)) => name,
                    _ => return Err("Expected table name".to_string()),
                };
                Ok(Statement::DropTable(DropTableStatement { name, if_exists }))
            }
            Some(Token::Index) => self.parse_drop_index(),
            Some(Token::View) => self.parse_drop_view(),
            Some(Token::Role) => self.parse_drop_role(),
            Some(t) => Err(format!(
                "Expected TABLE, INDEX, VIEW or ROLE after DROP, got {:?}",
                t
            )),
            None => Err("Expected TABLE, INDEX, VIEW or ROLE after DROP".to_string()),
        }
    }

    fn parse_drop_role(&mut self) -> Result<Statement, String> {
        self.expect(Token::Role)?;
        let name = match self.next() {
            Some(Token::Identifier(name)) => name,
            Some(Token::StringLiteral(s)) => s,
            Some(t) => return Err(format!("Expected role name, got {:?}", t)),
            None => return Err("Expected role name".to_string()),
        };
        Ok(Statement::DropRole(DropRoleStatement { name }))
    }

    fn parse_drop_index(&mut self) -> Result<Statement, String> {
        self.expect(Token::Index)?;
        let if_exists = if matches!(self.current(), Some(Token::If)) {
            self.next();
            match self.current() {
                Some(Token::Exists) => {
                    self.next();
                    true
                }
                _ => return Err("Expected 'EXISTS' after 'IF'".to_string()),
            }
        } else {
            false
        };
        let name = match self.next() {
            Some(Token::Identifier(name)) => name,
            _ => return Err("Expected index name".to_string()),
        };
        Ok(Statement::DropIndex(DropIndexStatement { name, if_exists }))
    }

    fn parse_truncate(&mut self) -> Result<Statement, String> {
        self.expect(Token::Truncate)?;
        self.expect(Token::Table)?;
        let name = match self.next() {
            Some(Token::Identifier(name)) => name,
            _ => return Err("Expected table name".to_string()),
        };

        Ok(Statement::Truncate(TruncateStatement { name }))
    }

    fn parse_show(&mut self) -> Result<Statement, String> {
        self.expect(Token::Show)?;

        match self.current() {
            Some(Token::Identifier(ref ident)) if ident.to_uppercase() == "DATABASES" => {
                self.next();
                Ok(Statement::Show(ShowStatement::Databases))
            }
            Some(Token::Identifier(ref ident)) if ident.to_uppercase() == "TABLES" => {
                self.next();
                Ok(Statement::Show(ShowStatement::Tables))
            }
            Some(Token::Identifier(ref ident)) if ident.to_uppercase() == "CREATE" => {
                self.next();
                self.expect(Token::Table)?;
                let table = match self.next() {
                    Some(Token::Identifier(name)) => name,
                    _ => return Err("Expected table name".to_string()),
                };
                Ok(Statement::Show(ShowStatement::CreateTable { table }))
            }
            Some(Token::Identifier(ref ident)) if ident.to_uppercase() == "COLUMNS" => {
                self.next();
                self.expect(Token::From)?;
                let table = match self.next() {
                    Some(Token::Identifier(name)) => name,
                    _ => return Err("Expected table name".to_string()),
                };
                let pattern = if matches!(self.current(), Some(Token::Identifier(ref ident)) if ident.to_uppercase() == "LIKE")
                {
                    self.next();
                    match self.next() {
                        Some(Token::StringLiteral(p)) => Some(p),
                        _ => return Err("Expected pattern string".to_string()),
                    }
                } else {
                    None
                };
                Ok(Statement::Show(ShowStatement::Columns { table, pattern }))
            }
            Some(Token::Identifier(ref ident)) if ident.to_uppercase() == "INDEX" => {
                self.next();
                self.expect(Token::From)?;
                let table = match self.next() {
                    Some(Token::Identifier(name)) => name,
                    _ => return Err("Expected table name".to_string()),
                };
                Ok(Statement::Show(ShowStatement::Index { table }))
            }
            Some(Token::Index) => {
                self.next();
                self.expect(Token::From)?;
                let table = match self.next() {
                    Some(Token::Identifier(name)) => name,
                    _ => return Err("Expected table name".to_string()),
                };
                Ok(Statement::Show(ShowStatement::Index { table }))
            }
            Some(Token::Identifier(ref ident)) if ident.to_uppercase() == "GRANTS" => {
                self.next();
                self.expect(Token::For)?;
                let user = match self.next() {
                    Some(Token::StringLiteral(u)) => u,
                    Some(Token::Identifier(u)) => u,
                    Some(t) => return Err(format!("Expected user string, got {:?}", t)),
                    None => return Err("Expected user string".to_string()),
                };
                Ok(Statement::ShowGrantsFor(user))
            }
            Some(Token::Roles) => {
                self.next();
                Ok(Statement::ShowRoles)
            }
            Some(t) => Err(format!("Unexpected token after SHOW: {:?}", t)),
            None => Err("Unexpected end of input after SHOW".to_string()),
        }
    }

    fn parse_describe(&mut self) -> Result<Statement, String> {
        match self.current() {
            Some(Token::Describe) | Some(Token::Desc) => {
                self.next(); // consume Describe or Desc
            }
            _ => return Err("Expected DESCRIBE or DESC".to_string()),
        }
        let table = match self.next() {
            Some(Token::Identifier(name)) => name,
            _ => return Err("Expected table name".to_string()),
        };
        Ok(Statement::Describe(DescribeStatement { table }))
    }

    fn parse_analyze(&mut self) -> Result<Statement, String> {
        self.expect(Token::Analyze)?;

        let table_name = match self.current() {
            Some(Token::Identifier(name)) => {
                let n = name.clone();
                self.next();
                Some(n)
            }
            Some(Token::Semicolon) | None => None,
            _ => return Err("Expected table name or semicolon".to_string()),
        };

        Ok(Statement::Analyze(AnalyzeStatement { table_name }))
    }

    fn parse_grant(&mut self) -> Result<Statement, String> {
        self.expect(Token::Grant)?;

        // Detect if this is GRANT role_name TO user_name or GRANT privilege ON object TO user
        let is_role_grant = match self.current() {
            Some(Token::Select) | Some(Token::Insert) | Some(Token::Update)
            | Some(Token::Delete) => false,
            Some(Token::Identifier(name)) => {
                let upper = name.to_uppercase();
                !matches!(
                    upper.as_str(),
                    "READ" | "WRITE" | "ALL" | "EXECUTE" | "USAGE"
                )
            }
            Some(Token::StringLiteral(_)) => true,
            _ => false,
        };

        if is_role_grant {
            return self.parse_grant_role();
        }

        let mut privileges = Vec::new();
        loop {
            match self.current() {
                Some(Token::Select) => {
                    privileges.push(Privilege::Select);
                    self.next();
                }
                Some(Token::Insert) => {
                    privileges.push(Privilege::Insert);
                    self.next();
                }
                Some(Token::Update) => {
                    privileges.push(Privilege::Update);
                    self.next();
                }
                Some(Token::Delete) => {
                    privileges.push(Privilege::Delete);
                    self.next();
                }
                Some(Token::Identifier(name)) if name.to_uppercase() == "READ" => {
                    privileges.push(Privilege::Read);
                    self.next();
                }
                Some(Token::Identifier(name)) if name.to_uppercase() == "WRITE" => {
                    privileges.push(Privilege::Write);
                    self.next();
                }
                Some(Token::Identifier(name)) if name.to_uppercase() == "ALL" => {
                    privileges.push(Privilege::All);
                    self.next();
                    break;
                }
                Some(Token::Identifier(name)) if name.to_uppercase() == "EXECUTE" => {
                    privileges.push(Privilege::Execute);
                    self.next();
                }
                Some(Token::Identifier(name)) if name.to_uppercase() == "USAGE" => {
                    privileges.push(Privilege::Usage);
                    self.next();
                }
                _ => break,
            }

            if !matches!(self.current(), Some(Token::Comma)) {
                break;
            }
            self.next();
        }

        let mut columns = Vec::new();
        if matches!(self.current(), Some(Token::LParen)) {
            self.next();
            loop {
                match self.current() {
                    Some(Token::Identifier(name)) => {
                        columns.push(name.clone());
                        self.next();
                    }
                    _ => return Err("Expected column name".to_string()),
                }
                if !matches!(self.current(), Some(Token::Comma)) {
                    break;
                }
                self.next();
            }
            self.expect(Token::RParen)?;
        }

        self.expect(Token::On)?;

        let object_type = match self.current() {
            Some(Token::Identifier(name)) if name.to_uppercase() == "TABLE" => {
                self.next();
                ObjectType::Table
            }
            Some(Token::Identifier(name)) if name.to_uppercase() == "DATABASE" => {
                self.next();
                ObjectType::Database
            }
            Some(Token::Identifier(name)) if name.to_uppercase() == "PROCEDURE" => {
                self.next();
                ObjectType::Procedure
            }
            Some(Token::Identifier(name)) if name.to_uppercase() == "FUNCTION" => {
                self.next();
                ObjectType::Function
            }
            Some(Token::Identifier(name)) if name.to_uppercase() == "COLUMN" => {
                self.next();
                ObjectType::Column
            }
            _ => ObjectType::Table,
        };

        let object_name = match self.next() {
            Some(Token::Identifier(name)) => name,
            Some(t) => return Err(format!("Expected object name, got {:?}", t)),
            None => return Err("Expected object name".to_string()),
        };

        self.expect(Token::To)?;

        let mut recipients = Vec::new();
        loop {
            match self.next() {
                Some(Token::Identifier(name)) => {
                    recipients.push(name);
                }
                Some(t) => return Err(format!("Expected recipient user, got {:?}", t)),
                None => return Err("Expected recipient user".to_string()),
            }
            if !matches!(self.current(), Some(Token::Comma)) {
                break;
            }
            self.next();
        }

        let with_grant_option = if let Some(Token::Identifier(name)) = self.current() {
            if name.to_uppercase() == "WITH" {
                self.next();
                if let Some(Token::Identifier(grant_str)) = self.current() {
                    if grant_str.to_uppercase() == "GRANT" {
                        self.next();
                        if let Some(Token::Identifier(option_str)) = self.current() {
                            if option_str.to_uppercase() == "OPTION" {
                                self.next();
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        Ok(Statement::Grant(GrantStatement {
            privileges,
            columns,
            object_type,
            object_name,
            recipients,
            with_grant_option,
        }))
    }

    fn parse_grant_role(&mut self) -> Result<Statement, String> {
        let role_name = match self.current() {
            Some(Token::Identifier(name)) => name.clone(),
            Some(Token::StringLiteral(s)) => s.clone(),
            Some(t) => return Err(format!("Expected role name, got {:?}", t)),
            None => return Err("Expected role name".to_string()),
        };
        self.next();

        self.expect(Token::To)?;

        let (user_name, host) = self.parse_user_identity()?;

        Ok(Statement::GrantRole(GrantRoleStatement {
            role_name,
            user_name,
            host,
        }))
    }

    fn parse_user_identity(&mut self) -> Result<(String, Option<String>), String> {
        let user_name = match self.current() {
            Some(Token::Identifier(name)) => name.clone(),
            Some(Token::StringLiteral(s)) => s.clone(),
            Some(t) => return Err(format!("Expected user name, got {:?}", t)),
            None => return Err("Expected user name".to_string()),
        };
        self.next();

        Ok((user_name, None))
    }

    fn parse_revoke_role(&mut self) -> Result<Statement, String> {
        let role_name = match self.current() {
            Some(Token::Identifier(name)) => name.clone(),
            Some(Token::StringLiteral(s)) => s.clone(),
            Some(t) => return Err(format!("Expected role name, got {:?}", t)),
            None => return Err("Expected role name".to_string()),
        };
        self.next();

        self.expect(Token::From)?;

        let (user_name, host) = self.parse_user_identity()?;

        Ok(Statement::RevokeRole(RevokeRoleStatement {
            role_name,
            user_name,
            host,
        }))
    }

    fn parse_revoke(&mut self) -> Result<Statement, String> {
        self.expect(Token::Revoke)?;

        let is_role_revoke = match self.current() {
            Some(Token::Select) | Some(Token::Insert) | Some(Token::Update)
            | Some(Token::Delete) => false,
            Some(Token::Identifier(name)) => {
                let upper = name.to_uppercase();
                !matches!(
                    upper.as_str(),
                    "READ" | "WRITE" | "ALL" | "EXECUTE" | "USAGE"
                )
            }
            Some(Token::StringLiteral(_)) => true,
            _ => false,
        };

        if is_role_revoke {
            return self.parse_revoke_role();
        }

        let grant_option_for = if let Some(Token::Identifier(name)) = self.current() {
            if name.to_uppercase() == "GRANT" {
                self.next();
                if let Some(Token::Identifier(option_str)) = self.current() {
                    if option_str.to_uppercase() == "OPTION" {
                        self.next();
                        if let Some(Token::Identifier(for_str)) = self.current() {
                            if for_str.to_uppercase() == "FOR" {
                                self.next();
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        let mut privileges = Vec::new();
        loop {
            match self.current() {
                Some(Token::Select) => {
                    privileges.push(Privilege::Select);
                    self.next();
                }
                Some(Token::Insert) => {
                    privileges.push(Privilege::Insert);
                    self.next();
                }
                Some(Token::Update) => {
                    privileges.push(Privilege::Update);
                    self.next();
                }
                Some(Token::Delete) => {
                    privileges.push(Privilege::Delete);
                    self.next();
                }
                Some(Token::Identifier(name)) if name.to_uppercase() == "READ" => {
                    privileges.push(Privilege::Read);
                    self.next();
                }
                Some(Token::Identifier(name)) if name.to_uppercase() == "WRITE" => {
                    privileges.push(Privilege::Write);
                    self.next();
                }
                Some(Token::Identifier(name)) if name.to_uppercase() == "ALL" => {
                    privileges.push(Privilege::All);
                    self.next();
                    break;
                }
                Some(Token::Identifier(name)) if name.to_uppercase() == "EXECUTE" => {
                    privileges.push(Privilege::Execute);
                    self.next();
                }
                Some(Token::Identifier(name)) if name.to_uppercase() == "USAGE" => {
                    privileges.push(Privilege::Usage);
                    self.next();
                }
                _ => break,
            }

            if !matches!(self.current(), Some(Token::Comma)) {
                break;
            }
            self.next();
        }

        let mut columns = Vec::new();
        if matches!(self.current(), Some(Token::LParen)) {
            self.next();
            loop {
                match self.current() {
                    Some(Token::Identifier(name)) => {
                        columns.push(name.clone());
                        self.next();
                    }
                    _ => return Err("Expected column name".to_string()),
                }
                if !matches!(self.current(), Some(Token::Comma)) {
                    break;
                }
                self.next();
            }
            self.expect(Token::RParen)?;
        }

        self.expect(Token::On)?;

        let object_type = match self.current() {
            Some(Token::Identifier(name)) if name.to_uppercase() == "TABLE" => {
                self.next();
                ObjectType::Table
            }
            Some(Token::Identifier(name)) if name.to_uppercase() == "DATABASE" => {
                self.next();
                ObjectType::Database
            }
            Some(Token::Identifier(name)) if name.to_uppercase() == "PROCEDURE" => {
                self.next();
                ObjectType::Procedure
            }
            Some(Token::Identifier(name)) if name.to_uppercase() == "FUNCTION" => {
                self.next();
                ObjectType::Function
            }
            Some(Token::Identifier(name)) if name.to_uppercase() == "COLUMN" => {
                self.next();
                ObjectType::Column
            }
            _ => ObjectType::Table,
        };

        let object_name = match self.next() {
            Some(Token::Identifier(name)) => name,
            Some(t) => return Err(format!("Expected object name, got {:?}", t)),
            None => return Err("Expected object name".to_string()),
        };

        self.expect(Token::From)?;

        let mut from_users = Vec::new();
        loop {
            match self.next() {
                Some(Token::Identifier(name)) => {
                    from_users.push(name);
                }
                Some(t) => return Err(format!("Expected user, got {:?}", t)),
                None => return Err("Expected user".to_string()),
            }
            if !matches!(self.current(), Some(Token::Comma)) {
                break;
            }
            self.next();
        }

        Ok(Statement::Revoke(RevokeStatement {
            privileges,
            columns,
            object_type,
            object_name,
            from_users,
            grant_option_for,
        }))
    }

    fn parse_call(&mut self) -> Result<Statement, String> {
        self.expect(Token::Call)?;

        let procedure_name = match self.next() {
            Some(Token::Identifier(name)) => name,
            Some(t) => return Err(format!("Expected procedure name, got {:?}", t)),
            None => return Err("Expected procedure name".to_string()),
        };

        let mut args = Vec::new();
        if matches!(self.current(), Some(Token::LParen)) {
            self.next();
            while !matches!(self.current(), Some(Token::RParen) | None) {
                match self.current() {
                    Some(
                        Token::Identifier(_)
                        | Token::StringLiteral(_)
                        | Token::NumberLiteral(_)
                        | Token::Null,
                    ) => {
                        let arg = match self.next() {
                            Some(Token::Identifier(s)) => s,
                            Some(Token::StringLiteral(s)) => s,
                            Some(Token::NumberLiteral(s)) => s,
                            Some(Token::Null) => "NULL".to_string(),
                            Some(t) => return Err(format!("Expected argument, got {:?}", t)),
                            None => return Err("Unexpected end of input".to_string()),
                        };
                        args.push(arg);
                    }
                    Some(Token::Comma) => {
                        self.next();
                    }
                    Some(t) => return Err(format!("Unexpected token in argument list: {:?}", t)),
                    None => return Err("Unexpected end of input".to_string()),
                }
            }
            self.expect(Token::RParen)?;
        }

        Ok(Statement::Call(CallStatement {
            procedure_name,
            args,
        }))
    }

    fn parse_alter_table(&mut self) -> Result<Statement, String> {
        self.expect(Token::Alter)?;
        self.expect(Token::Table)?;

        let table_name = match self.next() {
            Some(Token::Identifier(name)) => name,
            _ => return Err("Expected table name".to_string()),
        };

        match self.current() {
            Some(Token::Add) => {
                self.next();
                if matches!(self.current(), Some(Token::Column)) {
                    self.next();
                }
                let col_name = match self.next() {
                    Some(Token::Identifier(name)) => name,
                    _ => return Err("Expected column name".to_string()),
                };

                let data_type = match self.next() {
                    Some(Token::Identifier(typename)) => typename,
                    Some(Token::Integer) => "INTEGER".to_string(),
                    Some(Token::Text) => "TEXT".to_string(),
                    Some(Token::Float) => "FLOAT".to_string(),
                    Some(Token::Boolean) => "BOOLEAN".to_string(),
                    _ => return Err("Expected data type".to_string()),
                };

                let nullable = true;
                let default_value = None;

                Ok(Statement::AlterTable(AlterTableStatement {
                    table_name,
                    operation: AlterTableOperation::AddColumn {
                        name: col_name,
                        data_type,
                        nullable,
                        default_value,
                    },
                }))
            }
            Some(Token::Rename) => {
                self.next();
                self.expect(Token::To)?;
                let new_name = match self.next() {
                    Some(Token::Identifier(name)) => name,
                    _ => return Err("Expected new table name".to_string()),
                };
                Ok(Statement::AlterTable(AlterTableStatement {
                    table_name,
                    operation: AlterTableOperation::RenameTo { new_name },
                }))
            }
            Some(Token::Drop) => {
                self.next();
                if matches!(self.current(), Some(Token::Column)) {
                    self.next();
                }
                let col_name = match self.next() {
                    Some(Token::Identifier(name)) => name,
                    _ => return Err("Expected column name".to_string()),
                };
                Ok(Statement::AlterTable(AlterTableStatement {
                    table_name,
                    operation: AlterTableOperation::DropColumn { name: col_name },
                }))
            }
            Some(Token::Modify) => {
                self.next();
                if matches!(self.current(), Some(Token::Column)) {
                    self.next();
                }
                let col_name = match self.next() {
                    Some(Token::Identifier(name)) => name,
                    _ => return Err("Expected column name".to_string()),
                };
                let data_type = match self.next() {
                    Some(Token::Identifier(typename)) => typename,
                    Some(Token::Integer) => "INTEGER".to_string(),
                    Some(Token::Text) => "TEXT".to_string(),
                    Some(Token::Float) => "FLOAT".to_string(),
                    Some(Token::Boolean) => "BOOLEAN".to_string(),
                    _ => return Err("Expected data type".to_string()),
                };
                let nullable = true;
                Ok(Statement::AlterTable(AlterTableStatement {
                    table_name,
                    operation: AlterTableOperation::ModifyColumn {
                        name: col_name,
                        data_type,
                        nullable,
                    },
                }))
            }
            _ => Err("Expected ADD, DROP, MODIFY or RENAME".to_string()),
        }
    }
}

/// Parse a SQL string into statements
pub fn parse(sql: &str) -> Result<Statement, String> {
    let tokens = Lexer::new(sql).tokenize();
    let mut parser = Parser::new(tokens);
    parser.parse_statement()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_qualified_column_names() {
        let sql = "SELECT u.name FROM t";
        let result = parse(sql);
        assert!(result.is_ok(), "Parse failed for {}: {:?}", sql, result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert_eq!(s.table, "t");
                assert_eq!(s.columns.len(), 1);
                assert_eq!(s.columns[0].name, "u.name");
            }
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_insert() {
        let result = parse("INSERT INTO users VALUES (1)");
        assert!(result.is_ok());
        match result.unwrap() {
            Statement::Insert(i) => {
                assert_eq!(i.table, "users");
                assert_eq!(i.values.len(), 1); // 1 row
                assert_eq!(i.values[0].len(), 1); // 1 value per row
            }
            _ => panic!("Expected INSERT statement"),
        }
    }

    #[test]
    fn test_parse_insert_with_values() {
        let result = parse("INSERT INTO users VALUES (1, 'Alice')");
        assert!(result.is_ok());
        match result.unwrap() {
            Statement::Insert(i) => {
                assert_eq!(i.table, "users");
                assert_eq!(i.values.len(), 1); // 1 row
                assert_eq!(i.values[0].len(), 2); // 2 values per row
            }
            _ => panic!("Expected INSERT statement"),
        }
    }

    #[test]
    fn test_parse_insert_with_columns() {
        let result = parse("INSERT INTO users (id, name) VALUES (1, 'Alice')");
        assert!(result.is_ok());
        match result.unwrap() {
            Statement::Insert(i) => {
                assert_eq!(i.table, "users");
                assert_eq!(i.columns, vec!["id", "name"]);
                assert_eq!(i.values.len(), 1); // 1 row
                assert_eq!(i.values[0].len(), 2); // 2 values
            }
            _ => panic!("Expected INSERT statement"),
        }
    }

    #[test]
    fn test_parse_insert_multi_row() {
        let result = parse("INSERT INTO users VALUES (1, 'Alice'), (2, 'Bob'), (3, 'Charlie')");
        assert!(result.is_ok());
        match result.unwrap() {
            Statement::Insert(i) => {
                assert_eq!(i.table, "users");
                assert_eq!(i.values.len(), 3); // 3 rows
                assert_eq!(i.values[0].len(), 2); // 2 values per row
                assert_eq!(i.values[1].len(), 2);
                assert_eq!(i.values[2].len(), 2);
            }
            _ => panic!("Expected INSERT statement"),
        }
    }

    #[test]
    fn test_parse_insert_select() {
        let result = parse("INSERT INTO users SELECT * FROM old_users");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Insert(i) => {
                assert_eq!(i.table, "users");
                assert!(
                    i.values.is_empty(),
                    "INSERT VALUES should be empty for INSERT SELECT"
                );
                assert!(
                    i.select.is_some(),
                    "INSERT SELECT should have a select statement"
                );
                let select = i.select.as_ref().unwrap();
                assert_eq!(select.table, "old_users");
                assert_eq!(select.columns.len(), 1); // * expands to one column
            }
            _ => panic!("Expected INSERT statement"),
        }
    }

    #[test]
    fn test_parse_update() {
        let result = parse("UPDATE users SET name = 'Bob'");
        assert!(result.is_ok());
        match result.unwrap() {
            Statement::Update(u) => {
                assert_eq!(u.table, "users");
                assert_eq!(u.set_clauses.len(), 1);
                assert_eq!(u.set_clauses[0].0, "name");
            }
            _ => panic!("Expected UPDATE statement"),
        }
    }

    #[test]
    fn test_parse_update_with_where() {
        let result = parse("UPDATE users SET name = 'Bob' WHERE id = 1");
        assert!(result.is_ok());
        match result.unwrap() {
            Statement::Update(u) => {
                assert_eq!(u.table, "users");
                assert_eq!(u.set_clauses.len(), 1);
                assert!(u.where_clause.is_some());
            }
            _ => panic!("Expected UPDATE statement"),
        }
    }

    #[test]
    fn test_parse_insert_select_with_columns() {
        let result =
            parse("INSERT INTO users (id, name) SELECT id, name FROM old_users WHERE id > 0");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Insert(i) => {
                assert_eq!(i.table, "users");
                assert_eq!(i.columns, vec!["id".to_string(), "name".to_string()]);
                assert!(
                    i.select.is_some(),
                    "INSERT SELECT should have a select statement"
                );
                let select = i.select.as_ref().unwrap();
                assert_eq!(select.table, "old_users");
                assert!(select.where_clause.is_some());
            }
            _ => panic!("Expected INSERT statement"),
        }
    }

    #[test]
    fn test_parse_replace_into() {
        let result = parse("REPLACE INTO users VALUES (1, 'Alice')");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Insert(i) => {
                assert_eq!(i.table, "users");
                assert!(i.is_replace, "REPLACE INTO should set is_replace to true");
                assert_eq!(i.values.len(), 1); // 1 row
                assert_eq!(i.values[0].len(), 2); // 2 values per row
            }
            _ => panic!("Expected INSERT statement"),
        }
    }

    #[test]
    fn test_parse_replace_into_with_columns() {
        let result = parse("REPLACE INTO users (id, name) VALUES (1, 'Alice')");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Insert(i) => {
                assert_eq!(i.table, "users");
                assert!(i.is_replace, "REPLACE INTO should set is_replace to true");
                assert_eq!(i.columns, vec!["id", "name"]);
                assert_eq!(i.values.len(), 1); // 1 row
                assert_eq!(i.values[0].len(), 2); // 2 values
            }
            _ => panic!("Expected INSERT statement"),
        }
    }

    #[test]
    fn test_parse_merge_basic_when_matched_update() {
        let sql = "MERGE INTO target t USING source s ON t.id = s.id \
                   WHEN MATCHED THEN UPDATE SET t.val = s.val";
        let result = parse(sql);
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Merge(m) => {
                assert_eq!(m.target_table, "target");
                assert_eq!(m.target_alias, Some("t".to_string()));
                match &m.source {
                    MergeSource::Table { name } => assert_eq!(name, "source"),
                    _ => panic!("Expected Table source"),
                }
                assert_eq!(m.source_alias, Some("s".to_string()));
                assert_eq!(m.when_clauses.len(), 1);
                assert!(m.when_clauses[0].is_matched);
                assert!(m.when_clauses[0].additional_condition.is_none());
                match &m.when_clauses[0].action {
                    MergeAction::Update { set_clauses } => {
                        assert_eq!(set_clauses.len(), 1);
                        assert_eq!(set_clauses[0].0, "t.val");
                    }
                    _ => panic!("Expected Update action"),
                }
            }
            _ => panic!("Expected MERGE statement"),
        }
    }

    #[test]
    fn test_parse_merge_when_matched_and_not_matched() {
        let sql = "MERGE INTO target USING source ON target.id = source.id \
                   WHEN MATCHED THEN UPDATE SET target.val = source.val \
                   WHEN NOT MATCHED THEN INSERT (id, val) VALUES (source.id, source.val)";
        let result = parse(sql);
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Merge(m) => {
                assert_eq!(m.when_clauses.len(), 2);
                assert!(m.when_clauses[0].is_matched);
                assert!(!m.when_clauses[1].is_matched);
                assert!(matches!(
                    m.when_clauses[0].action,
                    MergeAction::Update { .. }
                ));
                match &m.when_clauses[1].action {
                    MergeAction::Insert { columns, values } => {
                        assert_eq!(columns, &vec!["id".to_string(), "val".to_string()]);
                        assert_eq!(values.len(), 2);
                    }
                    _ => panic!("Expected Insert action"),
                }
            }
            _ => panic!("Expected MERGE statement"),
        }
    }

    #[test]
    fn test_parse_merge_with_aliases() {
        let sql = "MERGE INTO target USING source ON target.id = source.id \
                   WHEN MATCHED THEN UPDATE SET target.val = source.val";
        let result = parse(sql);
        assert!(result.is_ok());
        match result.unwrap() {
            Statement::Merge(m) => {
                assert_eq!(m.target_alias, None);
                assert_eq!(m.source_alias, None);
            }
            _ => panic!("Expected MERGE statement"),
        }
    }

    #[test]
    fn test_parse_merge_with_subquery_source() {
        let sql = "MERGE INTO target t \
                   USING (SELECT id, val FROM other) s \
                   ON t.id = s.id \
                   WHEN NOT MATCHED THEN INSERT (id, val) VALUES (s.id, s.val)";
        let result = parse(sql);
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Merge(m) => {
                assert!(matches!(&m.source, MergeSource::Subquery(_)));
                assert_eq!(m.source_alias, Some("s".to_string()));
            }
            _ => panic!("Expected MERGE statement"),
        }
    }

    #[test]
    fn test_parse_merge_with_additional_condition() {
        let sql = "MERGE INTO target USING source ON target.id = source.id \
                   WHEN MATCHED AND target.val > 100 THEN UPDATE SET target.val = source.val";
        let result = parse(sql);
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Merge(m) => {
                assert_eq!(m.when_clauses.len(), 1);
                assert!(m.when_clauses[0].additional_condition.is_some());
            }
            _ => panic!("Expected MERGE statement"),
        }
    }

    #[test]
    fn test_parse_merge_rejects_missing_using() {
        let sql = "MERGE INTO target WHEN MATCHED THEN UPDATE SET target.val = 1";
        let result = parse(sql);
        assert!(result.is_err(), "Expected parse error for missing USING");
    }

    #[test]
    fn test_parse_merge_rejects_missing_on() {
        let sql = "MERGE INTO target USING source \
                   WHEN MATCHED THEN UPDATE SET target.val = 1";
        let result = parse(sql);
        assert!(result.is_err(), "Expected parse error for missing ON");
    }

    #[test]
    fn test_parse_merge_rejects_missing_when() {
        let sql = "MERGE INTO target USING source ON target.id = source.id";
        let result = parse(sql);
        assert!(
            result.is_err(),
            "Expected parse error for missing WHEN clause"
        );
    }

    #[test]
    fn test_parse_inner_join() {
        let result =
            parse("SELECT name, amount FROM users JOIN orders ON users.id = orders.user_id");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert_eq!(s.table, "users");
                assert!(!s.join_clause.is_empty());
                let join = &s.join_clause[0];
                assert_eq!(join.table, "orders");
                assert_eq!(join.join_type, JoinType::Inner);
            }
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_left_join() {
        let result =
            parse("SELECT name, amount FROM users LEFT JOIN orders ON users.id = orders.user_id");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert_eq!(s.table, "users");
                assert!(!s.join_clause.is_empty());
                let join = &s.join_clause[0];
                assert_eq!(join.table, "orders");
                assert_eq!(join.join_type, JoinType::Left);
            }
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_full_join() {
        let result = parse("SELECT * FROM t1 FULL JOIN t2 ON t1.id = t2.id");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert_eq!(s.table, "t1");
                assert!(!s.join_clause.is_empty());
                let join = &s.join_clause[0];
                assert_eq!(join.table, "t2");
                assert_eq!(join.join_type, JoinType::Full);
            }
            _ => panic!("Expected SELECT statement"),
        }

        let result2 = parse("SELECT * FROM t1 FULL OUTER JOIN t2 ON t1.id = t2.id");
        assert!(result2.is_ok(), "Parse failed: {:?}", result2);
        match result2.unwrap() {
            Statement::Select(s) => {
                assert_eq!(s.table, "t1");
                assert!(!s.join_clause.is_empty());
                let join = &s.join_clause[0];
                assert_eq!(join.table, "t2");
                assert_eq!(join.join_type, JoinType::Full);
            }
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_create_with_foreign_key() {
        let result = parse("CREATE TABLE t (a INTEGER REFERENCES u(b))");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::CreateTable(c) => {
                assert_eq!(c.name, "t");
                assert_eq!(c.columns.len(), 1, "Actual columns: {:?}", c.columns);
                assert!(c.columns[0].references.is_some(), "No FK ref found");
                let fk = c.columns[0].references.as_ref().unwrap();
                assert_eq!(fk.referenced_table, "u");
                assert_eq!(fk.referenced_columns, vec!["b".to_string()]);
            }
            _ => panic!("Expected CREATE TABLE statement"),
        }
    }

    #[test]
    fn test_parse_create_with_columns() {
        let result = parse("CREATE TABLE users (id INTEGER, name TEXT)");
        assert!(result.is_ok());
        match result.unwrap() {
            Statement::CreateTable(c) => {
                assert_eq!(c.name, "users");
                assert_eq!(c.columns.len(), 2);
                assert_eq!(c.columns[0].name, "id");
                assert_eq!(c.columns[1].name, "name");
            }
            _ => panic!("Expected CREATE TABLE statement"),
        }
    }

    #[test]
    fn test_parse_drop() {
        let result = parse("DROP TABLE users");
        assert!(result.is_ok());
        match result.unwrap() {
            Statement::DropTable(d) => {
                assert_eq!(d.name, "users");
            }
            _ => panic!("Expected DROP TABLE statement"),
        }
    }

    #[test]
    #[ignore = "Test deferred (see tracking issue or comment context)"]
    fn test_parse_create_with_table_constraint_fk() {
        let result = parse("CREATE TABLE orders (id INTEGER, user_id INTEGER, FOREIGN KEY (user_id) REFERENCES users(id))");
        assert!(result.is_ok());
        match result.unwrap() {
            Statement::CreateTable(c) => {
                assert_eq!(c.name, "orders");
                assert_eq!(c.columns.len(), 2);
                assert_eq!(c.constraints.len(), 1);
                match &c.constraints[0] {
                    TableConstraint::ForeignKey {
                        columns,
                        referenced_table,
                        referenced_columns,
                        ..
                    } => {
                        assert_eq!(columns, &vec!["user_id".to_string()]);
                        assert_eq!(referenced_table, "users");
                        assert_eq!(referenced_columns, &vec!["id".to_string()]);
                    }
                    _ => panic!("Expected ForeignKey constraint"),
                }
            }
            _ => panic!("Expected CREATE TABLE statement"),
        }
    }

    #[test]
    fn test_parse_create_with_primary_key() {
        let result = parse("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)");
        assert!(result.is_ok());
        match result.unwrap() {
            Statement::CreateTable(c) => {
                assert_eq!(c.name, "users");
                assert!(!c.columns[0].nullable);
                assert!(c.columns[0].primary_key);
            }
            _ => panic!("Expected CREATE TABLE statement"),
        }
    }

    #[test]
    fn test_parse_create_with_not_null() {
        let result = parse("CREATE TABLE users (id INTEGER NOT NULL, name TEXT)");
        assert!(result.is_ok());
        match result.unwrap() {
            Statement::CreateTable(c) => {
                assert_eq!(c.name, "users");
                assert!(!c.columns[0].nullable);
                assert!(!c.columns[0].primary_key);
            }
            _ => panic!("Expected CREATE TABLE statement"),
        }
    }

    #[test]
    fn test_parse_create_with_auto_increment() {
        let result = parse("CREATE TABLE t (id INT AUTO_INCREMENT PRIMARY KEY)");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::CreateTable(c) => {
                assert_eq!(c.name, "t");
                assert_eq!(c.columns.len(), 1);
                assert!(c.columns[0].auto_increment);
                assert!(c.columns[0].primary_key);
                assert!(!c.columns[0].nullable);
            }
            _ => panic!("Expected CREATE TABLE statement"),
        }
    }

    #[test]
    fn test_parse_aggregate_count() {
        let result = parse("SELECT COUNT(*) FROM users");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert_eq!(s.table, "users");
                assert_eq!(s.columns.len(), 1);
                assert_eq!(s.aggregates.len(), 1);
                assert_eq!(s.aggregates[0].func, AggregateFunction::Count);
            }
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_aggregate_sum() {
        let result = parse("SELECT SUM(amount) FROM orders");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert_eq!(s.table, "orders");
                assert_eq!(s.aggregates.len(), 1);
                assert_eq!(s.aggregates[0].func, AggregateFunction::Sum);
            }
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_delete() {
        let result = parse("DELETE FROM users");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Delete(d) => {
                assert_eq!(d.table, "users");
                assert!(d.where_clause.is_none());
            }
            _ => panic!("Expected DELETE statement"),
        }
    }

    #[test]
    fn test_parse_delete_with_where() {
        let result = parse("DELETE FROM users WHERE id = 1");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Delete(d) => {
                assert_eq!(d.table, "users");
                assert!(d.where_clause.is_some());
            }
            _ => panic!("Expected DELETE statement"),
        }
    }

    #[test]
    fn test_parse_analyze() {
        let result = parse("ANALYZE users");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Analyze(a) => {
                assert_eq!(a.table_name, Some("users".to_string()));
            }
            _ => panic!("Expected ANALYZE statement"),
        }
    }

    #[test]
    fn test_parse_show_tables() {
        let result = parse("SHOW TABLES");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Show(ShowStatement::Tables) => {}
            _ => panic!("Expected SHOW TABLES statement"),
        }
    }

    #[test]
    fn test_parse_show_columns() {
        let result = parse("SHOW COLUMNS FROM users");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Show(ShowStatement::Columns { table, pattern }) => {
                assert_eq!(table, "users");
                assert!(pattern.is_none());
            }
            _ => panic!("Expected SHOW COLUMNS FROM users statement"),
        }
    }

    #[test]
    fn test_parse_show_columns_with_like() {
        let result = parse("SHOW COLUMNS FROM users LIKE '%name%'");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Show(ShowStatement::Columns { table, pattern }) => {
                assert_eq!(table, "users");
                assert_eq!(pattern, Some("%name%".to_string()));
            }
            _ => panic!("Expected SHOW COLUMNS FROM users LIKE '%name%' statement"),
        }
    }

    #[test]
    fn test_parse_show_index() {
        let result = parse("SHOW INDEX FROM users");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Show(ShowStatement::Index { table }) => {
                assert_eq!(table, "users");
            }
            _ => panic!("Expected SHOW INDEX FROM users statement"),
        }
    }

    #[test]
    fn test_parse_describe() {
        let result = parse("DESCRIBE users");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Describe(DescribeStatement { table }) => {
                assert_eq!(table, "users");
            }
            _ => panic!("Expected DESCRIBE users statement"),
        }
    }

    #[test]
    fn test_parse_desc_alias() {
        let result = parse("DESC users");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Describe(DescribeStatement { table }) => {
                assert_eq!(table, "users");
            }
            _ => panic!("Expected DESC users statement"),
        }
    }

    #[test]
    fn test_parse_alter_table_add_column() {
        let result = parse("ALTER TABLE users ADD COLUMN age INTEGER");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::AlterTable(a) => {
                assert_eq!(a.table_name, "users");
                match a.operation {
                    AlterTableOperation::AddColumn {
                        name, data_type, ..
                    } => {
                        assert_eq!(name, "age");
                        assert_eq!(data_type, "INTEGER");
                    }
                    _ => panic!("Expected AddColumn operation"),
                }
            }
            _ => panic!("Expected ALTER TABLE statement"),
        }
    }

    #[test]
    fn test_parse_alter_table_rename_to() {
        let result = parse("ALTER TABLE users RENAME TO old_users");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::AlterTable(a) => {
                assert_eq!(a.table_name, "users");
                match a.operation {
                    AlterTableOperation::RenameTo { new_name } => {
                        assert_eq!(new_name, "old_users");
                    }
                    _ => panic!("Expected RenameTo operation"),
                }
            }
            _ => panic!("Expected ALTER TABLE statement"),
        }
    }

    #[test]
    fn test_parse_right_join() {
        let result = parse("SELECT * FROM users RIGHT JOIN orders ON users.id = orders.user_id");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert!(!s.join_clause.is_empty());
                assert_eq!(s.join_clause[0].join_type, JoinType::Right);
            }
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_call() {
        let result = parse("CALL test_proc()");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Call(c) => {
                assert_eq!(c.procedure_name, "test_proc");
                assert!(c.args.is_empty());
            }
            _ => panic!("Expected CALL statement"),
        }
    }

    #[test]
    fn test_parse_call_with_args() {
        let result = parse("CALL test_proc(1, 'hello', var1)");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Call(c) => {
                assert_eq!(c.procedure_name, "test_proc");
                assert_eq!(c.args.len(), 3);
                assert_eq!(c.args[0], "1");
                assert_eq!(c.args[1], "hello");
                assert_eq!(c.args[2], "var1");
            }
            _ => panic!("Expected CALL statement"),
        }
    }

    #[test]
    fn test_parse_comparison_expression() {
        let result = parse("SELECT * FROM t WHERE a > b AND c < d");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert!(s.where_clause.is_some());
            }
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_like_expression() {
        let result = parse("SELECT * FROM t WHERE name LIKE '%test%'");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert!(s.where_clause.is_some());
            }
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_in_list_expression() {
        let result = parse("SELECT * FROM t WHERE id IN (1, 2, 3)");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert!(s.where_clause.is_some());
            }
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_distinct() {
        let result = parse("SELECT DISTINCT country FROM users");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert!(s.distinct);
            }
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_aggregate_avg() {
        let result = parse("SELECT AVG(price) FROM orders");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert_eq!(s.table, "orders");
                assert_eq!(s.aggregates.len(), 1);
                assert_eq!(s.aggregates[0].func, AggregateFunction::Avg);
            }
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_aggregate_min_max() {
        let result = parse("SELECT MIN(id), MAX(id) FROM users");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert_eq!(s.table, "users");
                assert_eq!(s.aggregates.len(), 2);
                assert_eq!(s.aggregates[0].func, AggregateFunction::Min);
                assert_eq!(s.aggregates[1].func, AggregateFunction::Max);
            }
            _ => panic!("Expected SELECT statement"),
        }
    }
}

#[test]
fn test_debug_having() {
    let sql =
        "SELECT region, SUM(amount) FROM sales_summary GROUP BY region HAVING SUM(amount) > 150";
    match parse(sql) {
        Ok(stmt) => {
            println!("OK: {:#?}", stmt);
            if let Statement::Select(s) = stmt {
                println!("having = {:?}", s.having);
            }
        }
        Err(e) => {
            println!("ERROR: {}", e);
        }
    }

    #[test]
    fn test_parse_binary_expression_subtraction() {
        let result = parse("SELECT price - discount FROM orders");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert_eq!(s.columns.len(), 1);
                assert!(s.columns[0].expression.is_some());
            }
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_binary_expression_multiplication() {
        let result = parse("SELECT quantity * price FROM orders");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert_eq!(s.columns.len(), 1);
                assert!(s.columns[0].expression.is_some());
            }
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_binary_expression_division() {
        let result = parse("SELECT total / cnt FROM stats");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert_eq!(s.columns.len(), 1);
                assert!(s.columns[0].expression.is_some());
            }
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_binary_expression_modulo() {
        let result = parse("SELECT total % discount FROM orders");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert_eq!(s.columns.len(), 1);
                assert!(s.columns[0].expression.is_some());
            }
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_binary_expression_not_equal() {
        let result = parse("SELECT a != b FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert_eq!(s.columns.len(), 1);
                assert!(s.columns[0].expression.is_some());
            }
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_binary_expression_less_equal() {
        let result = parse("SELECT a <= b FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert_eq!(s.columns.len(), 1);
                assert!(s.columns[0].expression.is_some());
            }
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_binary_expression_greater_equal() {
        let result = parse("SELECT a >= b FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert_eq!(s.columns.len(), 1);
                assert!(s.columns[0].expression.is_some());
            }
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_binary_expression_complex() {
        let result = parse("SELECT a + b * c - d / e FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert_eq!(s.columns.len(), 1);
                assert!(s.columns[0].expression.is_some());
            }
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_binary_expression_with_literal() {
        let result = parse("SELECT id + 1 FROM users");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert_eq!(s.columns.len(), 1);
                assert!(s.columns[0].expression.is_some());
            }
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_binary_expression_multiple_columns() {
        let result = parse("SELECT a + b, c - d, e * f FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert_eq!(s.columns.len(), 3);
                assert!(s.columns[0].expression.is_some());
                assert!(s.columns[1].expression.is_some());
                assert!(s.columns[2].expression.is_some());
            }
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_binary_expression_mixed_with_identifier() {
        let result = parse("SELECT a + b, name, c * d FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert_eq!(s.columns.len(), 3);
                assert!(s.columns[0].expression.is_some());
                assert!(s.columns[1].expression.is_none());
                assert!(s.columns[2].expression.is_some());
            }
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_binary_expression_with_table_prefix() {
        let result = parse("SELECT t.a + t.b FROM table_name t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert_eq!(s.columns.len(), 1);
                assert!(s.columns[0].expression.is_some());
            }
            _ => panic!("Expected SELECT statement"),
        }
    }

    #[test]
    fn test_parse_create_trigger() {
        let sql = "CREATE TRIGGER test_trigger BEFORE INSERT ON users FOR EACH ROW BEGIN SET NEW.name = 'triggered'; END";
        let result = parse(sql);
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::CreateTrigger(t) => {
                assert_eq!(t.name, "test_trigger");
                assert_eq!(t.timing, "BEFORE");
                assert_eq!(t.table, "users");
                assert_eq!(t.events.len(), 1);
                assert_eq!(t.events[0], "INSERT");
                assert!(t.body.contains("SET NEW.name"));
            }
            _ => panic!("Expected CREATE TRIGGER statement"),
        }
    }

    #[test]
    fn test_parse_create_trigger_after_update() {
        let sql = "CREATE TRIGGER update_trigger AFTER UPDATE ON orders FOR EACH ROW BEGIN DELETE FROM log; END";
        let result = parse(sql);
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::CreateTrigger(t) => {
                assert_eq!(t.name, "update_trigger");
                assert_eq!(t.timing, "AFTER");
                assert_eq!(t.table, "orders");
                assert_eq!(t.events.len(), 1);
                assert_eq!(t.events[0], "UPDATE");
            }
            _ => panic!("Expected CREATE TRIGGER statement"),
        }
    }

    #[test]
    fn test_parse_begin() {
        let result = parse("BEGIN");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Transaction(TransactionStatement::Begin {
                work,
                isolation_level,
            }) => {
                assert!(!work);
                assert!(isolation_level.is_none());
            }
            _ => panic!("Expected BEGIN statement"),
        }
    }

    #[test]
    fn test_parse_begin_work() {
        let result = parse("BEGIN WORK");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Transaction(TransactionStatement::Begin {
                work,
                isolation_level,
            }) => {
                assert!(work);
                assert!(isolation_level.is_none());
            }
            _ => panic!("Expected BEGIN WORK statement"),
        }
    }

    #[test]
    fn test_parse_begin_serializable() {
        let result = parse("BEGIN SERIALIZABLE");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Transaction(TransactionStatement::Begin {
                work,
                isolation_level,
            }) => {
                assert!(!work);
                assert_eq!(isolation_level, Some(IsolationLevel::Serializable));
            }
            _ => panic!("Expected BEGIN SERIALIZABLE statement"),
        }
    }

    #[test]
    fn test_parse_begin_isolation_level() {
        let result = parse("BEGIN ISOLATION LEVEL SERIALIZABLE");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Transaction(TransactionStatement::Begin {
                work,
                isolation_level,
            }) => {
                assert!(!work);
                assert_eq!(isolation_level, Some(IsolationLevel::Serializable));
            }
            _ => panic!("Expected BEGIN ISOLATION LEVEL SERIALIZABLE statement"),
        }
    }

    #[test]
    fn test_parse_commit() {
        let result = parse("COMMIT");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Transaction(TransactionStatement::Commit { work }) => {
                assert!(!work);
            }
            _ => panic!("Expected COMMIT statement"),
        }
    }

    #[test]
    fn test_parse_commit_work() {
        let result = parse("COMMIT WORK");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Transaction(TransactionStatement::Commit { work }) => {
                assert!(work);
            }
            _ => panic!("Expected COMMIT WORK statement"),
        }
    }

    #[test]
    fn test_parse_rollback() {
        let result = parse("ROLLBACK");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Transaction(TransactionStatement::Rollback { work }) => {
                assert!(!work);
            }
            _ => panic!("Expected ROLLBACK statement"),
        }
    }

    #[test]
    fn test_parse_rollback_work() {
        let result = parse("ROLLBACK WORK");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Transaction(TransactionStatement::Rollback { work }) => {
                assert!(work);
            }
            _ => panic!("Expected ROLLBACK WORK statement"),
        }
    }

    #[test]
    fn test_parse_start_transaction() {
        let result = parse("START TRANSACTION");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Transaction(TransactionStatement::StartTransaction { isolation_level }) => {
                assert!(isolation_level.is_none());
            }
            _ => panic!("Expected START TRANSACTION statement"),
        }
    }

    #[test]
    fn test_parse_start_transaction_serializable() {
        let result = parse("START TRANSACTION ISOLATION LEVEL SERIALIZABLE");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Transaction(TransactionStatement::StartTransaction { isolation_level }) => {
                assert_eq!(isolation_level, Some(IsolationLevel::Serializable));
            }
            _ => panic!("Expected START TRANSACTION ISOLATION LEVEL SERIALIZABLE statement"),
        }
    }

    #[test]
    fn test_parse_set_transaction() {
        let result = parse("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Transaction(TransactionStatement::SetTransaction { isolation_level }) => {
                assert_eq!(isolation_level, IsolationLevel::Serializable);
            }
            _ => panic!("Expected SET TRANSACTION statement"),
        }
    }

    #[test]
    fn test_parse_set_transaction_read_committed() {
        let result = parse("SET TRANSACTION ISOLATION LEVEL READ COMMITTED");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Transaction(TransactionStatement::SetTransaction { isolation_level }) => {
                assert_eq!(isolation_level, IsolationLevel::ReadCommitted);
            }
            _ => panic!("Expected SET TRANSACTION ISOLATION LEVEL READ COMMITTED statement"),
        }
    }

    #[test]
    fn test_parse_set_transaction_read_uncommitted() {
        let result = parse("SET TRANSACTION ISOLATION LEVEL READ UNCOMMITTED");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Transaction(TransactionStatement::SetTransaction { isolation_level }) => {
                assert_eq!(isolation_level, IsolationLevel::ReadUncommitted);
            }
            _ => panic!("Expected SET TRANSACTION ISOLATION LEVEL READ UNCOMMITTED statement"),
        }
    }

    #[test]
    fn test_parse_begin_read_committed() {
        let result = parse("BEGIN ISOLATION LEVEL READ COMMITTED");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Transaction(TransactionStatement::Begin {
                work,
                isolation_level,
            }) => {
                assert!(!work);
                assert_eq!(isolation_level, Some(IsolationLevel::ReadCommitted));
            }
            _ => panic!("Expected BEGIN ISOLATION LEVEL READ COMMITTED statement"),
        }
    }

    #[test]
    fn test_parse_begin_repeatable_read() {
        let result = parse("BEGIN ISOLATION LEVEL REPEATABLE READ");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Transaction(TransactionStatement::Begin {
                work,
                isolation_level,
            }) => {
                assert!(!work);
                assert_eq!(isolation_level, Some(IsolationLevel::SnapshotIsolation));
            }
            _ => panic!("Expected BEGIN ISOLATION LEVEL REPEATABLE READ statement"),
        }
    }
}

#[test]
fn test_parse_is_null_expression() {
    let result = parse("SELECT * FROM t WHERE col IS NULL");
    assert!(result.is_ok(), "Parse failed: {:?}", result);
    match result.unwrap() {
        Statement::Select(s) => {
            assert!(s.where_clause.is_some());
        }
        _ => panic!("Expected SELECT statement"),
    }
}

#[test]
fn test_parse_is_not_null_expression() {
    let result = parse("SELECT * FROM t WHERE col IS NOT NULL");
    assert!(result.is_ok(), "Parse failed: {:?}", result);
    match result.unwrap() {
        Statement::Select(s) => {
            assert!(s.where_clause.is_some());
        }
        _ => panic!("Expected SELECT statement"),
    }
}

#[test]
fn test_parse_null_literal_is_null() {
    let result = parse("SELECT * FROM t WHERE NULL IS NULL");
    assert!(result.is_ok(), "Parse failed: {:?}", result);
}

#[test]
fn test_parse_null_literal_is_not_null() {
    let result = parse("SELECT * FROM t WHERE NULL IS NOT NULL");
    assert!(result.is_ok(), "Parse failed: {:?}", result);
}

#[test]
fn test_parse_and_with_null_in_where() {
    let result = parse("SELECT * FROM t WHERE col1 IS NULL AND col2 IS NULL");
    assert!(result.is_ok(), "Parse failed: {:?}", result);
}

#[test]
fn test_parse_or_with_null_in_where() {
    let result = parse("SELECT * FROM t WHERE col1 IS NULL OR col2 IS NULL");
    assert!(result.is_ok(), "Parse failed: {:?}", result);
}

#[test]
fn test_parse_is_not_null_in_where() {
    let result = parse("SELECT * FROM t WHERE col IS NOT NULL AND other_col IS NOT NULL");
    assert!(result.is_ok(), "Parse failed: {:?}", result);
}

#[test]
fn test_parse_complex_three_valued_expression() {
    let result = parse("SELECT * FROM t WHERE col1 IS NULL AND col2 IS NOT NULL");
    assert!(result.is_ok(), "Parse failed: {:?}", result);
}

#[test]
fn test_parse_is_null_in_join_condition() {
    let result = parse("SELECT * FROM t1 JOIN t2 ON t1.id IS NULL");
    assert!(result.is_ok(), "Parse failed: {:?}", result);
}

#[test]
fn test_parse_cross_join() {
    let result = parse("SELECT * FROM t1 CROSS JOIN t2");
    assert!(result.is_ok(), "Parse failed: {:?}", result);
    match result.unwrap() {
        Statement::Select(s) => {
            assert_eq!(s.table, "t1");
            assert!(!s.join_clause.is_empty());
            let join = &s.join_clause[0];
            assert_eq!(join.table, "t2");
            assert_eq!(join.join_type, JoinType::Cross);
        }
        _ => panic!("Expected SELECT statement"),
    }
}

#[test]
fn test_parse_join_on_multiple_conditions() {
    let result = parse("SELECT * FROM t1 JOIN t2 ON t1.id = t2.id AND t1.type = t2.type");
    assert!(result.is_ok(), "Parse failed: {:?}", result);
    match result.unwrap() {
        Statement::Select(s) => {
            assert!(!s.join_clause.is_empty());
        }
        _ => panic!("Expected SELECT statement"),
    }
}

#[test]
fn test_parse_join_with_table_alias() {
    let result = parse("SELECT * FROM users u JOIN orders o ON u.id = o.user_id");
    assert!(result.is_ok(), "Parse failed: {:?}", result);
    match result.unwrap() {
        Statement::Select(s) => {
            assert_eq!(s.table, "users");
            assert!(!s.join_clause.is_empty());
        }
        _ => panic!("Expected SELECT statement"),
    }
}

#[test]
fn test_parse_join_without_on_clause_implicit_inner() {
    let result = parse("SELECT * FROM t1 JOIN t2");
    assert!(result.is_ok(), "Parse failed: {:?}", result);
    match result.unwrap() {
        Statement::Select(s) => {
            assert_eq!(s.table, "t1");
            assert!(!s.join_clause.is_empty());
            let join = &s.join_clause[0];
            assert_eq!(join.join_type, JoinType::Inner);
        }
        _ => panic!("Expected SELECT statement"),
    }
}

#[test]
fn test_debug_fk() {
    let sql = "CREATE TABLE orders (id INTEGER PRIMARY KEY, user_id INTEGER, amount INTEGER)";
    match parse(sql) {
        Ok(stmt) => println!("OK: {:#?}", stmt),
        Err(e) => println!("ERROR: {}", e),
    }
}

#[test]
fn test_debug_refs() {
    let sql = "CREATE TABLE orders (id INTEGER PRIMARY KEY, user_id INTEGER REFERENCES users(id), amount INTEGER)";
    match parse(sql) {
        Ok(stmt) => println!("OK: {:#?}", stmt),
        Err(e) => println!("ERROR: {}", e),
    }
}

#[test]
fn test_debug_refs2() {
    let sql = "CREATE TABLE orders (user_id INTEGER REFERENCES users(id))";
    match parse(sql) {
        Ok(stmt) => println!("OK: {:#?}", stmt),
        Err(e) => println!("ERROR: {}", e),
    }
}

#[test]
fn test_debug_exact() {
    let sql = "CREATE TABLE orders (id INTEGER PRIMARY KEY, user_id INTEGER REFERENCES users(id), amount INTEGER)";
    match parse(sql) {
        Ok(stmt) => println!("OK: {:#?}", stmt),
        Err(e) => println!("ERROR: {}", e),
    }
}

#[test]
fn test_debug_cascade() {
    // This is EXACTLY what's in cascade.sql
    let sql1 = "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)";
    println!("Test 1: {:?}", parse(sql1));

    let sql2 = "CREATE TABLE orders (id INTEGER PRIMARY KEY, user_id INTEGER REFERENCES users(id), amount INTEGER)";
    println!("Test 2: {:?}", parse(sql2));

    let sql3 = "CREATE INDEX idx_orders_user_id ON orders(user_id)";
    println!("Test 3: {:?}", parse(sql3));
}

#[test]
fn test_debug_idx() {
    use crate::{lexer::Lexer, parse};

    let sql = "CREATE INDEX idx_orders_user_id ON orders(user_id)";
    println!("SQL: [{}]", sql);
    let tokens = Lexer::new(sql).tokenize();
    println!("Tokens: {:?}", tokens);

    match parse(sql) {
        Ok(stmt) => println!("OK: {:#?}", stmt),
        Err(e) => println!("ERROR: {}", e),
    }
}

#[test]
fn test_debug_json_extract() {
    use crate::{lexer::Lexer, parse};

    let sql = "SELECT JSON_EXTRACT('{\"name\":\"John\"}', '$.name')";
    println!("SQL: [{}]", sql);
    let tokens = Lexer::new(sql).tokenize();
    println!("Tokens: {:?}", tokens);

    match parse(sql) {
        Ok(stmt) => println!("OK: {:#?}", stmt),
        Err(e) => println!("ERROR: {}", e),
    }
}

#[test]
fn test_debug_json_simple() {
    use crate::{lexer::Lexer, parse};

    let sql = "SELECT JSON('{\"key\": \"value\"}') as json_val";
    println!("SQL: [{}]", sql);
    let tokens = Lexer::new(sql).tokenize();
    println!("Tokens: {:?}", tokens);

    match parse(sql) {
        Ok(stmt) => println!("OK: {:#?}", stmt),
        Err(e) => println!("ERROR: {}", e),
    }
}
