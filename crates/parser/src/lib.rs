// SQLRustGo Parser Module
pub use sqlrustgo_common::{SqlError, SqlResult};

pub mod lexer;
pub mod parser;
pub mod token;
pub mod transaction;

pub use lexer::Lexer;
pub use parser::Parser;
pub use token::Token;

pub use parser::{
    date_trunc_unit_list, get_and_clear_derived_subqueries, validate_date_trunc_unit,
    AggregateCall, AggregateFunction, AlterColumnOperation, AlterTableOperation,
    AlterTableStatement, CallStatement, ColumnDefinition, CreateFulltextIndexStatement,
    CreateProcedureStatement, CreateSequenceStatement, CreateTableStatement,
    CreateTriggerStatement, CreateUserStatement, CreateVectorIndexStatement, CreateViewStatement,
    DeleteStatement, DropIndexStatement, DropSequenceStatement, DropTableStatement,
    DropTriggerStatement, DropUserStatement, DropViewStatement, Expression, FrameBound,
    FrameClause, FrameExclusion, FrameMode, IndexColumnSpec, IndexHint, IndexHintType,
    InsertStatement, JoinClause, JoinType, LockClause, MergeAction, MergeSource, MergeStatement,
    MergeWhenClause, OnConflictAction, OnConflictClause, ReindexStatement, SavepointOp,
    SelectColumn, SelectStatement, Statement, StoredProcParam, StoredProcParamMode,
    StoredProcStatement, TableConstraint, TableRef, UpdateStatement, VacuumStatement,
    VectorIndexAlgorithm, WhenClause, WindowCall, WindowSpecification, DATE_TRUNC_UNITS,
};
pub use parser::{parse, parse_expression_str, parse_statements, split_sql_statements};
pub use transaction::TransactionStatement;
