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
    get_and_clear_derived_subqueries, AggregateCall, AggregateFunction, AlterColumnOperation,
    AlterTableOperation, AlterTableStatement, CallStatement, ColumnDefinition,
    CommonTableExpression, CreateProcedureStatement, CreateSequenceStatement, CreateTableStatement,
    CreateTriggerStatement, CreateViewStatement, DeleteStatement, DropIndexStatement,
    DropSequenceStatement, DropTableStatement, DropViewStatement, Expression, InsertStatement,
    JoinClause, JoinType, LockClause, MergeAction, MergeSource, MergeStatement, MergeWhenClause,
    SavepointOp, SelectColumn, SelectStatement, Statement, StoredProcParam, StoredProcParamMode,
    StoredProcStatement, TableConstraint, TableRef, UpdateStatement, WhenClause,
};
pub use parser::{parse, parse_expression_str, parse_statements, split_sql_statements};
pub use transaction::TransactionStatement;
