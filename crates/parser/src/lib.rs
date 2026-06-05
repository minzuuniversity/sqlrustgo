// SQLRustGo Parser Module
pub use sqlrustgo_common::{SqlError, SqlResult};

pub mod lexer;
pub mod parser;
pub mod token;
pub mod transaction;

pub use lexer::Lexer;
pub use parser::Parser;
pub use token::Token;

pub use parser::parse;
pub use parser::{
    get_and_clear_derived_subqueries, AggregateCall, AggregateFunction, AlterTableOperation,
    AlterTableStatement, CallStatement, ColumnDefinition, CommonTableExpression,
    CreateProcedureStatement, CreateTableStatement, CreateTriggerStatement, CreateViewStatement,
    DeleteStatement, DropIndexStatement, DropTableStatement, DropViewStatement, Expression,
    ForeignKeyRef, InsertStatement, JoinClause, JoinType, MergeAction, MergeSource, MergeStatement,
    MergeWhenClause, SavepointOp, SelectColumn, SelectStatement, Statement, StoredProcParam,
    StoredProcParamMode, StoredProcStatement, TableConstraint, UpdateStatement, WithClause,
    WithSelect,
};
pub use transaction::TransactionStatement;
