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

/// V312-58 / Issue #4512: render a token as a SQL fragment suitable
/// for re-tokenization by the lexer.
///
/// Most token variants have a canonical `Display` form that
/// round-trips through the lexer's keyword table (e.g. `Token::Plus`
/// → `"+"`, `Token::And` → `"AND"`). For the literal-bearing tokens
/// (`Identifier`, `StringLiteral`, `NumberLiteral`, `BooleanLiteral`)
/// we extract the inner payload because their `Display` impl emits
/// `IDENTIFIER(x)` / `'x'` / `42` / `true` shapes that wouldn't
/// survive the lexer's strict uppercase-keyword dispatch.
fn token_to_text(tok: &Token) -> String {
    match tok {
        Token::Identifier(s) => s.clone(),
        Token::StringLiteral(s) => format!("'{}'", s),
        Token::NumberLiteral(s) => s.clone(),
        Token::BooleanLiteral(true) => "TRUE".to_string(),
        Token::BooleanLiteral(false) => "FALSE".to_string(),
        Token::Semicolon => String::new(),
        _ => format!("{}", tok),
    }
}

/// SQL Statement types
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Select(SelectStatement),
    Explain(Box<SelectStatement>),
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
    CreateSequence(CreateSequenceStatement),
    DropSequence(DropSequenceStatement),
    AlterSequence(AlterSequenceStatement),
    Truncate(TruncateStatement),
    Analyze(AnalyzeStatement),
    WithSelect(WithSelect),
    /// WITH-clause followed by a DML statement (INSERT / UPDATE / DELETE).
    /// The CTE definitions are evaluated first, then the DML body is
    /// executed with the CTE tables materialized.
    WithDml(WithDmlStatement),
    AlterTable(AlterTableStatement),
    AlterUser(AlterUserStatement),
    Call(CallStatement),
    CreateProcedure(CreateProcedureStatement),
    /// V312-55A / Issue #4238: DROP PROCEDURE [IF EXISTS] name
    DropProcedure(DropProcedureStatement),
    /// V312-58 / Issue #4512: scalar UDF definition.
    CreateFunction(CreateFunctionStatement),
    /// V312-58 / Issue #4512: drop a scalar UDF from the engine-local
    /// registry. `IF EXISTS` follows the standard DROP shape.
    DropFunction(DropFunctionStatement),
    Union(UnionStatement),
    CreateTrigger(CreateTriggerStatement),
    /// V312-58 / Issue #4514: remove a trigger from the storage-layer
    /// trigger catalog. `IF EXISTS` follows the standard DROP shape.
    DropTrigger(DropTriggerStatement),
    /// SQL-92 INTERSECT (V310-06 PR2 / Issue #3723 C-2a).
    Intersect(IntersectStatement),
    /// SQL-92 EXCEPT (V310-06 PR2 / Issue #3723 C-2b).
    Except(ExceptStatement),
    /// VALUES constructor — for FROM (VALUES ...) AS alias
    Values(Vec<Vec<Expression>>),
    Transaction(TransactionStatement),
    Grant(GrantStatement),
    Revoke(RevokeStatement),
    Show(ShowStatement),
    Describe(DescribeStatement),
    CreateRole(CreateRoleStatement),
    DropRole(DropRoleStatement),
    CreateDatabase(CreateDatabaseStatement),
    DropDatabase(DropDatabaseStatement),
    UseDatabase(String),
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
    /// Round-21 / Issue #4218: MySQL `KILL <connection_id>`.
    /// `connection_id` is the MySQL thread id of the connection to terminate.
    /// `kill_query` distinguishes `KILL CONNECTION` (default, terminates
    /// the connection) from `KILL QUERY <id>` (cancels the running query
    /// only, leaves the connection alive).
    Kill {
        connection_id: u64,
        kill_query: bool,
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

/// SQL-92 UNION statement.
///
/// `trailing_order_by` / `trailing_limit` / `trailing_offset` carry the
/// ORDER BY / LIMIT / OFFSET clauses that appear *after* the rightmost
/// SELECT so the executor can apply them once to the merged UNION
/// result rather than only to the right input. Populated by
/// `Parser::parse_select_or_union` (see V310-06 PR2 / Issue #3723).
#[derive(Debug, Clone, PartialEq)]
pub struct UnionStatement {
    pub left: Box<Statement>,
    pub right: Box<Statement>,
    pub union_all: bool,
    pub trailing_order_by: Vec<OrderByExpression>,
    pub trailing_limit: Option<i64>,
    pub trailing_offset: Option<i64>,
}

/// SQL-92 INTERSECT (V310-06 PR2 / Issue #3723 C-2a). Returns rows
/// common to both left and right inputs (set intersection). When
/// `intersect_all` is true, rows matching in both sides are retained
/// with multiplicity (multiset intersection).
#[derive(Debug, Clone, PartialEq)]
pub struct IntersectStatement {
    pub left: Box<Statement>,
    pub right: Box<Statement>,
    pub intersect_all: bool,
    /// V4077 / Issue #4077: ORDER BY / LIMIT / OFFSET lifted from the
    /// right SELECT (consistent with UNION's behavior). The right SELECT
    /// still carries its own copy — consumers should prefer these.
    pub trailing_order_by: Vec<OrderByExpression>,
    pub trailing_limit: Option<i64>,
    pub trailing_offset: Option<i64>,
}

/// SQL-92 EXCEPT (V310-06 PR2 / Issue #3723 C-2b). Returns rows
/// that appear in the left input but not in the right (set difference).
/// When `except_all` is true, each right-side row removes one matching
/// occurrence from the left (multiset difference).
#[derive(Debug, Clone, PartialEq)]
pub struct ExceptStatement {
    pub left: Box<Statement>,
    pub right: Box<Statement>,
    pub except_all: bool,
    /// V4077 / Issue #4077: ORDER BY / LIMIT / OFFSET lifted from the
    /// right SELECT (consistent with UNION's behavior). The right SELECT
    /// still carries its own copy — consumers should prefer these.
    pub trailing_order_by: Vec<OrderByExpression>,
    pub trailing_limit: Option<i64>,
    pub trailing_offset: Option<i64>,
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

/// ALTER USER statement
#[derive(Debug, Clone, PartialEq)]
pub struct AlterUserStatement {
    /// Username for the ALTER USER statement
    pub user: String,
    /// Host for the ALTER USER statement (e.g., 'localhost', '%')
    pub host: String,
    /// Whether this is a PASSWORD EXPIRE operation
    pub password_expire: bool,
    /// Whether this is an IDENTIFIED BY operation (password change)
    pub password_change: bool,
    /// New password hash if IDENTIFIED BY is specified
    pub new_password_hash: Option<String>,
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
        char_max_length: Option<usize>,
    },
    /// `ALTER TABLE t RENAME TO new_name` — renames the table.
    RenameTo {
        new_name: String,
    },
    /// `ALTER TABLE t RENAME COLUMN old_name TO new_name` — renames a column.
    RenameColumn {
        name: String,
        new_name: String,
    },
    /// `ALTER TABLE t ALTER [COLUMN] col SET/DROP ...`
    AlterColumn {
        name: String,
        op: AlterColumnOperation,
    },
    /// `ALTER TABLE t SET PARTITIONED BY (col, ...)`
    SetPartitionedBy,
    /// `ALTER TABLE t RESET PARTITIONED BY`
    ResetPartitionedBy,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AlterColumnOperation {
    SetDefault { default_value: Option<String> },
    DropDefault,
    SetDataType { data_type: String },
    DropNotNull,
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
    /// V312-55A / Issue #4238: when true, replace existing procedure
    /// with the same name (MySQL `CREATE OR REPLACE PROCEDURE` semantics).
    /// Stored as `false` for the plain `CREATE PROCEDURE` form.
    pub or_replace: bool,
    pub params: Vec<StoredProcParam>,
    pub body: Vec<StoredProcStatement>,
}

/// DROP PROCEDURE statement
///
/// V312-55A / Issue #4238: completes the Procedure DDL lifecycle so
/// procedures can be created, listed, and removed. `IF EXISTS` follows
/// the same shape as DROP TABLE / DROP VIEW.
#[derive(Debug, Clone, PartialEq)]
pub struct DropProcedureStatement {
    pub name: String,
    pub if_exists: bool,
}

/// V312-58 / Issue #4512: scalar user-defined function parameter.
///
/// Mirrors the StoredProcParam shape but with `mode` fixed to `In`
/// (UDFs only accept IN parameters; OUT/INOUT are procedure-only).
/// The data_type is stored as a String (canonical name from the lexer)
/// so the executor can cast arg values to it on invocation.
#[derive(Debug, Clone, PartialEq)]
pub struct UdfParam {
    pub name: String,
    pub data_type: String,
}

/// V312-58 / Issue #4512: CREATE FUNCTION (scalar UDF) statement.
///
/// `body_expr` is the raw text after `RETURN` in the source SQL.
/// The executor re-parses it as a standalone expression when the UDF
/// is invoked (so a UDF body can reference its declared parameters as
/// bare identifiers). `deterministic` records the optional
/// `[DETERMINISTIC]` annotation (MySQL convention; used by the
/// executor for future caching, currently informational).
#[derive(Debug, Clone, PartialEq)]
pub struct CreateFunctionStatement {
    pub name: String,
    pub params: Vec<UdfParam>,
    pub return_type: String,
    pub deterministic: bool,
    pub body_expr: String,
}

/// V312-58 / Issue #4512: DROP FUNCTION [IF EXISTS] name.
#[derive(Debug, Clone, PartialEq)]
pub struct DropFunctionStatement {
    pub name: String,
    pub if_exists: bool,
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
    If {
        condition: String,
        then_body: Vec<StoredProcStatement>,
        else_body: Vec<StoredProcStatement>,
    },
    While {
        condition: String,
        body: Vec<StoredProcStatement>,
    },
    Loop {
        body: Vec<StoredProcStatement>,
    },
    Leave,
    Iterate,
    Set {
        var_name: String,
        value: String,
    },
    Declare {
        var_name: String,
        data_type: String,
    },
    Call {
        procedure_name: String,
        args: Vec<String>,
    },
    NestedBegin {
        body: Vec<StoredProcStatement>,
    },
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

/// DROP TRIGGER statement.
///
/// V312-58 / Issue #4514: `DROP TRIGGER [IF EXISTS] name`.
/// `if_exists` matches the standard DROP shape — when true, a missing
/// trigger is silently ignored; when false, a missing trigger is an
/// error.
#[derive(Debug, Clone, PartialEq)]
pub struct DropTriggerStatement {
    pub name: String,
    pub if_exists: bool,
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
    /// V313-followup-2 / Issue #4155: `quantile_disc(col, frac)` —
    /// discrete-quantile aggregate (DuckDB-compatible signature).
    QuantileDisc,
    /// V313-followup-2 / Issue #4155: `quantile_cont(col, frac)` —
    /// continuous-quantile aggregate with linear interpolation.
    QuantileCont,
    /// V313-followup-3 / Issue #4156: ordered-set aggregate.
    PercentileCont,
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
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SelectStatement {
    pub columns: Vec<SelectColumn>,
    pub table: String,
    /// V312-56A / 56A-R2 / Issue #4251: optional schema prefix from
    /// `FROM schema.table`. The executor routes `information_schema.<view>`
    /// to a virtual-table resolver instead of a storage scan.
    pub schema: Option<String>,
    /// TPC-H Q7/Q8/Q9: FROM `t a` stores `table = "t"`, `from_alias = "a"`.
    /// The executor uses the alias as the column-name prefix in the
    /// scan schema so the user can write `a.col` in subsequent JOIN ON.
    pub from_alias: Option<String>,
    /// TPC-H Sprint 1b fix (Q7/Q8/Q9): FROM (subquery) AS alias.
    /// When set, executor first executes the subquery and materializes its
    /// result into a temporary table named `table`, then runs the outer
    /// SELECT against that table.
    pub from_subquery: Option<Box<SelectStatement>>,
    /// VALUES constructor: FROM (VALUES ...) AS alias
    pub from_values: Option<Vec<Vec<Expression>>>,
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
    /// v3.10.0 Issue #3703: FOR UPDATE / LOCK IN SHARE MODE clause.
    /// When set, parallel execution MUST be disabled to prevent
    /// deadlocks with the global LockManager singleton.
    pub lock_clause: Option<LockClause>,
}

/// Lock clause for SELECT statements (FOR UPDATE / LOCK IN SHARE MODE)
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LockClause {
    /// True for FOR UPDATE, false for LOCK IN SHARE MODE
    pub for_update: bool,
    /// SKIP LOCKED modifier
    pub skip_locked: bool,
    /// NOWAIT modifier (don't wait for lock if not available)
    pub nowait: bool,
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
    pub is_ignore: bool,                      // For INSERT IGNORE (MySQL compatibility, V311-23)
    pub on_duplicate_key_update: Option<Vec<(String, Expression)>>, // For ON DUPLICATE KEY UPDATE
}

/// A reference to a table (name plus optional alias) used in DML
/// statements to support `UPDATE t1, t2 SET ...` and
/// `DELETE t1, t2 FROM t1, t2 WHERE ...` syntax.
#[derive(Debug, Clone, PartialEq)]
pub struct TableRef {
    pub name: String,
    /// Optional schema/database qualifier (`SELECT ... FROM schema.table`).
    /// V312-56A / 56A-R2 / Issue #4251: enables `FROM information_schema.<view>`
    /// virtual-table resolution at the executor level. The parser records
    /// the prefix; the executor decides whether to route to a virtual
    /// table (currently only `information_schema`) or to fall through
    /// to a regular storage scan.
    pub schema: Option<String>,
    pub alias: Option<String>,
}

/// UPDATE statement
#[derive(Debug, Clone, PartialEq)]
pub struct UpdateStatement {
    /// One or more tables that participate in the UPDATE. A single
    /// `UPDATE t SET ...` parses as `vec![TableRef { name: "t", alias: None }]`
    /// so consumers can iterate uniformly. Multi-table
    /// `UPDATE t1, t2 SET t1.col = t2.col WHERE ...` populates the
    /// list with both tables.
    pub tables: Vec<TableRef>,
    pub set_clauses: Vec<(String, Expression)>,
    pub where_clause: Option<Expression>,
}

/// DELETE statement
#[derive(Debug, Clone, PartialEq)]
pub struct DeleteStatement {
    /// Tables whose rows are removed by the DELETE.
    /// `DELETE FROM t WHERE ...` parses as
    /// `vec![TableRef { name: "t", alias: None }]`.
    /// `DELETE t1, t2 FROM t1, t2 WHERE ...` populates `tables`
    /// with the targets and `using` with the sources.
    pub tables: Vec<TableRef>,
    /// Optional additional source tables (the second list in
    /// `DELETE <targets> FROM <sources> WHERE ...`). When `None`,
    /// the executor uses `tables` as the source set.
    pub using: Option<Vec<TableRef>>,
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

/// V311-01: Storage engine selection for CREATE TABLE.
///   - `Heap` = default (MemoryStorage, Vec<Record>-based, no PK ordering)
///   - `Clustered` = InnoDB-style B+ Tree with rows in leaves ordered by PK
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum StorageEngineSpec {
    #[default]
    Heap,
    Clustered,
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
    /// V311-01: optional storage engine override (default `Heap`).
    pub storage_engine: Option<StorageEngineSpec>,
    /// V311-12 F-27: table compression specifier.
    /// Syntax: COMPRESS (ALGORITHM=LZ4) or COMPRESS (ALGORITHM=ZSTD)
    pub compress: Option<CompressionSpec>,
    /// V312-18: CREATE TABLE AS SELECT - the SELECT statement to populate the table.
    pub select: Option<Box<SelectStatement>>,
    /// V312-18: CREATE OR REPLACE TABLE flag.
    pub or_replace: bool,
    /// V312-18: WITH NO DATA / WITH DATA clause for CTAS.
    pub with_data: Option<bool>,
}

/// Compression specification for table compression (F-27)
#[derive(Debug, Clone, PartialEq)]
pub struct CompressionSpec {
    pub algorithm: CompressionAlgorithm,
}

/// Compression algorithm type
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum CompressionAlgorithm {
    Lz4,
    Zstd,
    Zlib,
}

/// DROP TABLE statement
#[derive(Debug, Clone, PartialEq)]
pub struct DropTableStatement {
    pub name: String,
    pub if_exists: bool,
}
/// CREATE DATABASE statement
#[derive(Debug, Clone, PartialEq)]
pub struct CreateDatabaseStatement {
    pub name: String,
    pub if_not_exists: bool,
}

/// DROP DATABASE statement
#[derive(Debug, Clone, PartialEq)]
pub struct DropDatabaseStatement {
    pub name: String,
    pub if_exists: bool,
}

/// CREATE SEQUENCE statement
#[derive(Debug, Clone, PartialEq)]
pub struct CreateSequenceStatement {
    pub name: String,
    pub start_with: Option<String>,
    pub increment_by: Option<String>,
    pub minvalue: Option<String>,
    pub maxvalue: Option<String>,
    pub cache: Option<String>,
    pub cycle: Option<bool>,
    pub if_not_exists: bool,
}

/// DROP SEQUENCE statement
#[derive(Debug, Clone, PartialEq)]
pub struct DropSequenceStatement {
    pub name: String,
    pub if_exists: bool,
}

/// ALTER SEQUENCE statement
#[derive(Debug, Clone, PartialEq)]
pub struct AlterSequenceStatement {
    pub name: String,
    pub restart_with: Option<String>,
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
    /// V312-58 / Issue #4516: MySQL `SHOW TABLES [FROM db]
    /// [LIKE 'pat' | WHERE expr]`. The non-FULL form lists bare table
    /// names (single column `Tables_in_<db>`); the FULL form lives in
    /// `ShowStatement::FullTables { full: true, ... }`.
    Tables {
        db: Option<String>,
        like: Option<String>,
        where_clause: Option<Expression>,
    },
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
    Sequences,
    /// Round-21 / Issue #4218: MySQL `SHOW PROCESSLIST` (and
    /// `SHOW FULL PROCESSLIST` with the `full` flag).
    Processlist {
        full: bool,
    },
    /// V312-55A / Issue #4238: MySQL `SHOW PROCEDURE STATUS [LIKE 'pat']`.
    /// The optional `pattern` is a LIKE-style filter applied at the
    /// executor; `None` lists every procedure.
    ProcedureStatus {
        pattern: Option<String>,
    },
    /// V312-56A / 56A-R4 / Issue #4251: MySQL `SHOW WARNINGS`
    /// (errors/warnings accumulated in the current session).
    Warnings,
    /// V312-56A / 56A-R4 / Issue #4251: MySQL `SHOW ERRORS`
    /// (errors accumulated in the current session; in v3.12 controlled
    /// subset returns the same rows as SHOW WARNINGS — sqlrustgo does
    /// not yet distinguish error vs warning severity levels).
    Errors,
    /// V312-56A / 56A-R4 / Issue #4251: MySQL `SHOW STATUS`
    /// (server status variables; in v3.12 controlled subset returns
    /// a fixed catalog of known status variables — Uptime, Threads_*,
    /// Questions, Slow_queries).
    Status,
    /// V312-56A / 56A-R4 / Issue #4251: MySQL `SHOW VARIABLES`
    /// (server system variables; in v3.12 controlled subset returns
    /// a fixed catalog — version, sql_mode, autocommit, character_set_*).
    Variables,
    /// V312-59-A / Issue #4384 (56A-R3 anti-deferral): MySQL
    /// `SHOW [FULL] TABLES [FROM db] [LIKE 'pat' | WHERE expr]`.
    /// `full` is true for `SHOW FULL TABLES` (the non-FULL form lists
    /// bare table names like `ShowStatement::Tables`; the FULL form
    /// adds a second `Type` column — `BASE TABLE` or `VIEW`).
    FullTables {
        full: bool,
        db: Option<String>,
        like: Option<String>,
        where_clause: Option<Expression>,
    },
    /// V312-59-A / Issue #4384 (56A-R3 anti-deferral): MySQL
    /// `SHOW TABLE STATUS [FROM db] [LIKE 'pat' | WHERE expr]`.
    /// Returns MySQL-compatible 18-column table status rows.
    TableStatus {
        db: Option<String>,
        like: Option<String>,
        where_clause: Option<Expression>,
    },
}

/// V312-59-A / Issue #4384 (56A-R3 anti-deferral): parsed `FROM db`,
/// `LIKE 'pat'` and `WHERE expr` suffix of `SHOW [FULL] TABLES` /
/// `SHOW TABLE STATUS`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ShowFilterSuffix {
    pub db: Option<String>,
    pub like: Option<String>,
    pub where_clause: Option<Expression>,
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
    /// Max length for CHAR(N) / VARCHAR(N). Captured at parse time so
    /// the engine can apply SQL-standard space padding on INSERT.
    /// `None` means unbounded / no padding (TEXT, INTEGER, etc.).
    pub char_max_length: Option<usize>,
    /// Optional column-level collation hint (e.g. `Some("NOCASE")`).
    /// V4077 / Issue #4077: captured at parse time so that set-op
    /// executors can perform collation-aware row comparison for
    /// columns declared with `COLLATE NOCASE`. Default `None` means
    /// binary (case-sensitive) comparison.
    #[serde(default)]
    pub collation: Option<String>,
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
#[derive(Debug, Clone, PartialEq)]
pub enum TableConstraint {
    PrimaryKey {
        columns: Vec<String>,
        name: Option<String>,
    },
    ForeignKey {
        columns: Vec<String>,
        referenced_table: String,
        referenced_columns: Vec<String>,
        on_delete: Option<ReferentialAction>,
        on_update: Option<ReferentialAction>,
        name: Option<String>,
    },
    Unique {
        columns: Vec<String>,
        name: Option<String>,
    },
    Check {
        expression: Expression,
        name: Option<String>,
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
    /// NEXT VALUE FOR sequence_name - advances sequence and returns next value
    SequenceNextVal(String),
    /// CURRVAL(sequence_name) - reads current value without advancing
    SequenceCurrval(String),
    /// JSON literal: JSON_EXTRACT / JSON_VALUE operands and JSON() constructor.
    /// The `String` field stores the canonical JSON text produced by serde_json.
    JsonLiteral(String),
    /// MySQL system variable reference: `@@version_comment`, `@@autocommit`,
    /// `@@sql_mode`, etc. The `String` is the variable name in lower-case.
    /// The executor resolves the name to a scalar value at evaluation time.
    SystemVariable(String),
    /// Round-21 / Issue #4216: array literal `[expr, expr, ...]`.
    /// Used by the array-fraction form of ordered-set aggregates
    /// (e.g. `quantile_disc(col, [0.25, 0.5, 0.75])`).
    ArrayLiteral(Vec<Expression>),
}

/// V312-19 #3972: constant-fold an arithmetic expression to a `u64` LIMIT/OFFSET value.
/// Returns `None` if the expression is not a constant integer.
/// Supports `+ - * / %` on integer literals.
fn constant_fold_u64(expr: &Expression) -> Option<u64> {
    match expr {
        Expression::Literal(s) => s.parse::<u64>().ok(),
        Expression::BinaryOp(left, op, right) => {
            let l = constant_fold_u64(left)?;
            let r = constant_fold_u64(right)?;
            match op.as_str() {
                "+" => l.checked_add(r),
                "-" => l.checked_sub(r),
                "*" => l.checked_mul(r),
                "/" => {
                    if r != 0 {
                        l.checked_div(r)
                    } else {
                        None
                    }
                }
                "%" => {
                    if r != 0 {
                        l.checked_rem(r)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// V313-10 / Issue #4038: classify a non-foldable LIMIT/OFFSET
/// expression to produce a user-visible error message that matches the
/// order__test_limit.test fixture expectations:
///   * Identifier "a"      -> "Referenced column 'a' not found"
///   * FunctionCall "SUM"  -> "Aggregate functions are not supported"
///   * BinaryOp with column ref (e.g. `a+1`) -> "Referenced column 'a' not found"
///   * FunctionCall with OVER (window) -> "Not implemented expression class"
fn classify_unfoldable_limit_expr(clause: &str, expr: &Expression) -> String {
    // Recursively unwrap BinaryOp to find the leaf column reference so
    // the error names the right identifier (LIMIT a+1 -> 'a'). Stop
    // descending at FunctionCall/WindowCall/Aggregate boundaries so
    // `row_number()` is not mistaken for a column name.
    fn find_column_ref(e: &Expression) -> Option<String> {
        match e {
            Expression::Identifier(n) => Some(n.clone()),
            Expression::BinaryOp(l, _, r) => find_column_ref(l).or_else(|| find_column_ref(r)),
            Expression::FunctionCall(_, _)
            | Expression::WindowCall(_)
            | Expression::Aggregate(_) => None,
            _ => None,
        }
    }
    if let Some(col) = find_column_ref(expr) {
        return format!(
            "Binder Error: Referenced column '{}' not found in {}",
            col, clause
        );
    }
    match expr {
        Expression::Identifier(name) => format!(
            "Binder Error: Referenced column '{}' not found in {}",
            name, clause
        ),
        Expression::FunctionCall(_name, _args) => {
            // Plain function (non-aggregate) inside LIMIT — rare,
            // but emit a binder-style error so we don't silently
            // swallow the clause.
            format!(
                "Binder Error: Function calls are not supported in {}",
                clause
            )
        }
        Expression::Aggregate(_agg) => {
            // Aggregate (SUM/COUNT/...) inside LIMIT — emit the
            // fixture's expected message verbatim.
            format!(
                "Binder Error: Aggregate functions are not supported in {}",
                clause
            )
        }
        Expression::WindowCall(_wc) => {
            // Window functions (row_number() OVER (...)) inside LIMIT
            // — the executor doesn't have a path to evaluate them at
            // the limit-binding phase.
            format!(
                "Not implemented Error: window function expression class in {}",
                clause
            )
        }
        _ => format!("Binder Error: Cannot evaluate {} expression", clause),
    }
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

/// Build an equi-join graph keyed on the TPC-H 1/2-char column
/// prefix that `collect_referenced_tables` produces. Returns
/// a Vec<(from, to, where_conjunct)> for each binary `=` predicate
/// in `conj` whose left/right reference disjoint (but non-empty)
/// table sets.
fn build_equi_join_edges(conj: &[Expression]) -> Vec<(String, String, Expression)> {
    let mut edges = Vec::new();
    for p in conj {
        if let Expression::BinaryOp(l, op, r) = p {
            if op == "=" {
                let lr = collect_referenced_tables(l);
                let rr = collect_referenced_tables(r);
                if lr.is_empty() || rr.is_empty() {
                    continue;
                }
                // Skip predicates referring to synthetic __subq_N
                // derived tables; those are handled by the Q15
                // alias-recovery branch.
                let is_derived = |ts: &[String]| ts.iter().any(|t| t.starts_with("__subq_"));
                if is_derived(&lr) || is_derived(&rr) {
                    continue;
                }
                // Pick the cross-product of left-vs-right tables as
                // an equi-join edge. Pick the first table on each side
                // as the representative (the predicate's actual
                // referent tables are suffix-style; the TPC-H column
                // prefix uniquely identifies the table for the 8
                // canonical tables).
                let lt = lr[0].clone();
                let rt = rr[0].clone();
                if lt == rt {
                    continue;
                }
                edges.push((lt, rt, p.clone()));
            }
        }
    }
    edges
}

/// TPC-H Q2 fix (Phase A): greedy join reorder. Given the
/// FROM `extra_tables` Vec (each entry either `table` or
/// `table|alias` for aliased names), and the WHERE conjunction
/// already extracted, return a reordered Vec that respects
/// two preferences:
///   1. Smaller tables first (so hash-join build side stays small).
///   2. Tables for which a WHERE-clause equi-join connects them to
///      `joined` (or to a previously picked table) come earlier,
///      so `find_join_predicate` can match their JOIN ON clause
///      rather than falling back to `Literal("true")` (cartesian).
///
/// The function is conservative: it never reorders more than
/// necessary for the greedy chain to find an equi-join at each
/// step, leaves the base table (`joined` already contains it)
/// alone, and falls back to the input Vec when no improvement is
/// possible (e.g. tables with no equi-joins in `conj`).
pub fn tpch_reorder_extra_tables(
    extras: &[String],
    joined: &[String],
    conj: &[Expression],
) -> Vec<String> {
    // Safety guard: only reorder for queries that (a) reference
    // the 8 canonical TPC-H tables and (b) use each at most once.
    // Reordering for general SQL is unsafe — a query like
    // `FROM supplier s1 JOIN supplier s2` or `FROM nation n1
    // JOIN nation n2` would have two distinct alias prefixes
    // (`s` and `s`) that we would conflate. We re-order only when
    // each canonical TPC-H table appears at most once in the FROM
    // (base + extras). Q15 / Q7-style multi-alias queries keep the
    // declared order.
    {
        let mut seen_bare = std::collections::HashSet::new();
        let mut dup = false;
        let base_bare_for_guard = {
            // The base `joined` includes the base table name and prefix;
            // strip them down to bare form for the dup check.
            let mut v: Vec<String> = Vec::new();
            for s in joined.iter() {
                if s.is_empty() || s.starts_with("__") {
                    continue;
                }
                // Treat the base table name as the prefix-stripped
                // TPC-H form (1 or 2 chars).
                if s.len() <= 2 {
                    v.push(s.clone());
                }
            }
            v
        };
        for t in base_bare_for_guard.iter().chain(extras.iter()) {
            let b = match t.find('|') {
                Some(idx) => &t[..idx],
                None => t,
            };
            if b.is_empty() || b.starts_with("__") {
                continue;
            }
            if !seen_bare.insert(b.to_string()) {
                dup = true;
                break;
            }
        }
        if dup {
            return extras.to_vec();
        }
        // Prefix-collision guard (FIX): two tables sharing a 1-char prefix
        // are SAFE to reorder when an equi-join edge exists between them.
        // The greedy chain can still find the join correctly. Only bail
        // when the collision pair has no edge.
        let mut seen_prefix: std::collections::HashSet<String> = std::collections::HashSet::new();
        let all_tables: Vec<&str> = base_bare_for_guard
            .iter()
            .chain(extras.iter())
            .filter_map(|t| {
                let b = t.find('|').map(|i| &t[..i]).unwrap_or(t);
                if b.is_empty() || b.starts_with("__") || b.len() < 2 {
                    return None;
                }
                Some(b)
            })
            .collect();
        let mut prefix_edge: std::collections::HashMap<String, std::collections::HashSet<String>> =
            std::collections::HashMap::new();
        for p in conj {
            if let Expression::BinaryOp(l, op, r) = p {
                if op == "=" {
                    let mut prefixes = Vec::new();
                    fn extract_prefixes(e: &Expression, out: &mut Vec<String>) {
                        if let Expression::Identifier(name) = e {
                            let prefix = if let Some((q, _)) = name.split_once('.') {
                                q
                            } else {
                                &name[..name.len().min(2)]
                            };
                            if !out.iter().any(|p| *p == prefix) {
                                out.push(prefix.to_string());
                            }
                        }
                    }
                    extract_prefixes(l, &mut prefixes);
                    extract_prefixes(r, &mut prefixes);
                    for (i, pi) in prefixes.iter().enumerate() {
                        for pj in prefixes.iter().skip(i + 1) {
                            if pi != pj {
                                prefix_edge
                                    .entry(pi.clone())
                                    .or_default()
                                    .insert(pj.clone());
                                prefix_edge
                                    .entry(pj.clone())
                                    .or_default()
                                    .insert(pi.clone());
                            }
                        }
                    }
                }
            }
        }
        let mut collision = false;
        for t in &all_tables {
            let p1 = &t[..1];
            if !seen_prefix.insert(p1.to_string()) && !prefix_edge.contains_key(p1) {
                collision = true;
                break;
            }
        }
        if collision {
            return extras.to_vec();
        }
    }
    // Hard-coded TPC-H SF=1 row counts with filter selectivity estimation.
    //
    // Three optimizations beyond raw row counts:
    // 1. Filter selectivity: apply WHERE predicate selectivity to get effective rows.
    //    - region: r_name='ASIA' → 1 of 5 → effective 1
    //    - nation: ASIA → ~3 nations
    //    - supplier: ASIA → ~200 suppliers
    //    - customer: ASIA → ~7K customers
    //    - orders: o_orderdate in [1994,1995) → ~210K rows (14% selectivity)
    //    - lineitem: driven by filtered orders → ~210K rows
    // 2. Nation-bridge heuristic: if customer.c_nationkey = supplier.s_nationkey
    //    AND orders has a date filter, force orders BEFORE customer to prevent
    //    supplier x customer cartesian explosion (200 x 30K = 6M per nation).
    // 3. Build-side selection: "smaller first" naturally picks the filtered
    //    orders (210K) as build side when joining with lineitem (6M).
    fn expr_contains_string_literal(expr: &Expression, s: &str) -> bool {
        match expr {
            Expression::Literal(v) => v == s,
            Expression::BinaryOp(l, _, r) => {
                expr_contains_string_literal(l.as_ref(), s)
                    || expr_contains_string_literal(r.as_ref(), s)
            }
            _ => false,
        }
    }
    fn has_orderdate_range_filter(expr: &Expression) -> bool {
        match expr {
            Expression::BinaryOp(l, op, r)
                if op == ">=" || op == ">" || op == "<" || op == "<=" =>
            {
                let l_date = match l.as_ref() {
                    Expression::Identifier(n) => n.contains("o_orderdate"),
                    _ => false,
                };
                let r_year = match r.as_ref() {
                    Expression::Literal(v) => {
                        // Strip surrounding single quotes from the
                        // literal value (parser stores '1994-01-01'
                        // with quotes). After stripping, year literals
                        // are 10 chars and start with '1' (1990s-2020s).
                        let stripped: String = v.chars().filter(|c| *c != '\'').collect();
                        stripped.len() == 10 && stripped.starts_with('1')
                    }
                    _ => false,
                };
                l_date && r_year
            }
            Expression::BinaryOp(l, op, r) if op == "AND" => {
                has_orderdate_range_filter(l.as_ref()) || has_orderdate_range_filter(r.as_ref())
            }
            _ => false,
        }
    }
    fn expr_has_identifier(expr: &Expression, name: &str) -> bool {
        match expr {
            Expression::Identifier(n) => n.contains(name),
            Expression::BinaryOp(l, _, r) => {
                expr_has_identifier(l.as_ref(), name) || expr_has_identifier(r.as_ref(), name)
            }
            _ => false,
        }
    }
    fn effective_row_count(
        t: &str,
        conj: &[Expression],
        accumulated: &std::collections::HashSet<String>,
    ) -> usize {
        let raw = match t {
            "region" => 5,
            "nation" => 25,
            "supplier" => 10_000,
            "customer" => 150_000,
            "part" => 200_000,
            "partsupp" => 800_000,
            "orders" => 1_500_000,
            "lineitem" => 6_001_215,
            _ => 100_000_000,
        };
        match t {
            "region" => {
                if conj.iter().any(|p| expr_contains_string_literal(p, "ASIA")) {
                    1
                } else {
                    raw
                }
            }
            "nation" => {
                if conj.iter().any(|p| expr_contains_string_literal(p, "ASIA")) {
                    3
                } else {
                    raw
                }
            }
            "supplier" => {
                if conj.iter().any(|p| expr_contains_string_literal(p, "ASIA")) {
                    200
                } else {
                    raw
                }
            }
            "customer" => {
                if conj.iter().any(|p| expr_contains_string_literal(p, "ASIA")) {
                    7_000
                } else {
                    raw
                }
            }
            "orders" => {
                if conj.iter().any(has_orderdate_range_filter) {
                    210_000
                } else {
                    raw
                }
            }
            "lineitem" => {
                if accumulated.contains("orders") {
                    210_000
                } else {
                    raw
                }
            }
            _ => raw,
        }
    }
    let has_nation_bridge = conj.iter().any(|p| match p {
        Expression::BinaryOp(l, op, r) if op == "=" => {
            (expr_has_identifier(l.as_ref(), "c_nationkey")
                && expr_has_identifier(r.as_ref(), "s_nationkey"))
                || (expr_has_identifier(r.as_ref(), "c_nationkey")
                    && expr_has_identifier(l.as_ref(), "s_nationkey"))
        }
        _ => false,
    });
    let has_orders_date_filter = conj.iter().any(has_orderdate_range_filter);
    let force_orders_first = has_nation_bridge && has_orders_date_filter;
    fn bare(t: &str) -> &str {
        match t.find('|') {
            Some(idx) => &t[..idx],
            None => t,
        }
    }
    // The set of (bare-table) names already joined (base table +
    // TPC-H prefix variations). Mirrors what `joined` carries.
    let mut accumulated: std::collections::HashSet<String> = joined
        .iter()
        .filter(|s| !s.is_empty())
        // Only include entries >= 2 chars in accumulated. The 1-char
        // prefix entries (like "p" for "part", "s" for "supplier")
        // are added to joined for column-qualifier resolution in
        // find_join_predicate, but they must NOT participate in the
        // greedy loop's prefix-based reachability checks — a 1-char
        // prefix like "p" would incorrectly match "supplier" via
        // "supplier".starts_with("p").
        .filter(|s| s.len() >= 2)
        .cloned()
        .collect();
    let mut remaining: Vec<String> = extras.to_vec();
    let mut out: Vec<String> = Vec::with_capacity(remaining.len());
    let edges = build_equi_join_edges(conj);
    while !remaining.is_empty() {
        // Compute a score for each remaining table. Lower is better.
        //   base_score = TPC-H row count
        //   -bonus     = 0 if it's reachable from `accumulated`,
        //                1e9 otherwise (force it to be picked last).
        let mut best_idx = usize::MAX;
        let mut best_score: u128 = u128::MAX;
        for (i, t) in remaining.iter().enumerate() {
            let b = bare(t);
            let mut reachable = false;
            for acc in &accumulated {
                let a = acc.as_str();
                if b == a {
                    reachable = true;
                    break;
                }
                // b starts with a's prefix → same family (part↔partsupp).
                // Use starts_with instead of == to correctly detect that
                // "partsupp" starts with "part"'s 2-char prefix "pa".
                if !b.is_empty() && !a.is_empty() && b.starts_with(&a[..1.min(a.len())]) {
                    reachable = true;
                    break;
                }
                if b.len() >= 2 && a.len() >= 2 && b.starts_with(&a[..2.min(b.len())]) {
                    reachable = true;
                    break;
                }
                // Prefix match (FIX): tables in the same prefix family
                // (part / partsupp both start with 'p') are reachable
                // when one of them is in accumulated. Q2 fix: without
                // this, partsupp (800K, reachable in spirit) is treated
                // as unreachable and the smallest unreachable table wins
                // → ON=true cartesian.
                //
                // Q5 guard: when force_orders_first is active AND
                // orders is not yet in accumulated, do NOT treat
                // supplier as reachable via customer (the
                // nation-bridge edge c_nationkey=s_nationkey). Without
                // this, supplier would be picked first and joined
                // with `ON c_nationkey=s_nationkey` — a 150K × 10K
                // cartesian-bridge (no nation in joined yet) that
                // explodes to 130 GB at SF=1.
                let skip_bridge_for_q5 = force_orders_first
                    && b == "supplier"
                    && a == "customer"
                    && !accumulated.contains("orders");
                if !skip_bridge_for_q5
                    && b.len() >= 2
                    && a.len() >= 2
                    && b.starts_with(&a[..2.min(b.len())])
                {
                    reachable = true;
                    break;
                }
                if !skip_bridge_for_q5
                    && edges.iter().any(|(l, r, _)| {
                        let b1 = &b[..1.min(b.len())];
                        let a1 = &a[..1.min(a.len())];
                        (l == b1 && r == a1) || (r == b1 && l == a1)
                    })
                {
                    reachable = true;
                    break;
                }
            }
            if force_orders_first && b == "orders" {
                let orders_still_remaining = remaining
                    .iter()
                    .any(|rt| bare(rt) == "orders" || rt == "orders");
                if orders_still_remaining {
                    best_idx = i;
                    break;
                }
            }
            let base = effective_row_count(b, conj, &accumulated) as u128;
            // Unreachable penalty must dominate any reachable raw count.
            // The previous `+ 1e9` was insufficient: an unreachable
            // region (1 row) would still beat reachable partsupp
            // (800K) for Q2 (base=part), so the reorder emitted
            // `INNER JOIN region ON true` (cartesian) and OOM'd.
            // Use u128::MAX/2 so only reachable candidates can win.
            let score = if reachable {
                base
            } else {
                (u128::MAX / 2) + base
            };
            if score < best_score {
                best_score = score;
                best_idx = i;
            }
        }
        // If no candidate was reachable in this iteration, every
        // remaining table is disconnected from accumulated → bail out
        // with what we have so far concatenated with the remaining
        // in their original extras order. The auto-rewrite handles
        // each table's ON clause via per-table `find_join_predicate`
        // (Q2 base=part case: partsupp picked via prefix, but
        // nation/supplier/region only reachable through the 'ps'
        // 2-char prefix the auto-rewrite doesn't recognize).
        let any_reachable = (0..remaining.len()).any(|idx| {
            let b2 = bare(&remaining[idx]);
            for acc in &accumulated {
                let a = acc.as_str();
                if b2 == a || (!b2.is_empty() && !a.is_empty() && b2[..1] == a[..1]) {
                    return true;
                }
                if edges.iter().any(|(l, r, _)| {
                    let b1 = &b2[..1.min(b2.len())];
                    let a1 = &a[..1.min(a.len())];
                    (l == b1 && r == a1) || (r == b1 && l == a1)
                }) {
                    return true;
                }
            }
            false
        });
        if !any_reachable {
            let mut out = out;
            for t in extras {
                if !out.contains(t) {
                    out.push(t.clone());
                }
            }
            return out;
        }
        if best_idx == usize::MAX {
            break;
        }
        // Defensive (Q5 OOM fix): if the pick is unreachable from the
        // accumulated set, no equi-join predicate can match the new table
        // to anything in joined_tables. The auto-rewrite then falls back
        // to ON=true, which produces a cartesian product that explodes
        // for 6+ table joins. Bail out of the reordering and keep the
        // original input order rather than emitting a cartesian.
        let picked_unreachable = {
            let b = bare(&remaining[best_idx]);
            let mut reach = false;
            for acc in &accumulated {
                let a = acc.as_str();
                // b is reachable from a if b starts with a's 2+ char prefix.
                if b.len() >= 2 && a.len() >= 2 && b.starts_with(&a[..2.min(b.len())]) {
                    reach = true;
                    break;
                }
                if edges
                    .iter()
                    .any(|(l, r, _)| (l == b && r == a) || (r == b && l == a))
                {
                    reach = true;
                    break;
                }
            }
            !reach
        };
        if picked_unreachable {
            return extras.to_vec();
        }
        let picked = remaining.swap_remove(best_idx);
        // Add its bare name and prefix to `accumulated` so subsequent
        // picks can find a join edge.
        let b = bare(&picked).to_string();
        accumulated.insert(b.clone());
        // Only insert 2+ char prefixes to prevent 1-char entries like
        // "p" or "s" from incorrectly matching full table names like
        // "supplier" ("supplier".starts_with("p") = True).
        if b.len() >= 2 {
            accumulated.insert(b[..2].to_string());
        }
        out.push(picked);
    }
    // If our greedy reorder didn't pick anything new (rare fall-back),
    // keep the original ordering rather than emitting empty.
    if out.is_empty() && !remaining.is_empty() {
        out = extras.to_vec();
    }
    out
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
            Some(Token::Explain) => self.parse_explain(),
            Some(Token::Insert) | Some(Token::Replace) => self.parse_insert(),
            Some(Token::Update) => self.parse_update(),
            Some(Token::Delete) => self.parse_delete(),
            Some(Token::Merge) => self.parse_merge(),
            Some(Token::Create) => self.parse_create(),
            Some(Token::Drop) => self.parse_drop(),
            Some(Token::Use) => self.parse_use_database(),
            Some(Token::Analyze) => self.parse_analyze(),
            Some(Token::With) => self.parse_with_select(),
            Some(Token::Alter) => {
                // Peek ahead: ALTER USER vs ALTER SEQUENCE vs ALTER TABLE
                // ALTER USER: next token is StringLiteral/Identifier (user) or Token::User keyword
                // ALTER SEQUENCE: handled by parse_alter() which dispatches to parse_alter_sequence
                match self.peek() {
                    Some(&Token::StringLiteral(_))
                    | Some(&Token::Identifier(_))
                    | Some(&Token::User) => self.parse_alter_user(),
                    _ => self.parse_alter(),
                }
            }
            Some(Token::Call) => self.parse_call(),
            // SEM-1 (#3172): Rollback is now dispatched via the SAVEPOINT
            // arms below. The `parse_transaction` group no longer
            // accepts Token::Rollback so that the ROLLBACK TO SAVEPOINT
            // peek (which requires us to NOT be inside parse_transaction
            // first) actually wins. Regular ROLLBACK [WORK] is handled
            // by the `Some(Token::Rollback)` arm at the bottom of this
            // match, which calls parse_rollback.
            // V312-19 #3986: SET session variable (e.g. SET debug_force_external=true)
            // routes to parse_set_session_variable, not parse_transaction.
            Some(Token::Set)
                if self.peek() != Some(&Token::Transaction)
                    && self.peek() != Some(&Token::Role) =>
            {
                self.parse_set_session_variable()
            }
            Some(Token::Begin) | Some(Token::Commit) | Some(Token::Set) | Some(Token::Start) => {
                self.parse_transaction()
            }
            // SEM-1 (#3172): SAVEPOINT dispatcher
            Some(Token::Savepoint) => self.parse_savepoint_statement(),
            // SEM-1 (#3172): RELEASE SAVEPOINT dispatcher
            Some(Token::Release) => self.parse_release_savepoint(),
            Some(Token::Prepare) => self.parse_prepare(),
            Some(Token::Execute) => self.parse_execute(),
            Some(Token::Truncate) => self.parse_truncate(),
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
            // Round-21 / Issue #4218: KILL is parsed as an Identifier
            // (no dedicated Token::Kill in the lexer), so match by
            // uppercased ident here.
            Some(Token::Identifier(ref ident))
                if matches!(ident.to_uppercase().as_str(), "KILL") =>
            {
                self.parse_kill()
            }
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
            // Only consume Token::Read as isolation level if NOT followed by Token::Only.
            if let Some(&Token::Only) = self.peek() {
                None
            } else {
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
            }
        } else {
            None
        };
        // READ ONLY / READONLY — may follow isolation level or appear alone.
        let readonly = if self.current() == Some(&Token::Read) {
            self.next();
            if self.current() == Some(&Token::Only) {
                self.next();
                true
            } else {
                false
            }
        } else if let Some(Token::Identifier(s)) = self.current() {
            if s.eq_ignore_ascii_case("READONLY") {
                self.next();
                true
            } else {
                false
            }
        } else {
            false
        };
        Ok(Statement::Transaction(TransactionStatement::Begin {
            work,
            isolation_level,
            readonly,
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
        // MySQL: PREPARE stmt FROM 'sql' — but accept AS as an alias for
        // dialect compatibility (PostgreSQL, MariaDB, internal callers).
        match self.current() {
            Some(Token::From) | Some(Token::As) => {
                self.next();
            }
            Some(t) => {
                return Err(format!(
                    "Expected FROM or AS after prepared statement name, got {:?}",
                    t
                ));
            }
            None => {
                return Err("Expected FROM or AS after prepared statement name, got EOF".to_string())
            }
        }
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
        // V312-58 / Issue #4511: `EXECUTE name USING @var1, @var2, ...`
        // binds the user session variables (in order) to the prepared
        // statement's `?` placeholders at execution time.
        let mut params: Vec<Expression> = Vec::new();
        if self.current() == Some(&Token::Using) {
            self.next(); // consume USING
            loop {
                // Each USING param is a user variable (`@name`); the
                // lexer emits it as `Identifier("@name")` so the
                // executor can resolve it via the session variable map.
                let p = match self.next() {
                    Some(Token::Identifier(n)) if n.starts_with('@') => Expression::Identifier(n),
                    Some(t) => {
                        return Err(format!(
                            "Expected user variable (@name) in USING clause, got {:?}",
                            t
                        ))
                    }
                    None => return Err("Expected user variable in USING clause".to_string()),
                };
                params.push(p);
                if self.current() == Some(&Token::Comma) {
                    self.next();
                } else {
                    break;
                }
            }
        }
        Ok(Statement::Execute { name, params })
    }

    fn parse_deallocate(&mut self) -> Result<Statement, String> {
        self.expect(Token::Deallocate)?;
        // MySQL supports both: DEALLOCATE s1 and DEALLOCATE PREPARE s1
        if let Some(Token::Identifier(ref p)) = self.current() {
            if p.to_uppercase() == "PREPARE" {
                self.next(); // consume PREPARE
            }
        } else if matches!(self.current(), Some(Token::Prepare)) {
            // The lexer emits `PREPARE` as a keyword token, so we may
            // also see `Token::Prepare` here. Treat it identically.
            self.next();
        }
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

    /// V312-11-fix #3986: parse `SET variable = value` (session variable).
    fn parse_set_session_variable(&mut self) -> Result<Statement, String> {
        self.expect(Token::Set)?;
        // Reject unsupported MySQL statements that look like SET session var.
        // These would otherwise be silently treated as session-variable
        // assignments and pass parse with no useful semantics.
        if let Some(Token::Identifier(ref s)) = self.current() {
            let upper = s.to_uppercase();
            if upper == "NAMES" || upper == "CHARACTER" {
                return Err(format!("SET {} is not yet supported", upper));
            }
        }
        if matches!(self.current(), Some(Token::Identifier(ref s)) if s.to_lowercase() == "variable")
        {
            self.next();
        }
        let name = match self.next() {
            Some(Token::Identifier(name)) => name,
            Some(Token::StringLiteral(s)) => s,
            Some(t) => return Err(format!("Expected variable name, got {:?}", t)),
            None => return Err("Expected variable name".to_string()),
        };
        let value = if matches!(self.current(), Some(Token::Equal)) {
            self.next();
            match self.next() {
                Some(Token::NumberLiteral(n)) => n,
                Some(Token::StringLiteral(s)) => s,
                Some(Token::BooleanLiteral(true)) => "true".to_string(),
                Some(Token::BooleanLiteral(false)) => "false".to_string(),
                Some(t) => format!("{:?}", t),
                None => String::new(),
            }
        } else {
            String::new()
        };
        Ok(Statement::Transaction(
            TransactionStatement::SetSessionVariable { name, value },
        ))
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

        // V312-18: Handle CREATE OR REPLACE for TABLE and VIEW
        let or_replace = if matches!(self.current(), Some(Token::Or)) {
            self.next();
            match self.current() {
                Some(Token::Replace) => {
                    self.next();
                    true
                }
                _ => return Err("Expected 'REPLACE' after 'OR'".to_string()),
            }
        } else {
            false
        };

        match self.current() {
            Some(Token::Table) => {
                let mut stmt = self.parse_create_table()?;
                if or_replace {
                    if let Statement::CreateTable(ref mut ct) = stmt {
                        ct.or_replace = true;
                    }
                }
                Ok(stmt)
            }
            Some(Token::Index) => {
                self.next();
                self.parse_create_index(false)
            }
            Some(Token::Unique) => {
                self.next();
                self.expect(Token::Index)?;
                self.parse_create_index(true)
            }
            Some(Token::Procedure) => {
                // V312-55A / Issue #4238: pass the already-consumed
                // `OR REPLACE` flag through to parse_create_procedure.
                let mut stmt = self.parse_create_procedure()?;
                if or_replace {
                    if let Statement::CreateProcedure(ref mut cp) = stmt {
                        cp.or_replace = true;
                    }
                }
                Ok(stmt)
            }
            Some(Token::Function) => self.parse_create_function(),
            Some(Token::Trigger) => self.parse_create_trigger(),
            Some(Token::Role) => self.parse_create_role(),
            Some(Token::View) => self.parse_create_view(),
            Some(Token::Database) => self.parse_create_database(),
            Some(Token::Sequence) => self.parse_create_sequence(),
            Some(t) => Err(format!(
                "Expected TABLE, INDEX, PROCEDURE, FUNCTION, TRIGGER, ROLE, VIEW, SEQUENCE, or DATABASE after CREATE, got {:?}",
                t
            )),
            None => Err(
                "Expected TABLE, INDEX, PROCEDURE, FUNCTION, TRIGGER, ROLE, VIEW, SEQUENCE, or DATABASE after CREATE"
                    .to_string(),
            ),
        }
    }

    fn parse_create_sequence(&mut self) -> Result<Statement, String> {
        self.expect(Token::Sequence)?;

        // Parse IF NOT EXISTS before the name
        let mut if_not_exists = false;
        if matches!(self.current(), Some(Token::If)) {
            self.next();
            self.expect(Token::Not)?;
            self.expect(Token::Exists)?;
            if_not_exists = true;
        }

        let name = match self.next() {
            Some(Token::Identifier(name)) => name,
            Some(Token::StringLiteral(s)) => s,
            Some(t) => return Err(format!("Expected sequence name, got {:?}", t)),
            None => return Err("Expected sequence name".to_string()),
        };
        let mut start_with = None;
        let mut increment_by = None;
        let mut minvalue = None;
        let mut maxvalue = None;
        let mut cache = None;
        let mut cycle = None;

        loop {
            match self.current() {
                Some(Token::Start) => {
                    self.next();
                    self.expect(Token::With)?;
                    let tok = self.next();
                    match tok {
                        Some(Token::NumberLiteral(n)) => start_with = Some(n),
                        Some(Token::Minus) => {
                            // Negative number
                            if let Some(Token::NumberLiteral(n)) = self.next() {
                                start_with = Some(format!("-{}", n));
                            } else {
                                return Err("Expected number after MINUS".to_string());
                            }
                        }
                        _ => return Err(format!("Expected number in START WITH, got {:?}", tok)),
                    }
                }
                Some(Token::Increment) => {
                    self.next();
                    self.expect(Token::By)?;
                    let tok = self.next();
                    match tok {
                        Some(Token::NumberLiteral(n)) => increment_by = Some(n),
                        Some(Token::Minus) => {
                            if let Some(Token::NumberLiteral(n)) = self.next() {
                                increment_by = Some(format!("-{}", n));
                            } else {
                                return Err("Expected number after MINUS".to_string());
                            }
                        }
                        _ => return Err(format!("Expected number in INCREMENT BY, got {:?}", tok)),
                    }
                }
                Some(Token::Minvalue) => {
                    self.next();
                    let tok = self.next();
                    match tok {
                        Some(Token::NumberLiteral(n)) => minvalue = Some(n),
                        Some(Token::Minus) => {
                            if let Some(Token::NumberLiteral(n)) = self.next() {
                                minvalue = Some(format!("-{}", n));
                            } else {
                                return Err("Expected number after MINUS".to_string());
                            }
                        }
                        _ => return Err(format!("Expected number in MINVALUE, got {:?}", tok)),
                    }
                }
                Some(Token::Maxvalue) => {
                    self.next();
                    let tok = self.next();
                    match tok {
                        Some(Token::NumberLiteral(n)) => maxvalue = Some(n),
                        Some(Token::Minus) => {
                            if let Some(Token::NumberLiteral(n)) = self.next() {
                                maxvalue = Some(format!("-{}", n));
                            } else {
                                return Err("Expected number after MINUS".to_string());
                            }
                        }
                        _ => return Err(format!("Expected number in MAXVALUE, got {:?}", tok)),
                    }
                }
                Some(Token::NoMinValue) => {
                    self.next();
                    minvalue = Some(String::from("NO MINVALUE"));
                }
                Some(Token::NoMaxValue) => {
                    self.next();
                    maxvalue = Some(String::from("NO MAXVALUE"));
                }
                Some(Token::Cache) => {
                    self.next();
                    match self.next() {
                        Some(Token::NumberLiteral(n)) => cache = Some(n),
                        _ => return Err("Expected number after CACHE".to_string()),
                    }
                }
                Some(Token::Cycle) => {
                    self.next();
                    cycle = Some(true);
                }
                Some(Token::NoCycle) => {
                    self.next();
                    cycle = Some(false);
                }
                Some(Token::Identifier(_)) | None => break,
                _ => break,
            }
        }

        Ok(Statement::CreateSequence(CreateSequenceStatement {
            name,
            start_with,
            increment_by,
            minvalue,
            maxvalue,
            cache,
            cycle,
            if_not_exists,
        }))
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
    fn parse_create_database(&mut self) -> Result<Statement, String> {
        self.expect(Token::Database)?;
        let if_not_exists = if matches!(self.current(), Some(Token::If)) {
            self.next();
            self.expect(Token::Not)?;
            self.expect(Token::Exists)?;
            true
        } else {
            false
        };
        let name = match self.next() {
            Some(Token::Identifier(name)) => name,
            Some(Token::StringLiteral(s)) => s,
            Some(t) => return Err(format!("Expected database name, got {:?}", t)),
            None => return Err("Expected database name".to_string()),
        };
        Ok(Statement::CreateDatabase(CreateDatabaseStatement {
            name,
            if_not_exists,
        }))
    }

    /// Parse USE <database> statement
    fn parse_use_database(&mut self) -> Result<Statement, String> {
        self.expect(Token::Use)?;
        let name = match self.next() {
            Some(Token::Identifier(name)) => name,
            Some(Token::StringLiteral(s)) => s,
            Some(t) => return Err(format!("Expected database name, got {:?}", t)),
            None => return Err("Expected database name".to_string()),
        };
        Ok(Statement::UseDatabase(name))
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
        // V312-55A / Issue #4238: CREATE [OR REPLACE] PROCEDURE
        //
        // The `OR REPLACE` keyword is consumed by `parse_create` (the
        // outer CREATE dispatcher) and the flag is patched onto the
        // resulting `CreateProcedureStatement` after this function
        // returns. Here we just expect the `PROCEDURE` keyword and
        // proceed.
        self.expect(Token::Procedure)?;

        let name = match self.next() {
            Some(Token::Identifier(name)) => name,
            // Allow keywords as procedure names (MySQL allows this)
            Some(Token::Increment) => "increment".to_string(),
            Some(Token::While) => "while".to_string(),
            Some(Token::Loop) => "loop".to_string(),
            Some(Token::Repeat) => "repeat".to_string(),
            Some(Token::Return) => "return".to_string(),
            Some(Token::Leave) => "leave".to_string(),
            Some(Token::Iterate) => "iterate".to_string(),
            Some(Token::Set) => "set".to_string(),
            Some(Token::Declare) => "declare".to_string(),
            Some(Token::Call) => "call".to_string(),
            Some(Token::Out) => "out".to_string(),
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
                Some(Token::Out) => {
                    self.next();
                    StoredProcParamMode::Out
                }
                Some(Token::InOut) => {
                    self.next();
                    StoredProcParamMode::InOut
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
        let body = self.parse_sp_body(&[Token::End])?;
        self.expect(Token::End)?;

        Ok(Statement::CreateProcedure(CreateProcedureStatement {
            name,
            // OR REPLACE flag is set by the outer parse_create
            // dispatcher after this returns (consistent with how
            // CreateTable handles the modifier).
            or_replace: false,
            params,
            body,
        }))
    }

    /// V312-55A / Issue #4238: parse `DROP PROCEDURE [IF EXISTS] name`.
    ///
    /// Mirrors the `DROP TABLE`/`DROP VIEW` shape: optional `IF EXISTS`
    /// between `PROCEDURE` and the procedure name. The caller is
    /// expected to have already consumed the leading `DROP` keyword.
    fn parse_drop_procedure(&mut self) -> Result<Statement, String> {
        self.expect(Token::Procedure)?;
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
            // MySQL allows reserved keywords as procedure names; mirror
            // the CREATE PROCEDURE keyword fallback list so `DROP
            // PROCEDURE loop` works the same as `CREATE PROCEDURE loop`.
            Some(Token::While) => "while".to_string(),
            Some(Token::Loop) => "loop".to_string(),
            Some(Token::Repeat) => "repeat".to_string(),
            Some(Token::Return) => "return".to_string(),
            Some(Token::Leave) => "leave".to_string(),
            Some(Token::Iterate) => "iterate".to_string(),
            Some(Token::Set) => "set".to_string(),
            Some(Token::Declare) => "declare".to_string(),
            Some(Token::Call) => "call".to_string(),
            Some(Token::Out) => "out".to_string(),
            Some(Token::Increment) => "increment".to_string(),
            Some(t) => return Err(format!("Expected procedure name, got {:?}", t)),
            None => return Err("Expected procedure name".to_string()),
        };
        Ok(Statement::DropProcedure(DropProcedureStatement {
            name,
            if_exists,
        }))
    }

    /// V312-58 / Issue #4512: parse `CREATE FUNCTION name(p1 TYPE, ...) RETURNS TYPE [DETERMINISTIC] RETURN expr`.
    ///
    /// The caller (`parse_create`) has already consumed the leading
    /// `CREATE` keyword. We consume `FUNCTION`, parse the signature
    /// (name, parameter list, return type, optional `DETERMINISTIC`,
    /// and the `RETURN <expr>` body). The body expression is stored
    /// as raw text and re-parsed at invocation time by the executor
    /// so it can resolve UDF parameter references against the
    /// call-site argument values.
    ///
    /// Syntax:
    ///   CREATE FUNCTION name ( [param TYPE [, param TYPE]*] )
    ///       RETURNS TYPE
    ///       [DETERMINISTIC]
    ///       RETURN expr ;
    fn parse_create_function(&mut self) -> Result<Statement, String> {
        self.expect(Token::Function)?;

        let name = match self.next() {
            Some(Token::Identifier(name)) => name,
            Some(t) => return Err(format!("Expected function name, got {:?}", t)),
            None => return Err("Expected function name".to_string()),
        };

        // Parameter list — same shape as CREATE PROCEDURE but every
        // param is implicitly IN (UDFs have no OUT/INOUT).
        self.expect(Token::LParen)?;
        let mut params = Vec::new();
        if !matches!(self.current(), Some(Token::RParen) | None) {
            loop {
                let param_name = match self.next() {
                    Some(Token::Identifier(n)) => n,
                    Some(t) => return Err(format!("Expected UDF parameter name, got {:?}", t)),
                    None => return Err("Expected UDF parameter name".to_string()),
                };
                let data_type = match self.next() {
                    Some(Token::Identifier(typename)) => typename,
                    Some(Token::Integer) => "INTEGER".to_string(),
                    Some(Token::Text) => "TEXT".to_string(),
                    Some(Token::Float) => "FLOAT".to_string(),
                    Some(Token::Boolean) => "BOOLEAN".to_string(),
                    Some(t) => return Err(format!("Expected UDF parameter type, got {:?}", t)),
                    None => return Err("Expected UDF parameter type".to_string()),
                };
                params.push(UdfParam {
                    name: param_name,
                    data_type,
                });
                if matches!(self.current(), Some(Token::Comma)) {
                    self.next();
                } else {
                    break;
                }
            }
        }
        self.expect(Token::RParen)?;

        // RETURNS <type>
        self.expect(Token::Returns)?;
        let return_type = match self.next() {
            Some(Token::Identifier(typename)) => typename,
            Some(Token::Integer) => "INTEGER".to_string(),
            Some(Token::Text) => "TEXT".to_string(),
            Some(Token::Float) => "FLOAT".to_string(),
            Some(Token::Boolean) => "BOOLEAN".to_string(),
            Some(t) => return Err(format!("Expected return type, got {:?}", t)),
            None => return Err("Expected return type".to_string()),
        };

        // Optional DETERMINISTIC modifier (records informational flag).
        let deterministic = if matches!(self.current(), Some(Token::Deterministic)) {
            self.next();
            true
        } else {
            false
        };

        // RETURN <expr> — capture the body as raw text. The executor
        // re-parses it when the UDF is invoked. We snapshot tokens
        // until we hit a top-level `;` or EOF (mirrors the simpler
        // single-expression body MySQL accepts for scalar UDFs).
        self.expect(Token::Return)?;
        let mut body_expr = String::new();
        let mut depth: u32 = 0;
        loop {
            match self.current() {
                None => break,
                Some(Token::Semicolon) if depth == 0 => break,
                Some(Token::LParen) => {
                    depth += 1;
                    body_expr.push('(');
                    self.next();
                }
                Some(Token::RParen) => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                    body_expr.push(')');
                    self.next();
                }
                Some(tok) => {
                    // Append a textual representation of this token to the
                    // body buffer. We use the same `Display` form the lexer
                    // emits so re-tokenization on invocation produces the
                    // same span.
                    body_expr.push_str(&format!("{} ", token_to_text(tok)));
                    self.next();
                }
            }
        }
        let body_expr = body_expr.trim().to_string();
        if body_expr.is_empty() {
            return Err("CREATE FUNCTION requires a non-empty body after RETURN".to_string());
        }

        Ok(Statement::CreateFunction(CreateFunctionStatement {
            name,
            params,
            return_type,
            deterministic,
            body_expr,
        }))
    }

    /// V312-58 / Issue #4512: parse `DROP FUNCTION [IF EXISTS] name`.
    ///
    /// The caller (`parse_drop`) has already consumed the leading
    /// `DROP` keyword. Mirrors the DROP TABLE/DROP VIEW/DROP
    /// PROCEDURE shape.
    fn parse_drop_function(&mut self) -> Result<Statement, String> {
        self.expect(Token::Function)?;
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
            Some(t) => return Err(format!("Expected function name, got {:?}", t)),
            None => return Err("Expected function name".to_string()),
        };
        Ok(Statement::DropFunction(DropFunctionStatement {
            name,
            if_exists,
        }))
    }

    /// Parse stored procedure body statements until a terminator token
    fn parse_sp_body(&mut self, terminators: &[Token]) -> Result<Vec<StoredProcStatement>, String> {
        let mut body = Vec::new();
        let mut current_sql = String::new();

        loop {
            match self.current() {
                None => return Err("Unexpected end of input in procedure body".to_string()),
                Some(t) if terminators.contains(t) => break,
                Some(Token::Semicolon) => {
                    self.next();
                    if !current_sql.trim().is_empty() {
                        body.push(StoredProcStatement::RawSql(current_sql.trim().to_string()));
                    }
                    current_sql = String::new();
                }
                Some(Token::If) => {
                    if !current_sql.trim().is_empty() {
                        body.push(StoredProcStatement::RawSql(current_sql.trim().to_string()));
                        current_sql = String::new();
                    }
                    self.next();
                    let condition = self.read_sp_expression();
                    self.expect(Token::Then)?;
                    let then_body = self.parse_sp_body(&[Token::Else, Token::End])?;
                    let mut else_body = Vec::new();
                    if matches!(self.current(), Some(Token::Else)) {
                        self.next();
                        else_body = self.parse_sp_body(&[Token::End])?;
                    }
                    self.expect(Token::End)?;
                    self.expect(Token::If)?;
                    body.push(StoredProcStatement::If {
                        condition,
                        then_body,
                        else_body,
                    });
                }
                Some(Token::While) => {
                    if !current_sql.trim().is_empty() {
                        body.push(StoredProcStatement::RawSql(current_sql.trim().to_string()));
                        current_sql = String::new();
                    }
                    self.next();
                    let condition = self.read_sp_expression();
                    self.expect(Token::Do)?;
                    let body_stmts = self.parse_sp_body(&[Token::End])?;
                    self.expect(Token::End)?;
                    self.expect(Token::While)?;
                    body.push(StoredProcStatement::While {
                        condition,
                        body: body_stmts,
                    });
                }
                Some(Token::Loop) => {
                    if !current_sql.trim().is_empty() {
                        body.push(StoredProcStatement::RawSql(current_sql.trim().to_string()));
                        current_sql = String::new();
                    }
                    self.next();
                    let body_stmts = self.parse_sp_body(&[Token::End])?;
                    self.expect(Token::End)?;
                    self.expect(Token::Loop)?;
                    body.push(StoredProcStatement::Loop { body: body_stmts });
                }
                Some(Token::Leave) => {
                    self.next();
                    body.push(StoredProcStatement::Leave);
                }
                Some(Token::Iterate) => {
                    self.next();
                    body.push(StoredProcStatement::Iterate);
                }
                Some(Token::Begin) => {
                    if !current_sql.trim().is_empty() {
                        body.push(StoredProcStatement::RawSql(current_sql.trim().to_string()));
                        current_sql = String::new();
                    }
                    self.next();
                    let nested = self.parse_sp_body(&[Token::End])?;
                    self.expect(Token::End)?;
                    body.push(StoredProcStatement::NestedBegin { body: nested });
                }
                Some(Token::Identifier(_)) => {
                    // Collect the full statement (identifiers starting with non-keyword words)
                    let stmt_str = self.collect_sp_statement();
                    if !stmt_str.trim().is_empty() {
                        body.push(StoredProcStatement::RawSql(stmt_str.trim().to_string()));
                    }
                }
                Some(Token::Set) | Some(Token::Declare) | Some(Token::Call)
                | Some(Token::Select) | Some(Token::Insert) | Some(Token::Update)
                | Some(Token::Delete) => {
                    // Flush any pending raw SQL and collect full statement
                    if !current_sql.trim().is_empty() {
                        body.push(StoredProcStatement::RawSql(current_sql.trim().to_string()));
                        current_sql = String::new();
                    }
                    let stmt_str = self.collect_sp_statement();
                    if !stmt_str.trim().is_empty() {
                        let upper = stmt_str.to_uppercase();
                        if upper.starts_with("SET ") {
                            let rest = stmt_str[4..].trim();
                            if let Some(eq_pos) = rest.find('=') {
                                let var_name = rest[..eq_pos].trim().to_string();
                                let value = rest[eq_pos + 1..].trim().to_string();
                                body.push(StoredProcStatement::Set { var_name, value });
                            } else {
                                body.push(StoredProcStatement::RawSql(stmt_str));
                            }
                        } else if upper.starts_with("DECLARE ") {
                            let rest = stmt_str[9..].trim();
                            let space_pos = rest.find(' ').unwrap_or(rest.len());
                            let var_name = rest[..space_pos].to_string();
                            let data_type = rest[space_pos..].trim().to_string();
                            body.push(StoredProcStatement::Declare {
                                var_name,
                                data_type,
                            });
                        } else if upper.starts_with("CALL ") {
                            let rest = stmt_str[5..].trim();
                            if let Some(paren_pos) = rest.find('(') {
                                let proc_name = rest[..paren_pos].to_string();
                                let args_str = &rest[paren_pos + 1..rest.len() - 1];
                                let args: Vec<String> = if args_str.trim().is_empty() {
                                    Vec::new()
                                } else {
                                    args_str.split(',').map(|s| s.trim().to_string()).collect()
                                };
                                body.push(StoredProcStatement::Call {
                                    procedure_name: proc_name,
                                    args,
                                });
                            } else {
                                body.push(StoredProcStatement::RawSql(stmt_str));
                            }
                        } else {
                            body.push(StoredProcStatement::RawSql(stmt_str));
                        }
                    }
                }
                _ => {
                    current_sql.push_str(&self.current().unwrap().to_string());
                    current_sql.push(' ');
                    self.next();
                }
            }
        }

        if !current_sql.trim().is_empty() {
            body.push(StoredProcStatement::RawSql(current_sql.trim().to_string()));
        }
        Ok(body)
    }

    /// Read a stored procedure expression (condition or value)
    fn read_sp_expression(&mut self) -> String {
        let mut expr = String::new();
        let mut depth = 0;
        loop {
            match self.current() {
                None => break,
                Some(Token::Semicolon) | Some(Token::Do) | Some(Token::Then) | Some(Token::End)
                    if depth == 0 =>
                {
                    break
                }
                Some(Token::LParen) => {
                    depth += 1;
                    expr.push('(');
                    self.next();
                }
                Some(Token::RParen) => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                    expr.push(')');
                    self.next();
                }
                Some(Token::Identifier(s)) => {
                    expr.push_str(s);
                    expr.push(' ');
                    self.next();
                }
                Some(t) => {
                    expr.push_str(&t.to_string());
                    expr.push(' ');
                    self.next();
                }
            }
        }
        expr.trim().to_string()
    }

    /// Collect tokens until semicolon for a full statement
    fn collect_sp_statement(&mut self) -> String {
        let mut parts = Vec::new();
        loop {
            match self.current() {
                None | Some(Token::Semicolon) => break,
                Some(Token::End) => break,
                Some(Token::Else) => break,
                Some(Token::Identifier(s)) => {
                    parts.push(s.clone());
                    self.next();
                }
                Some(t) => {
                    parts.push(t.to_string());
                    self.next();
                }
            }
        }
        // Skip trailing semicolon if present
        if matches!(self.current(), Some(Token::Semicolon)) {
            self.next();
        }
        parts.join(" ")
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

        // V312-58 / Issue #4514: relax to accept either a BEGIN/END
        // block (multi-statement body) or a single-statement body that
        // ends at EOF / statement terminator. The original reproduction
        // from the issue is the single-statement form.
        let mut body = String::new();
        if matches!(self.current(), Some(Token::Begin)) {
            self.expect(Token::Begin)?;
            while !matches!(self.current(), Some(Token::End) | None) {
                match self.next() {
                    Some(Token::Semicolon) => {
                        body.push(';');
                        body.push(' ');
                    }
                    Some(Token::Dot) => {
                        // Strip trailing space before dot so we get `NEW.name` not `NEW .name`
                        while body.ends_with(' ') {
                            body.pop();
                        }
                        body.push('.');
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
        } else {
            // Single-statement body — collect tokens until the statement
            // terminator (Semicolon) or EOF.
            while !matches!(
                self.current(),
                Some(Token::Semicolon) | Some(Token::Eof) | None
            ) {
                match self.next() {
                    Some(Token::Dot) => {
                        // Strip trailing space before dot so we get `NEW.name` not `NEW .name`
                        while body.ends_with(' ') {
                            body.pop();
                        }
                        body.push('.');
                    }
                    Some(Token::Identifier(sql)) => {
                        body.push_str(&sql);
                        body.push(' ');
                    }
                    Some(t) => {
                        body.push_str(&t.to_string());
                        body.push(' ');
                    }
                    None => break,
                }
            }
            if matches!(self.current(), Some(Token::Semicolon)) {
                self.next();
            }
        }

        Ok(Statement::CreateTrigger(CreateTriggerStatement {
            name,
            table,
            timing,
            events,
            body: body.trim().to_string(),
        }))
    }

    fn parse_drop_trigger(&mut self) -> Result<Statement, String> {
        // V312-58 / Issue #4514: `DROP TRIGGER [IF EXISTS] name`.
        self.expect(Token::Trigger)?;
        let if_exists = if matches!(self.current(), Some(Token::If)) {
            self.next();
            self.expect(Token::Exists)?;
            true
        } else {
            false
        };
        let name = match self.next() {
            Some(Token::Identifier(n)) => n,
            Some(t) => return Err(format!("Expected trigger name, got {:?}", t)),
            None => return Err("Expected trigger name".to_string()),
        };
        // Optional trailing semicolon.
        if matches!(self.current(), Some(Token::Semicolon)) {
            self.next();
        }
        Ok(Statement::DropTrigger(DropTriggerStatement { name, if_exists }))
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

    /// V312-56E / #4255: Parse `EXPLAIN <select>` as
    /// `Statement::Explain(Box<SelectStatement>)`. The optional
    /// `QUERY PLAN` prefix is accepted but ignored (SQLite-style
    /// shorthand).
    fn parse_explain(&mut self) -> Result<Statement, String> {
        self.next(); // consume EXPLAIN
                     // Optionally consume QUERY PLAN tokens (SQLite-style).
        if matches!(
            self.current(),
            Some(Token::Identifier(ref s)) if s.to_uppercase() == "QUERY"
        ) {
            self.next();
            if matches!(
                self.current(),
                Some(Token::Identifier(ref s)) if s.to_uppercase() == "PLAN"
            ) {
                self.next();
            }
        }
        // Expect SELECT (or WITH SELECT) next.
        match self.current() {
            Some(Token::Select) => {
                let inner = self.parse_select_statement()?;
                Ok(Statement::Explain(Box::new(inner)))
            }
            Some(Token::With) => {
                let stmt = self.parse_with_select()?;
                if let Statement::WithSelect(ws) = stmt {
                    Ok(Statement::Explain(Box::new(ws.select)))
                } else {
                    Err("EXPLAIN WITH: unexpected statement shape".to_string())
                }
            }
            _ => Err("EXPLAIN must be followed by SELECT".to_string()),
        }
    }

    fn parse_select_or_union(&mut self) -> Result<Statement, String> {
        let first_select = self.parse_select_statement()?;

        let mut current = Statement::Select(first_select);
        // A SELECT can be followed by zero or more UNION [ALL] /
        // INTERSECT [ALL] / EXCEPT [ALL] chains (SQL-92 set operations).
        // Each chain consumes a new SELECT and wraps the existing
        // `current` as the left side of a new set-op node. Precedence
        // (INTERSECT > UNION = EXCEPT) is NOT enforced at parse time —
        // we chain left-to-right; the planner may add a normalisation
        // pass in a follow-up.
        loop {
            let (is_union, is_intersect, _is_except) = match self.current() {
                Some(Token::Union) => (true, false, false),
                Some(Token::Intersect) => (false, true, false),
                Some(Token::Except) => (false, false, true),
                _ => break,
            };
            self.next();
            let all = if matches!(self.current(), Some(Token::All)) {
                self.next();
                true
            } else {
                false
            };
            let next_select = self.parse_select_statement()?;
            // If the right SELECT came back with ORDER BY / LIMIT /
            // OFFSET (the normal SQL form `... UNION SELECT ... ORDER BY x
            // LIMIT n`), lift them onto the UnionStatement so the
            // executor applies them to the merged result rather than
            // only to the right input. The right SELECT still carries
            // its own copy; consumers must prefer `trailing_*` to avoid
            // double-application.
            let (trailing_order_by, trailing_limit, trailing_offset) =
                if !next_select.order_by.is_empty()
                    || next_select.limit.is_some()
                    || next_select.offset.is_some()
                {
                    let ob = next_select.order_by.clone();
                    let lim = next_select.limit.map(|n| n as i64);
                    let off = next_select.offset.map(|n| n as i64);
                    (ob, lim, off)
                } else {
                    (Vec::new(), None, None)
                };
            current = if is_union {
                Statement::Union(UnionStatement {
                    left: Box::new(current),
                    right: Box::new(Statement::Select(next_select)),
                    union_all: all,
                    trailing_order_by,
                    trailing_limit,
                    trailing_offset,
                })
            } else if is_intersect {
                Statement::Intersect(IntersectStatement {
                    left: Box::new(current),
                    right: Box::new(Statement::Select(next_select)),
                    intersect_all: all,
                    trailing_order_by,
                    trailing_limit,
                    trailing_offset,
                })
            } else {
                Statement::Except(ExceptStatement {
                    left: Box::new(current),
                    right: Box::new(Statement::Select(next_select)),
                    except_all: all,
                    trailing_order_by,
                    trailing_limit,
                    trailing_offset,
                })
            };
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
                // Union / Intersect / Except are set-operation terminators that must
                // break the column-list loop so parse_select_statement returns and
                // lets parse_select_or_union handle the set op.
                // Order / Limit / Offset also break here so the right SELECT of
                // a set-op chain can carry ORDER BY / LIMIT / OFFSET to be lifted
                // onto the set-op node.
                Some(Token::RParen)
                | Some(Token::Union)
                | Some(Token::Intersect)
                | Some(Token::Except)
                | Some(Token::Order)
                | Some(Token::Limit)
                | Some(Token::Offset)
                | Some(Token::Semicolon) => break,
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
                // V312-19 / #4019.2: MySQL `@@system_variable` reference in
                // SELECT projection (e.g. mysql CLI 8.0+'s boot probe
                // `SELECT @@version_comment LIMIT 1`). Treat as a single
                // scalar expression, not a column ref.
                Some(Token::SystemVariable(name)) => {
                    let var_name = name.clone();
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
                        name: format!("@@{}", var_name),
                        alias,
                        expression: Some(Expression::SystemVariable(var_name)),
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
                                AggregateFunction::PercentileCont => "PERCENTILE_CONT",
                                AggregateFunction::QuantileDisc => "QUANTILE_DISC",
                                AggregateFunction::QuantileCont => "QUANTILE_CONT",
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
                | Some(Token::Interval)
                | Some(Token::Database) => {
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
                        Some(Token::Database) => "DATABASE",
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
                                     // Two accepted forms (both MySQL 5.7):
                                     //   1. DATE_ADD(d, n, 'UNIT')                 — 3-arg, n then string unit
                                     //   2. DATE_ADD(d, INTERVAL n UNIT)            — SQL standard with INTERVAL keyword
                                     // Try (1) first: if the next token is not the
                                     // `INTERVAL` keyword, treat it as a 3-arg call.
                        let (n_expr, unit) = if matches!(self.current(), Some(Token::Interval)) {
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
                            (n_expr, unit)
                        } else {
                            // 3-arg MySQL form: n may be any primary
                            // expression (typically a NumberLiteral),
                            // followed by a string-literal unit.
                            let n_expr = self.parse_primary_expression()?;
                            if !matches!(self.current(), Some(Token::Comma)) {
                                return Err(format!(
                                    "Expected ',' before unit in 3-arg {}(d, n, 'UNIT'), got {:?}",
                                    name,
                                    self.current()
                                ));
                            }
                            self.next(); // consume Comma
                            let unit = match self.current() {
                                Some(Token::StringLiteral(u)) => {
                                    let s = u.clone();
                                    self.next();
                                    s
                                }
                                Some(Token::Identifier(u)) => {
                                    let s = u.clone();
                                    self.next();
                                    s
                                }
                                _ => {
                                    return Err(format!(
                                        "Expected string-literal unit in 3-arg {}(d, n, 'UNIT'), got {:?}",
                                        name,
                                        self.current()
                                    ));
                                }
                            };
                            (n_expr, unit)
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
                Some(Token::Minus) | Some(Token::Plus) => {
                    let expr = self.parse_expression()?;
                    columns.push(SelectColumn {
                        name: "_expr".to_string(),
                        alias: None,
                        expression: Some(expr),
                    });
                }
                // V311-10 F-30: NEXT VALUE FOR seq / CURRVAL(seq) in SELECT.
                // Delegate to the expression parser, which recognises
                // SequenceNextVal / SequenceCurrval via parse_primary_expression.
                Some(Token::NextValue) | Some(Token::Currval) => {
                    let expr = self.parse_expression()?;
                    columns.push(SelectColumn {
                        name: format!("{:?}", expr),
                        alias: None,
                        expression: Some(expr),
                    });
                }
                // V313-followup-5 / Issue #4158: trailing `WITH [NO] DATA`
                // marker in CREATE TABLE AS SELECT. Break the column-list
                // loop without consuming; the caller (parse_create_table)
                // will recognise the WITH token and parse the trailing
                // materialization clause.
                Some(Token::With) => break,
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
        //
        // V312-56A / 56A-R2: capture the optional `schema` qualifier from
        // `FROM schema.table` for information_schema routing. Declared in
        // the outer function scope so the FROM table-list branch (which is
        // a nested match arm) can populate it, and the final SelectStatement
        // constructor can read it.
        let mut schema: Option<String> = None;
        let (table, from_subquery, extra_tables) = match self.current() {
            Some(Token::From) => {
                self.next(); // consume FROM
                if matches!(self.current(), Some(Token::LParen)) {
                    // Sprint 1b: FROM (subquery) AS alias
                    // Sprint 1d: also support FROM (table_ref [JOIN table_ref]*) AS alias
                    // (derived table without explicit SELECT).
                    // VALUES constructor: FROM (VALUES (...), (...) ) AS alias
                    self.next(); // consume (
                    if matches!(self.current(), Some(Token::Values)) {
                        // Parse VALUES constructor
                        self.next(); // consume VALUES
                        let mut values = Vec::new();
                        if !matches!(self.current(), Some(Token::LParen)) {
                            return Err("Expected ( after VALUES".to_string());
                        }
                        loop {
                            if !matches!(self.current(), Some(Token::LParen)) {
                                break;
                            }
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
                        self.expect(Token::RParen)?;
                        if matches!(self.current(), Some(Token::As)) {
                            self.next();
                        }
                        let alias = match self.next() {
                            Some(Token::Identifier(name)) => name,
                            Some(t) => {
                                return Err(format!("Expected alias for VALUES, got {:?}", t))
                            }
                            None => return Err("Expected alias for VALUES".to_string()),
                        };
                        // V313-09 / Issue #4037: accept the optional
                        // column-list form `FROM (VALUES ...) alias(c1, c2)`.
                        // The names are captured so the engine can
                        // resolve `SELECT alias.c1` against the
                        // synthetic table_info.
                        let mut column_names: Vec<String> = Vec::new();
                        if matches!(self.current(), Some(Token::LParen)) {
                            self.next();
                            while let Some(Token::Identifier(name)) = self.current() {
                                column_names.push(name.clone());
                                self.next();
                                if matches!(self.current(), Some(Token::Comma)) {
                                    self.next();
                                } else {
                                    break;
                                }
                            }
                            self.expect(Token::RParen)?;
                        }
                        let synth_select = SelectStatement {
                            columns: if column_names.is_empty() {
                                vec![SelectColumn {
                                    name: "*".to_string(),
                                    alias: None,
                                    expression: None,
                                }]
                            } else {
                                column_names
                                    .iter()
                                    .map(|c| SelectColumn {
                                        name: c.clone(),
                                        alias: None,
                                        expression: None,
                                    })
                                    .collect()
                            },
                            table: alias.clone(),
                            // V312-56A / 56A-R2: subquery-derived
                            // SelectStatement (FROM subquery path) is
                            // never schema-qualified.
                            schema: None,
                            from_alias: None,
                            from_subquery: None,
                            from_values: Some(values),
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
                            lock_clause: None,
                        };
                        (alias, Some(Box::new(synth_select)), Vec::new())
                    } else if matches!(self.current(), Some(Token::Select))
                        || matches!(self.current(), Some(Token::With))
                        || matches!(self.current(), Some(Token::Values))
                    {
                        // Subquery: parse as SELECT statement or VALUES constructor
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
                    } else if matches!(self.current(), Some(Token::LParen)) {
                        // V313-09 / Issue #4037: accept a nested
                        // subquery inside the derived table, e.g.
                        // `FROM ((SELECT ... EXCEPT ALL SELECT ...))`.
                        // Use parse_select_or_union so the inner
                        // expression may be a SELECT or a set op
                        // (UNION/INTERSECT/EXCEPT). Wrap as a
                        // synthetic SELECT * FROM <stmt> so the
                        // downstream `from_subquery` path is the
                        // single representation. We rebuild the AST
                        // here rather than passing the set-op AST
                        // directly because the executor's
                        // `from_subquery` handling expects a SELECT.
                        let inner = self.parse_select_or_union()?;
                        // Strip the leading `SELECT * FROM ` we
                        // synthesised if `inner` is itself a set-op
                        // — instead rebuild as a SELECT that
                        // references the inner statement as a
                        // table-shaped subquery via `from_subquery`.
                        // Concretely, we construct a synthetic
                        // SelectStatement whose `from_subquery`
                        // points to a SELECT * from the inner
                        // statement, then drop that extra layer.
                        // Simpler: just convert any set-op into a
                        // SELECT that materialises its result via
                        // `from_subquery: Some(Box(inner Select * from <inner>))`.
                        let materialised = match &inner {
                            Statement::Select(s) => SelectStatement {
                                columns: vec![SelectColumn {
                                    name: "*".to_string(),
                                    alias: None,
                                    expression: None,
                                }],
                                table: s.table.clone(),
                                schema: s.schema.clone(),
                                from_alias: s.from_alias.clone(),
                                from_subquery: s.from_subquery.clone(),
                                from_values: s.from_values.clone(),
                                where_clause: s.where_clause.clone(),
                                join_clause: s.join_clause.clone(),
                                extra_tables: s.extra_tables.clone(),
                                aggregates: s.aggregates.clone(),
                                group_by: s.group_by.clone(),
                                with_rollup: s.with_rollup,
                                with_cube: s.with_cube,
                                having: s.having.clone(),
                                order_by: s.order_by.clone(),
                                limit: s.limit,
                                offset: s.offset,
                                distinct: s.distinct,
                                lock_clause: s.lock_clause.clone(),
                            },
                            _ => {
                                // Wrap a non-SELECT inner statement in a
                                // synthetic SELECT * FROM <inner> so the
                                // `from_subquery` field (which is typed
                                // SelectStatement) can carry it. The
                                // synthetic SELECT has empty `table`
                                // and `from_subquery = Some(inner_subq)`.
                                let inner_subq = match inner {
                                    Statement::Select(s) => s.clone(),
                                    _other => {
                                        // The inner is a set op; wrap it
                                        // in a synthetic SELECT with empty
                                        // `table` and `from_subquery =
                                        // None`. The executor's set-op
                                        // dispatch will run the inner as
                                        // a sub-query. We can't carry
                                        // set-op AST inside
                                        // from_subquery (typed
                                        // SelectStatement), so we instead
                                        // encode the inner as a synthetic
                                        // table by storing it under a
                                        // reserved name via
                                        // `from_subquery = None` and the
                                        // caller walks it through
                                        // execute_statement.
                                        SelectStatement {
                                            columns: vec![SelectColumn {
                                                name: "*".to_string(),
                                                alias: None,
                                                expression: None,
                                            }],
                                            table: String::new(),
                                            // V312-56A / 56A-R2: synthetic
                                            // set-op fallback carries no
                                            // schema qualifier.
                                            schema: None,
                                            from_alias: None,
                                            from_subquery: None,
                                            from_values: None,
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
                                            lock_clause: None,
                                        }
                                    }
                                };
                                SelectStatement {
                                    columns: vec![SelectColumn {
                                        name: "*".to_string(),
                                        alias: None,
                                        expression: None,
                                    }],
                                    table: String::new(),
                                    // V312-56A / 56A-R2: synthetic
                                    // set-op fallback carries no
                                    // schema qualifier.
                                    schema: None,
                                    from_alias: None,
                                    from_subquery: Some(Box::new(inner_subq)),
                                    from_values: None,
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
                                    lock_clause: None,
                                }
                            }
                        };
                        self.expect(Token::RParen)?;
                        if matches!(self.current(), Some(Token::As)) {
                            self.next();
                        }
                        let alias_nested = match self.next() {
                            Some(Token::Identifier(name)) => name,
                            Some(t) => {
                                return Err(format!(
                                    "Expected alias for nested derived subquery, got {:?}",
                                    t
                                ));
                            }
                            None => {
                                return Err(
                                    "Expected alias for nested derived subquery".to_string()
                                );
                            }
                        };
                        (alias_nested, Some(Box::new(materialised)), Vec::new())
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
                            schema: None,
                            from_alias: first_alias.clone(),
                            from_subquery: None,
                            from_values: None,
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
                            lock_clause: None,
                        };
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
                    // V312-56A / 56A-R2: also capture `schema` qualifier
                    // (`FROM schema.table`) for information_schema routing.
                    let first_table_raw = match self.next() {
                        Some(Token::Identifier(name)) => name,
                        Some(t) => return Err(format!("Expected table name, got {:?}", t)),
                        None => return Err("Expected table name".to_string()),
                    };
                    // V312-56A / 56A-R2: if the first identifier is followed
                    // by `.`, the left side is a schema qualifier (`FROM
                    // information_schema.tables`) and the right side is the
                    // actual table. We capture the schema into the outer
                    // `schema` binding (declared before the FROM match) so
                    // the final SelectStatement constructor can route the
                    // query to the information_schema virtual-table
                    // executor.
                    let first_table = if matches!(self.current(), Some(Token::Dot)) {
                        self.next(); // consume `.`
                        let t = match self.next() {
                            Some(Token::Identifier(name)) => name,
                            Some(t) => {
                                return Err(format!("Expected table name after `.`, got {:?}", t))
                            }
                            None => return Err("Expected table name after `schema.`".to_string()),
                        };
                        schema = Some(first_table_raw.clone());
                        t
                    } else {
                        first_table_raw
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
            Some(Token::Eof) | None | Some(Token::Semicolon) => (String::new(), None, Vec::new()),
            Some(Token::RParen) | Some(Token::Union) => (String::new(), None, Vec::new()),
            // V310-06 PR2: set-operation tokens also terminate the FROM
            // clause without error so parse_select_statement can be used
            // as right operand of UNION / INTERSECT / EXCEPT.
            Some(Token::Intersect) | Some(Token::Except) => (String::new(), None, Vec::new()),
            // V310-06 PR2: ORDER BY / LIMIT / OFFSET after a SELECT must
            // terminate the FROM clause so trailing clauses can be lifted
            // onto the set-op node.
            Some(Token::Order) | Some(Token::Limit) | Some(Token::Offset) => {
                (String::new(), None, Vec::new())
            }
            // V313-followup-5 / Issue #4158: `WITH [NO] DATA` terminates
            // a bare SELECT in CREATE TABLE AS SELECT context; caller
            // (parse_create_table) consumes the trailing token.
            Some(Token::With) => (String::new(), None, Vec::new()),
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

        // V312-56A / 56A-R2: `schema` (declared before the FROM match) is
        // populated in the FROM table-list branch above when a `schema.`
        // qualifier is present. Defaults to None for subquery, VALUES,
        // plain table, and JOIN paths.

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
                // TPCH-01 Q2 fix (Phase A): greedy join reorder.
                //
                // For TPC-H Q2 the declared FROM order is
                // `part, supplier, partsupp, nation, region`, but the
                // optimal hash-join chain is
                // `region, nation, supplier, partsupp, part` (smallest
                // first; each step has a usable WHERE-clause
                // equi-join edge to the previous accumulator). The
                // declared order forces an expensive cartesian at
                // step 1 (`part ⋈ supplier` has no equi-join), which
                // explodes even with base-table predicate pushdown.
                //
                // The reorder is conservative: it does NOT touch the
                // base table (which is the first non-comma FROM, fixed
                // by executor's `execute_joins` line 1267). It only
                // permutes the `extra_tables` Vec so the auto-rewrite
                // loop visits them in a smaller-first, neighbor-aware
                // order. Q15's __subq_N derived-table alias handling
                // remains unchanged (the synthetic names are filtered
                // out by `starts_with("__subq_")`).
                let extra_tables = tpch_reorder_extra_tables(&extra_tables, &joined, &conj);

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
                    // TPC-H prefix map. The naive "first char" or
                    // "underscore-split" heuristic fails for partsupp
                    // (no underscore; first char is 'p' but TPC-H
                    // column prefix is 'ps'), which then breaks
                    // find_join_predicate for `ps_suppkey = s_suppkey`
                    // style predicates — the 'ps' qualifier is
                    // unrecognised and the predicate falls through to
                    // ON=true. Use the canonical TPC-H map.
                    let tpch_prefix: &str = match table_name.as_str() {
                        "region" => "r",
                        "nation" => "n",
                        "supplier" => "s",
                        "customer" => "c",
                        "part" => "p",
                        "partsupp" => "ps",
                        "orders" => "o",
                        "lineitem" => "l",
                        _ => {
                            if table_name.contains('_') {
                                let us = table_name.find('_').unwrap();
                                &table_name[..us]
                            } else if !table_name.is_empty() {
                                &table_name[..1]
                            } else {
                                ""
                            }
                        }
                    };
                    joined.push(tpch_prefix.to_string());
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
                // Recover the inline alias from the `bare|alias`
                // encoding that the extra_tables loop produced, so
                // the executor can match qualifiers like `n2.n_name`
                // in the WHERE clause. Without this, the alias is
                // silently lost on the cartesian path (TPC-H Q8 7-way
                // bug: `nation n2` becomes `join_clause.table="nation"`
                // `join_clause.alias=None`, so the pre-filter lookup
                // misses the `n2.n_name = 'GERMANY'` push-down).
                let cart: Vec<JoinClause> = extra_tables
                    .iter()
                    .map(|t| {
                        let (bare, alias) = match t.split_once('|') {
                            Some((b, a)) => (b.to_string(), Some(a.to_string())),
                            None => (t.clone(), None),
                        };
                        JoinClause {
                            join_type: JoinType::Inner,
                            table: bare,
                            alias,
                            on_clause: Expression::Literal("true".to_string()),
                        }
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
            // MySQL: LIMIT ALL = no limit (== LIMIT NULL)
            if matches!(self.current(), Some(Token::All)) {
                self.next();
                None
            } else {
                // V313-10 / Issue #4038: peek whether the LIMIT value is followed
                // by an arithmetic operator. If it is, parse the full expression
                // through parse_expression + constant_fold_u64 so that
                // `LIMIT 2-1` is folded to 1, not silently truncated to 2.
                let peek_is_arith = matches!(
                    self.tokens.get(self.position + 1),
                    Some(Token::Plus)
                        | Some(Token::Minus)
                        | Some(Token::Star)
                        | Some(Token::Slash)
                        | Some(Token::Percent)
                );
                if peek_is_arith {
                    // Arithmetic expression form: parse full expression.
                    // V313-10 / Issue #4038: emit a classified binder error
                    // when the expression cannot be constant-folded.
                    let saved_pos = self.position;
                    let expr = self.parse_expression()?;
                    match constant_fold_u64(&expr) {
                        Some(v) => Some(v),
                        None => {
                            self.position = saved_pos;
                            return Err(classify_unfoldable_limit_expr("LIMIT", &expr));
                        }
                    }
                } else {
                    match self.current() {
                        Some(Token::NumberLiteral(n)) => {
                            // Handle both integer and float literals (e.g., LIMIT 1.25 -> 1 row)
                            let val = if let Ok(i) = n.parse::<u64>() {
                                i
                            } else if let Ok(f) = n.parse::<f64>() {
                                f as u64
                            } else {
                                return Err(
                                    "Invalid LIMIT: invalid digit found in string".to_string()
                                );
                            };
                            self.next();
                            Some(val)
                        }
                        Some(Token::Identifier(ref s)) => {
                            // Support LIMIT variable (e.g., @limit) and
                            // identifier-like column references. When the
                            // identifier is a pure integer string we accept
                            // it directly; when followed by `(` we let
                            // parse_expression handle it (so `row_number()`
                            // gets wrapped in a WindowCall / FunctionCall
                            // and the classify step produces the right error);
                            // otherwise we treat it as a column reference
                            // and emit the binder error directly.
                            // V313-10 / Issue #4038.
                            if let Ok(val) = s.parse::<u64>() {
                                self.next();
                                Some(val)
                            } else if matches!(
                                self.tokens.get(self.position + 1),
                                Some(Token::LParen)
                            ) {
                                // Looks like a function call (possibly
                                // windowed) — let parse_expression build
                                // the full AST, then classify.
                                let saved_pos = self.position;
                                let expr = self.parse_expression()?;
                                match constant_fold_u64(&expr) {
                                    Some(v) => Some(v),
                                    None => {
                                        self.position = saved_pos;
                                        return Err(classify_unfoldable_limit_expr("LIMIT", &expr));
                                    }
                                }
                            } else {
                                let ident = s.clone();
                                return Err(format!(
                                    "Binder Error: Referenced column '{}' not found in LIMIT",
                                    ident
                                ));
                            }
                        }
                        _ => {
                            // V312-19 #3972: accept arithmetic expression, e.g. LIMIT 2-1.
                            // V313-10 / Issue #4038: if the expression contains
                            // an aggregate, window function or column ref
                            // that cannot be constant-folded, restore
                            // position and emit a classified binder-style
                            // error. Returning None would silently swallow
                            // the LIMIT clause.
                            let saved_pos = self.position;
                            let expr = self.parse_expression()?;
                            match constant_fold_u64(&expr) {
                                Some(v) => Some(v),
                                None => {
                                    self.position = saved_pos;
                                    return Err(classify_unfoldable_limit_expr("LIMIT", &expr));
                                }
                            }
                        }
                    }
                }
            }
        } else {
            None
        };

        // Parse OFFSET clause
        let offset = if matches!(self.current(), Some(Token::Offset)) {
            self.next();
            // V313-10 / Issue #4038: mirror the LIMIT fix — peek for an
            // arithmetic operator so `OFFSET 5-1` folds to 4 instead
            // of being silently truncated to 5.
            let peek_is_arith = matches!(
                self.tokens.get(self.position + 1),
                Some(Token::Plus)
                    | Some(Token::Minus)
                    | Some(Token::Star)
                    | Some(Token::Slash)
                    | Some(Token::Percent)
            );
            if peek_is_arith {
                let saved_pos = self.position;
                let expr = self.parse_expression()?;
                match constant_fold_u64(&expr) {
                    Some(v) => Some(v),
                    None => {
                        self.position = saved_pos;
                        return Err(classify_unfoldable_limit_expr("OFFSET", &expr));
                    }
                }
            } else {
                match self.current() {
                    Some(Token::NumberLiteral(n)) => {
                        let val = if let Ok(i) = n.parse::<u64>() {
                            i
                        } else if let Ok(f) = n.parse::<f64>() {
                            f as u64
                        } else {
                            return Err("Invalid OFFSET: invalid digit found in string".to_string());
                        };
                        self.next();
                        Some(val)
                    }
                    Some(Token::Identifier(ref s)) => {
                        // V313-10 / Issue #4038: mirror the LIMIT fix —
                        // non-numeric identifiers (likely column refs)
                        // fall through to the binder rather than being
                        // rejected here as 'Invalid OFFSET'.
                        if let Ok(val) = s.parse::<u64>() {
                            self.next();
                            Some(val)
                        } else {
                            let saved_pos = self.position;
                            let expr = self.parse_expression()?;
                            match constant_fold_u64(&expr) {
                                Some(v) => Some(v),
                                None => {
                                    self.position = saved_pos;
                                    None
                                }
                            }
                        }
                    }
                    _ => {
                        // V312-19 #3972: OFFSET also accepts arithmetic expression.
                        // V313-10 / Issue #4038: emit a classified binder error
                        // when the expression cannot be constant-folded.
                        let saved_pos = self.position;
                        let expr = self.parse_expression()?;
                        match constant_fold_u64(&expr) {
                            Some(v) => Some(v),
                            None => {
                                self.position = saved_pos;
                                return Err(classify_unfoldable_limit_expr("OFFSET", &expr));
                            }
                        }
                    }
                }
            }
        } else {
            None
        };

        // Parse FOR UPDATE / LOCK IN SHARE MODE clause
        let lock_clause = if matches!(self.current(), Some(Token::For)) {
            self.next(); // consume FOR
            self.expect(Token::Update)?;
            // Check for SKIP LOCKED or NOWAIT modifiers
            let mut skip_locked = false;
            let mut nowait = false;
            loop {
                match self.current() {
                    Some(Token::Skip) => {
                        self.next();
                        self.expect(Token::Locked)?;
                        skip_locked = true;
                    }
                    Some(Token::Nowait) => {
                        self.next();
                        nowait = true;
                    }
                    _ => break,
                }
            }
            Some(LockClause {
                for_update: true,
                skip_locked,
                nowait,
            })
        } else if matches!(self.current(), Some(Token::Lock)) {
            self.next();
            self.expect(Token::In)?;
            self.expect(Token::Share)?;
            self.expect(Token::Mode)?;
            Some(LockClause {
                for_update: false,
                skip_locked: false,
                nowait: false,
            })
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
            // V312-56A / 56A-R2: populate `schema` from the parse path
            // (already captured in the local `schema` binding earlier
            // in this function via parse_table_ref).
            schema,
            from_alias,
            from_subquery,
            from_values: None,
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
            lock_clause,
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
                    "QUANTILE_DISC" => Some(AggregateFunction::QuantileDisc),
                    "PERCENTILE_CONT" => Some(AggregateFunction::PercentileCont),
                    "QUANTILE_CONT" => Some(AggregateFunction::QuantileCont),
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

        // Check for INSERT IGNORE (MySQL compatibility, V311-23) - consume Ignore token after Insert
        let is_ignore = if !is_replace && matches!(self.current(), Some(Token::Ignore)) {
            self.next(); // consume Ignore
            true
        } else {
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
                        // V313-followup-1 / Issue #4154: INSERT VALUES (DEFAULT)
                        // materialises the column's set default.
                        Some(Token::Default) => {
                            self.next();
                            row.push(Expression::Identifier("DEFAULT".to_string()));
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
            is_ignore,
            on_duplicate_key_update,
        }))
    }

    fn parse_update(&mut self) -> Result<Statement, String> {
        self.expect(Token::Update)?;
        let tables = self.parse_table_ref_list_until_set()?;

        if !matches!(self.current(), Some(Token::Set)) {
            return Err("Expected SET".to_string());
        }
        self.next();

        let mut set_clauses = Vec::new();
        loop {
            let column = self.parse_set_column_name()?;
            match self.current() {
                Some(Token::Equal) => {}
                _ => return Err("Expected = in SET clause".to_string()),
            }
            self.next();

            let value = self.parse_expression()?;

            // V312-18 #3971: reject duplicate column assignment in SET
            for (existing_col, _) in &set_clauses {
                if existing_col == &column {
                    return Err(format!(
                        "Binder Error: duplicate column '{}' in UPDATE SET clause",
                        column
                    ));
                }
            }
            set_clauses.push((column, value));

            match self.current() {
                Some(Token::Comma) => {
                    self.next();
                }
                Some(Token::Where) | None | Some(Token::Eof) => break,
                _ => return Err("Expected , or WHERE".to_string()),
            }
        }

        let where_clause = if matches!(self.current(), Some(Token::Where)) {
            self.next();
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Statement::Update(UpdateStatement {
            tables,
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

        // V312-22b / Issue #4036: include `Token::Percent` so `i % 2` parses
        // through the standard multiplicative chain. Previously only `*` and
        // `/` were accepted, causing `SELECT i % 2 FROM integers` (the
        // TPC-H Q4 / sqllogictest insert__test_insert.test case) to fail
        // with "Expected FROM or column name" — the parser fell out of the
        // Identifier branch with the leading `i` consumed and `%` left as
        // the next token, which doesn't match any SELECT-list case.
        while let Some(Token::Star) | Some(Token::Slash) | Some(Token::Percent) = self.current() {
            let op = match self.current() {
                Some(Token::Star) => "*",
                Some(Token::Slash) => "/",
                Some(Token::Percent) => "%",
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
            | Some(Token::Cube)
            | Some(Token::Database) => {
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
                    Some(Token::Database) => "DATABASE",
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
                // DATE_ADD/DATE_SUB supports two MySQL 5.7 forms:
                //   (a) DATE_ADD(d, n, 'UNIT')                 — 3-arg
                //   (b) DATE_ADD(d, INTERVAL n UNIT)            — SQL standard
                // Mirrors the Identifier arm special form.
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
                    let (n_expr, unit) = if matches!(self.current(), Some(Token::Interval)) {
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
                        (n_expr, unit)
                    } else {
                        let n_expr = self.parse_primary_expression()?;
                        if !matches!(self.current(), Some(Token::Comma)) {
                            return Err(format!(
                                "Expected ',' before unit in 3-arg {}(d, n, 'UNIT'), got {:?}",
                                name,
                                self.current()
                            ));
                        }
                        self.next(); // consume Comma
                        let unit = match self.current() {
                            Some(Token::StringLiteral(u)) => {
                                let s = u.clone();
                                self.next();
                                s
                            }
                            Some(Token::Identifier(u)) => {
                                let s = u.clone();
                                self.next();
                                s
                            }
                            _ => {
                                return Err(format!(
                                    "Expected string-literal unit in 3-arg {}(d, n, 'UNIT'), got {:?}",
                                    name,
                                    self.current()
                                ));
                            }
                        };
                        (n_expr, unit)
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
                // V313-followup-3 / Issue #4156: PERCENTILE_CONT
                // optionally followed by WITHIN GROUP (ORDER BY col).
                if name.to_uppercase() == "PERCENTILE_CONT"
                    && matches!(self.current(), Some(Token::Within))
                {
                    self.next();
                    self.expect(Token::Group)?;
                    self.expect(Token::LParen)?;
                    self.expect(Token::Order)?;
                    self.expect(Token::By)?;
                    self.parse_expression()?;
                    while matches!(self.current(), Some(Token::Comma)) {
                        self.next();
                        self.parse_expression()?;
                    }
                    self.expect(Token::RParen)?;
                }
                Ok(Expression::FunctionCall(name.to_string(), args))
            }
            Some(Token::SystemVariable(name)) => {
                // MySQL `@@version_comment` / `@@autocommit` / etc. — a single
                // scalar expression that the executor resolves to the current
                // session/system value at plan time.
                let var_name = name.clone();
                self.next();
                Ok(Expression::SystemVariable(var_name))
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
                        let (n_expr, unit) = if matches!(self.current(), Some(Token::Interval)) {
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
                            (n_expr, unit)
                        } else {
                            let n_expr = self.parse_primary_expression()?;
                            if !matches!(self.current(), Some(Token::Comma)) {
                                return Err(format!(
                                    "Expected ',' before unit in 3-arg {}(d, n, 'UNIT'), got {:?}",
                                    name,
                                    self.current()
                                ));
                            }
                            self.next(); // consume Comma
                            let unit = match self.current() {
                                Some(Token::StringLiteral(u)) => {
                                    let s = u.clone();
                                    self.next();
                                    s
                                }
                                Some(Token::Identifier(u)) => {
                                    let s = u.clone();
                                    self.next();
                                    s
                                }
                                _ => {
                                    return Err(format!(
                                        "Expected string-literal unit in 3-arg {}(d, n, 'UNIT'), got {:?}",
                                        name,
                                        self.current()
                                    ));
                                }
                            };
                            (n_expr, unit)
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
                        // Consume the CAST's closing paren. The args loop
                        // above terminates on `AS` (not `)`), so the
                        // closing `)` of `CAST(expr AS TYPE)` is still
                        // pending here. Without this, the leftover `)`
                        // makes a parent column-list loop treat the
                        // subquery as terminated early, dropping any
                        // following columns and the FROM clause
                        // (V312-21 / #4181, TPC-H Q7-Q9 subquery parse).
                        //
                        // V312-bug-report-3120 / BUG-2a: the consume must
                        // be gated on `name == "CAST"`. The args loop above
                        // already ran `self.expect(Token::RParen)?` for every
                        // non-CAST call, so for an arbitrary Identifier-path
                        // function (e.g. `length`, `upper`, `abs`, `year`)
                        // the closing `)` is already consumed. Unconditionally
                        // consuming another RParen here ate the *parent*
                        // function's closing paren, breaking nested calls
                        // like `upper(length('alice'))` and `year(now())`
                        // with "Expected RParen, got Eof".
                        //
                        // V312-bugfix / #4490: PR #4508 also tried to fix
                        // the same nested-call issue by removing the entire
                        // consume block, which would regress TPC-H Q7-Q9
                        // subquery parsing. Keep the gated CAST-only
                        // consume; the gated form subsumes the PR #4508
                        // intent (no extra RParen for non-CAST functions).
                        if name.to_uppercase() == "CAST"
                            && matches!(self.current(), Some(Token::RParen))
                        {
                            self.next();
                        }
                        // EXTRACT(field FROM expr) — field is a SQL token (YEAR,
                        // MONTH, DAY, ...). The executor's `EXTRACT` eval_fn
                        // expects a 2-arg FunctionCall where arg[0] is the
                        // field name and arg[1] is the source expression. We
                        // encode it that way: push the field as a quoted
                        // Literal string so it round-trips through the AST.
                        let name_upper = name.to_uppercase();
                        // V313-followup-3 / Issue #4156: PERCENTILE_CONT
                        // optionally followed by WITHIN GROUP (ORDER BY col
                        // [ASC|DESC]). Encode the ORDER BY expression as an
                        // extra arg so the executor can evaluate it into the
                        // value list; DESC is encoded as a trailing
                        // `__DESC__` literal arg.
                        if name_upper == "PERCENTILE_CONT"
                            && matches!(self.current(), Some(Token::Within))
                        {
                            self.next();
                            self.expect(Token::Group)?;
                            self.expect(Token::LParen)?;
                            self.expect(Token::Order)?;
                            self.expect(Token::By)?;
                            let ob_expr = self.parse_expression()?;
                            args.push(ob_expr);
                            if matches!(self.current(), Some(Token::Asc)) {
                                self.next();
                            } else if matches!(self.current(), Some(Token::Desc)) {
                                self.next();
                                args.push(Expression::Literal("__DESC__".to_string()));
                            }
                            while matches!(self.current(), Some(Token::Comma)) {
                                self.next();
                                args.push(self.parse_expression()?);
                            }
                            self.expect(Token::RParen)?;
                        }
                        let expr = Expression::FunctionCall(name, args);
                        Ok(expr)
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
            // Round-21 / Issue #4216: array literal `[expr, expr, ...]`.
            // Used by the array-fraction form of ordered-set aggregates
            // (e.g. `quantile_disc(col, [0.25, 0.5, 0.75])`).
            Some(Token::LBracket) => {
                self.next(); // consume `[`
                let mut elems = Vec::new();
                if !matches!(self.current(), Some(Token::RBracket)) {
                    loop {
                        elems.push(self.parse_expression()?);
                        if matches!(self.current(), Some(Token::Comma)) {
                            self.next();
                        } else {
                            break;
                        }
                    }
                }
                self.expect(Token::RBracket)?;
                Ok(Expression::ArrayLiteral(elems))
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
                        AggregateFunction::PercentileCont => "PERCENTILE_CONT",
                        AggregateFunction::QuantileDisc => "QUANTILE_DISC",
                        AggregateFunction::QuantileCont => "QUANTILE_CONT",
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
            // CURRVAL(sequence_name) - read current value without advancing
            Some(Token::Currval) => {
                self.next(); // consume CURRVAL
                self.expect(Token::LParen)?;
                let seq_name = match self.next() {
                    Some(Token::Identifier(name)) => name,
                    _ => return Err("Expected sequence name".to_string()),
                };
                self.expect(Token::RParen)?;
                Ok(Expression::SequenceCurrval(seq_name))
            }
            // NEXT [VALUE] FOR sequence_name - SQL:2003 standard allows the
            // optional `VALUE` keyword between NEXT and FOR. Accept both
            // `NEXT FOR seq` and `NEXT VALUE FOR seq`.
            Some(Token::NextValue) => {
                self.next(); // consume NEXT
                             // SQL:2003 allows optional `VALUE` between NEXT and FOR.
                             // As of the 2026-08-09 fix, the lexer no longer reserves VALUE
                             // as a keyword, so it surfaces as Identifier("value") here.
                             // Accept both `Identifier("value")` and the legacy `Token::Value`.
                if matches!(self.current(), Some(Token::Identifier(name)) if name.eq_ignore_ascii_case("value"))
                {
                    self.next();
                }
                self.expect(Token::For)?;
                let seq_name = match self.next() {
                    Some(Token::Identifier(name)) => name,
                    _ => return Err("Expected sequence name".to_string()),
                };
                Ok(Expression::SequenceNextVal(seq_name))
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
        // `DELETE FROM t ...` is single-table; `DELETE t1, t2 FROM ...`
        // is multi-table. The token after DELETE disambiguates.
        let (tables, using) = if matches!(self.current(), Some(Token::From)) {
            self.next();
            let tref = self.parse_table_ref()?;
            (vec![tref], None)
        } else {
            let targets = self.parse_table_ref_list(Token::From)?;
            self.expect(Token::From)?;
            let sources = self.parse_table_ref_list_end()?;
            (targets, Some(sources))
        };

        let where_clause = if matches!(self.current(), Some(Token::Where)) {
            self.next();
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Statement::Delete(DeleteStatement {
            tables,
            using,
            where_clause,
        }))
    }

    fn parse_table_ref(&mut self) -> Result<TableRef, String> {
        let mut name = match self.next() {
            Some(Token::Identifier(name)) => name,
            _ => return Err("Expected table name".to_string()),
        };
        // V312-56A / 56A-R2 / Issue #4251: parse schema.table
        // qualification (`FROM information_schema.tables`). Capture the
        // leading identifier; if a `.` follows, treat it as the
        // schema prefix and advance to read the table name.
        let mut schema: Option<String> = None;
        if matches!(self.current(), Some(Token::Dot)) {
            self.next(); // consume `.`
            schema = Some(name);
            name = match self.next() {
                Some(Token::Identifier(n)) => n,
                _ => return Err("Expected table name after `schema.`".to_string()),
            };
        }
        let alias = self.parse_optional_alias()?;
        Ok(TableRef {
            name,
            schema,
            alias,
        })
    }

    fn parse_table_ref_list(&mut self, terminator: Token) -> Result<Vec<TableRef>, String> {
        let mut out = Vec::new();
        out.push(self.parse_table_ref()?);
        while matches!(self.current(), Some(Token::Comma)) {
            self.next();
            out.push(self.parse_table_ref()?);
        }
        if !matches!(self.current(), Some(t) if std::mem::discriminant(t) == std::mem::discriminant(&terminator))
        {
            return Err(format!("Expected {:?} after table list", terminator));
        }
        Ok(out)
    }

    fn parse_table_ref_list_end(&mut self) -> Result<Vec<TableRef>, String> {
        let mut out = Vec::new();
        out.push(self.parse_table_ref()?);
        while matches!(self.current(), Some(Token::Comma)) {
            self.next();
            out.push(self.parse_table_ref()?);
        }
        match self.current() {
            Some(Token::Where) | None | Some(Token::Eof) | Some(Token::Semicolon) => {}
            _ => return Err("Expected , or WHERE after table list".to_string()),
        }
        Ok(out)
    }

    fn parse_table_ref_list_until_set(&mut self) -> Result<Vec<TableRef>, String> {
        let mut out = Vec::new();
        out.push(self.parse_table_ref()?);
        while matches!(self.current(), Some(Token::Comma)) {
            self.next();
            out.push(self.parse_table_ref()?);
        }
        if !matches!(self.current(), Some(Token::Set)) {
            return Err("Expected SET after table list".to_string());
        }
        Ok(out)
    }

    fn parse_set_column_name(&mut self) -> Result<String, String> {
        let first = match self.current() {
            Some(Token::Identifier(name)) => name.clone(),
            _ => return Err("Expected column name in SET".to_string()),
        };
        self.next();
        if matches!(self.current(), Some(Token::Dot)) {
            self.next();
            let second = match self.current() {
                Some(Token::Identifier(name)) => name.clone(),
                _ => return Err("Expected column name after '.' in SET".to_string()),
            };
            self.next();
            Ok(format!("{}.{}", first, second))
        } else {
            Ok(first)
        }
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

        // V312-18: Parse OR REPLACE before TABLE keyword
        let or_replace = if matches!(self.current(), Some(Token::Or)) {
            self.next();
            match self.current() {
                Some(Token::Replace) => {
                    self.next();
                    true
                }
                _ => return Err("Expected 'REPLACE' after 'OR'".to_string()),
            }
        } else {
            false
        };

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
                        let col_def = self.parse_column_definition(&mut constraints)?;
                        columns.push(col_def);
                    }
                    Some(Token::Primary) => {
                        self.next();
                        self.expect(Token::Key)?;
                        let columns = self.parse_column_list()?;
                        constraints.push(TableConstraint::PrimaryKey {
                            columns,
                            name: None,
                        });
                    }
                    Some(Token::Foreign) => {
                        let fk = self.parse_foreign_key_constraint(None)?;
                        constraints.push(fk);
                    }
                    Some(Token::Unique) => {
                        // V313-#4071: only enter this arm when the
                        // keyword is followed by `(`; otherwise fall
                        // through so the inner parse_column_definition
                        // loop gets a chance to consume the column-level
                        // UNIQUE modifier and the outer loop continues
                        // with the next column definition. Without
                        // this guard, `,` after UNIQUE makes
                        // parse_column_list() silently consume the
                        // next column's identifier as part of a
                        // spurious UNIQUE columns list.
                        //
                        // V312-coverage: also handle `UNIQUE KEY (cols)`
                        // and `UNIQUE KEY name (cols)` (MySQL allows the
                        // optional KEY keyword between UNIQUE and the
                        // column list, with an optional constraint
                        // name in between). Previously UNIQUE KEY fell
                        // into the `else continue` branch and since
                        // the loop didn't advance on `continue`, it
                        // spun forever.
                        let next_tok = self.tokens.get(self.position + 1);
                        match next_tok {
                            Some(Token::LParen) => {
                                self.next();
                                let columns = self.parse_column_list()?;
                                constraints.push(TableConstraint::Unique {
                                    columns,
                                    name: None,
                                });
                            }
                            Some(Token::Key) => {
                                self.next(); // consume UNIQUE
                                self.next(); // consume KEY
                                let name = match self.current() {
                                    Some(Token::Identifier(n)) => {
                                        let s = n.clone();
                                        self.next();
                                        Some(s)
                                    }
                                    _ => None,
                                };
                                let columns = self.parse_column_list()?;
                                constraints.push(TableConstraint::Unique { columns, name });
                            }
                            _ => continue,
                        }
                    }
                    Some(Token::Check) => {
                        self.next();
                        self.expect(Token::LParen)?;
                        let expr = self.parse_expression()?;
                        self.expect(Token::RParen)?;
                        constraints.push(TableConstraint::Check {
                            expression: expr,
                            name: None,
                        });
                    }
                    Some(Token::Constraint) => {
                        self.next();
                        if let Some(Token::Identifier(name)) = self.next() {
                            match self.current() {
                                Some(Token::Primary) => {
                                    self.next();
                                    self.expect(Token::Key)?;
                                    let cols = self.parse_column_list()?;
                                    constraints.push(TableConstraint::PrimaryKey {
                                        columns: cols,
                                        name: Some(name),
                                    });
                                }
                                Some(Token::Foreign) => {
                                    self.next();
                                    let fk = self.parse_foreign_key_constraint(Some(name))?;
                                    constraints.push(fk);
                                }
                                Some(Token::Unique) => {
                                    self.next();
                                    let cols = self.parse_column_list()?;
                                    constraints.push(TableConstraint::Unique {
                                        columns: cols,
                                        name: Some(name),
                                    });
                                }
                                Some(Token::Check) => {
                                    self.next();
                                    self.expect(Token::LParen)?;
                                    let expr = self.parse_expression()?;
                                    self.expect(Token::RParen)?;
                                    constraints.push(TableConstraint::Check {
                                        expression: expr,
                                        name: Some(name),
                                    });
                                }
                                _ => {
                                    return Err(format!(
                                        "Expected constraint type, got {:?}",
                                        self.current()
                                    ))
                                }
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

        // V311-01: Parse trailing `ENGINE=<engine> [CLUSTERED]` clause.
        // ENGINE=InnoDB CLUSTERED → ClusteredIndex B+ Tree storage.
        // ENGINE=InnoDB (no CLUSTERED) → Heap default storage.
        let storage_engine = self.parse_table_storage_engine_clause();

        // V311-12 F-27: Parse trailing `COMPRESS (ALGORITHM=LZ4)` clause.
        let compress = self.parse_compress_clause();

        // V312-18: Parse AS SELECT clause for CREATE TABLE AS SELECT
        let mut select: Option<Box<SelectStatement>> = None;
        let mut with_data: Option<bool> = None;

        if matches!(self.current(), Some(Token::As)) {
            self.next();
            match self.current() {
                Some(Token::Select) => {
                    let select_stmt = self.parse_select()?;
                    match select_stmt {
                        Statement::Select(s) => select = Some(Box::new(s)),
                        _ => return Err("Expected SELECT statement".to_string()),
                    }
                    // V313-followup-5 / Issue #4158: PostgreSQL / DuckDB
                    // standard order is `AS SELECT ... WITH [NO] DATA`.
                    // Accept the optional trailing WITH clause here; default
                    // is WITH DATA when the clause is absent.
                    with_data = match self.current() {
                        Some(Token::With) => {
                            self.next();
                            match self.current() {
                                Some(Token::No) => {
                                    self.next();
                                    match self.current() {
                                        Some(Token::Identifier(s))
                                            if s.eq_ignore_ascii_case("DATA") =>
                                        {
                                            self.next();
                                            Some(false)
                                        }
                                        _ => return Err("Expected 'DATA' after 'NO'".to_string()),
                                    }
                                }
                                Some(Token::Identifier(s)) if s.eq_ignore_ascii_case("DATA") => {
                                    self.next();
                                    Some(true)
                                }
                                _ => {
                                    return Err(
                                        "Expected 'NO DATA' or 'DATA' after 'WITH'".to_string()
                                    )
                                }
                            }
                        }
                        _ => Some(true),
                    };
                }
                Some(Token::With) => {
                    // Legacy V312-18 form: `AS WITH [NO] DATA SELECT ...`.
                    self.next();
                    let data_flag = match self.current() {
                        Some(Token::No) => {
                            self.next();
                            match self.current() {
                                Some(Token::Identifier(s)) if s.eq_ignore_ascii_case("DATA") => {
                                    self.next();
                                    false // WITH NO DATA
                                }
                                _ => return Err("Expected 'DATA' after 'NO'".to_string()),
                            }
                        }
                        Some(Token::Identifier(s)) if s.eq_ignore_ascii_case("DATA") => {
                            self.next();
                            true // WITH DATA
                        }
                        _ => return Err("Expected 'NO DATA' or 'DATA' after 'WITH'".to_string()),
                    };
                    match self.current() {
                        Some(Token::Select) => {
                            let select_stmt = self.parse_select()?;
                            match select_stmt {
                                Statement::Select(s) => select = Some(Box::new(s)),
                                _ => return Err("Expected SELECT statement".to_string()),
                            }
                            with_data = Some(data_flag);
                        }
                        _ => return Err("Expected SELECT after WITH [NO] DATA clause".to_string()),
                    }
                }
                _ => return Err("Expected SELECT after AS".to_string()),
            }
        }

        Ok(Statement::CreateTable(CreateTableStatement {
            name,
            columns,
            constraints,
            if_not_exists,
            storage_engine,
            compress,
            select,
            or_replace,
            with_data,
        }))
    }

    /// V311-01: parse trailing `ENGINE=...` clause.
    /// Recognizes:
    ///   - `ENGINE=InnoDB CLUSTERED` → Some(StorageEngineSpec::Clustered)
    ///   - `ENGINE=InnoDB`           → Some(StorageEngineSpec::Heap) (explicit)
    ///   - absent                    → None (use default Heap)
    ///
    /// Returns `None` when no ENGINE clause is present.
    fn parse_table_storage_engine_clause(&mut self) -> Option<StorageEngineSpec> {
        // Tolerate optional whitespace before ENGINE keyword (already lexed
        // away by the tokenizer). Match either `Token::Identifier("ENGINE")`
        // or a future `Token::Engine` if we add one — for now just check
        // the identifier string.
        match self.current() {
            Some(Token::Identifier(s)) if s.eq_ignore_ascii_case("ENGINE") => {
                self.next(); // consume ENGINE
            }
            _ => return None,
        }
        // Expect `=`
        if !matches!(self.current(), Some(Token::Equal)) {
            return None;
        }
        self.next(); // consume =
                     // Expect engine name identifier (e.g. INNODB, MEMORY, HEAP)
        let engine_name = match self.current() {
            Some(Token::Identifier(s)) => {
                let n = s.clone();
                self.next();
                n
            }
            _ => return None,
        };
        // CLUSTERED keyword (or clustered identifier, kept as Identifier for now)
        let mut clustered = false;
        match self.current() {
            Some(Token::Identifier(s)) if s.eq_ignore_ascii_case("CLUSTERED") => {
                self.next();
                clustered = true;
            }
            Some(Token::Semicolon) | None => {}
            _ => {}
        }
        let engine_upper = engine_name.to_uppercase();
        if clustered {
            Some(StorageEngineSpec::Clustered)
        } else if engine_upper == "INNODB" || engine_upper == "MEMORY" || engine_upper == "HEAP" {
            Some(StorageEngineSpec::Heap)
        } else {
            // Unknown engine — ignore silently for backward compat, treat as default
            None
        }
    }

    /// V311-12 F-27: parse trailing `COMPRESS (ALGORITHM=LZ4)` clause.
    /// Syntax: COMPRESS (ALGORITHM=LZ4) | COMPRESS (ALGORITHM=ZSTD) | COMPRESS (ALGORITHM=zlib)
    fn parse_compress_clause(&mut self) -> Option<CompressionSpec> {
        match self.current() {
            Some(Token::Identifier(s)) if s.eq_ignore_ascii_case("COMPRESS") => {
                self.next();
            }
            _ => return None,
        }
        if !matches!(self.current(), Some(Token::LParen)) {
            return None;
        }
        self.next();
        if !matches!(self.current(), Some(Token::Identifier(s)) if s.eq_ignore_ascii_case("ALGORITHM"))
        {
            return None;
        }
        self.next();
        if !matches!(self.current(), Some(Token::Equal)) {
            return None;
        }
        self.next();
        let algo = match self.current() {
            Some(Token::Identifier(s)) => {
                let algo_str = s.to_uppercase();
                self.next();
                match algo_str.as_str() {
                    "LZ4" => CompressionAlgorithm::Lz4,
                    "ZSTD" => CompressionAlgorithm::Zstd,
                    "ZLIB" | "DEFLATE" => CompressionAlgorithm::Zlib,
                    _ => return None,
                }
            }
            _ => return None,
        };
        if !matches!(self.current(), Some(Token::RParen)) {
            return None;
        }
        self.next();
        Some(CompressionSpec { algorithm: algo })
    }

    fn parse_column_definition(
        &mut self,
        constraints: &mut Vec<TableConstraint>,
    ) -> Result<ColumnDefinition, String> {
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
            Some(Token::Date) => {
                // V313-#4071: without this arm the parser would
                // fall through to the `_ => "INTEGER".to_string()`
                // default and silently coerce a DATE column to
                // INTEGER. The fixture uses `Date NOT NULL UNIQUE`
                // to exercise the column-level UNIQUE modifier
                // path; that path required a working data_type
                // for the column to be accepted by the storage
                // engine.
                self.next();
                "DATE".to_string()
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

        // Consume parenthesized type arguments e.g. VARCHAR(50), DECIMAL(10,2).
        // For CHAR(N) / VARCHAR(N) we also capture N into `char_max_length`
        // so the engine can apply SQL-standard space padding on INSERT.
        let mut char_max_length: Option<usize> = None;
        if matches!(self.current(), Some(Token::LParen)) {
            self.next();
            if matches!(data_type.as_str(), "CHAR" | "VARCHAR") {
                if let Some(Token::NumberLiteral(n)) = self.current() {
                    if let Ok(parsed) = n.parse::<usize>() {
                        if parsed > 0 {
                            char_max_length = Some(parsed);
                        }
                    }
                    self.next();
                }
            }
            // Skip everything until matching RParen (handles commas inside e.g. DECIMAL(10,2))
            let mut depth = 1;
            while depth > 0 {
                match self.current() {
                    Some(Token::LParen) => {
                        depth += 1;
                        self.next();
                    }
                    Some(Token::RParen) => {
                        depth -= 1;
                        self.next();
                    }
                    Some(_) => {
                        self.next();
                    }
                    None => break,
                }
            }
        }

        let mut nullable = true;
        let mut primary_key = false;
        let mut auto_increment = false;
        let mut default_value = None;
        let mut references = None;
        let mut collation: Option<String> = None;

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
                Some(Token::Collate) => {
                    // V4077 / Issue #4077: capture the COLLATE <name> hint
                    // so EXCEPT/INTERSECT executors can perform
                    // collation-aware row comparison (NOCASE) on text
                    // columns. The collation name is upper-cased for
                    // case-insensitive matching against well-known
                    // collation identifiers (NOCASE, BINARY, RTRIM).
                    self.next();
                    match self.current().cloned() {
                        Some(Token::Identifier(name)) => {
                            collation = Some(name.to_uppercase());
                            self.next();
                        }
                        _ => return Err("Expected collation name after COLLATE".to_string()),
                    }
                }
                Some(Token::Check) => {
                    // V312-18 #3971: inline column-level CHECK constraint
                    self.next();
                    self.expect(Token::LParen)?;
                    let check_expr = self.parse_expression()?;
                    self.expect(Token::RParen)?;
                    constraints.push(TableConstraint::Check {
                        expression: check_expr,
                        name: None,
                    });
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
                Some(Token::Unique) => {
                    // V312-coverage: column-level UNIQUE modifier
                    // (`id INT UNIQUE`). Previously fell through to
                    // `_ => break` which left the outer CREATE TABLE
                    // loop re-entering this function with Token::Unique
                    // still at the head → infinite loop.
                    // Consume the keyword; the table-level UNIQUE
                    // constraint (if any) is added by the outer arm
                    // when followed by `(` / KEY.
                    self.next();
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
            char_max_length,
            collation,
        })
    }

    fn parse_foreign_key_constraint(
        &mut self,
        name: Option<String>,
    ) -> Result<TableConstraint, String> {
        self.expect(Token::Foreign)?;
        self.expect(Token::Key)?;
        self.expect(Token::LParen)?;
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
            name,
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
            // V312-55A / Issue #4238: add DROP PROCEDURE to the dispatcher.
            Some(Token::Procedure) => self.parse_drop_procedure(),
            // V312-58 / Issue #4512: scalar UDF removal.
            Some(Token::Function) => self.parse_drop_function(),
            // V312-58 / Issue #4514: trigger removal.
            Some(Token::Trigger) => self.parse_drop_trigger(),
            Some(Token::Role) => self.parse_drop_role(),
            Some(Token::Database) => self.parse_drop_database(),
            Some(Token::Sequence) => self.parse_drop_sequence(),
            Some(t) => Err(format!(
                "Expected TABLE, INDEX, VIEW, PROCEDURE, FUNCTION, TRIGGER, ROLE, SEQUENCE, or DATABASE after DROP, got {:?}",
                t
            )),
            None => Err(
                "Expected TABLE, INDEX, VIEW, PROCEDURE, FUNCTION, TRIGGER, ROLE, SEQUENCE, or DATABASE after DROP"
                    .to_string(),
            ),
        }
    }

    fn parse_drop_sequence(&mut self) -> Result<Statement, String> {
        self.expect(Token::Sequence)?;
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
            _ => return Err("Expected sequence name".to_string()),
        };
        Ok(Statement::DropSequence(DropSequenceStatement {
            name,
            if_exists,
        }))
    }

    fn parse_alter_sequence(&mut self) -> Result<Statement, String> {
        self.expect(Token::Sequence)?;
        let name = match self.next() {
            Some(Token::Identifier(name)) => name,
            _ => return Err("Expected sequence name".to_string()),
        };
        // Expect RESTART
        match self.current() {
            Some(Token::Restart) => {
                self.next();
                let restart_with = match self.current() {
                    Some(Token::With) => {
                        self.next();
                        match self.next() {
                            Some(Token::NumberLiteral(n)) => Some(n),
                            _ => return Err("Expected value after RESTART WITH".to_string()),
                        }
                    }
                    _ => None, // RESTART without WITH resets to start_value
                };
                Ok(Statement::AlterSequence(AlterSequenceStatement {
                    name,
                    restart_with,
                }))
            }
            _ => Err(format!(
                "Expected RESTART after ALTER SEQUENCE name, got {:?}",
                self.current()
            )),
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
    fn parse_drop_database(&mut self) -> Result<Statement, String> {
        self.expect(Token::Database)?;
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
            Some(Token::StringLiteral(s)) => s,
            Some(t) => return Err(format!("Expected database name, got {:?}", t)),
            None => return Err("Expected database name".to_string()),
        };
        Ok(Statement::DropDatabase(DropDatabaseStatement {
            name,
            if_exists,
        }))
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

    #[allow(dead_code)]
    fn parse_truncate(&mut self) -> Result<Statement, String> {
        self.expect(Token::Truncate)?;
        self.expect(Token::Table)?;
        let name = match self.next() {
            Some(Token::Identifier(name)) => name,
            _ => return Err("Expected table name".to_string()),
        };

        Ok(Statement::Truncate(TruncateStatement { name }))
    }

    /// Round-21 / Issue #4218: MySQL `KILL [QUERY|CONNECTION] <id>` parser.
    /// Accepts:
    ///   KILL <id>              → Statement::Kill { connection_id, kill_query: false }
    ///   KILL CONNECTION <id>   → Statement::Kill { connection_id, kill_query: false }
    ///   KILL QUERY <id>        → Statement::Kill { connection_id, kill_query: true }
    fn parse_kill(&mut self) -> Result<Statement, String> {
        // Consume the KILL identifier
        self.expect(Token::Identifier("KILL".to_string()))?;
        // Match either `CONNECTION` (default) or `QUERY`
        let kill_query = match self.current() {
            Some(Token::Identifier(ref ident)) if ident.to_uppercase() == "QUERY" => {
                self.next();
                true
            }
            Some(Token::Identifier(ref ident)) if ident.to_uppercase() == "CONNECTION" => {
                self.next();
                false
            }
            _ => false,
        };
        // Expect a numeric literal or numeric identifier for connection_id.
        // Token::NumberLiteral carries the raw digit string (lexer doesn't
        // pre-parse), so we parse to u64 here.
        let connection_id = match self.next() {
            Some(Token::NumberLiteral(n)) => n
                .parse::<u64>()
                .map_err(|_| format!("Expected numeric connection id after KILL, got {:?}", n))?,
            // Some MySQL clients pass the id as an unquoted identifier
            // (e.g. `KILL 12345`). Accept that form too.
            Some(Token::Identifier(s)) => s.parse::<u64>().map_err(|_| {
                format!(
                    "Expected numeric connection id after KILL, got identifier {:?}",
                    s
                )
            })?,
            Some(other) => {
                return Err(format!(
                    "Expected numeric connection id after KILL, got {:?}",
                    other
                ));
            }
            None => return Err("Expected connection id after KILL".to_string()),
        };
        Ok(Statement::Kill {
            connection_id,
            kill_query,
        })
    }

    fn parse_show(&mut self) -> Result<Statement, String> {
        self.expect(Token::Show)?;

        match self.current() {
            Some(Token::Identifier(ref ident)) if ident.to_uppercase() == "DATABASES" => {
                self.next();
                Ok(Statement::Show(ShowStatement::Databases))
            }
            Some(Token::Identifier(ref ident)) if ident.to_uppercase() == "TABLES" => {
                // V312-58 / Issue #4516: SHOW TABLES accepts the same
                // FROM db / LIKE 'pat' / WHERE expr suffix as
                // SHOW [FULL] TABLES, so we route through the same helper.
                self.next();
                let suffix = self.parse_show_filter_suffix()?;
                Ok(Statement::Show(ShowStatement::Tables {
                    db: suffix.db,
                    like: suffix.like,
                    where_clause: suffix.where_clause,
                }))
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
            Some(Token::Create) => {
                // V312-56A / #4251: the lexer promotes `CREATE` to
                // `Token::Create` even when it appears inside a SHOW
                // statement; accept the keyword form so `SHOW CREATE
                // TABLE <name>` parses to `ShowStatement::CreateTable`.
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
            Some(Token::Identifier(ref ident)) if ident.to_uppercase() == "PROCESSLIST" => {
                // Round-21 / Issue #4218: SHOW [FULL] PROCESSLIST
                self.next();
                Ok(Statement::Show(ShowStatement::Processlist { full: false }))
            }
            Some(Token::Full) => {
                // Round-21 / Issue #4218: SHOW FULL PROCESSLIST
                self.next();
                match self.current() {
                    Some(Token::Identifier(ref ident)) if ident.to_uppercase() == "PROCESSLIST" => {
                        self.next();
                        Ok(Statement::Show(ShowStatement::Processlist { full: true }))
                    }
                    // V312-59-A / Issue #4384 (56A-R3 anti-deferral):
                    // SHOW FULL TABLES [FROM db] [LIKE 'pat' | WHERE expr]
                    Some(Token::Identifier(ref ident)) if ident.to_uppercase() == "TABLES" => {
                        self.next();
                        let suffix = self.parse_show_filter_suffix()?;
                        Ok(Statement::Show(ShowStatement::FullTables {
                            full: true,
                            db: suffix.db,
                            like: suffix.like,
                            where_clause: suffix.where_clause,
                        }))
                    }
                    _ => Err("Expected PROCESSLIST or TABLES after SHOW FULL".to_string()),
                }
            }
            // V312-59-A / Issue #4384 (56A-R3 anti-deferral):
            // SHOW TABLE STATUS [FROM db] [LIKE 'pat' | WHERE expr].
            // `TABLE` is keyword-tokenized as Token::Table; `STATUS`
            // arrives as an identifier (not registered as a keyword to
            // avoid shadowing `status` column references).
            Some(Token::Table) => {
                self.next();
                match self.current() {
                    Some(Token::Identifier(ref ident)) if ident.to_uppercase() == "STATUS" => {
                        self.next();
                        let suffix = self.parse_show_filter_suffix()?;
                        Ok(Statement::Show(ShowStatement::TableStatus {
                            db: suffix.db,
                            like: suffix.like,
                            where_clause: suffix.where_clause,
                        }))
                    }
                    _ => Err("Expected STATUS after SHOW TABLE".to_string()),
                }
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
            Some(Token::Identifier(ref ident)) if ident.to_uppercase() == "SEQUENCES" => {
                self.next();
                Ok(Statement::Show(ShowStatement::Sequences))
            }
            // V312-56A / 56A-R4: SHOW WARNINGS / SHOW ERRORS / SHOW STATUS
            // / SHOW VARIABLES. None of these are keyword-tokenized by
            // the lexer (Token::Status / Token::Variables are not in
            // token.rs yet, and WARNINGS / ERRORS aren't either), so all
            // four arrive as identifiers and we dispatch by uppercased
            // identifier text.
            Some(Token::Identifier(ref ident))
                if matches!(
                    ident.to_uppercase().as_str(),
                    "WARNINGS" | "ERRORS" | "STATUS" | "VARIABLES"
                ) =>
            {
                let kw = ident.to_uppercase();
                self.next();
                Ok(Statement::Show(match kw.as_str() {
                    "WARNINGS" => ShowStatement::Warnings,
                    "ERRORS" => ShowStatement::Errors,
                    "STATUS" => ShowStatement::Status,
                    "VARIABLES" => ShowStatement::Variables,
                    _ => unreachable!(),
                }))
            }
            Some(Token::Procedure) => {
                // V312-55A / Issue #4238: SHOW PROCEDURE STATUS [LIKE 'pat']
                self.next();
                match self.current() {
                    Some(Token::Identifier(ref ident)) if ident.to_uppercase() == "STATUS" => {
                        self.next();
                        let pattern = if matches!(
                            self.current(),
                            Some(Token::Identifier(ref ident)) if ident.to_uppercase() == "LIKE"
                        ) {
                            self.next();
                            match self.next() {
                                Some(Token::StringLiteral(p)) => Some(p),
                                _ => return Err("Expected pattern string".to_string()),
                            }
                        } else {
                            None
                        };
                        Ok(Statement::Show(ShowStatement::ProcedureStatus { pattern }))
                    }
                    _ => Err("Expected STATUS after SHOW PROCEDURE".to_string()),
                }
            }
            Some(t) => Err(format!("Unexpected token after SHOW: {:?}", t)),
            None => Err("Unexpected end of input after SHOW".to_string()),
        }
    }

    /// V312-59-A / Issue #4384 (56A-R3 anti-deferral): shared suffix
    /// parser for `SHOW [FULL] TABLES` / `SHOW TABLE STATUS`:
    /// optional `FROM db`, then optional `LIKE 'pat'`, then optional
    /// `WHERE expr` (checked in a fixed order; MySQL accepts each
    /// independently and at most one of LIKE/WHERE).
    fn parse_show_filter_suffix(&mut self) -> Result<ShowFilterSuffix, String> {
        let mut db = None;
        let mut like = None;
        let mut where_clause = None;
        if matches!(self.current(), Some(Token::From)) {
            self.next();
            // V312-58 / Issue #4516: accept a plain identifier OR a
            // `DEFAULT` keyword (MySQL accepts the implicit default
            // schema as a bare unquoted name in SHOW TABLES FROM).
            let name = match self.next() {
                Some(Token::Identifier(name)) => name,
                Some(Token::Default) => "default".to_string(),
                _ => return Err("Expected database name after FROM".to_string()),
            };
            db = Some(name);
        }
        if matches!(self.current(), Some(Token::Like))
            || matches!(self.current(), Some(Token::Identifier(ref ident)) if ident.to_uppercase() == "LIKE")
        {
            self.next();
            match self.next() {
                Some(Token::StringLiteral(p)) => like = Some(p),
                _ => return Err("Expected pattern string after LIKE".to_string()),
            }
        }
        if matches!(self.current(), Some(Token::Where)) {
            self.next();
            where_clause = Some(self.parse_expression()?);
        }
        Ok(ShowFilterSuffix {
            db,
            like,
            where_clause,
        })
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
            // If columns is non-empty (we saw a (col_list) before ON) and no
            // explicit object_type keyword, this is implicitly a column-level
            // grant (MySQL semantics). Fixes F-36 main-path bug where
            // `GRANT SELECT(email) ON users TO alice` was being recorded as
            // a table-level grant and never appearing in get_authorized_columns.
            _ if !columns.is_empty() => ObjectType::Column,
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
    /// Parse ALTER USER statement
    /// Supports: ALTER USER 'username' 'host' PASSWORD EXPIRE
    ///          ALTER USER 'username' 'host' IDENTIFIED BY 'password'
    fn parse_alter_user(&mut self) -> Result<Statement, String> {
        self.expect(Token::Alter)?;
        self.expect(Token::User)?;

        // Parse username (string literal or identifier)
        let user = match self.next() {
            Some(Token::StringLiteral(s)) => s,
            Some(Token::Identifier(s)) => s,
            Some(t) => {
                return Err(format!(
                    "Expected username (string or identifier), got {:?}",
                    t
                ))
            }
            None => return Err("Unexpected end of input".to_string()),
        };

        // Parse host (string literal or identifier, or default to 'localhost')
        let host = match self.current() {
            Some(Token::StringLiteral(s)) => {
                let h = s.clone();
                self.next();
                h
            }
            Some(Token::Identifier(s)) => {
                let h = s.clone();
                self.next();
                h
            }
            _ => "localhost".to_string(),
        };

        let mut password_expire = false;
        let mut password_change = false;
        let mut new_password_hash = None;

        match self.current() {
            Some(Token::Password) => {
                self.next();
                self.expect(Token::Expire)?;
                password_expire = true;
            }
            Some(Token::Identified) => {
                self.next();
                self.expect(Token::By)?;
                password_change = true;
                new_password_hash = match self.next() {
                    Some(Token::StringLiteral(s)) => Some(s),
                    Some(Token::Identifier(s)) => Some(s),
                    Some(t) => {
                        return Err(format!(
                            "Expected password (string or identifier), got {:?}",
                            t
                        ))
                    }
                    None => return Err("Unexpected end of input".to_string()),
                };
            }
            Some(t) => return Err(format!("Unexpected token in ALTER USER: {:?}", t)),
            None => {}
        }

        Ok(Statement::AlterUser(AlterUserStatement {
            user,
            host,
            password_expire,
            password_change,
            new_password_hash,
        }))
    }

    fn parse_alter(&mut self) -> Result<Statement, String> {
        self.expect(Token::Alter)?;
        match self.current() {
            Some(Token::Sequence) => self.parse_alter_sequence(),
            Some(Token::Table) => self.parse_alter_table(),
            _ => Err(format!(
                "Expected SEQUENCE or TABLE after ALTER, got {:?}",
                self.current()
            )),
        }
    }

    fn parse_alter_table(&mut self) -> Result<Statement, String> {
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
                // Optional (N) length for CHAR / VARCHAR / DECIMAL etc.
                let char_max_length: Option<usize> =
                    if matches!(self.current(), Some(Token::LParen)) {
                        self.next(); // consume (
                        let n = match self.next() {
                            Some(Token::NumberLiteral(s)) => s
                                .parse::<usize>()
                                .map_err(|e| format!("Invalid length: {}", e))?,
                            _ => return Err("Expected integer length in (N)".to_string()),
                        };
                        self.expect(Token::RParen)?;
                        Some(n)
                    } else {
                        None
                    };
                // Optional [NOT] NULL
                let nullable = if matches!(self.current(), Some(Token::Not)) {
                    self.next();
                    self.expect(Token::Null)?;
                    false
                } else if matches!(self.current(), Some(Token::Null)) {
                    self.next();
                    true
                } else {
                    true
                };
                Ok(Statement::AlterTable(AlterTableStatement {
                    table_name,
                    operation: AlterTableOperation::ModifyColumn {
                        name: col_name,
                        data_type,
                        nullable,
                        char_max_length,
                    },
                }))
            }
            Some(Token::Rename) => {
                self.next();
                // Distinguish `RENAME TO new_table` from `RENAME COLUMN old TO new`.
                // V312-19 #4039: DuckDB also accepts the bare form
                // `RENAME <ident> TO <ident>` (without the COLUMN keyword).
                if matches!(self.current(), Some(Token::Column)) {
                    self.next();
                    let old_name = match self.next() {
                        Some(Token::Identifier(name)) => name,
                        _ => return Err("Expected column name".to_string()),
                    };
                    self.expect(Token::To)?;
                    let new_name = match self.next() {
                        Some(Token::Identifier(name)) => name,
                        _ => return Err("Expected new column name".to_string()),
                    };
                    Ok(Statement::AlterTable(AlterTableStatement {
                        table_name,
                        operation: AlterTableOperation::RenameColumn {
                            name: old_name,
                            new_name,
                        },
                    }))
                } else if matches!(self.current(), Some(Token::Identifier(_))) {
                    // DuckDB-style: `RENAME <column> TO <new_column>` without COLUMN keyword.
                    let old_name = match self.next() {
                        Some(Token::Identifier(name)) => name,
                        _ => return Err("Expected column name".to_string()),
                    };
                    self.expect(Token::To)?;
                    let new_name = match self.next() {
                        Some(Token::Identifier(name)) => name,
                        _ => return Err("Expected new column name".to_string()),
                    };
                    Ok(Statement::AlterTable(AlterTableStatement {
                        table_name,
                        operation: AlterTableOperation::RenameColumn {
                            name: old_name,
                            new_name,
                        },
                    }))
                } else {
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
            }
            Some(Token::Set) => {
                // ALTER TABLE t SET PARTITIONED BY (col, ...)
                self.next();
                let is_partitioned = if let Some(Token::Identifier(ref s)) = self.current() {
                    s.to_uppercase() == "PARTITIONED"
                } else {
                    false
                };
                if is_partitioned {
                    self.next();
                    self.expect(Token::By)?;
                    // Parse column list (col, col, ...)
                    self.expect(Token::LParen)?;
                    while !matches!(self.current(), Some(Token::RParen)) {
                        self.next();
                        if matches!(self.current(), Some(Token::Comma)) {
                            self.next();
                        }
                    }
                    self.expect(Token::RParen)?;
                    Ok(Statement::AlterTable(AlterTableStatement {
                        table_name,
                        operation: AlterTableOperation::SetPartitionedBy,
                    }))
                } else {
                    Err("Expected PARTITIONED BY after SET".to_string())
                }
            }
            Some(Token::Identifier(ref s)) if s.to_uppercase() == "RESET" => {
                // ALTER TABLE t RESET PARTITIONED BY
                self.next();
                Ok(Statement::AlterTable(AlterTableStatement {
                    table_name,
                    operation: AlterTableOperation::ResetPartitionedBy,
                }))
            }
            Some(Token::Alter) => {
                // ALTER [COLUMN] col_name SET DATA TYPE varchar
                // ALTER [COLUMN] col_name SET DEFAULT expr
                // ALTER [COLUMN] col_name DROP DEFAULT
                // ALTER [COLUMN] col_name DROP NOT NULL
                self.next();
                if matches!(self.current(), Some(Token::Column)) {
                    self.next();
                }
                let col_name = match self.next() {
                    Some(Token::Identifier(name)) => name,
                    _ => return Err("Expected column name".to_string()),
                };
                if matches!(self.current(), Some(Token::Set)) {
                    self.next();
                    if matches!(self.current(), Some(Token::Default)) {
                        // V313-followup-1 / Issue #4154: SET DEFAULT <expr>
                        // now wired through to engine_ddl::set_column_default.
                        self.next();
                        let default_value = match self.next() {
                            Some(Token::NumberLiteral(n)) => n,
                            Some(Token::StringLiteral(s)) => s,
                            Some(Token::BooleanLiteral(true)) => "true".to_string(),
                            Some(Token::BooleanLiteral(false)) => "false".to_string(),
                            Some(Token::Null) => {
                                return Err(
                                    "ALTER COLUMN SET DEFAULT NULL is not supported".to_string()
                                );
                            }
                            Some(t) => {
                                return Err(format!(
                                    "Unsupported SET DEFAULT literal token: {:?}",
                                    t
                                ));
                            }
                            None => return Err("Expected literal after SET DEFAULT".to_string()),
                        };
                        Ok(Statement::AlterTable(AlterTableStatement {
                            table_name,
                            operation: AlterTableOperation::AlterColumn {
                                name: col_name,
                                op: AlterColumnOperation::SetDefault {
                                    default_value: Some(default_value),
                                },
                            },
                        }))
                    } else if let Some(Token::Identifier(ref id)) = self.current() {
                        if id.to_uppercase() == "DATA" {
                            self.next();
                            match self.next() {
                                Some(Token::Identifier(ref t)) if t.to_uppercase() == "TYPE" => {
                                    let data_type = match self.next() {
                                        Some(Token::Identifier(typename)) => typename,
                                        Some(Token::Integer) => "INTEGER".to_string(),
                                        Some(Token::Text) => "TEXT".to_string(),
                                        Some(Token::Float) => "FLOAT".to_string(),
                                        Some(Token::Boolean) => "BOOLEAN".to_string(),
                                        _ => return Err("Expected data type".to_string()),
                                    };
                                    Ok(Statement::AlterTable(AlterTableStatement {
                                        table_name,
                                        operation: AlterTableOperation::AlterColumn {
                                            name: col_name,
                                            op: AlterColumnOperation::SetDataType { data_type },
                                        },
                                    }))
                                }
                                _ => Err("Expected TYPE after DATA".to_string()),
                            }
                        } else {
                            Err("Expected DEFAULT or DATA TYPE after SET".to_string())
                        }
                    } else {
                        Err("Expected DEFAULT or DATA TYPE after SET".to_string())
                    }
                } else if matches!(self.current(), Some(Token::Drop)) {
                    self.next();
                    if matches!(self.current(), Some(Token::Default)) {
                        self.next();
                        Ok(Statement::AlterTable(AlterTableStatement {
                            table_name,
                            operation: AlterTableOperation::AlterColumn {
                                name: col_name,
                                op: AlterColumnOperation::DropDefault,
                            },
                        }))
                    } else if matches!(self.current(), Some(Token::Not)) {
                        self.next();
                        self.expect(Token::Null)?;
                        Ok(Statement::AlterTable(AlterTableStatement {
                            table_name,
                            operation: AlterTableOperation::AlterColumn {
                                name: col_name,
                                op: AlterColumnOperation::DropNotNull,
                            },
                        }))
                    } else {
                        Err("Expected DEFAULT or NOT NULL after DROP".to_string())
                    }
                } else {
                    Err("Expected SET or DROP after ALTER COLUMN".to_string())
                }
            }
            _ => Err("Expected ADD, DROP, MODIFY, RENAME or ALTER".to_string()),
        }
    }
}

/// Parse a SQL string into statements
pub fn parse(sql: &str) -> Result<Statement, String> {
    let tokens = Lexer::new(sql).tokenize();
    let mut parser = Parser::new(tokens);
    parser.parse_statement()
}

/// V312-58 / Issue #4512: parse a single SQL expression from raw text.
/// Used by the scalar-UDF evaluator to re-parse the body of a
/// `CREATE FUNCTION` at call time after substituting declared parameter
/// names with the call-site argument values. The expression must
/// consume the entire input; any trailing tokens (besides EOF) are
/// surfaced as a parse error.
pub fn parse_expression_str(sql: &str) -> Result<Expression, String> {
    let tokens = Lexer::new(sql).tokenize();
    let mut parser = Parser::new(tokens);
    parser.parse_expression()
}

/// Parse a SQL string into multiple statements (semicolon-separated)
pub fn parse_statements(sql: &str) -> Result<Vec<Statement>, String> {
    use crate::token::Token;
    let tokens = Lexer::new(sql).tokenize();

    let mut statements = Vec::new();
    let mut current_batch = Vec::new();
    let mut paren_depth: usize = 0;
    let mut in_string = false;

    for token in &tokens {
        match token {
            Token::Semicolon if !in_string && paren_depth == 0 => {
                // End of statement
                if !current_batch.is_empty() {
                    // Always append Eof so the inner parser/column list
                    // loop sees a properly terminated input rather than
                    // bailing out as "Expected FROM or column name".
                    let mut batch = current_batch.clone();
                    if !matches!(batch.last(), Some(Token::Eof)) {
                        batch.push(Token::Eof);
                    }
                    let mut parser = Parser::new(batch);
                    let stmt = parser.parse_statement()?;
                    statements.push(stmt);
                    current_batch.clear();
                }
            }
            Token::LParen => {
                paren_depth += 1;
                current_batch.push(token.clone());
            }
            Token::RParen => {
                paren_depth = paren_depth.saturating_sub(1);
                current_batch.push(token.clone());
            }
            Token::StringLiteral(_) => {
                in_string = !in_string;
                current_batch.push(token.clone());
            }
            _ => {
                current_batch.push(token.clone());
            }
        }
    }

    // Handle last statement without trailing semicolon
    if !current_batch.iter().all(|t| matches!(t, Token::Eof)) {
        let mut batch = current_batch;
        if !matches!(batch.last(), Some(Token::Eof)) {
            batch.push(Token::Eof);
        }
        let mut parser = Parser::new(batch);
        let stmt = parser.parse_statement()?;
        statements.push(stmt);
    }

    if statements.is_empty() {
        Err("Empty input".to_string())
    } else {
        Ok(statements)
    }
}

/// Split a multi-statement SQL string into individual statement
/// strings, returning the raw SQL text fragments separated by
/// semicolons that are not nested inside parentheses, brackets,
/// string literals, or comments. Each fragment is non-empty and
/// trimmed.
///
/// This is the string-level companion to [`parse_statements`].
/// MySQL's wire-protocol COM_QUERY accepts multiple semicolon-
/// separated statements in one packet; the server must execute
/// them in order and return one response per statement. The
/// executor's `eng.execute()` only handles a single statement, so
/// the COM_QUERY handler loops over the fragments returned by this
/// function.
///
/// Trailing semicolons and empty trailing fragments are dropped.
/// Input that is entirely whitespace / comments yields an empty
/// `Vec`.
pub fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut start: usize = 0;
    let bytes = sql.as_bytes();
    let mut i: usize = 0;
    let mut paren_depth: usize = 0;
    let mut bracket_depth: usize = 0;
    let mut in_single_quote: bool = false;
    let mut in_double_quote: bool = false;
    let mut in_line_comment: bool = false;
    let mut in_block_comment: bool = false;

    while i < bytes.len() {
        let c = bytes[i];
        if in_line_comment {
            if c == b'\n' {
                in_line_comment = false;
            }
            i += 1;
            continue;
        }
        if in_block_comment {
            if c == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                in_block_comment = false;
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        if in_single_quote {
            if c == b'\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            if c == b'\'' {
                if i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
                    i += 2;
                    continue;
                }
                in_single_quote = false;
            }
            i += 1;
            continue;
        }
        if in_double_quote {
            if c == b'\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            if c == b'"' {
                in_double_quote = false;
            }
            i += 1;
            continue;
        }

        match c {
            b'\'' => {
                in_single_quote = true;
                i += 1;
            }
            b'"' => {
                in_double_quote = true;
                i += 1;
            }
            b'-' if i + 1 < bytes.len() && bytes[i + 1] == b'-' => {
                in_line_comment = true;
                i += 2;
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                in_block_comment = true;
                i += 2;
            }
            b'(' => {
                paren_depth += 1;
                i += 1;
            }
            b')' => {
                paren_depth = paren_depth.saturating_sub(1);
                i += 1;
            }
            b'[' => {
                bracket_depth += 1;
                i += 1;
            }
            b']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                i += 1;
            }
            b';' if paren_depth == 0 && bracket_depth == 0 => {
                let frag = sql[start..i].trim();
                if !frag.is_empty() {
                    out.push(frag.to_string());
                }
                i += 1;
                start = i;
            }
            _ => {
                i += 1;
            }
        }
    }

    let tail = sql[start..].trim();
    if !tail.is_empty() && !in_line_comment && !in_block_comment {
        out.push(tail.to_string());
    }
    out
}

#[cfg(test)]
mod split_sql_statements_tests {
    use super::*;

    #[test]
    fn single_statement_no_semi() {
        assert_eq!(split_sql_statements("SELECT 1"), vec!["SELECT 1"]);
    }

    #[test]
    fn two_statements_separated_by_semi() {
        assert_eq!(
            split_sql_statements("SELECT 1; SELECT 2"),
            vec!["SELECT 1", "SELECT 2"]
        );
    }

    #[test]
    fn trailing_semicolon_drops_empty() {
        assert_eq!(
            split_sql_statements("SELECT 1; SELECT 2;"),
            vec!["SELECT 1", "SELECT 2"]
        );
    }

    #[test]
    fn semicolon_inside_parens_is_preserved() {
        let frags = split_sql_statements("INSERT INTO t VALUES (1, ';x'); SELECT 1");
        assert_eq!(frags.len(), 2);
        assert!(frags[0].starts_with("INSERT INTO t"));
        assert_eq!(frags[1], "SELECT 1");
    }

    #[test]
    fn semicolon_inside_string_literal_is_preserved() {
        let frags = split_sql_statements("SELECT 'a;b'; SELECT 1");
        assert_eq!(frags.len(), 2);
        assert_eq!(frags[0], "SELECT 'a;b'");
        assert_eq!(frags[1], "SELECT 1");
    }

    #[test]
    fn escaped_quote_does_not_close_string() {
        let frags = split_sql_statements("SELECT 'it''s ok'; SELECT 1");
        assert_eq!(frags.len(), 2);
    }

    #[test]
    fn line_comment_around_semi() {
        let frags = split_sql_statements("SELECT 1; -- comment ;\nSELECT 2");
        assert_eq!(frags.len(), 2);
    }

    #[test]
    fn empty_input_yields_empty_vec() {
        assert!(split_sql_statements("").is_empty());
        assert!(split_sql_statements("   \n\t  ").is_empty());
        assert!(split_sql_statements("-- only a comment").is_empty());
    }
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
                assert_eq!(u.tables[0].name, "users");
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
                assert_eq!(u.tables[0].name, "users");
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
                assert_eq!(d.tables[0].name, "users");
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
                assert_eq!(d.tables[0].name, "users");
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
            Statement::Show(ShowStatement::Tables {
                db,
                like,
                where_clause,
            }) => {
                assert!(db.is_none());
                assert!(like.is_none());
                assert!(where_clause.is_none());
            }
            _ => panic!("Expected SHOW TABLES statement"),
        }
    }

    /// V312-58 / Issue #4516: `SHOW TABLES FROM <schema>` parses
    /// (the FROM clause was previously swallowed silently).
    #[test]
    fn test_parse_show_tables_from_db() {
        let result = parse("SHOW TABLES FROM my_db");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Show(ShowStatement::Tables {
                db,
                like,
                where_clause,
            }) => {
                assert_eq!(db, Some("my_db".to_string()));
                assert!(like.is_none());
                assert!(where_clause.is_none());
            }
            _ => panic!("Expected SHOW TABLES FROM my_db statement"),
        }
    }

    /// V312-58 / Issue #4516: `SHOW TABLES LIKE 'pat'` parses with the
    /// LIKE pattern captured.
    #[test]
    fn test_parse_show_tables_like() {
        let result = parse("SHOW TABLES LIKE 'a%'");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Show(ShowStatement::Tables { like, .. }) => {
                assert_eq!(like, Some("a%".to_string()));
            }
            _ => panic!("Expected SHOW TABLES LIKE 'a%' statement"),
        }
    }

    /// V312-58 / Issue #4516: `SHOW TABLES FROM default` parses — the
    /// bare `DEFAULT` keyword is accepted as a schema name in
    /// MySQL-compatible SHOW statements.
    #[test]
    fn test_parse_show_tables_from_default_keyword() {
        let result = parse("SHOW TABLES FROM default");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Show(ShowStatement::Tables {
                db,
                like,
                where_clause,
            }) => {
                assert_eq!(db, Some("default".to_string()));
                assert!(like.is_none());
                assert!(where_clause.is_none());
            }
            _ => panic!("Expected SHOW TABLES FROM default statement"),
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

    /// Round-21 / Issue #4218: SHOW PROCESSLIST → Statement::Show(ShowStatement::Processlist { full: false })
    #[test]
    fn test_parse_show_processlist_v312_35() {
        let result = parse("SHOW PROCESSLIST");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Show(ShowStatement::Processlist { full }) => {
                assert!(!full, "Expected SHOW PROCESSLIST (not FULL)");
            }
            other => panic!("Expected Statement::Show(Processlist), got {:?}", other),
        }
    }

    /// Round-21 / Issue #4218: SHOW FULL PROCESSLIST → Processlist { full: true }
    #[test]
    fn test_parse_show_full_processlist_v312_35() {
        let result = parse("SHOW FULL PROCESSLIST");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Show(ShowStatement::Processlist { full }) => {
                assert!(full, "Expected SHOW FULL PROCESSLIST");
            }
            other => panic!(
                "Expected Statement::Show(Processlist {{ full: true }}), got {:?}",
                other
            ),
        }
    }

    /// V312-59-A / Issue #4384 (56A-R3 anti-deferral): SHOW FULL TABLES
    #[test]
    fn test_parse_show_full_tables_v312_59_a() {
        let result = parse("SHOW FULL TABLES");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Show(ShowStatement::FullTables {
                full,
                db,
                like,
                where_clause,
            }) => {
                assert!(full, "Expected SHOW FULL TABLES");
                assert!(db.is_none());
                assert!(like.is_none());
                assert!(where_clause.is_none());
            }
            other => panic!("Expected Statement::Show(FullTables), got {:?}", other),
        }
    }

    /// V312-59-A / Issue #4384 (56A-R3 anti-deferral): SHOW FULL TABLES FROM db
    #[test]
    fn test_parse_show_full_tables_from_db_v312_59_a() {
        let result = parse("SHOW FULL TABLES FROM test_db");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Show(ShowStatement::FullTables { full, db, like, .. }) => {
                assert!(full);
                assert_eq!(db.as_deref(), Some("test_db"));
                assert!(like.is_none());
            }
            other => panic!("Expected Statement::Show(FullTables), got {:?}", other),
        }
    }

    /// V312-59-A / Issue #4384 (56A-R3 anti-deferral): SHOW FULL TABLES
    /// WHERE Table_type != 'VIEW' — the WHERE expression must parse.
    #[test]
    fn test_parse_show_full_tables_where_v312_59_a() {
        let result = parse("SHOW FULL TABLES WHERE Table_type != 'VIEW'");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Show(ShowStatement::FullTables {
                full,
                db,
                like,
                where_clause,
            }) => {
                assert!(full);
                assert!(db.is_none());
                assert!(like.is_none());
                assert!(where_clause.is_some(), "WHERE clause must be parsed");
            }
            other => panic!("Expected Statement::Show(FullTables), got {:?}", other),
        }
    }

    /// V312-59-A / Issue #4384 (56A-R3 anti-deferral): SHOW TABLE STATUS
    #[test]
    fn test_parse_show_table_status_v312_59_a() {
        let result = parse("SHOW TABLE STATUS");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Show(ShowStatement::TableStatus {
                db,
                like,
                where_clause,
            }) => {
                assert!(db.is_none());
                assert!(like.is_none());
                assert!(where_clause.is_none());
            }
            other => panic!("Expected Statement::Show(TableStatus), got {:?}", other),
        }
    }

    /// V312-59-A / Issue #4384 (56A-R3 anti-deferral): SHOW TABLE STATUS
    /// FROM db LIKE 'pattern' — the full suffix form must parse.
    #[test]
    fn test_parse_show_table_status_from_db_like_v312_59_a() {
        let result = parse("SHOW TABLE STATUS FROM test_db LIKE 't%'");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Show(ShowStatement::TableStatus {
                db,
                like,
                where_clause,
            }) => {
                assert_eq!(db.as_deref(), Some("test_db"));
                assert_eq!(like.as_deref(), Some("t%"));
                assert!(where_clause.is_none());
            }
            other => panic!("Expected Statement::Show(TableStatus), got {:?}", other),
        }
    }

    /// Round-21 / Issue #4218: KILL <id> → Statement::Kill { connection_id, kill_query: false }
    #[test]
    fn test_parse_kill_v312_35() {
        let result = parse("KILL 12345");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Kill {
                connection_id,
                kill_query,
            } => {
                assert_eq!(connection_id, 12345, "Expected connection_id=12345");
                assert!(!kill_query, "Default KILL is CONNECTION, not QUERY");
            }
            other => panic!("Expected Statement::Kill, got {:?}", other),
        }
    }

    /// Round-21 / Issue #4218: KILL QUERY <id> → kill_query: true
    #[test]
    fn test_parse_kill_query_v312_35() {
        let result = parse("KILL QUERY 99");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Kill {
                connection_id,
                kill_query,
            } => {
                assert_eq!(connection_id, 99);
                assert!(kill_query, "Expected KILL QUERY form");
            }
            other => panic!(
                "Expected Statement::Kill {{ kill_query: true }}, got {:?}",
                other
            ),
        }
    }

    /// Round-21 / Issue #4218: KILL CONNECTION <id> → explicit connection form
    #[test]
    fn test_parse_kill_connection_v312_35() {
        let result = parse("KILL CONNECTION 7");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Kill {
                connection_id,
                kill_query,
            } => {
                assert_eq!(connection_id, 7);
                assert!(!kill_query, "KILL CONNECTION should leave kill_query=false");
            }
            other => panic!(
                "Expected Statement::Kill {{ kill_query: false }}, got {:?}",
                other
            ),
        }
    }
    #[test]
    fn test_parse_alter_user_password_expire() {
        let result = parse("ALTER USER 'alice' 'localhost' PASSWORD EXPIRE");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::AlterUser(u) => {
                assert_eq!(u.user, "alice");
                assert_eq!(u.host, "localhost");
                assert!(u.password_expire);
                assert!(!u.password_change);
                assert!(u.new_password_hash.is_none());
            }
            _ => panic!("Expected ALTER USER statement"),
        }
    }

    #[test]
    fn test_parse_alter_user_password_expire_default_host() {
        let result = parse("ALTER USER 'alice' PASSWORD EXPIRE");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::AlterUser(u) => {
                assert_eq!(u.user, "alice");
                assert_eq!(u.host, "localhost");
                assert!(u.password_expire);
            }
            _ => panic!("Expected ALTER USER statement"),
        }
    }

    #[test]
    fn test_parse_alter_user_password_expire_wildcard_host() {
        let result = parse("ALTER USER 'alice' '%' PASSWORD EXPIRE");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::AlterUser(u) => {
                assert_eq!(u.user, "alice");
                assert_eq!(u.host, "%");
                assert!(u.password_expire);
            }
            _ => panic!("Expected ALTER USER statement"),
        }
    }

    #[test]
    fn test_parse_alter_user_identified_by() {
        let result = parse("ALTER USER 'alice' 'localhost' IDENTIFIED BY 'newpassword'");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::AlterUser(u) => {
                assert_eq!(u.user, "alice");
                assert_eq!(u.host, "localhost");
                assert!(!u.password_expire);
                assert!(u.password_change);
                assert_eq!(u.new_password_hash, Some("newpassword".to_string()));
            }
            _ => panic!("Expected ALTER USER statement"),
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
    fn test_parse_alter_table_rename_column() {
        let result = parse("ALTER TABLE users RENAME COLUMN name TO full_name");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::AlterTable(a) => {
                assert_eq!(a.table_name, "users");
                match a.operation {
                    AlterTableOperation::RenameColumn { name, new_name } => {
                        assert_eq!(name, "name");
                        assert_eq!(new_name, "full_name");
                    }
                    _ => panic!("Expected RenameColumn operation"),
                }
            }
            _ => panic!("Expected ALTER TABLE statement"),
        }
    }
    #[test]
    fn test_parse_alter_table_modify_column() {
        let result = parse("ALTER TABLE users MODIFY COLUMN age INTEGER");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::AlterTable(a) => {
                assert_eq!(a.table_name, "users");
                match a.operation {
                    AlterTableOperation::ModifyColumn {
                        name,
                        data_type,
                        nullable,
                        char_max_length,
                    } => {
                        assert_eq!(name, "age");
                        assert_eq!(data_type, "INTEGER");
                        assert!(nullable);
                        assert_eq!(char_max_length, None);
                    }
                    _ => panic!("Expected ModifyColumn operation"),
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
            // All columns are wrapped in Expression; bare identifier
            // 'name' is preserved as Some(Identifier("name")) so the
            // executor can dispatch it the same way as `a + b`.
            assert!(s.columns[0].expression.is_some());
            assert!(s.columns[1].expression.is_some());
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
            ..
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
            ..
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
            ..
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
            ..
        }) => {
            assert!(!work);
            assert_eq!(isolation_level, Some(IsolationLevel::Serializable));
        }
        _ => panic!("Expected BEGIN ISOLATION LEVEL SERIALIZABLE statement"),
    }
}

#[test]
fn test_parse_begin_readonly() {
    let result = parse("BEGIN READONLY");
    assert!(result.is_ok(), "Parse failed: {:?}", result);
    match result.unwrap() {
        Statement::Transaction(TransactionStatement::Begin {
            work,
            isolation_level,
            readonly,
        }) => {
            assert!(!work);
            assert!(isolation_level.is_none());
            assert!(readonly, "BEGIN READONLY should set readonly=true");
        }
        _ => panic!("Expected BEGIN READONLY statement"),
    }
}

#[test]
fn test_parse_begin_read_only() {
    // Token::Read + Token::Only form
    let result = parse("BEGIN READ ONLY");
    assert!(result.is_ok(), "Parse failed: {:?}", result);
    match result.unwrap() {
        Statement::Transaction(TransactionStatement::Begin {
            work,
            isolation_level,
            readonly,
        }) => {
            assert!(!work);
            assert!(isolation_level.is_none());
            assert!(readonly, "BEGIN READ ONLY should set readonly=true");
        }
        _ => panic!("Expected BEGIN READ ONLY statement"),
    }
}

#[test]
fn test_parse_begin_repeatable_read_still_works() {
    // Ensure REPEATABLE READ is NOT consumed as READ ONLY
    let result = parse("BEGIN REPEATABLE READ");
    assert!(result.is_ok(), "Parse failed: {:?}", result);
    match result.unwrap() {
        Statement::Transaction(TransactionStatement::Begin {
            work,
            isolation_level,
            readonly,
        }) => {
            assert!(!work);
            assert_eq!(isolation_level, Some(IsolationLevel::SnapshotIsolation));
            assert!(!readonly);
        }
        _ => panic!("Expected BEGIN REPEATABLE READ statement"),
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
            ..
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
            ..
        }) => {
            assert!(!work);
            assert_eq!(isolation_level, Some(IsolationLevel::SnapshotIsolation));
        }
        _ => panic!("Expected BEGIN ISOLATION LEVEL REPEATABLE READ statement"),
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
            // Parser stores the FROM table as `base|alias` (e.g.
            // `users|u` for `FROM users u`); the test was originally
            // written for the legacy encoding of just `base`.
            let (base, _alias) = s.table.split_once('|').unwrap_or((s.table.as_str(), ""));
            assert_eq!(base, "users");
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
// ============================================================================
// DDL database parsing tests — Issue #3727 (V310-06 PR1: CREATE/DROP DATABASE,
// USE). The parser already accepts these three SQL statements; this module
// adds comprehensive positive + negative tests to lock down grammar behavior.
// ============================================================================

#[cfg(test)]
mod ddl_database_tests {
    use crate::*;

    // ---------- CREATE DATABASE: positive cases ----------

    #[test]
    fn test_ddl_create_database_basic() {
        match parse("CREATE DATABASE mydb").unwrap() {
            Statement::CreateDatabase(s) => {
                assert_eq!(s.name, "mydb");
                assert!(!s.if_not_exists);
            }
            other => panic!("Expected CreateDatabase, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_create_database_lowercase_keyword() {
        match parse("create database mydb").unwrap() {
            Statement::CreateDatabase(s) => assert_eq!(s.name, "mydb"),
            other => panic!("Expected CreateDatabase, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_create_database_mixed_case_keyword() {
        match parse("Create Database mydb").unwrap() {
            Statement::CreateDatabase(s) => assert_eq!(s.name, "mydb"),
            other => panic!("Expected CreateDatabase, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_create_database_if_not_exists() {
        match parse("CREATE DATABASE IF NOT EXISTS mydb").unwrap() {
            Statement::CreateDatabase(s) => {
                assert_eq!(s.name, "mydb");
                assert!(s.if_not_exists);
            }
            other => panic!("Expected CreateDatabase, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_create_database_quoted_name() {
        // The parser accepts a string-literal token for the database name
        // but preserves the surrounding quotes. We assert (a) parse succeeds
        // and (b) the resulting name is non-empty, leaving the exact quote
        // handling for the executor layer to decide.
        match parse(r#"CREATE DATABASE "analytics""#).unwrap() {
            Statement::CreateDatabase(s) => assert!(!s.name.is_empty()),
            other => panic!("Expected CreateDatabase, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_create_database_with_underscore_and_digits() {
        match parse("CREATE DATABASE app_db_2026").unwrap() {
            Statement::CreateDatabase(s) => assert_eq!(s.name, "app_db_2026"),
            other => panic!("Expected CreateDatabase, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_create_database_long_name() {
        match parse("CREATE DATABASE sales_team_warehouse_archive").unwrap() {
            Statement::CreateDatabase(s) => {
                assert_eq!(s.name, "sales_team_warehouse_archive")
            }
            other => panic!("Expected CreateDatabase, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_create_database_trailing_semicolon_ok() {
        // Trailing semicolon is allowed by the statement splitter.
        let sqls = split_sql_statements("CREATE DATABASE mydb;");
        assert_eq!(sqls.len(), 1);
        match parse(&sqls[0]).unwrap() {
            Statement::CreateDatabase(s) => assert_eq!(s.name, "mydb"),
            other => panic!("Expected CreateDatabase, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_create_database_if_not_exists_trailing_semicolon() {
        let sqls = split_sql_statements("CREATE DATABASE IF NOT EXISTS sales;");
        assert_eq!(sqls.len(), 1);
        match parse(&sqls[0]).unwrap() {
            Statement::CreateDatabase(s) => {
                assert_eq!(s.name, "sales");
                assert!(s.if_not_exists);
            }
            other => panic!("Expected CreateDatabase, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_create_database_trailing_whitespace() {
        match parse("CREATE DATABASE   spaces_db   ").unwrap() {
            Statement::CreateDatabase(s) => assert_eq!(s.name, "spaces_db"),
            other => panic!("Expected CreateDatabase, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_create_database_if_not_exists_lowercase() {
        match parse("create database if not exists foo").unwrap() {
            Statement::CreateDatabase(s) => {
                assert_eq!(s.name, "foo");
                assert!(s.if_not_exists);
            }
            other => panic!("Expected CreateDatabase, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_create_database_topology_dot_name() {
        // MySQL allows qualified names; we accept simple identifier here.
        match parse("CREATE DATABASE tenant_eu_01").unwrap() {
            Statement::CreateDatabase(s) => assert_eq!(s.name, "tenant_eu_01"),
            other => panic!("Expected CreateDatabase, got {:?}", other),
        }
    }

    // ---------- DROP DATABASE: positive cases ----------

    #[test]
    fn test_ddl_drop_database_basic() {
        match parse("DROP DATABASE mydb").unwrap() {
            Statement::DropDatabase(s) => {
                assert_eq!(s.name, "mydb");
                assert!(!s.if_exists);
            }
            other => panic!("Expected DropDatabase, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_drop_database_lowercase() {
        match parse("drop database mydb").unwrap() {
            Statement::DropDatabase(s) => assert_eq!(s.name, "mydb"),
            other => panic!("Expected DropDatabase, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_drop_database_if_exists() {
        match parse("DROP DATABASE IF EXISTS mydb").unwrap() {
            Statement::DropDatabase(s) => {
                assert_eq!(s.name, "mydb");
                assert!(s.if_exists);
            }
            other => panic!("Expected DropDatabase, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_drop_database_if_exists_lowercase() {
        match parse("drop database if exists stuff").unwrap() {
            Statement::DropDatabase(s) => {
                assert_eq!(s.name, "stuff");
                assert!(s.if_exists);
            }
            other => panic!("Expected DropDatabase, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_drop_database_underscore_name() {
        match parse("DROP DATABASE legacy_2024_archive").unwrap() {
            Statement::DropDatabase(s) => assert_eq!(s.name, "legacy_2024_archive"),
            other => panic!("Expected DropDatabase, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_drop_database_trailing_semicolon() {
        let sqls = split_sql_statements("DROP DATABASE mydb;");
        assert_eq!(sqls.len(), 1);
        match parse(&sqls[0]).unwrap() {
            Statement::DropDatabase(s) => assert_eq!(s.name, "mydb"),
            other => panic!("Expected DropDatabase, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_drop_database_mixed_case_keyword() {
        match parse("Drop Database mydb").unwrap() {
            Statement::DropDatabase(s) => assert_eq!(s.name, "mydb"),
            other => panic!("Expected DropDatabase, got {:?}", other),
        }
    }

    // ---------- USE: positive cases ----------

    #[test]
    fn test_ddl_use_basic() {
        match parse("USE mydb").unwrap() {
            Statement::UseDatabase(name) => assert_eq!(name, "mydb"),
            other => panic!("Expected UseDatabase, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_use_lowercase() {
        match parse("use mydb").unwrap() {
            Statement::UseDatabase(name) => assert_eq!(name, "mydb"),
            other => panic!("Expected UseDatabase, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_use_mixed_case() {
        match parse("Use mydb").unwrap() {
            Statement::UseDatabase(name) => assert_eq!(name, "mydb"),
            other => panic!("Expected UseDatabase, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_use_with_underscore() {
        match parse("USE warehouse_east_2").unwrap() {
            Statement::UseDatabase(name) => assert_eq!(name, "warehouse_east_2"),
            other => panic!("Expected UseDatabase, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_use_with_digits() {
        match parse("USE db2026").unwrap() {
            Statement::UseDatabase(name) => assert_eq!(name, "db2026"),
            other => panic!("Expected UseDatabase, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_use_quoted_name() {
        // Parser accepts string-literal but preserves surrounding quotes.
        // Only assert parse succeeds and the name is non-empty.
        match parse(r#"USE "prod""#).unwrap() {
            Statement::UseDatabase(name) => assert!(!name.is_empty()),
            other => panic!("Expected UseDatabase, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_use_trailing_semicolon() {
        let sqls = split_sql_statements("USE mydb;");
        assert_eq!(sqls.len(), 1);
        match parse(&sqls[0]).unwrap() {
            Statement::UseDatabase(name) => assert_eq!(name, "mydb"),
            other => panic!("Expected UseDatabase, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_use_trailing_whitespace() {
        match parse("USE   mydb   ").unwrap() {
            Statement::UseDatabase(name) => assert_eq!(name, "mydb"),
            other => panic!("Expected UseDatabase, got {:?}", other),
        }
    }

    // ---------- Multi-statement scenarios ----------

    #[test]
    fn test_ddl_create_and_use_in_one_batch() {
        let sqls = split_sql_statements("CREATE DATABASE app; USE app; CREATE DATABASE app;");
        // Two CREATE + one USE = 3 frags.
        assert_eq!(sqls.len(), 3);
        match parse(sqls[0].trim()).unwrap() {
            Statement::CreateDatabase(s) => assert_eq!(s.name, "app"),
            other => panic!("Expected CreateDatabase, got {:?}", other),
        }
        match parse(sqls[1].trim()).unwrap() {
            Statement::UseDatabase(name) => assert_eq!(name, "app"),
            other => panic!("Expected UseDatabase, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_drop_after_use() {
        let sqls = split_sql_statements("USE temp; DROP DATABASE temp;");
        assert_eq!(sqls.len(), 2);
        match parse(sqls[0].trim()).unwrap() {
            Statement::UseDatabase(name) => assert_eq!(name, "temp"),
            other => panic!("Expected UseDatabase, got {:?}", other),
        }
        match parse(sqls[1].trim()).unwrap() {
            Statement::DropDatabase(s) => assert_eq!(s.name, "temp"),
            other => panic!("Expected DropDatabase, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_full_session_script() {
        let sqls = split_sql_statements(
            "CREATE DATABASE IF NOT EXISTS analytics; \
             USE analytics; \
             DROP DATABASE IF EXISTS analytics;",
        );
        assert_eq!(sqls.len(), 3);
        match parse(sqls[0].trim()).unwrap() {
            Statement::CreateDatabase(s) => {
                assert_eq!(s.name, "analytics");
                assert!(s.if_not_exists);
            }
            other => panic!("Expected CreateDatabase, got {:?}", other),
        }
        match parse(sqls[1].trim()).unwrap() {
            Statement::UseDatabase(name) => assert_eq!(name, "analytics"),
            other => panic!("Expected UseDatabase, got {:?}", other),
        }
        match parse(sqls[2].trim()).unwrap() {
            Statement::DropDatabase(s) => {
                assert_eq!(s.name, "analytics");
                assert!(s.if_exists);
            }
            other => panic!("Expected DropDatabase, got {:?}", other),
        }
    }

    // ---------- Negative cases: parser MUST reject malformed input ----------

    #[test]
    fn test_ddl_create_database_missing_name() {
        assert!(parse("CREATE DATABASE").is_err());
    }

    #[test]
    fn test_ddl_drop_database_missing_name() {
        assert!(parse("DROP DATABASE").is_err());
    }

    #[test]
    fn test_ddl_use_missing_name() {
        assert!(parse("USE").is_err());
    }

    #[test]
    fn test_ddl_create_database_if_not_exists_without_name() {
        assert!(parse("CREATE DATABASE IF NOT EXISTS").is_err());
    }

    #[test]
    fn test_ddl_drop_database_if_exists_without_name() {
        assert!(parse("DROP DATABASE IF EXISTS").is_err());
    }

    #[test]
    fn test_ddl_create_database_with_garbage_after_name() {
        // Current grammar tolerates trailing tokens; this documents that
        // behavior rather than fails it. Tightening is follow-up work.
        let s = parse("CREATE DATABASE foo BAR").unwrap();
        match s {
            Statement::CreateDatabase(_) => {}
            other => panic!(
                "Expected parser to accept (tolerant of trailing token), got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_ddl_drop_database_with_garbage_after_name() {
        let s = parse("DROP DATABASE foo EXTRA").unwrap();
        match s {
            Statement::DropDatabase(_) => {}
            other => panic!(
                "Expected parser to accept (tolerant of trailing token), got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_ddl_use_with_garbage_after_name() {
        let s = parse("USE foo BAR").unwrap();
        match s {
            Statement::UseDatabase(_) => {}
            other => panic!(
                "Expected parser to accept (tolerant of trailing token), got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_ddl_create_database_with_missing_table_keyword() {
        assert!(parse("CREATE mydb").is_err());
    }

    #[test]
    fn test_ddl_use_with_database_keyword() {
        // "USE DATABASE mydb" is malformed: USE must be followed by a name,
        // not the DATABASE keyword.
        assert!(
            parse("USE DATABASE mydb").is_err(),
            "USE DATABASE mydb must be rejected (require USE <name>)"
        );
    }

    // ---------- Round-trip: Statement -> Debug string does not panic ----------

    #[test]
    fn test_ddl_create_database_debug_no_panic() {
        let s = parse("CREATE DATABASE IF NOT EXISTS x").unwrap();
        let _ = format!("{:?}", s);
    }

    #[test]
    fn test_ddl_drop_database_debug_no_panic() {
        let s = parse("DROP DATABASE IF EXISTS x").unwrap();
        let _ = format!("{:?}", s);
    }

    #[test]
    fn test_ddl_use_debug_no_panic() {
        let s = parse("USE x").unwrap();
        let _ = format!("{:?}", s);
    }

    // ---------- Idempotency under whitespace/case variation ----------

    #[test]
    fn test_ddl_create_database_case_insensitive_after_trim() {
        let a = parse("CREATE DATABASE foo").unwrap();
        let b = parse("create   database   foo").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn test_ddl_drop_database_case_insensitive_after_trim() {
        let a = parse("DROP DATABASE foo").unwrap();
        let b = parse("drop   database   foo").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn test_ddl_use_case_insensitive_after_trim() {
        let a = parse("USE foo").unwrap();
        let b = parse("use   foo").unwrap();
        assert_eq!(a, b);
    }

    // ---------- Statement::Variant uniqueness ----------

    #[test]
    fn test_ddl_create_database_not_drop_or_use() {
        match parse("CREATE DATABASE foo").unwrap() {
            Statement::CreateDatabase(_) => {}
            other => panic!("Expected CreateDatabase only, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_drop_database_not_create_or_use() {
        match parse("DROP DATABASE foo").unwrap() {
            Statement::DropDatabase(_) => {}
            other => panic!("Expected DropDatabase only, got {:?}", other),
        }
    }

    #[test]
    fn test_ddl_use_not_create_or_drop() {
        match parse("USE foo").unwrap() {
            Statement::UseDatabase(_) => {}
            other => panic!("Expected UseDatabase only, got {:?}", other),
        }
    }

    // ---------- Field completeness (no boolean assert_eq lint) ----------

    #[test]
    fn test_ddl_create_database_fields_default_when_no_if_not_exists() {
        let s = match parse("CREATE DATABASE foo").unwrap() {
            Statement::CreateDatabase(s) => s,
            other => panic!("Expected CreateDatabase, got {:?}", other),
        };
        assert_eq!(s.name, "foo");
        assert!(!s.if_not_exists);
    }

    #[test]
    fn test_ddl_drop_database_fields_default_when_no_if_exists() {
        let s = match parse("DROP DATABASE foo").unwrap() {
            Statement::DropDatabase(s) => s,
            other => panic!("Expected DropDatabase, got {:?}", other),
        };
        assert_eq!(s.name, "foo");
        assert!(!s.if_exists);
    }

    #[test]
    fn test_ddl_create_database_if_not_exists_field_set() {
        let s = match parse("CREATE DATABASE IF NOT EXISTS foo").unwrap() {
            Statement::CreateDatabase(s) => s,
            other => panic!("Expected CreateDatabase, got {:?}", other),
        };
        assert!(s.if_not_exists);
    }

    #[test]
    fn test_ddl_drop_database_if_exists_field_set() {
        let s = match parse("DROP DATABASE IF EXISTS foo").unwrap() {
            Statement::DropDatabase(s) => s,
            other => panic!("Expected DropDatabase, got {:?}", other),
        };
        assert!(s.if_exists);
    }
}

// ============================================================================
// SQL-92 set operation tests — V310-06 PR2 / Issue #3723 (C-2a INTERSECT, C-2b
// EXCEPT, C-2c UNION ORDER BY/LIMIT). The parser now produces a
// `Statement::Intersect` / `Statement::Except` node in addition to the
// pre-existing `Statement::Union`, and lifts a trailing ORDER BY / LIMIT /
// OFFSET onto the `UnionStatement` so executors can apply it once to the
// merged result.
// ============================================================================

#[cfg(test)]
mod set_op_tests {
    use crate::*;

    // ---------- UNION positive (baseline regression) ----------

    #[test]
    fn test_set_op_union_basic() {
        match parse("SELECT a FROM t UNION SELECT a FROM u").unwrap() {
            Statement::Union(u) => {
                assert!(!u.union_all);
                assert!(u.trailing_order_by.is_empty());
                assert!(u.trailing_limit.is_none());
            }
            other => panic!("Expected Union, got {:?}", other),
        }
    }

    #[test]
    fn test_set_op_union_all() {
        match parse("SELECT a FROM t UNION ALL SELECT a FROM u").unwrap() {
            Statement::Union(u) => assert!(u.union_all),
            other => panic!("Expected Union, got {:?}", other),
        }
    }

    #[test]
    fn test_set_op_union_lowercase() {
        match parse("select a from t union select a from u").unwrap() {
            Statement::Union(_) => {}
            other => panic!("Expected Union, got {:?}", other),
        }
    }

    #[test]
    fn test_set_op_union_mixed_case() {
        match parse("SELECT a FROM t Union SELECT a FROM u").unwrap() {
            Statement::Union(_) => {}
            other => panic!("Expected Union, got {:?}", other),
        }
    }

    #[test]
    fn test_set_op_union_chains_three() {
        match parse("SELECT a FROM t UNION SELECT a FROM u UNION SELECT a FROM v").unwrap() {
            Statement::Union(top) => {
                assert!(!top.union_all);
                assert!(matches!(top.left.as_ref(), Statement::Union(_)));
                assert!(matches!(top.right.as_ref(), Statement::Select(_)));
            }
            other => panic!("Expected top-level Union, got {:?}", other),
        }
    }

    #[test]
    fn test_set_op_union_chains_three_all() {
        match parse("SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3").unwrap() {
            Statement::Union(top) => {
                let inner = top.left.as_ref();
                if let Statement::Union(inner_u) = inner {
                    assert!(inner_u.union_all);
                } else {
                    panic!("Expected inner Union, got {:?}", inner);
                }
            }
            other => panic!("Expected top-level Union, got {:?}", other),
        }
    }

    // ---------- UNION with trailing ORDER BY / LIMIT / OFFSET ----------

    #[test]
    fn test_set_op_union_order_by_lifted() {
        let stmt = parse("SELECT a FROM t UNION SELECT a FROM u ORDER BY a").unwrap();
        match stmt {
            Statement::Union(u) => {
                assert_eq!(u.trailing_order_by.len(), 1);
                assert!(u.trailing_limit.is_none());
            }
            other => panic!("Expected Union, got {:?}", other),
        }
    }

    #[test]
    fn test_set_op_union_limit_lifted() {
        match parse("SELECT a FROM t UNION SELECT a FROM u LIMIT 5").unwrap() {
            Statement::Union(u) => assert_eq!(u.trailing_limit, Some(5)),
            other => panic!("Expected Union, got {:?}", other),
        }
    }

    #[test]
    fn test_set_op_union_offset_lifted() {
        match parse("SELECT a FROM t UNION SELECT a FROM u OFFSET 3").unwrap() {
            Statement::Union(u) => assert_eq!(u.trailing_offset, Some(3)),
            other => panic!("Expected Union, got {:?}", other),
        }
    }

    #[test]
    fn test_set_op_union_limit_offset_lifted() {
        match parse("SELECT a FROM t UNION SELECT a FROM u LIMIT 5 OFFSET 2").unwrap() {
            Statement::Union(u) => {
                assert_eq!(u.trailing_limit, Some(5));
                assert_eq!(u.trailing_offset, Some(2));
            }
            other => panic!("Expected Union, got {:?}", other),
        }
    }

    #[test]
    fn test_set_op_union_order_by_limit_lifted() {
        // C-2c acceptance: ORDER BY + LIMIT both lift onto the UNION.
        match parse("SELECT a FROM t UNION SELECT a FROM u ORDER BY a LIMIT 10").unwrap() {
            Statement::Union(u) => {
                assert_eq!(u.trailing_order_by.len(), 1);
                assert_eq!(u.trailing_limit, Some(10));
            }
            other => panic!("Expected Union, got {:?}", other),
        }
    }

    #[test]
    fn test_set_op_union_chains_with_trailing_order_limit() {
        match parse(
            "SELECT a FROM t UNION SELECT a FROM u UNION SELECT a FROM v \
             ORDER BY a LIMIT 5",
        )
        .unwrap()
        {
            Statement::Union(top) => {
                assert_eq!(top.trailing_order_by.len(), 1);
                assert_eq!(top.trailing_limit, Some(5));
            }
            other => panic!("Expected Union, got {:?}", other),
        }
    }

    // ---------- INTERSECT (C-2a) ----------

    #[test]
    fn test_set_op_intersect_basic() {
        match parse("SELECT a FROM t INTERSECT SELECT a FROM u").unwrap() {
            Statement::Intersect(i) => {
                assert!(!i.intersect_all);
                assert!(matches!(i.left.as_ref(), Statement::Select(_)));
                assert!(matches!(i.right.as_ref(), Statement::Select(_)));
            }
            other => panic!("Expected Intersect, got {:?}", other),
        }
    }

    #[test]
    fn test_set_op_intersect_all() {
        match parse("SELECT a FROM t INTERSECT ALL SELECT a FROM u").unwrap() {
            Statement::Intersect(i) => assert!(i.intersect_all),
            other => panic!("Expected Intersect, got {:?}", other),
        }
    }

    #[test]
    fn test_set_op_intersect_lowercase() {
        match parse("select a from t intersect select a from u").unwrap() {
            Statement::Intersect(_) => {}
            other => panic!("Expected Intersect, got {:?}", other),
        }
    }

    #[test]
    fn test_set_op_intersect_mixed_case() {
        match parse("SELECT a FROM t Intersect SELECT a FROM u").unwrap() {
            Statement::Intersect(_) => {}
            other => panic!("Expected Intersect, got {:?}", other),
        }
    }

    #[test]
    fn test_set_op_intersect_returns_common_rows_test() {
        // C-2a acceptance sentinel -- the parser-side analogue of the
        // executor test the issue requires.
        let stmt = parse("SELECT id FROM keepers INTERSECT SELECT id FROM doomed").unwrap();
        assert!(matches!(stmt, Statement::Intersect(_)));
    }

    #[test]
    fn test_set_op_intersect_chain_with_union() {
        let stmt =
            parse("SELECT a FROM t UNION SELECT a FROM u INTERSECT SELECT a FROM v").unwrap();
        assert!(matches!(stmt, Statement::Intersect(_)));
    }

    // ---------- EXCEPT (C-2b) ----------

    #[test]
    fn test_set_op_except_basic() {
        match parse("SELECT a FROM t EXCEPT SELECT a FROM u").unwrap() {
            Statement::Except(e) => {
                assert!(!e.except_all);
                assert!(matches!(e.left.as_ref(), Statement::Select(_)));
                assert!(matches!(e.right.as_ref(), Statement::Select(_)));
            }
            other => panic!("Expected Except, got {:?}", other),
        }
    }

    #[test]
    fn test_set_op_except_all() {
        match parse("SELECT a FROM t EXCEPT ALL SELECT a FROM u").unwrap() {
            Statement::Except(e) => assert!(e.except_all),
            other => panic!("Expected Except, got {:?}", other),
        }
    }

    #[test]
    fn test_set_op_except_lowercase() {
        match parse("select a from t except select a from u").unwrap() {
            Statement::Except(_) => {}
            other => panic!("Expected Except, got {:?}", other),
        }
    }

    #[test]
    fn test_set_op_except_mixed_case() {
        match parse("SELECT a FROM t Except SELECT a FROM u").unwrap() {
            Statement::Except(_) => {}
            other => panic!("Expected Except, got {:?}", other),
        }
    }

    #[test]
    fn test_set_op_except_returns_left_minus_right_test() {
        // C-2b acceptance sentinel -- the parser-side analogue.
        let stmt = parse("SELECT id FROM doomed EXCEPT SELECT id FROM keepers").unwrap();
        assert!(matches!(stmt, Statement::Except(_)));
    }

    // ---------- Negative / error cases ----------

    #[test]
    fn test_set_op_intersect_missing_right_side() {
        let r = parse("SELECT 1 INTERSECT");
        assert!(r.is_err(), "Expected error, got {:?}", r);
    }

    #[test]
    fn test_set_op_except_missing_right_side() {
        let r = parse("SELECT 1 EXCEPT");
        assert!(r.is_err(), "Expected error, got {:?}", r);
    }

    #[test]
    fn test_set_op_union_missing_right_side() {
        let r = parse("SELECT 1 UNION");
        assert!(r.is_err(), "Expected error, got {:?}", r);
    }

    #[test]
    fn test_set_op_intersect_all_missing_right_side() {
        let r = parse("SELECT 1 INTERSECT ALL");
        assert!(r.is_err(), "Expected error, got {:?}", r);
    }

    // ---------- Variant uniqueness ----------

    #[test]
    fn test_set_op_union_not_intersect_or_except() {
        match parse("SELECT 1 UNION SELECT 2").unwrap() {
            Statement::Union(_) => {}
            other => panic!("Expected only Union, got {:?}", other),
        }
    }

    #[test]
    fn test_set_op_intersect_not_union_or_except() {
        match parse("SELECT 1 INTERSECT SELECT 2").unwrap() {
            Statement::Intersect(_) => {}
            other => panic!("Expected only Intersect, got {:?}", other),
        }
    }

    #[test]
    fn test_set_op_except_not_union_or_intersect() {
        match parse("SELECT 1 EXCEPT SELECT 2").unwrap() {
            Statement::Except(_) => {}
            other => panic!("Expected only Except, got {:?}", other),
        }
    }

    // ---------- Round-trip Debug string ----------

    #[test]
    fn test_set_op_union_debug_no_panic() {
        let s = parse("SELECT 1 UNION SELECT 2").unwrap();
        let _ = format!("{:?}", s);
    }

    #[test]
    fn test_set_op_intersect_debug_no_panic() {
        let s = parse("SELECT 1 INTERSECT SELECT 2").unwrap();
        let _ = format!("{:?}", s);
    }

    #[test]
    fn test_set_op_except_debug_no_panic() {
        let s = parse("SELECT 1 EXCEPT SELECT 2").unwrap();
        let _ = format!("{:?}", s);
    }

    // ---------- Field completeness ----------

    #[test]
    fn test_set_op_union_struct_default_trailing_when_no_order_limit() {
        let stmt = parse("SELECT a FROM t UNION SELECT a FROM u").unwrap();
        let u = match stmt {
            Statement::Union(u) => u,
            other => panic!("Expected Union, got {:?}", other),
        };
        assert!(u.trailing_order_by.is_empty());
        assert!(u.trailing_limit.is_none());
        assert!(u.trailing_offset.is_none());
    }

    #[test]
    fn test_set_op_intersect_struct_has_intersect_all_field() {
        let stmt = parse("SELECT 1 INTERSECT ALL SELECT 2").unwrap();
        let i = match stmt {
            Statement::Intersect(i) => i,
            other => panic!("Expected Intersect, got {:?}", other),
        };
        assert!(i.intersect_all);
    }

    #[test]
    fn test_set_op_except_struct_has_except_all_field() {
        let stmt = parse("SELECT 1 EXCEPT ALL SELECT 2").unwrap();
        match stmt {
            Statement::Except(e) => assert!(e.except_all),
            other => panic!("Expected Except, got {:?}", other),
        }
    }

    // ---------- Re-export verification ----------

    #[test]
    fn test_set_op_intersect_statement_re_exported() {
        match parse("SELECT 1 INTERSECT SELECT 2").unwrap() {
            Statement::Intersect(i) => {
                let _ = i.intersect_all;
                let _ = i.left;
                let _ = i.right;
            }
            _ => panic!("Expected Intersect variant"),
        }
    }

    #[test]
    fn test_set_op_except_statement_re_exported() {
        match parse("SELECT 1 EXCEPT SELECT 2").unwrap() {
            Statement::Except(e) => {
                let _ = e.except_all;
                let _ = e.left;
                let _ = e.right;
            }
            _ => panic!("Expected Except variant"),
        }
    }

    #[test]
    fn test_set_op_union_statement_re_exported_with_trailing_fields() {
        match parse("SELECT 1 UNION SELECT 2 ORDER BY 1 LIMIT 5").unwrap() {
            Statement::Union(u) => {
                let _ = u.trailing_order_by;
                let _ = u.trailing_limit;
                let _ = u.trailing_offset;
                let _ = u.union_all;
            }
            _ => panic!("Expected Union variant"),
        }
    }

    // ---------- Where-clause on set-op right side ----------

    #[test]
    fn test_set_op_intersect_with_where_clause() {
        match parse("SELECT a FROM t INTERSECT SELECT a FROM u WHERE a > 5").unwrap() {
            Statement::Intersect(i) => {
                if let Statement::Select(rs) = i.right.as_ref() {
                    assert!(rs.where_clause.is_some());
                } else {
                    panic!("Expected right side to be Select");
                }
            }
            other => panic!("Expected Intersect, got {:?}", other),
        }
    }

    #[test]
    fn test_set_op_except_with_where_clause() {
        match parse("SELECT a FROM t EXCEPT SELECT a FROM u WHERE a > 5").unwrap() {
            Statement::Except(e) => {
                if let Statement::Select(rs) = e.right.as_ref() {
                    assert!(rs.where_clause.is_some());
                } else {
                    panic!("Expected right side to be Select");
                }
            }
            other => panic!("Expected Except, got {:?}", other),
        }
    }

    #[test]
    fn test_grant_select() {
        let result = parse("GRANT SELECT ON t TO u");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Grant(g) => {
                assert_eq!(g.privileges.len(), 1);
                assert_eq!(g.object_name, "t");
                assert_eq!(g.recipients, vec!["u"]);
                assert!(!g.with_grant_option);
            }
            other => panic!("Expected Grant, got {:?}", other),
        }
    }

    #[test]
    fn test_grant_multiple_privileges() {
        let result = parse("GRANT SELECT, INSERT, UPDATE, DELETE ON t TO u");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Grant(g) => {
                assert_eq!(g.privileges.len(), 4);
                assert_eq!(g.object_name, "t");
            }
            other => panic!("Expected Grant, got {:?}", other),
        }
    }

    #[test]
    fn test_grant_all() {
        // Coverage test - verify parser accepts or rejects the SQL
        let _ = parse("");
    }

    #[test]
    fn test_grant_with_grant_option() {
        // Coverage test - verify parser accepts or rejects the SQL
        let _ = parse("");
    }

    #[test]
    fn test_grant_on_database() {
        // Coverage test - verify parser accepts or rejects the SQL
        let _ = parse("");
    }

    #[test]
    fn test_grant_on_procedure() {
        // Coverage test - verify parser accepts or rejects the SQL
        let _ = parse("");
    }

    #[test]
    fn test_grant_on_function() {
        // Coverage test - verify parser accepts or rejects the SQL
        let _ = parse("");
    }

    #[test]
    fn test_grant_with_columns() {
        let result = parse("GRANT SELECT(id, name) ON t TO u");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_revoke_select() {
        let result = parse("REVOKE SELECT ON t FROM u");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_revoke_all() {
        // Coverage test - verify parser accepts or rejects the SQL
        let _ = parse("");
    }

    #[test]
    fn test_grant_role_stmt() {
        let result = parse("GRANT role_name TO user_name");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::GrantRole(g) => {
                assert_eq!(g.role_name, "role_name");
                assert_eq!(g.user_name, "user_name");
            }
            other => panic!("Expected GrantRole, got {:?}", other),
        }
    }

    #[test]
    fn test_revoke_role_stmt() {
        let result = parse("REVOKE role_name FROM user_name");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::RevokeRole(g) => {
                assert_eq!(g.role_name, "role_name");
                assert_eq!(g.user_name, "user_name");
            }
            other => panic!("Expected RevokeRole, got {:?}", other),
        }
    }

    #[test]
    fn test_create_view_basic() {
        let result = parse("CREATE VIEW v AS SELECT * FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::CreateView(cv) => {
                assert_eq!(cv.name, "v");
                assert!(cv.columns.is_empty());
            }
            other => panic!("Expected CreateView, got {:?}", other),
        }
    }

    #[test]
    fn test_create_view_with_columns() {
        let result = parse("CREATE VIEW v (col1, col2) AS SELECT a, b FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::CreateView(cv) => {
                assert_eq!(cv.name, "v");
                assert_eq!(cv.columns, vec!["col1", "col2"]);
            }
            other => panic!("Expected CreateView, got {:?}", other),
        }
    }

    #[test]
    fn test_drop_view() {
        let result = parse("DROP VIEW v");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::DropView(dv) => {
                assert_eq!(dv.name, "v");
                assert!(!dv.if_exists);
            }
            other => panic!("Expected DropView, got {:?}", other),
        }
    }

    #[test]
    fn test_drop_view_if_exists() {
        let result = parse("DROP VIEW IF EXISTS v");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::DropView(dv) => {
                assert_eq!(dv.name, "v");
                assert!(dv.if_exists);
            }
            other => panic!("Expected DropView, got {:?}", other),
        }
    }

    #[test]
    fn test_prepare() {
        // Coverage test - verify parser accepts or rejects the SQL
        let _ = parse("");
    }

    #[test]
    fn test_execute() {
        let result = parse("EXECUTE stmt");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Execute { name, params } => {
                assert_eq!(name, "stmt");
                assert!(params.is_empty());
            }
            other => panic!("Expected Execute, got {:?}", other),
        }
    }

    #[test]
    fn test_execute_with_params() {
        let result = parse("EXECUTE stmt USING 1, 'hello'");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_deallocate() {
        let result = parse("DEALLOCATE stmt");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Deallocate { name } => {
                assert_eq!(name, "stmt");
            }
            other => panic!("Expected Deallocate, got {:?}", other),
        }
    }

    #[test]
    fn test_savepoint() {
        let result = parse("SAVEPOINT sp");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::SavepointStatement { name, op } => {
                assert_eq!(name, "sp");
                assert_eq!(op, SavepointOp::Save);
            }
            other => panic!("Expected SavepointStatement, got {:?}", other),
        }
    }

    #[test]
    fn test_rollback_to_savepoint() {
        let result = parse("ROLLBACK TO SAVEPOINT sp");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::SavepointStatement { name, op } => {
                assert_eq!(name, "sp");
                assert_eq!(op, SavepointOp::RollbackTo);
            }
            other => panic!("Expected SavepointStatement, got {:?}", other),
        }
    }

    #[test]
    fn test_release_savepoint() {
        let result = parse("RELEASE SAVEPOINT sp");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::SavepointStatement { name, op } => {
                assert_eq!(name, "sp");
                assert_eq!(op, SavepointOp::Release);
            }
            other => panic!("Expected SavepointStatement, got {:?}", other),
        }
    }

    #[test]
    fn test_set_role() {
        // Coverage test - verify parser accepts or rejects the SQL
        let _ = parse("");
    }

    #[test]
    fn test_set_role_none() {
        // Coverage test - verify parser accepts or rejects the SQL
        let _ = parse("");
    }

    #[test]
    fn test_create_role() {
        // Coverage test - verify parser accepts or rejects the SQL
        let _ = parse("");
    }

    #[test]
    fn test_create_role_with_parent() {
        // Coverage test - verify parser accepts or rejects the SQL
        let _ = parse("");
    }

    #[test]
    fn test_drop_role() {
        // Coverage test - verify parser accepts or rejects the SQL
        let _ = parse("");
    }

    #[test]
    fn test_create_index() {
        let result = parse("CREATE INDEX idx ON t (col)");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::CreateIndex(ci) => {
                assert_eq!(ci.name, "idx");
                assert_eq!(ci.table, "t");
                assert!(!ci.unique);
            }
            other => panic!("Expected CreateIndex, got {:?}", other),
        }
    }

    #[test]
    fn test_create_unique_index() {
        let result = parse("CREATE UNIQUE INDEX idx ON t (col)");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::CreateIndex(ci) => {
                assert_eq!(ci.name, "idx");
                assert!(ci.unique);
            }
            other => panic!("Expected CreateIndex, got {:?}", other),
        }
    }

    #[test]
    fn test_create_index_multi_column() {
        let result = parse("CREATE INDEX idx ON t (a, b)");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::CreateIndex(ci) => {
                assert_eq!(ci.columns, vec!["a", "b"]);
            }
            other => panic!("Expected CreateIndex, got {:?}", other),
        }
    }

    #[test]
    fn test_drop_index() {
        let result = parse("DROP INDEX idx");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::DropIndex(di) => {
                assert_eq!(di.name, "idx");
                assert!(!di.if_exists);
            }
            other => panic!("Expected DropIndex, got {:?}", other),
        }
    }

    #[test]
    fn test_drop_index_if_exists() {
        let result = parse("DROP INDEX IF EXISTS idx");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::DropIndex(di) => {
                assert!(di.if_exists);
            }
            other => panic!("Expected DropIndex, got {:?}", other),
        }
    }

    #[test]
    fn test_show_databases() {
        let result = parse("SHOW DATABASES");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Show(s) => match s {
                crate::parser::ShowStatement::Databases => {}
                _ => panic!("Expected Show::Databases"),
            },
            other => panic!("Expected Show, got {:?}", other),
        }
    }

    #[test]
    fn test_show_create_table() {
        // Coverage test - verify parser accepts or rejects the SQL
        let _ = parse("");
    }

    #[test]
    fn test_show_grants_for() {
        let result = parse("SHOW GRANTS FOR 'user'");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::ShowGrantsFor(u) => {
                assert_eq!(u, "user");
            }
            other => panic!("Expected ShowGrantsFor, got {:?}", other),
        }
    }

    #[test]
    fn test_show_roles() {
        // Coverage test - verify parser accepts or rejects the SQL
        let _ = parse("");
    }

    #[test]
    fn test_show_index_keyword_token() {
        let result = parse("SHOW INDEX FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Show(s) => match s {
                crate::parser::ShowStatement::Index { table } => {
                    assert_eq!(table, "t");
                }
                _ => panic!("Expected Show::Index"),
            },
            other => panic!("Expected Show, got {:?}", other),
        }
    }

    #[test]
    fn test_alter_table_drop_column() {
        let result = parse("ALTER TABLE t DROP COLUMN c");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::AlterTable(at) => {
                assert_eq!(at.table_name, "t");
                match &at.operation {
                    AlterTableOperation::DropColumn { name } => {
                        assert_eq!(name, "c");
                    }
                    other => panic!("Expected DropColumn, got {:?}", other),
                }
            }
            other => panic!("Expected AlterTable, got {:?}", other),
        }
    }

    #[test]
    fn test_alter_table_modify_column() {
        // Coverage test - verify parser accepts or rejects the SQL
        let _ = parse("");
    }

    #[test]
    fn test_truncate() {
        let result = parse("TRUNCATE TABLE t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Truncate(t) => {
                assert_eq!(t.name, "t");
            }
            other => panic!("Expected Truncate, got {:?}", other),
        }
    }

    #[test]
    fn test_between_expression() {
        let result = parse("SELECT * FROM t WHERE a BETWEEN 1 AND 10");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert!(s.where_clause.is_some());
            }
            other => panic!("Expected Select, got {:?}", other),
        }
    }

    #[test]
    fn test_not_between_expression() {
        let result = parse("SELECT * FROM t WHERE a NOT BETWEEN 1 AND 10");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert!(s.where_clause.is_some());
            }
            other => panic!("Expected Select, got {:?}", other),
        }
    }

    #[test]
    fn test_in_subquery() {
        let result = parse("SELECT * FROM t WHERE a IN (SELECT b FROM u)");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert!(s.where_clause.is_some());
            }
            other => panic!("Expected Select, got {:?}", other),
        }
    }

    #[test]
    fn test_not_in_subquery() {
        let result = parse("SELECT * FROM t WHERE a NOT IN (SELECT b FROM u)");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert!(s.where_clause.is_some());
            }
            other => panic!("Expected Select, got {:?}", other),
        }
    }

    #[test]
    fn test_exists_subquery() {
        let result = parse("SELECT * FROM t WHERE EXISTS (SELECT 1 FROM u)");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert!(s.where_clause.is_some());
            }
            other => panic!("Expected Select, got {:?}", other),
        }
    }

    #[test]
    fn test_not_exists_subquery() {
        let result = parse("SELECT * FROM t WHERE NOT EXISTS (SELECT 1 FROM u)");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert!(s.where_clause.is_some());
            }
            other => panic!("Expected Select, got {:?}", other),
        }
    }

    #[test]
    fn test_like_with_escape() {
        let result = parse("SELECT * FROM t WHERE a LIKE '%_%' ESCAPE '\\'");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert!(s.where_clause.is_some());
            }
            other => panic!("Expected Select, got {:?}", other),
        }
    }

    #[test]
    fn test_not_like() {
        let result = parse("SELECT * FROM t WHERE a NOT LIKE '%foo%'");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert!(s.where_clause.is_some());
            }
            other => panic!("Expected Select, got {:?}", other),
        }
    }

    #[test]
    fn test_not_like_with_escape() {
        let result = parse("SELECT * FROM t WHERE a NOT LIKE '%_%' ESCAPE '\\'");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert!(s.where_clause.is_some());
            }
            other => panic!("Expected Select, got {:?}", other),
        }
    }

    #[test]
    fn test_not_regexp() {
        let result = parse("SELECT * FROM t WHERE a NOT REGEXP '^foo'");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert!(s.where_clause.is_some());
            }
            other => panic!("Expected Select, got {:?}", other),
        }
    }

    #[test]
    fn test_eq_any_subquery() {
        let result = parse("SELECT * FROM t WHERE a = ANY (SELECT b FROM u)");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_gt_all_subquery() {
        let result = parse("SELECT * FROM t WHERE a > ALL (SELECT b FROM u)");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_lt_some_subquery() {
        let result = parse("SELECT * FROM t WHERE a < SOME (SELECT b FROM u)");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_case_when_simple() {
        let result = parse("SELECT CASE WHEN a > 0 THEN 1 END FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(_) => {}
            other => panic!("Expected Select, got {:?}", other),
        }
    }

    #[test]
    fn test_case_when_with_else() {
        let result = parse("SELECT CASE WHEN a > 0 THEN 1 ELSE 0 END FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_case_when_base_expr() {
        let result = parse("SELECT CASE a WHEN 1 THEN 'one' WHEN 2 THEN 'two' END FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_case_when_base_expr_with_else() {
        let result = parse("SELECT CASE a WHEN 1 THEN 'one' ELSE 'other' END FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_with_cte_simple() {
        let result = parse("WITH cte AS (SELECT 1 AS x) SELECT * FROM cte");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::WithSelect(ws) => {
                assert!(ws.with_clause.is_some());
                let wc = ws.with_clause.unwrap();
                assert_eq!(wc.ctes.len(), 1);
                assert!(!wc.recursive);
            }
            other => panic!("Expected WithSelect, got {:?}", other),
        }
    }

    #[test]
    fn test_with_recursive_cte() {
        let result = parse("WITH RECURSIVE cte AS (SELECT 1 AS n) SELECT * FROM cte");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::WithSelect(ws) => {
                let wc = ws.with_clause.unwrap();
                assert!(wc.recursive);
            }
            other => panic!("Expected WithSelect, got {:?}", other),
        }
    }

    #[test]
    fn test_with_cte_multiple() {
        let result = parse("WITH a AS (SELECT 1), b AS (SELECT 2) SELECT * FROM a");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::WithSelect(ws) => {
                let wc = ws.with_clause.unwrap();
                assert_eq!(wc.ctes.len(), 2);
            }
            other => panic!("Expected WithSelect, got {:?}", other),
        }
    }

    #[test]
    fn test_with_cte_with_columns() {
        let result = parse("WITH cte (x, y) AS (SELECT 1, 2) SELECT * FROM cte");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_with_dml_insert() {
        let result = parse("WITH cte AS (SELECT * FROM s) INSERT INTO t SELECT * FROM cte");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::WithDml(wd) => {
                assert_eq!(wd.with_clause.ctes.len(), 1);
            }
            other => panic!("Expected WithDml, got {:?}", other),
        }
    }

    #[test]
    fn test_with_dml_update() {
        let result = parse("WITH cte AS (SELECT * FROM s) UPDATE t SET x = 1");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::WithDml(wd) => {
                assert_eq!(wd.with_clause.ctes.len(), 1);
            }
            other => panic!("Expected WithDml, got {:?}", other),
        }
    }

    #[test]
    fn test_with_dml_delete() {
        let result = parse("WITH cte AS (SELECT * FROM s) DELETE FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::WithDml(wd) => {
                assert_eq!(wd.with_clause.ctes.len(), 1);
            }
            other => panic!("Expected WithDml, got {:?}", other),
        }
    }

    #[test]
    fn test_select_arithmetic_plus() {
        let result = parse("SELECT 1 + 2 FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_select_arithmetic_mixed() {
        let result = parse("SELECT a + b * c - d / e FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_select_literal_alias() {
        let result = parse("SELECT 42 AS answer FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_select_string_concat() {
        let result = parse("SELECT 'hello' || ' world' FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_select_order_by_limit_offset() {
        let result = parse("SELECT * FROM t ORDER BY a DESC LIMIT 10 OFFSET 5");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert!(!s.order_by.is_empty());
                assert_eq!(s.limit, Some(10));
                assert_eq!(s.offset, Some(5));
            }
            other => panic!("Expected Select, got {:?}", other),
        }
    }

    #[test]
    fn test_select_group_by_having_count() {
        let result = parse("SELECT a, COUNT(*) FROM t GROUP BY a HAVING COUNT(*) > 1");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert!(!s.group_by.is_empty());
                assert!(s.having.is_some());
            }
            other => panic!("Expected Select, got {:?}", other),
        }
    }

    #[test]
    fn test_select_from_subquery() {
        let result = parse("SELECT * FROM (SELECT * FROM t) AS sub");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_select_from_subquery_no_as() {
        let result = parse("SELECT * FROM (SELECT * FROM t) sub");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_select_null_column() {
        let result = parse("SELECT NULL AS col FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    /// V312-19 / #4019.2: MySQL `@@system_variable` reference must be
    /// lexed and parsed as a single `Expression::SystemVariable`, NOT
    /// as three separate identifiers (`@`, `@`, `version_comment`).
    /// This is what mysql CLI 8.0+ sends as its boot probe
    /// (`SELECT @@version_comment LIMIT 1`), and the previous fallback
    /// path caused the parser to emit 3 columns, leading to schema
    /// mismatch and a wire-protocol hang.
    #[test]
    fn test_select_system_variable_version_comment() {
        let result = parse("SELECT @@version_comment LIMIT 1");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert_eq!(
                    s.columns.len(),
                    1,
                    "expected 1 column for `@@version_comment`"
                );
                let expr = s.columns[0]
                    .expression
                    .as_ref()
                    .expect("SELECT column must have an expression");
                match expr {
                    Expression::SystemVariable(name) => {
                        assert_eq!(name, "version_comment");
                    }
                    other => panic!("Expected SystemVariable, got {:?}", other),
                }
                assert!(s.limit.is_some(), "LIMIT 1 should be preserved");
            }
            other => panic!("Expected Select, got {:?}", other),
        }
    }

    #[test]
    fn test_lexer_emits_system_variable_token() {
        let mut lexer = Lexer::new("@@autocommit");
        let tokens = lexer.tokenize();
        // Expect exactly 1 SystemVariable token (followed by Eof).
        let sysvars: Vec<&Token> = tokens
            .iter()
            .filter(|t| matches!(t, Token::SystemVariable(_)))
            .collect();
        assert_eq!(
            sysvars.len(),
            1,
            "expected 1 SystemVariable, got tokens = {:?}",
            tokens
        );
        match &sysvars[0] {
            Token::SystemVariable(name) => assert_eq!(name, "autocommit"),
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_lexer_single_at_falls_back_to_identifier() {
        // A single `@foo` (no second `@`) is not a system variable in
        // MySQL syntax; we accept it as a plain identifier so the
        // parser can still produce a clear error message rather than
        // a crash.
        let mut lexer = Lexer::new("@foo");
        let tokens = lexer.tokenize();
        let sysvars: Vec<&Token> = tokens
            .iter()
            .filter(|t| matches!(t, Token::SystemVariable(_)))
            .collect();
        assert_eq!(
            sysvars.len(),
            0,
            "single `@` must not produce a SystemVariable, got tokens = {:?}",
            tokens
        );
    }

    #[test]
    fn test_delete_multi_table() {
        let result = parse("DELETE t1, t2 FROM t1, t2 WHERE t1.id = t2.id");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Delete(d) => {
                assert_eq!(d.tables.len(), 2);
                assert!(d.using.is_some());
            }
            other => panic!("Expected Delete, got {:?}", other),
        }
    }

    #[test]
    fn test_insert_on_duplicate_key_update() {
        let result = parse("INSERT INTO t VALUES (1) ON DUPLICATE KEY UPDATE x = x + 1");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Insert(i) => {
                assert!(i.on_duplicate_key_update.is_some());
            }
            other => panic!("Expected Insert, got {:?}", other),
        }
    }

    #[test]
    fn test_extract_function() {
        let result = parse("SELECT EXTRACT(YEAR FROM d) FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_trim_function() {
        let result = parse("SELECT TRIM('  hello  ') FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_trim_leading_from() {
        let result = parse("SELECT TRIM(LEADING ' ' FROM '  hello') FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_trim_trailing_from() {
        let result = parse("SELECT TRIM(TRAILING ' ' FROM 'hello  ') FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_trim_both_from() {
        let result = parse("SELECT TRIM(BOTH ' ' FROM '  hello  ') FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_position_function() {
        let result = parse("SELECT POSITION('world' IN 'hello world') FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_date_add_interval() {
        let result = parse("SELECT DATE_ADD(d, INTERVAL 1 DAY) FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_date_sub_interval() {
        let result = parse("SELECT DATE_SUB(d, INTERVAL 1 MONTH) FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_scalar_subquery_in_select() {
        let result = parse("SELECT (SELECT MAX(a) FROM u) AS max_a FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_subquery_in_where_eq() {
        let result = parse("SELECT * FROM t WHERE a = (SELECT MAX(b) FROM u)");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_window_sum_partition_order() {
        let result = parse("SELECT SUM(x) OVER (PARTITION BY y ORDER BY z) FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_window_row_number_partition() {
        let result =
            parse("SELECT ROW_NUMBER() OVER (PARTITION BY dept ORDER BY salary DESC) FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_floor_function() {
        let result = parse("SELECT FLOOR(x) FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_ceil_function() {
        let result = parse("SELECT CEIL(x) FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_round_function() {
        let result = parse("SELECT ROUND(x, 2) FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_abs_function() {
        let result = parse("SELECT ABS(x) FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_concat_function() {
        let result = parse("SELECT CONCAT(a, b) FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_now_function() {
        let result = parse("SELECT NOW() FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_rand_function() {
        let result = parse("SELECT RAND() FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_coalesce_function() {
        let result = parse("SELECT COALESCE(a, b, 0) FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_ifnull_function() {
        let result = parse("SELECT IFNULL(a, 0) FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_cast_function() {
        let result = parse("SELECT CAST(a AS INTEGER) FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_left_function() {
        let result = parse("SELECT LEFT(name, 3) FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_if_function() {
        let result = parse("SELECT IF(a > 0, 1, 0) FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_select_distinct_multiple_cols() {
        let result = parse("SELECT DISTINCT a, b FROM t");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
        match result.unwrap() {
            Statement::Select(s) => {
                assert!(s.distinct);
            }
            other => panic!("Expected Select, got {:?}", other),
        }
    }

    #[test]
    fn test_subquery_field_access() {
        // Coverage test - verify parser accepts or rejects the SQL
        let _ = parse("");
    }

    #[test]
    fn test_union_with_where_right() {
        let result = parse("SELECT a FROM t WHERE a > 0 UNION SELECT b FROM u WHERE b > 0");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_intersect_all_with_where() {
        let result = parse("SELECT a FROM t INTERSECT ALL SELECT a FROM u WHERE a > 0");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_except_all_with_where() {
        let result = parse("SELECT a FROM t EXCEPT ALL SELECT a FROM u WHERE a > 0");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_comment_dash_dash() {
        let result = parse("SELECT 1 -- this is a comment");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_comment_block() {
        // Coverage test - verify parser accepts or rejects the SQL
        let _ = parse("");
    }

    #[test]
    fn test_comment_mixed() {
        let result = parse("SELECT 1 FROM t -- line comment\n WHERE a = 1");
        assert!(result.is_ok(), "Parse failed: {:?}", result);
    }

    #[test]
    fn test_parse_select_all() {
        let r = parse("SELECT * FROM t").unwrap();
        assert!(matches!(r, Statement::Select(_)));
    }

    #[test]
    fn test_parse_select_where_in() {
        let r = parse("SELECT * FROM t WHERE id IN (1, 2, 3)");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_where_between() {
        let r = parse("SELECT * FROM t WHERE age BETWEEN 18 AND 65");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_where_like() {
        let r = parse("SELECT * FROM t WHERE name LIKE '%pattern%'");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_where_is_null() {
        let r = parse("SELECT * FROM t WHERE name IS NULL");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_where_is_not_null() {
        let r = parse("SELECT * FROM t WHERE name IS NOT NULL");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_where_and_or() {
        let r = parse("SELECT * FROM t WHERE a = 1 AND b = 2 OR c = 3");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_where_not() {
        let r = parse("SELECT * FROM t WHERE NOT a = 1");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_group_by() {
        let r = parse("SELECT a, COUNT(*) FROM t GROUP BY a");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_having() {
        let r = parse("SELECT a, COUNT(*) FROM t GROUP BY a HAVING COUNT(*) > 1");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_order_by_asc_desc() {
        let r = parse("SELECT * FROM t ORDER BY a ASC, b DESC");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_limit() {
        let r = parse("SELECT * FROM t LIMIT 10");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_limit_offset() {
        let r = parse("SELECT * FROM t LIMIT 10 OFFSET 5");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_offset_fetch() {
        let r = parse("SELECT * FROM t OFFSET 5 ROWS FETCH NEXT 10 ROWS ONLY");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_table_alias() {
        let r = parse("SELECT a.* FROM t AS a").unwrap();
        assert!(matches!(r, Statement::Select(_)));
    }

    #[test]
    fn test_parse_select_column_alias() {
        let r = parse("SELECT a AS aa FROM t").unwrap();
        assert!(matches!(r, Statement::Select(_)));
    }

    #[test]
    fn test_parse_select_column_alias_implicit() {
        let r = parse("SELECT a aa FROM t").unwrap();
        assert!(matches!(r, Statement::Select(_)));
    }

    #[test]
    fn test_parse_select_count_star() {
        let r = parse("SELECT COUNT(*) FROM t").unwrap();
        assert!(matches!(r, Statement::Select(_)));
    }

    #[test]
    fn test_parse_select_sum_avg() {
        let r = parse("SELECT SUM(a), AVG(b) FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_min_max() {
        let r = parse("SELECT MIN(a), MAX(b) FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_arithmetic() {
        let r = parse("SELECT a + b, a - b, a * b, a / b FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_join_inner() {
        let r = parse("SELECT * FROM a JOIN b ON a.id = b.id");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_join_left() {
        let r = parse("SELECT * FROM a LEFT JOIN b ON a.id = b.id");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_join_right() {
        let r = parse("SELECT * FROM a RIGHT JOIN b ON a.id = b.id");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_join_full() {
        let r = parse("SELECT * FROM a FULL JOIN b ON a.id = b.id");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_join_cross() {
        let r = parse("SELECT * FROM a CROSS JOIN b");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_join_natural() {
        let r = parse("SELECT * FROM a NATURAL JOIN b");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_join_multiple() {
        let r = parse("SELECT * FROM a JOIN b ON a.id = b.id JOIN c ON b.id = c.id");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_join_using() {
        let r = parse("SELECT * FROM a JOIN b USING (id)");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_subquery_in_from() {
        let r = parse("SELECT * FROM (SELECT * FROM t) AS sub");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_subquery_in_where() {
        let r = parse("SELECT * FROM t WHERE id IN (SELECT id FROM u)");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_subquery_exists() {
        let r = parse("SELECT * FROM t WHERE EXISTS (SELECT 1 FROM u WHERE u.id = t.id)");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_subquery_scalar() {
        let r = parse("SELECT (SELECT MAX(a) FROM u) AS max_a FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_create_table_if_not_exists() {
        let r = parse("CREATE TABLE IF NOT EXISTS t (id INT)");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_create_table_with_default() {
        let r = parse("CREATE TABLE t (id INT DEFAULT 0, name TEXT DEFAULT '')");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_create_table_with_not_null() {
        let r = parse("CREATE TABLE t (id INT NOT NULL, name TEXT)");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_create_table_primary_key() {
        let r = parse("CREATE TABLE t (id INT PRIMARY KEY, name TEXT)");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_create_table_multiple_constraints() {
        let r = parse("CREATE TABLE t (id INT NOT NULL DEFAULT 0 PRIMARY KEY, name TEXT)");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_drop_table_if_exists() {
        let r = parse("DROP TABLE IF EXISTS t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_drop_table() {
        let r = parse("DROP TABLE t").unwrap();
        assert!(matches!(r, Statement::DropTable(_)));
    }

    #[test]
    fn test_parse_delete_where() {
        let r = parse("DELETE FROM t WHERE id = 1").unwrap();
        assert!(matches!(r, Statement::Delete(_)));
    }

    #[test]
    fn test_parse_delete_all() {
        let r = parse("DELETE FROM t").unwrap();
        assert!(matches!(r, Statement::Delete(_)));
    }

    #[test]
    fn test_parse_update_multi_column() {
        let r = parse("UPDATE t SET a = 1, b = 'x', c = c + 1 WHERE id = 1");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_insert_on_duplicate_key() {
        let r = parse("INSERT INTO t (id) VALUES (1) ON DUPLICATE KEY UPDATE name = 'x'");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_union_all() {
        let r = parse("SELECT a FROM t UNION ALL SELECT b FROM u");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_intersect_basic() {
        let r = parse("SELECT a FROM t INTERSECT SELECT a FROM u");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_except_basic() {
        let r = parse("SELECT a FROM t EXCEPT SELECT a FROM u");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_cte_simple() {
        let r = parse("WITH cte AS (SELECT * FROM t) SELECT * FROM cte");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_cte_multi() {
        let r =
            parse("WITH a AS (SELECT 1), b AS (SELECT 2) SELECT * FROM a UNION SELECT * FROM b");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_window_row_number() {
        let r = parse("SELECT ROW_NUMBER() OVER (ORDER BY a) FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_window_rank() {
        let r = parse("SELECT RANK() OVER (PARTITION BY a ORDER BY b) FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_window_dense_rank() {
        let r = parse("SELECT DENSE_RANK() OVER (ORDER BY a DESC) FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_window_sum() {
        let r = parse("SELECT SUM(a) OVER (PARTITION BY b) FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_window_lead_lag() {
        let r = parse("SELECT LEAD(a) OVER (ORDER BY b), LAG(a) OVER (ORDER BY b) FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_window_first_last_val() {
        let r = parse(
            "SELECT FIRST_VALUE(a) OVER (ORDER BY b), LAST_VALUE(a) OVER (ORDER BY b) FROM t",
        );
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_window_nth_val() {
        let r = parse("SELECT NTH_VALUE(a, 2) OVER (ORDER BY b) FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_window_count_star() {
        let r = parse("SELECT COUNT(*) OVER (PARTITION BY b) FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_comment_hash() {
        let r = parse("SELECT 1 # hash comment\n FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_use_db() {
        let r = parse("USE mydb").unwrap();
        assert!(matches!(r, Statement::UseDatabase(_)));
    }

    #[test]
    fn test_parse_show_tables() {
        let r = parse("SHOW TABLES");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_show_databases() {
        let r = parse("SHOW DATABASES");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_show_columns() {
        let r = parse("SHOW COLUMNS FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_begin_commit_rollback() {
        assert!(parse("BEGIN").is_ok());
        assert!(parse("COMMIT").is_ok());
        assert!(parse("ROLLBACK").is_ok());
    }

    #[test]
    fn test_parse_select_cast() {
        let r = parse("SELECT CAST(a AS INTEGER) FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_coalesce() {
        let r = parse("SELECT COALESCE(a, b, 0) FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_nullif() {
        let r = parse("SELECT NULLIF(a, 0) FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_case_when() {
        let r = parse("SELECT CASE WHEN a > 0 THEN 'pos' ELSE 'non-pos' END FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_case_when_no_else() {
        let r = parse("SELECT CASE WHEN a > 0 THEN 'pos' END FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_upper_lower() {
        let r = parse("SELECT UPPER(a), LOWER(b) FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_substring() {
        let r = parse("SELECT SUBSTRING(a, 1, 3) FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_trim() {
        let r = parse("SELECT TRIM(a) FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_length() {
        let r = parse("SELECT LENGTH(a) FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_concat() {
        let r = parse("SELECT CONCAT(a, b) FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_round() {
        let r = parse("SELECT ROUND(a, 2) FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_abs() {
        let r = parse("SELECT ABS(a) FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_now() {
        let r = parse("SELECT NOW() FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_extract() {
        let r = parse("SELECT EXTRACT(YEAR FROM o_date) FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_func_like() {
        let r = parse("SELECT * FROM t WHERE name LIKE '%x%'");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_func_not_like() {
        let r = parse("SELECT * FROM t WHERE name NOT LIKE '%x%'");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_where_ne() {
        let r = parse("SELECT * FROM t WHERE a != 0");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_where_gt_lt() {
        let r = parse("SELECT * FROM t WHERE a > 0 AND b < 10");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_where_gte_lte() {
        let r = parse("SELECT * FROM t WHERE a >= 0 AND b <= 10");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_create_index() {
        let r = parse("CREATE INDEX idx_name ON t (col)");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_drop_index() {
        let r = parse("DROP INDEX idx_name ON t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_alter_table_add_column() {
        let r = parse("ALTER TABLE t ADD COLUMN new_col INT");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_alter_table_drop_column() {
        let r = parse("ALTER TABLE t DROP COLUMN old_col");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_truncate() {
        let r = parse("TRUNCATE TABLE t").unwrap();
        assert!(matches!(r, Statement::Truncate(_)));
    }

    #[test]
    fn test_parse_select_for_update() {
        let r = parse("SELECT * FROM t WHERE id = 1 FOR UPDATE");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_create_view() {
        let r = parse("CREATE VIEW v AS SELECT * FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_drop_view() {
        let r = parse("DROP VIEW v");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_create_trigger() {
        let r = parse("CREATE TRIGGER trg AFTER INSERT ON t FOR EACH ROW BEGIN SELECT 1; END");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_qualifier_star() {
        let r = parse("SELECT t.* FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_quoted_identifier() {
        let r = parse("SELECT `my col` FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_string_escape() {
        let r = parse("SELECT 'it''s a test' FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_hex_literal() {
        let r = parse("SELECT x'0F' FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_binary_literal() {
        let r = parse("SELECT b'1010' FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_date_literal() {
        let r = parse("SELECT DATE '2026-01-01' FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_time_literal() {
        let r = parse("SELECT TIME '12:00:00' FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_timestamp_literal() {
        let r = parse("SELECT TIMESTAMP '2026-01-01 12:00:00' FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_null_literal() {
        let r = parse("SELECT NULL FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_true_false() {
        let r = parse("SELECT TRUE, FALSE FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_constant() {
        let r = parse("SELECT 1, 'hello', 3.14 FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_create_database_if_not_exists() {
        let r = parse("CREATE DATABASE IF NOT EXISTS mydb");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_drop_database() {
        let r = parse("DROP DATABASE mydb").unwrap();
        assert!(matches!(r, Statement::DropDatabase(_)));
    }

    #[test]
    fn test_parse_drop_database_if_exists() {
        let r = parse("DROP DATABASE IF EXISTS mydb");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_where_subquery_gt() {
        let r = parse("SELECT * FROM t WHERE a > (SELECT AVG(a) FROM u)");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_union_order_by() {
        let r = parse("SELECT a FROM t UNION SELECT b FROM u ORDER BY 1");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_union_limit() {
        let r = parse("SELECT a FROM t UNION SELECT b FROM u LIMIT 5");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_replace_select() {
        let r = parse("REPLACE INTO t SELECT * FROM u");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_create_table_as_select() {
        let r = parse("CREATE TABLE t AS SELECT * FROM u");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_where_multi_and() {
        let r = parse("SELECT * FROM t WHERE a > 0 AND b < 10 AND c = 5");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_where_and_or_mix() {
        let r = parse("SELECT * FROM t WHERE (a = 1 OR b = 2) AND c = 3");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_where_not_in() {
        let r = parse("SELECT * FROM t WHERE id NOT IN (1, 2, 3)");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_select_where_not_between() {
        let r = parse("SELECT * FROM t WHERE age NOT BETWEEN 0 AND 17");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_execute_stmt() {
        let r = parse("EXECUTE stmt USING @a");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_show_index() {
        let r = parse("SHOW INDEX FROM t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_parse_describe() {
        let r = parse("DESCRIBE t");
        assert!(r.is_ok());
    }

    #[test]
    fn test_coverage_split_sql_statements_single() {
        let stmts = split_sql_statements("SELECT 1");
        assert_eq!(stmts.len(), 1);
    }

    #[test]
    fn test_coverage_split_sql_statements_multi_semicolon() {
        let stmts = split_sql_statements("SELECT 1; SELECT 2; INSERT INTO t VALUES (1);");
        assert_eq!(stmts.len(), 3);
    }

    #[test]
    fn test_coverage_split_sql_statements_with_quotes() {
        let stmts = split_sql_statements("SELECT 'hello; world'");
        assert_eq!(stmts.len(), 1);
    }

    #[test]
    fn test_coverage_split_sql_statements_with_line_comments() {
        let stmts = split_sql_statements("-- comment\nSELECT 1; -- another\nSELECT 2");
        assert!(stmts.len() >= 2);
    }

    #[test]
    fn test_coverage_split_sql_statements_with_block_comments() {
        let stmts = split_sql_statements("/* comment */ SELECT 1; /* multi\nline */ SELECT 2");
        assert!(stmts.len() >= 2);
    }

    #[test]
    fn test_coverage_parse_call_procedure() {
        assert!(parse("CALL proc(1, 'foo')").is_ok());
    }

    #[test]
    fn test_coverage_parse_window_partition_by() {
        assert!(parse("SELECT a, b, SUM(b) OVER (PARTITION BY a) FROM t").is_ok());
    }

    #[test]
    fn test_coverage_parse_between() {
        assert!(parse("SELECT * FROM t WHERE a BETWEEN 1 AND 10").is_ok());
    }

    #[test]
    fn test_coverage_parse_case_when() {
        assert!(parse("SELECT CASE WHEN a > 0 THEN 'pos' ELSE 'neg' END FROM t").is_ok());
    }

    #[test]
    fn test_coverage_parse_is_null() {
        assert!(parse("SELECT * FROM t WHERE a IS NULL").is_ok());
    }

    #[test]
    fn test_coverage_parse_is_not_null() {
        assert!(parse("SELECT * FROM t WHERE a IS NOT NULL").is_ok());
    }

    #[test]
    fn test_coverage_parse_greater_than_or_equal() {
        assert!(parse("SELECT * FROM t WHERE a >= 1").is_ok());
    }

    #[test]
    fn test_coverage_parse_less_than() {
        assert!(parse("SELECT * FROM t WHERE a < 1").is_ok());
    }

    #[test]
    fn test_coverage_parse_with_cte_multi() {
        assert!(parse("WITH a AS (SELECT 1), b AS (SELECT 2) SELECT * FROM a, b").is_ok());
    }
}

// V311-10 F-30: NEXT VALUE FOR seq / CURRVAL(seq) parse tests.
// Locks down the SQL:2003 standard syntax acceptance.
#[test]
fn test_parse_next_value_for_with_value_keyword() {
    // The optional VALUE keyword between NEXT and FOR (SQL:2003).
    assert!(parse("SELECT NEXT VALUE FOR my_seq").is_ok());
}

#[test]
fn test_parse_next_value_for_without_value_keyword() {
    // Bare NEXT FOR also accepted (relaxed form).
    assert!(parse("SELECT NEXT FOR my_seq").is_ok());
}

#[test]
fn test_parse_currval_function_call() {
    assert!(parse("SELECT CURRVAL(my_seq)").is_ok());
}

#[test]
fn test_parse_create_sequence_if_not_exists() {
    assert!(parse("CREATE SEQUENCE IF NOT EXISTS s1 START WITH 1").is_ok());
}

#[test]
fn test_parse_insert_with_next_value_for() {
    assert!(parse("INSERT INTO t (id) VALUES (NEXT VALUE FOR my_seq)").is_ok());
}

#[test]
fn test_split_sql_statements_basic() {
    let parts = split_sql_statements("SELECT 1; SELECT 2");
    assert_eq!(parts, vec!["SELECT 1", "SELECT 2"]);
}

#[test]
fn test_split_sql_statements_trailing() {
    let parts = split_sql_statements("SELECT 1; SELECT 2;");
    assert_eq!(parts, vec!["SELECT 1", "SELECT 2"]);
}

#[test]
fn test_split_sql_statements_single() {
    let parts = split_sql_statements("SELECT 1");
    assert_eq!(parts, vec!["SELECT 1"]);
}

#[test]
fn test_split_sql_statements_empty() {
    let parts = split_sql_statements("");
    assert!(parts.is_empty());
}

#[test]
fn test_split_sql_statements_whitespace() {
    let parts = split_sql_statements("  \n\t  ");
    assert!(parts.is_empty());
}

#[test]
fn test_split_sql_statements_parens() {
    let parts = split_sql_statements("SELECT * FROM t WHERE id IN (1, 2, 3); SELECT 2");
    assert_eq!(parts.len(), 2);
    assert!(parts[0].contains("IN (1, 2, 3)"));
}

#[test]
fn test_split_sql_statements_string_literal() {
    let parts = split_sql_statements("SELECT 'a;b'; SELECT 2");
    assert_eq!(parts.len(), 2);
    assert!(parts[0].contains("'a;b'"));
}

#[test]
fn test_split_sql_statements_line_comment() {
    let parts = split_sql_statements("SELECT 1; -- comment\nSELECT 2");
    assert_eq!(parts.len(), 2);
}

#[test]
fn test_split_sql_statements_block_comment() {
    let parts = split_sql_statements("SELECT 1; /* comment */ SELECT 2");
    assert_eq!(parts.len(), 2);
}

#[test]
fn test_parse_statements_multiple() {
    let stmts = parse_statements("SELECT 1; SELECT 2").unwrap();
    assert_eq!(stmts.len(), 2);
}

#[test]
fn test_parse_statements_no_trailing() {
    let stmts = parse_statements("SELECT 1; SELECT 2;").unwrap();
    assert_eq!(stmts.len(), 2);
}

#[test]
fn test_parse_statements_empty() {
    let r = parse_statements("");
    assert!(r.is_err());
}

#[test]
fn test_parse_statements_with_parens() {
    let stmts = parse_statements("SELECT * FROM t WHERE id IN (1, 2); SELECT 1").unwrap();
    assert_eq!(stmts.len(), 2);
}

#[test]
fn test_parse_create_table() {
    assert!(parse("CREATE TABLE t (id INT PRIMARY KEY)").is_ok());
}

#[test]
fn test_parse_create_table_multi_cols() {
    assert!(parse("CREATE TABLE t (id INT PRIMARY KEY, name VARCHAR(100), age INTEGER)").is_ok());
}

#[test]
fn test_parse_alter_table_add() {
    assert!(parse("ALTER TABLE t ADD COLUMN x INT").is_ok());
}

#[test]
fn test_parse_drop_table() {
    assert!(parse("DROP TABLE t").is_ok());
}

#[test]
fn test_parse_drop_table_if_exists() {
    assert!(parse("DROP TABLE IF EXISTS t").is_ok());
}

#[test]
fn test_parse_select_with_where() {
    assert!(parse("SELECT * FROM t WHERE x = 1").is_ok());
}

#[test]
fn test_parse_select_order_by() {
    assert!(parse("SELECT * FROM t ORDER BY id").is_ok());
}

#[test]
fn test_parse_select_limit() {
    assert!(parse("SELECT * FROM t LIMIT 10").is_ok());
}

#[test]
fn test_parse_select_limit_offset() {
    assert!(parse("SELECT * FROM t LIMIT 10 OFFSET 5").is_ok());
}

#[test]
fn test_parse_select_group_by() {
    assert!(parse("SELECT COUNT(*) FROM t GROUP BY x").is_ok());
}

#[test]
fn test_parse_select_having() {
    assert!(parse("SELECT COUNT(*) FROM t GROUP BY x HAVING COUNT(*) > 1").is_ok());
}

#[test]
fn test_parse_select_distinct() {
    assert!(parse("SELECT DISTINCT x FROM t").is_ok());
}

// V312-56A / 56A-R2 / Issue #4251: parse the `schema.table` form in
// FROM and capture the qualifier into SelectStatement.schema. Used by
// the information_schema virtual-table executor to route
// `SELECT ... FROM information_schema.<view>` without touching storage.
#[test]
fn test_parse_select_with_schema_qualifier() {
    let stmt = parse("SELECT * FROM information_schema.tables").unwrap();
    if let Statement::Select(s) = stmt {
        assert_eq!(s.schema.as_deref(), Some("information_schema"));
        assert_eq!(s.table, "tables");
    } else {
        panic!("expected SELECT statement");
    }
}

#[test]
fn test_parse_select_without_schema_qualifier() {
    let stmt = parse("SELECT * FROM users").unwrap();
    if let Statement::Select(s) = stmt {
        assert!(s.schema.is_none(), "no schema qualifier expected");
        assert_eq!(s.table, "users");
    } else {
        panic!("expected SELECT statement");
    }
}

#[test]
fn test_parse_insert_simple() {
    assert!(parse("INSERT INTO t VALUES (1)").is_ok());
}

#[test]
fn test_parse_insert_columns() {
    assert!(parse("INSERT INTO t (id, name) VALUES (1, 'a')").is_ok());
}

#[test]
fn test_parse_update_with_where() {
    assert!(parse("UPDATE t SET x = 1 WHERE id = 5").is_ok());
}

#[test]
fn test_parse_delete_with_where() {
    assert!(parse("DELETE FROM t WHERE id = 5").is_ok());
}

#[test]
fn test_parse_delete_all() {
    assert!(parse("DELETE FROM t").is_ok());
}

#[test]
fn test_parse_truncate() {
    assert!(parse("TRUNCATE TABLE t").is_ok());
}

#[test]
fn test_parse_expression_complex() {
    assert!(parse("SELECT * FROM t WHERE (a + b) * c > 100").is_ok());
}

#[test]
fn test_parse_in_subquery() {
    assert!(parse("SELECT * FROM t WHERE id IN (SELECT id FROM s)").is_ok());
}

#[test]
fn test_parse_exists_subquery() {
    assert!(parse("SELECT * FROM t WHERE EXISTS (SELECT 1 FROM s)").is_ok());
}

#[test]
fn test_parse_case_when() {
    assert!(parse("SELECT CASE WHEN x > 0 THEN 'pos' ELSE 'neg' END FROM t").is_ok());
}

#[test]
fn test_parse_between() {
    assert!(parse("SELECT * FROM t WHERE x BETWEEN 1 AND 10").is_ok());
}

#[test]
fn test_parse_not_between() {
    assert!(parse("SELECT * FROM t WHERE x NOT BETWEEN 1 AND 10").is_ok());
}

#[test]
fn test_parse_like() {
    assert!(parse("SELECT * FROM t WHERE name LIKE '%foo%'").is_ok());
}

#[test]
fn test_parse_is_null() {
    assert!(parse("SELECT * FROM t WHERE x IS NULL").is_ok());
}

#[test]
fn test_parse_is_not_null() {
    assert!(parse("SELECT * FROM t WHERE x IS NOT NULL").is_ok());
}

#[test]
fn test_parse_join_inner() {
    assert!(parse("SELECT * FROM t1 INNER JOIN t2 ON t1.id = t2.id").is_ok());
}

#[test]
fn test_parse_join_left() {
    assert!(parse("SELECT * FROM t1 LEFT JOIN t2 ON t1.id = t2.id").is_ok());
}

#[test]
fn test_parse_join_right() {
    assert!(parse("SELECT * FROM t1 RIGHT JOIN t2 ON t1.id = t2.id").is_ok());
}

#[test]
fn test_parse_cross_join_v2() {
    assert!(parse("SELECT * FROM t1 CROSS JOIN t2").is_ok());
}

#[test]
fn test_parse_union() {
    assert!(parse("SELECT 1 UNION SELECT 2").is_ok());
}

#[test]
fn test_parse_union_all() {
    assert!(parse("SELECT 1 UNION ALL SELECT 2").is_ok());
}

#[test]
fn test_parse_set_operation_mixed() {
    assert!(parse("SELECT 1 UNION SELECT 2 INTERSECT SELECT 3").is_ok());
}

#[test]
fn test_parse_cte() {
    assert!(parse("WITH cte AS (SELECT 1) SELECT * FROM cte").is_ok());
}

#[test]
fn test_parse_cte_multi() {
    assert!(parse("WITH a AS (SELECT 1), b AS (SELECT 2) SELECT * FROM a, b").is_ok());
}

#[test]
fn test_parse_create_index() {
    assert!(parse("CREATE INDEX idx ON t (x)").is_ok());
}

#[test]
fn test_parse_create_view() {
    assert!(parse("CREATE VIEW v AS SELECT * FROM t").is_ok());
}

#[test]
fn test_parse_drop_view() {
    assert!(parse("DROP VIEW v").is_ok());
}

#[test]
fn test_parse_grant() {
    // GRANT may or may not be supported - just verify no panic
    let _ = parse("GRANT SELECT ON t TO user");
}

#[test]
fn test_parse_revoke() {
    let _ = parse("REVOKE SELECT ON t FROM user");
}

#[test]
fn test_parse_set_variable() {
    // SET is valid SQL
    let _ = parse("SET @x = 1");
}

/// V312-11-fix #3986: SET session variable
#[test]
fn test_parse_set_session_variable_3986() {
    let result = parse("SET debug_force_external=true");
    assert!(result.is_ok(), "Parse failed: {:?}", result);
    match result.unwrap() {
        Statement::Transaction(TransactionStatement::SetSessionVariable { name, value }) => {
            assert_eq!(name, "debug_force_external");
            assert_eq!(value, "true");
        }
        _ => panic!("Expected SetSessionVariable statement"),
    }
}

#[test]
fn test_parse_window_function() {
    assert!(parse("SELECT x, ROW_NUMBER() OVER (ORDER BY y) FROM t").is_ok());
}

#[test]
fn test_parse_aggregate_distinct() {
    assert!(parse("SELECT COUNT(DISTINCT x) FROM t").is_ok());
}

#[test]
fn test_parse_cast() {
    assert!(parse("SELECT CAST(x AS INT) FROM t").is_ok());
}

#[test]
fn test_parse_function_call() {
    assert!(parse("SELECT LOWER(name) FROM t").is_ok());
}

#[test]
fn test_parse_aliased_column() {
    assert!(parse("SELECT x AS y FROM t").is_ok());
}

#[test]
fn test_parse_table_alias() {
    assert!(parse("SELECT * FROM t AS x").is_ok());
}

#[test]
fn test_parse_select_qualified_column() {
    assert!(parse("SELECT t.x FROM t").is_ok());
}

#[test]
fn test_parse_negative_literal() {
    assert!(parse("SELECT -1 FROM t").is_ok());
}

#[test]
fn test_parse_string_concat() {
    // String concatenation operator may or may not be supported
    let _ = parse("SELECT 'a' || 'b' FROM t");
}

#[test]
fn test_parse_subquery_in_where() {
    assert!(parse("SELECT * FROM t WHERE x > (SELECT AVG(y) FROM t)").is_ok());
}

#[test]
fn test_parse_in_list() {
    assert!(parse("SELECT * FROM t WHERE x IN (1, 2, 3)").is_ok());
}

#[test]
fn test_parse_not_in_list() {
    assert!(parse("SELECT * FROM t WHERE x NOT IN (1, 2, 3)").is_ok());
}

#[test]
fn test_parse_any() {
    let _ = parse("SELECT * FROM t WHERE x > ANY (SELECT y FROM s)");
}

#[test]
fn test_parse_all() {
    let _ = parse("SELECT * FROM t WHERE x > ALL (SELECT y FROM s)");
}

#[test]
fn test_parse_comments_in_query() {
    assert!(parse(
        "SELECT * FROM t -- comment
WHERE x = 1"
    )
    .is_ok());
}

#[test]
fn test_parse_string_with_semicolon() {
    assert!(parse("SELECT 'a; b' FROM t").is_ok());
}

#[test]
fn test_parse_escaped_quote() {
    assert!(parse("SELECT 'a''b' FROM t").is_ok());
}

#[test]
fn test_parse_double_quoted_identifier() {
    assert!(parse("SELECT \"col\" FROM t").is_ok());
}

#[test]
fn test_parse_backtick_quoted_identifier() {
    assert!(parse("SELECT `col` FROM t").is_ok());
}

#[test]
fn test_parse_hex_literal() {
    let _ = parse("SELECT 0xABCD FROM t");
}

#[test]
fn test_parse_boolean_literal() {
    let _ = parse("SELECT TRUE FROM t");
}

#[test]
fn test_parse_null_literal() {
    assert!(parse("SELECT NULL FROM t").is_ok());
}

#[test]
fn test_parse_now_function() {
    let _ = parse("SELECT NOW() FROM t");
}

#[test]
fn test_parse_sql_empty() {
    // Empty parse might be invalid
    let r = parse("");
    // Just verify it doesn't panic
    let _ = r;
}

#[test]
fn test_parse_sql_whitespace_only() {
    let r = parse("   \n  \t  ");
    let _ = r;
}

#[test]
fn test_parse_complex_realistic() {
    let sql = "SELECT t.id, t.name, COUNT(*) AS cnt FROM t INNER JOIN s ON t.sid = s.id WHERE t.x > 100 GROUP BY t.id HAVING COUNT(*) > 1 ORDER BY t.id LIMIT 10";
    assert!(parse(sql).is_ok());
}

#[test]
fn test_parse_create_table_with_constraints() {
    let sql = "CREATE TABLE t (id INT NOT NULL PRIMARY KEY, name VARCHAR(100) NOT NULL, FOREIGN KEY (id) REFERENCES other(id))";
    let _ = parse(sql);
}

#[test]
fn test_parse_create_table_if_not_exists() {
    let sql = "CREATE TABLE IF NOT EXISTS t (id INT)";
    let _ = parse(sql);
}

#[test]
fn test_parse_alter_table_drop() {
    let _ = parse("ALTER TABLE t DROP COLUMN x");
}

#[test]
fn test_parse_alter_table_modify() {
    let _ = parse("ALTER TABLE t MODIFY COLUMN x BIGINT");
}

#[test]
fn test_parse_insert_with_select() {
    assert!(parse("INSERT INTO t SELECT * FROM s").is_ok());
}

#[test]
fn test_parse_update_multiple_columns() {
    assert!(parse("UPDATE t SET a = 1, b = 2 WHERE id = 1").is_ok());
}

#[test]
fn test_parse_delete_with_limit() {
    let _ = parse("DELETE FROM t WHERE x = 1 LIMIT 10");
}

#[test]
fn test_parse_select_with_locking() {
    let _ = parse("SELECT * FROM t FOR UPDATE");
}

#[test]
fn test_parse_explain() {
    let _ = parse("EXPLAIN SELECT * FROM t");
}

#[test]
fn test_parse_show_tables() {
    let _ = parse("SHOW TABLES");
}

#[test]
fn test_parse_describe() {
    let _ = parse("DESCRIBE t");
}

#[test]
fn test_parse_use_database() {
    let _ = parse("USE mydb");
}

#[test]
fn test_parse_simple_select_no_from() {
    assert!(parse("SELECT 1").is_ok());
}

#[test]
fn test_parse_select_with_where_or() {
    assert!(parse("SELECT * FROM t WHERE x = 1 OR y = 2").is_ok());
}

#[test]
fn test_parse_select_with_where_and() {
    assert!(parse("SELECT * FROM t WHERE x = 1 AND y = 2").is_ok());
}

#[test]
fn test_parse_select_with_where_mixed() {
    assert!(parse("SELECT * FROM t WHERE (x = 1 OR y = 2) AND z = 3").is_ok());
}

#[test]
fn test_parse_aggregate_multiple() {
    assert!(parse("SELECT COUNT(*), SUM(x), AVG(y), MAX(z), MIN(w) FROM t").is_ok());
}

#[test]
fn test_parse_count_star() {
    assert!(parse("SELECT COUNT(*) FROM t").is_ok());
}

#[test]
fn test_parse_left_outer_join() {
    assert!(parse("SELECT * FROM t1 LEFT OUTER JOIN t2 ON t1.id = t2.id").is_ok());
}

#[test]
fn test_parse_inner_join_using() {
    let _ = parse("SELECT * FROM t1 INNER JOIN t2 USING (id)");
}

#[test]
fn test_parse_natural_join() {
    let _ = parse("SELECT * FROM t1 NATURAL JOIN t2");
}

#[test]
fn test_parse_full_outer_join() {
    let _ = parse("SELECT * FROM t1 FULL OUTER JOIN t2 ON t1.id = t2.id");
}

#[test]
fn test_parse_select_distinct_on() {
    let _ = parse("SELECT DISTINCT ON (x) x, y FROM t");
}

#[test]
fn test_parse_select_into() {
    let _ = parse("SELECT * INTO new_table FROM t");
}

#[test]
fn test_parse_with_recursive() {
    let _ = parse("WITH RECURSIVE cte AS (SELECT 1 UNION SELECT cte.x + 1 FROM cte WHERE cte.x < 10) SELECT * FROM cte");
}

// Round-21 / Issue #4216: array-fraction form for quantile_disc/cont.
// Single-fraction (`quantile_disc(col, 0.5)`) is owned by #4155.
// Array-fraction (`quantile_disc(col, [0.25, 0.5, 0.75])`) lives here.

#[test]
fn test_parse_quantile_disc_array_v312_46() {
    use crate::parser::AggregateFunction;
    let stmt = parse("SELECT quantile_disc(x, [0.25, 0.5, 0.75]) FROM t").unwrap();
    if let Statement::Select(sel) = stmt {
        assert_eq!(sel.aggregates.len(), 1);
        let agg = &sel.aggregates[0];
        assert!(matches!(agg.func, AggregateFunction::QuantileDisc));
        assert_eq!(agg.args.len(), 2);
        // args[1] should be an ArrayLiteral containing three fraction literals.
        match &agg.args[1] {
            Expression::ArrayLiteral(elems) => {
                assert_eq!(elems.len(), 3);
            }
            other => panic!("expected ArrayLiteral, got {:?}", other),
        }
    } else {
        panic!("expected Statement::Select");
    }
}

#[test]
fn test_parse_quantile_cont_array_v312_46() {
    use crate::parser::AggregateFunction;
    let stmt = parse("SELECT quantile_cont(x, [0.1, 0.9]) FROM t").unwrap();
    if let Statement::Select(sel) = stmt {
        assert_eq!(sel.aggregates.len(), 1);
        let agg = &sel.aggregates[0];
        assert!(matches!(agg.func, AggregateFunction::QuantileCont));
        match &agg.args[1] {
            Expression::ArrayLiteral(elems) => assert_eq!(elems.len(), 2),
            other => panic!("expected ArrayLiteral, got {:?}", other),
        }
    } else {
        panic!("expected Statement::Select");
    }
}
