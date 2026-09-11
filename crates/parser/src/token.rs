//! Token definitions for SQL lexer
//! SQL token types and token struct

use std::fmt;

/// Token type enumeration representing all SQL lexical elements
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Value,
    Select,
    From,
    Where,
    Insert,
    Into,
    Values,
    Update,
    Set,
    Delete,
    Merge,
    Using,
    On,
    /// V312-64 / Issue #4662: separator word in `CREATE TRIGGER ... INSTEAD OF ...`
    /// (PostgreSQL/SQLite view trigger timing). Reserved alongside `Instead`
    /// so `parse_create_trigger` can consume it without treating it as an
    /// arbitrary identifier.
    Of,
    When,
    Matched,
    Create,
    Table,
    Drop,
    Alter,
    Index,
    Primary,
    Key,
    Add,
    Column,
    Rename,
    To,
    Begin,
    Commit,
    Rollback,
    Grant,
    Revoke,
    // v3.10.0 Issue #3703: lock clause keywords
    Lock,
    Locked,
    Share,
    Mode,
    Skip,
    Nowait,
    Analyze,
    Explain,
    Truncate,
    User,
    Identified,
    Ignore,
    // V312-69 / Issue #4653: RETURNING clause on INSERT (PostgreSQL/MySQL 8.0+).
    Returning,
    Replace,

    // Constraint keywords
    Foreign,
    References,
    Unique,
    Check,
    Constraint,
    Cascade,
    Restrict,
    No,
    Action,
    Default,
    AutoIncrement,

    // Subquery keywords
    Exists,
    Expire,
    Expired,
    In,
    Is,
    All,
    Any,
    Some,

    // Conditional keywords
    If,
    Collate,
    Case,
    Then,
    Else,

    // CTE keywords
    With,
    Recursive,
    As,

    // Aggregate function keywords
    Count,
    Sum,
    Avg,
    Min,
    Max,

    // JOIN keywords
    Join,
    Inner,
    Left,
    Right,
    Full,
    Cross,
    Outer,
    Natural,

    // Other SQL keywords
    Group,
    By,
    Having,
    Order,
    Limit,
    Offset,
    Distinct,
    Union,
    Intersect,
    Except,
    AsOf,
    Window,
    Partition,
    Partitions,
    Range,
    List,
    Fulltext,
    Maxvalue,
    Minvalue,
    Over,
    // V400-01 / Issue #4877: vector SQL syntax.
    // `VECTOR(N[, dtype])` column type and `distance(vec, vec)` function.
    Vector,
    Hnsw,
    Ivf,
    Distance,
    Between,
    Unbounded,
    Escape,
    Nulls,
    First,
    Last,
    Asc,
    Desc,
    Like,
    Preceding,
    Following,
    Current,
    Row,
    Rows,
    Grouping,
    Within,
    Rollup,
    Cube,
    HighPriority,
    SqlCache,
    SqlNoCache,
    SqlCalcFoundRows,
    Convert,
    Date,
    DateAdd,
    DateSub,
    // V312-64 / Issue #4662: `INSTEAD OF` trigger timing for views
    // (PostgreSQL/SQLite). Consumed by parse_create_trigger so the
    // dispatcher accepts `CREATE TRIGGER ... INSTEAD OF UPDATE ON v ...`.
    Instead,
    // V312-64 / Issue #4663: SQLite maintenance commands.
    // `VACUUM` and `REINDEX` are parsed as no-op DDL (executor returns
    // ExecutorResult::Empty). ANALYZE was already a keyword.
    Vacuum,
    Reindex,
    // V312-63 / Issue #4627: TIMESTAMPDIFF(unit, ts1, ts2) — MySQL 5.7 standard
    // function. The first argument is a unit keyword (MINUTE/HOUR/DAY/...) and
    // must NOT be bound to a column lookup.
    TimestampDiff,
    Substring,
    Position,
    Interval,

    // F-30 CREATE SEQUENCE keywords
    Sequence,
    Cycle,
    NoCycle,
    NoMinValue,
    NoMaxValue,
    Cache,
    Increment,
    Owned,
    NextValue,
    Currval,
    Restart,

    // MySQL-specific keywords
    Duplicate,
    // V312-63 / Issue #4642: SQLite/Postgres UPSERT clause:
    // `INSERT ... ON CONFLICT (col) DO UPDATE SET ...`. Reserved alongside
    // `ON` so `parse_insert` can dispatch to the SQLite/PG conflict handler.
    Conflict,
    Nothing,
    Database,
    Modify,
    Use,
    View,
    // Transaction keywords
    Transaction,
    Work,
    Savepoint,
    Release,
    Start,
    Isolation,
    Level,
    Serializable,
    Repeatable,
    Read,

    Prepare,
    Execute,
    Deallocate,
    Write,
    Only,
    Committed,
    Uncommitted,

    // Stored Procedure keywords
    Call,
    Procedure,
    /// V312-58 / Issue #4512: scalar UDF (CREATE FUNCTION). Reserved
    /// alongside `PROCEDURE` so the dispatcher in `parse_create` can
    /// distinguish `CREATE FUNCTION inc(x INT) RETURNS INT RETURN x+1`
    /// from `CREATE PROCEDURE ...`. MySQL accepts both forms; we follow
    /// the MySQL convention.
    Function,
    End,
    While,
    Do,
    Loop,
    Leave,
    Iterate,
    Declare,
    Return,
    /// V312-58 / Issue #4512: scalar UDF return-type prefix.
    /// `CREATE FUNCTION ... RETURNS INTEGER RETURN expr`. Stored
    /// alongside `Return` so the parser can require `RETURNS` after
    /// the parameter list before the optional `DETERMINISTIC` and
    /// the mandatory `RETURN <expr>` body.
    Returns,
    Condition,
    Signal,
    Resignal,
    Repeat,
    Until,
    Out,
    InOut,
    Cursor,
    Handler,
    SQL,
    Language,
    Deterministic,
    Contains,

    // Show keywords
    Show,
    Describe,
    Role,
    Roles,
    Parent,
    Password,
    Grants,
    For,

    // Trigger keywords
    Trigger,
    Before,
    After,
    ForEach,
    Each,

    // Data Types
    Integer,
    Text,
    Float,
    Boolean,
    Blob,
    Null,
    // Used by `ALTER TABLE ... ALTER COLUMN ... SET DATA TYPE`
    Data,
    Type,

    // Operators
    Equal,
    NotEqual,
    Greater,
    Less,
    GreaterEqual,
    /// MySQL 5.7 JSON path operator: `column -> path` returns JSON value
    JsonArrow,
    /// MySQL 5.7 JSON path operator (unquoted): `column ->> path` returns text
    JsonArrowText,
    LessEqual,
    And,
    Or,
    Not,
    True,
    False,
    Plus,
    Minus,
    Asterisk,
    Slash,
    Percent,

    // Syntax
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Dot,
    Semicolon,
    Colon,
    SingleQuote,

    // Wildcard
    Star, // * for SELECT *

    // Literals
    Identifier(String),
    StringLiteral(String),
    NumberLiteral(String),
    BooleanLiteral(bool),

    // MySQL system variable reference: `@@version_comment`, `@@autocommit`, etc.
    // Synthesized by the lexer when `@@` is followed by an identifier.
    SystemVariable(String),

    // Special
    Eof,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Identifier(s) => write!(f, "IDENTIFIER({})", s),
            Token::StringLiteral(s) => write!(f, "'{}'", s),
            Token::NumberLiteral(s) => write!(f, "{}", s),
            Token::BooleanLiteral(b) => write!(f, "{}", b),
            Token::SystemVariable(s) => write!(f, "@@{}", s),
            // Keywords and data types - uppercase
            Token::Select => write!(f, "SELECT"),
            Token::From => write!(f, "FROM"),
            Token::Where => write!(f, "WHERE"),
            Token::Insert => write!(f, "INSERT"),
            Token::Into => write!(f, "INTO"),
            Token::Values => write!(f, "VALUES"),
            Token::Value => write!(f, "VALUE"),
            Token::Update => write!(f, "UPDATE"),
            Token::Set => write!(f, "SET"),
            Token::Delete => write!(f, "DELETE"),
            Token::Merge => write!(f, "MERGE"),
            Token::Using => write!(f, "USING"),
            Token::On => write!(f, "ON"),
            Token::Of => write!(f, "OF"),
            Token::When => write!(f, "WHEN"),
            Token::Matched => write!(f, "MATCHED"),
            Token::Create => write!(f, "CREATE"),
            Token::Table => write!(f, "TABLE"),
            Token::Drop => write!(f, "DROP"),
            Token::Alter => write!(f, "ALTER"),
            Token::Index => write!(f, "INDEX"),
            Token::Primary => write!(f, "PRIMARY"),
            Token::Key => write!(f, "KEY"),
            Token::Add => write!(f, "ADD"),
            Token::Column => write!(f, "COLUMN"),
            Token::Rename => write!(f, "RENAME"),
            Token::To => write!(f, "TO"),
            Token::Begin => write!(f, "BEGIN"),
            Token::Commit => write!(f, "COMMIT"),
            Token::Rollback => write!(f, "ROLLBACK"),
            Token::Grant => write!(f, "GRANT"),
            Token::Revoke => write!(f, "REVOKE"),
            Token::Lock => write!(f, "LOCK"),
            Token::Locked => write!(f, "LOCKED"),
            Token::Share => write!(f, "SHARE"),
            Token::Skip => write!(f, "SKIP"),
            Token::Nowait => write!(f, "NOWAIT"),
            Token::Analyze => write!(f, "ANALYZE"),
            Token::Explain => write!(f, "EXPLAIN"),
            Token::Truncate => write!(f, "TRUNCATE"),
            Token::User => write!(f, "USER"),
            Token::Identified => write!(f, "IDENTIFIED"),
            Token::Replace => write!(f, "REPLACE"),
            Token::Ignore => write!(f, "IGNORE"),
            Token::Returning => write!(f, "RETURNING"),
            Token::Duplicate => write!(f, "DUPLICATE"),
            Token::Conflict => write!(f, "CONFLICT"),
            Token::Nothing => write!(f, "NOTHING"),
            Token::Database => write!(f, "DATABASE"),
            Token::Use => write!(f, "USE"),
            Token::View => write!(f, "VIEW"),
            Token::HighPriority => write!(f, "HIGH_PRIORITY"),
            Token::SqlCache => write!(f, "SQL_CACHE"),
            Token::SqlNoCache => write!(f, "SQL_NO_CACHE"),
            Token::SqlCalcFoundRows => write!(f, "SQL_CALC_FOUND_ROWS"),
            Token::Convert => write!(f, "CONVERT"),
            Token::Date => write!(f, "DATE"),
            Token::DateSub => write!(f, "DATE_SUB"),
            Token::Instead => write!(f, "INSTEAD"),
            Token::Vacuum => write!(f, "VACUUM"),
            Token::Reindex => write!(f, "REINDEX"),
            Token::TimestampDiff => write!(f, "TIMESTAMPDIFF"),
            Token::Substring => write!(f, "SUBSTRING"),
            Token::Position => write!(f, "POSITION"),
            Token::Interval => write!(f, "INTERVAL"),
            // Constraint keywords
            Token::Foreign => write!(f, "FOREIGN"),
            Token::References => write!(f, "REFERENCES"),
            Token::Unique => write!(f, "UNIQUE"),
            Token::Check => write!(f, "CHECK"),
            Token::Constraint => write!(f, "CONSTRAINT"),
            Token::Cascade => write!(f, "CASCADE"),
            Token::Restrict => write!(f, "RESTRICT"),
            Token::No => write!(f, "NO"),
            Token::Action => write!(f, "ACTION"),
            Token::Default => write!(f, "DEFAULT"),
            Token::AutoIncrement => write!(f, "AUTO_INCREMENT"),
            Token::Exists => write!(f, "EXISTS"),
            Token::Expire => write!(f, "EXPIRE"),
            Token::Expired => write!(f, "EXPIRED"),
            Token::If => write!(f, "IF"),
            Token::Case => write!(f, "CASE"),
            Token::Then => write!(f, "THEN"),
            Token::Else => write!(f, "ELSE"),
            Token::In => write!(f, "IN"),
            Token::Is => write!(f, "IS"),
            Token::All => write!(f, "ALL"),
            Token::Any => write!(f, "ANY"),
            Token::Some => write!(f, "SOME"),
            Token::With => write!(f, "WITH"),
            Token::Recursive => write!(f, "RECURSIVE"),
            Token::As => write!(f, "AS"),
            Token::Count => write!(f, "COUNT"),
            Token::Sum => write!(f, "SUM"),
            Token::Avg => write!(f, "AVG"),
            Token::Min => write!(f, "MIN"),
            Token::Max => write!(f, "MAX"),
            Token::Group => write!(f, "GROUP"),
            Token::By => write!(f, "BY"),
            Token::Having => write!(f, "HAVING"),
            Token::Order => write!(f, "ORDER"),
            Token::Limit => write!(f, "LIMIT"),
            Token::Offset => write!(f, "OFFSET"),
            Token::Distinct => write!(f, "DISTINCT"),
            Token::Join => write!(f, "JOIN"),
            Token::Inner => write!(f, "INNER"),
            Token::Left => write!(f, "LEFT"),
            Token::Right => write!(f, "RIGHT"),
            Token::Full => write!(f, "FULL"),
            Token::Cross => write!(f, "CROSS"),
            Token::Outer => write!(f, "OUTER"),
            Token::Natural => write!(f, "NATURAL"),
            Token::Union => write!(f, "UNION"),
            Token::Intersect => write!(f, "INTERSECT"),
            Token::Except => write!(f, "EXCEPT"),
            Token::Transaction => write!(f, "TRANSACTION"),
            Token::Work => write!(f, "WORK"),
            Token::Savepoint => write!(f, "SAVEPOINT"),
            Token::Release => write!(f, "RELEASE"),
            Token::Start => write!(f, "START"),
            Token::Isolation => write!(f, "ISOLATION"),
            Token::Level => write!(f, "LEVEL"),
            Token::Serializable => write!(f, "SERIALIZABLE"),
            Token::Repeatable => write!(f, "REPEATABLE"),
            Token::Read => write!(f, "READ"),
            Token::Write => write!(f, "WRITE"),
            Token::Prepare => write!(f, "PREPARE"),
            Token::Execute => write!(f, "EXECUTE"),
            Token::Deallocate => write!(f, "DEALLOCATE"),
            Token::Only => write!(f, "ONLY"),
            Token::Committed => write!(f, "COMMITTED"),
            Token::Uncommitted => write!(f, "UNCOMMITTED"),
            Token::Call => write!(f, "CALL"),
            Token::Procedure => write!(f, "PROCEDURE"),
            Token::Function => write!(f, "FUNCTION"),
            Token::End => write!(f, "END"),
            Token::While => write!(f, "WHILE"),
            Token::Do => write!(f, "DO"),
            Token::Loop => write!(f, "LOOP"),
            Token::Leave => write!(f, "LEAVE"),
            Token::Iterate => write!(f, "ITERATE"),
            Token::Declare => write!(f, "DECLARE"),
            Token::Return => write!(f, "RETURN"),
            Token::Returns => write!(f, "RETURNS"),
            Token::Condition => write!(f, "CONDITION"),
            Token::Signal => write!(f, "SIGNAL"),
            Token::Resignal => write!(f, "RESIGNAL"),
            Token::Repeat => write!(f, "REPEAT"),
            Token::Until => write!(f, "UNTIL"),
            Token::Out => write!(f, "OUT"),
            Token::Cursor => write!(f, "CURSOR"),
            Token::Handler => write!(f, "HANDLER"),
            Token::SQL => write!(f, "SQL"),
            Token::Language => write!(f, "LANGUAGE"),
            Token::Deterministic => write!(f, "DETERMINISTIC"),
            Token::Contains => write!(f, "CONTAINS"),
            Token::Show => write!(f, "SHOW"),
            Token::Describe => write!(f, "DESCRIBE"),
            Token::Role => write!(f, "ROLE"),
            Token::Roles => write!(f, "ROLES"),
            Token::Parent => write!(f, "PARENT"),
            Token::Password => write!(f, "PASSWORD"),
            Token::Grants => write!(f, "GRANTS"),
            Token::For => write!(f, "FOR"),
            Token::Trigger => write!(f, "TRIGGER"),
            Token::Before => write!(f, "BEFORE"),
            Token::After => write!(f, "AFTER"),
            Token::ForEach => write!(f, "FOR EACH"),
            Token::Each => write!(f, "EACH"),
            Token::Unbounded => write!(f, "UNBOUNDED"),
            Token::Preceding => write!(f, "PRECEDING"),
            Token::Following => write!(f, "FOLLOWING"),
            Token::Current => write!(f, "CURRENT"),
            Token::Row => write!(f, "ROW"),
            Token::Rows => write!(f, "ROWS"),
            Token::Grouping => write!(f, "GROUPING"),
            Token::Within => write!(f, "WITHIN"),
            Token::Rollup => write!(f, "ROLLUP"),
            Token::Cube => write!(f, "CUBE"),
            Token::AsOf => write!(f, "ASOF"),
            Token::Window => write!(f, "WINDOW"),
            Token::Partition => write!(f, "PARTITION"),
            Token::Partitions => write!(f, "PARTITIONS"),
            Token::Range => write!(f, "RANGE"),
            Token::List => write!(f, "LIST"),
            Token::Fulltext => write!(f, "FULLTEXT"),
            Token::Maxvalue => write!(f, "MAXVALUE"),
            Token::Minvalue => write!(f, "MINVALUE"),
            Token::Over => write!(f, "OVER"),
            // V400-01 / Issue #4877
            Token::Vector => write!(f, "VECTOR"),
            Token::Hnsw => write!(f, "HNSW"),
            Token::Ivf => write!(f, "IVF"),
            Token::Distance => write!(f, "DISTANCE"),
            Token::Between => write!(f, "BETWEEN"),
            Token::Escape => write!(f, "ESCAPE"),
            Token::Nulls => write!(f, "NULLS"),
            Token::First => write!(f, "FIRST"),
            Token::Last => write!(f, "LAST"),
            Token::Asc => write!(f, "ASC"),
            Token::Desc => write!(f, "DESC"),
            Token::Like => write!(f, "LIKE"),
            // F-30 CREATE SEQUENCE
            Token::Sequence => write!(f, "SEQUENCE"),
            Token::Cycle => write!(f, "CYCLE"),
            Token::NoCycle => write!(f, "NOCYCLE"),
            Token::Cache => write!(f, "CACHE"),
            Token::Increment => write!(f, "INCREMENT"),
            Token::Owned => write!(f, "OWNED"),
            Token::NextValue => write!(f, "NEXT"),
            Token::Currval => write!(f, "CURRVAL"),
            Token::Restart => write!(f, "RESTART"),
            Token::Integer => write!(f, "INTEGER"),
            Token::Text => write!(f, "TEXT"),
            Token::Float => write!(f, "FLOAT"),
            Token::Boolean => write!(f, "BOOLEAN"),
            Token::Blob => write!(f, "BLOB"),
            Token::Data => write!(f, "DATA"),
            Token::Type => write!(f, "TYPE"),
            Token::Null => write!(f, "NULL"),
            // Operators - uppercase
            Token::Equal => write!(f, "="),
            Token::NotEqual => write!(f, "<>"),
            Token::Greater => write!(f, ">"),
            Token::Less => write!(f, "<"),
            Token::GreaterEqual => write!(f, ">="),
            Token::JsonArrow => write!(f, "->"),
            Token::JsonArrowText => write!(f, "->>"),
            Token::LessEqual => write!(f, "<="),
            Token::And => write!(f, "AND"),
            Token::Or => write!(f, "OR"),
            Token::Not => write!(f, "NOT"),
            Token::True => write!(f, "TRUE"),
            Token::False => write!(f, "FALSE"),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Asterisk => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::Percent => write!(f, "%"),
            // Syntax
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::Comma => write!(f, ","),
            Token::Dot => write!(f, "."),
            Token::Semicolon => write!(f, ";"),
            Token::Colon => write!(f, ":"),
            Token::SingleQuote => write!(f, "'"),
            Token::Star => write!(f, "*"),
            Token::Eof => write!(f, "EOF"),
            _ => write!(f, "UNKNOWN"),
        }
    }
}

/// Check if a string is a SQL keyword
pub fn is_keyword(s: &str) -> bool {
    matches!(
        s.to_uppercase().as_str(),
        "SELECT"
            | "FROM"
            | "WHERE"
            | "INSERT"
            | "INTO"
            | "VALUES"
            | "VALUE"
            | "UPDATE"
            | "SET"
            | "DELETE"
            | "CREATE"
            | "TABLE"
            | "DROP"
            | "ALTER"
            | "INDEX"
            | "ON"
            | "PRIMARY"
            | "KEY"
            | "ADD"
            | "COLUMN"
            | "RENAME"
            | "TO"
            | "USE"
            | "BEGIN"
            | "COMMIT"
            | "ROLLBACK"
            | "GRANT"
            | "REVOKE"
            | "INTEGER"
            | "TEXT"
            | "FLOAT"
            | "BOOLEAN"
            | "BLOB"
            | "DATA"
            | "TYPE"
            | "NULL"
            | "TRUE"
            | "FALSE"
            | "AND"
            | "OR"
            | "NOT"
            | "EXISTS"
            | "IN"
            | "ALL"
            | "ANY"
            | "SOME"
            | "WITH"
            | "RECURSIVE"
            | "AS"
            | "TRUNCATE"
            | "SHOW"
            | "DESCRIBE"
            | "AUTO_INCREMENT"
    )
}

/// Convert a keyword string to its corresponding Token
#[allow(unreachable_patterns)]
pub fn from_keyword(s: &str) -> Option<Token> {
    match s.to_uppercase().as_str() {
        "SELECT" => Some(Token::Select),
        "FROM" => Some(Token::From),
        "WHERE" => Some(Token::Where),
        "INSERT" => Some(Token::Insert),
        "INTO" => Some(Token::Into),
        "VALUE" => Some(Token::Value),
        "VALUES" => Some(Token::Values),
        "UPDATE" => Some(Token::Update),
        "SET" => Some(Token::Set),
        "DELETE" => Some(Token::Delete),
        "CREATE" => Some(Token::Create),
        "TABLE" => Some(Token::Table),
        "DROP" => Some(Token::Drop),
        "ALTER" => Some(Token::Alter),
        "INDEX" => Some(Token::Index),
        "ON" => Some(Token::On),
        "OF" => Some(Token::Of),
        "PRIMARY" => Some(Token::Primary),
        "KEY" => Some(Token::Key),
        "ADD" => Some(Token::Add),
        "COLUMN" => Some(Token::Column),
        "RENAME" => Some(Token::Rename),
        "TO" => Some(Token::To),
        "BEGIN" => Some(Token::Begin),
        "COMMIT" => Some(Token::Commit),
        "ROLLBACK" => Some(Token::Rollback),
        "GRANT" => Some(Token::Grant),
        "REVOKE" => Some(Token::Revoke),
        "LOCK" => Some(Token::Lock),
        "LOCKED" => Some(Token::Locked),
        "SKIP" => Some(Token::Skip),
        "NOWAIT" => Some(Token::Nowait),
        "ANALYZE" => Some(Token::Analyze),
        "EXPLAIN" => Some(Token::Explain),
        "TRUNCATE" => Some(Token::Truncate),
        "USER" => Some(Token::User),
        "IDENTIFIED" => Some(Token::Identified),
        "REPLACE" => Some(Token::Replace),
        "IGNORE" => Some(Token::Ignore),
        "DUPLICATE" => Some(Token::Duplicate),
        "CONFLICT" => Some(Token::Conflict),
        "MODIFY" => Some(Token::Modify),
        "DATABASE" => Some(Token::Database),
        "USE" => Some(Token::Use),
        "SHOW" => Some(Token::Show),
        "DESCRIBE" => Some(Token::Describe),
        "ROLE" => Some(Token::Role),
        "ROLES" => Some(Token::Roles),
        "PARENT" => Some(Token::Parent),
        "PASSWORD" => Some(Token::Password),
        "GRANTS" => Some(Token::Grants),
        "FOR" => Some(Token::For),
        "INTEGER" => Some(Token::Integer),
        "TEXT" => Some(Token::Text),
        "FLOAT" => Some(Token::Float),
        "BOOLEAN" => Some(Token::Boolean),
        "BLOB" => Some(Token::Blob),
        "DATA" => Some(Token::Data),
        "TYPE" => Some(Token::Type),
        "NULL" => Some(Token::Null),
        "TRUE" => Some(Token::BooleanLiteral(true)),
        "FALSE" => Some(Token::BooleanLiteral(false)),
        "AND" => Some(Token::And),
        "OR" => Some(Token::Or),
        "NOT" => Some(Token::Not),
        "FOREIGN" => Some(Token::Foreign),
        "REFERENCES" => Some(Token::References),
        "UNIQUE" => Some(Token::Unique),
        "CHECK" => Some(Token::Check),
        "CONSTRAINT" => Some(Token::Constraint),
        "CASCADE" => Some(Token::Cascade),
        "RESTRICT" => Some(Token::Restrict),
        "NO" => Some(Token::No),
        "ACTION" => Some(Token::Action),
        "DEFAULT" => Some(Token::Default),
        "AUTO_INCREMENT" => Some(Token::AutoIncrement),
        "EXISTS" => Some(Token::Exists),
        "EXPIRE" => Some(Token::Expire),
        "EXPIRED" => Some(Token::Expired),
        "IN" => Some(Token::In),
        "IS" => Some(Token::Is),
        "ALL" => Some(Token::All),
        "ANY" => Some(Token::Any),
        "SOME" => Some(Token::Some),
        "WITH" => Some(Token::With),
        "RECURSIVE" => Some(Token::Recursive),
        "AS" => Some(Token::As),
        "COUNT" => Some(Token::Count),
        "SUM" => Some(Token::Sum),
        "AVG" => Some(Token::Avg),
        "MIN" => Some(Token::Min),
        "MAX" => Some(Token::Max),
        "GROUP" => Some(Token::Group),
        "WITHIN" => Some(Token::Within),
        "BY" => Some(Token::By),
        "HAVING" => Some(Token::Having),
        "ORDER" => Some(Token::Order),
        "LIMIT" => Some(Token::Limit),
        "OFFSET" => Some(Token::Offset),
        "DISTINCT" => Some(Token::Distinct),
        "JOIN" => Some(Token::Join),
        "INNER" => Some(Token::Inner),
        "LEFT" => Some(Token::Left),
        "RIGHT" => Some(Token::Right),
        "FULL" => Some(Token::Full),
        "CROSS" => Some(Token::Cross),
        "OUTER" => Some(Token::Outer),
        "NATURAL" => Some(Token::Natural),
        "UNION" => Some(Token::Union),
        "INTERSECT" => Some(Token::Intersect),
        "EXCEPT" => Some(Token::Except),
        "TRANSACTION" => Some(Token::Transaction),
        "WORK" => Some(Token::Work),
        "SAVEPOINT" => Some(Token::Savepoint),
        "RELEASE" => Some(Token::Release),
        "START" => Some(Token::Start),
        "ISOLATION" => Some(Token::Isolation),
        "LEVEL" => Some(Token::Level),
        "SERIALIZABLE" => Some(Token::Serializable),
        "REPEATABLE" => Some(Token::Repeatable),
        "READ" => Some(Token::Read),
        "WRITE" => Some(Token::Write),
        "ONLY" => Some(Token::Only),
        "COMMITTED" => Some(Token::Committed),
        "UNCOMMITTED" => Some(Token::Uncommitted),
        "CALL" => Some(Token::Call),
        "PROCEDURE" => Some(Token::Procedure),
        "END" => Some(Token::End),
        "UNBOUNDED" => Some(Token::Unbounded),
        "PRECEDING" => Some(Token::Preceding),
        "FOLLOWING" => Some(Token::Following),
        "CURRENT" => Some(Token::Current),
        "ROW" => Some(Token::Row),
        "ROWS" => Some(Token::Rows),
        "GROUPING" => Some(Token::Grouping),
        "ROLLUP" => Some(Token::Rollup),
        "CUBE" => Some(Token::Cube),
        "ASOF" => Some(Token::AsOf),
        "WINDOW" => Some(Token::Window),
        "PARTITION" => Some(Token::Partition),
        "PARTITIONS" => Some(Token::Partitions),
        "RANGE" => Some(Token::Range),
        "LIST" => Some(Token::List),
        "FULLTEXT" => Some(Token::Fulltext),
        "MAXVALUE" => Some(Token::Maxvalue),
        "MINVALUE" => Some(Token::Minvalue),
        "OVER" => Some(Token::Over),
        "BETWEEN" => Some(Token::Between),
        "ESCAPE" => Some(Token::Escape),
        "NULLS" => Some(Token::Nulls),
        "FIRST" => Some(Token::First),
        "ASC" => Some(Token::Asc),
        "DESC" => Some(Token::Desc),
        "LIKE" => Some(Token::Like),
        // V312-64 / Issue #4662: `INSTEAD OF` trigger timing for views.
        "INSTEAD" => Some(Token::Instead),
        // V312-64 / Issue #4663: SQLite-style maintenance commands.
        "VACUUM" => Some(Token::Vacuum),
        "REINDEX" => Some(Token::Reindex),
        // F-30 CREATE SEQUENCE
        "SEQUENCE" => Some(Token::Sequence),
        "CYCLE" => Some(Token::Cycle),
        "NOCYCLE" => Some(Token::NoCycle),
        "CACHE" => Some(Token::Cache),
        "INCREMENT" => Some(Token::Increment),
        "OWNED" => Some(Token::Owned),
        "NEXT" => Some(Token::NextValue),
        "CURRVAL" => Some(Token::Currval),
        "RESTART" => Some(Token::Restart),
        "NO" => Some(Token::No),
        "BY" => Some(Token::By),
        "START" => Some(Token::Start),
        "MINVALUE" => Some(Token::Minvalue),
        "MAXVALUE" => Some(Token::Maxvalue),
        "NOMINVALUE" => Some(Token::NoMinValue),
        "NOMAXVALUE" => Some(Token::NoMaxValue),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_display() {
        assert_eq!(Token::Select.to_string(), "SELECT");
        assert_eq!(Token::Integer.to_string(), "INTEGER");
        assert_eq!(
            Token::Identifier("users".to_string()).to_string(),
            "IDENTIFIER(users)"
        );
        assert_eq!(
            Token::StringLiteral("hello".to_string()).to_string(),
            "'hello'"
        );
        assert_eq!(Token::NumberLiteral("42".to_string()).to_string(), "42");
    }

    #[test]
    fn test_token_display_operators() {
        assert_eq!(Token::Equal.to_string(), "=");
        assert_eq!(Token::NotEqual.to_string(), "<>");
        assert_eq!(Token::Greater.to_string(), ">");
        assert_eq!(Token::Less.to_string(), "<");
        assert_eq!(Token::GreaterEqual.to_string(), ">=");
        assert_eq!(Token::LessEqual.to_string(), "<=");
        assert_eq!(Token::And.to_string(), "AND");
        assert_eq!(Token::Or.to_string(), "OR");
        assert_eq!(Token::Not.to_string(), "NOT");
        assert_eq!(Token::Plus.to_string(), "+");
        assert_eq!(Token::Minus.to_string(), "-");
        assert_eq!(Token::Asterisk.to_string(), "*");
        assert_eq!(Token::Slash.to_string(), "/");
        assert_eq!(Token::Percent.to_string(), "%");
    }

    #[test]
    fn test_token_display_syntax() {
        assert_eq!(Token::LParen.to_string(), "(");
        assert_eq!(Token::RParen.to_string(), ")");
        assert_eq!(Token::Comma.to_string(), ",");
        assert_eq!(Token::Dot.to_string(), ".");
        assert_eq!(Token::Semicolon.to_string(), ";");
        assert_eq!(Token::Colon.to_string(), ":");
        assert_eq!(Token::SingleQuote.to_string(), "'");
        assert_eq!(Token::Star.to_string(), "*");
        assert_eq!(Token::Eof.to_string(), "EOF");
    }

    #[test]
    fn test_token_display_keywords() {
        assert_eq!(Token::From.to_string(), "FROM");
        assert_eq!(Token::Where.to_string(), "WHERE");
        assert_eq!(Token::Insert.to_string(), "INSERT");
        assert_eq!(Token::Into.to_string(), "INTO");
        assert_eq!(Token::Values.to_string(), "VALUES");
        assert_eq!(Token::Update.to_string(), "UPDATE");
        assert_eq!(Token::Set.to_string(), "SET");
        assert_eq!(Token::Delete.to_string(), "DELETE");
        assert_eq!(Token::Create.to_string(), "CREATE");
        assert_eq!(Token::Table.to_string(), "TABLE");
        assert_eq!(Token::Drop.to_string(), "DROP");
        assert_eq!(Token::Alter.to_string(), "ALTER");
        assert_eq!(Token::Index.to_string(), "INDEX");
    }

    #[test]
    fn test_token_display_data_types() {
        assert_eq!(Token::Text.to_string(), "TEXT");
        assert_eq!(Token::Float.to_string(), "FLOAT");
        assert_eq!(Token::Boolean.to_string(), "BOOLEAN");
        assert_eq!(Token::Blob.to_string(), "BLOB");
        assert_eq!(Token::Null.to_string(), "NULL");
    }

    #[test]
    fn test_token_display_transaction() {
        assert_eq!(Token::Begin.to_string(), "BEGIN");
        assert_eq!(Token::Commit.to_string(), "COMMIT");
        assert_eq!(Token::Rollback.to_string(), "ROLLBACK");
        assert_eq!(Token::Grant.to_string(), "GRANT");
        assert_eq!(Token::Revoke.to_string(), "REVOKE");
    }

    #[test]
    fn test_token_display_other_keywords() {
        assert_eq!(Token::On.to_string(), "ON");
        assert_eq!(Token::Primary.to_string(), "PRIMARY");
        assert_eq!(Token::Key.to_string(), "KEY");
    }

    #[test]
    fn test_token_boolean_literal() {
        assert_eq!(Token::BooleanLiteral(true).to_string(), "true");
        assert_eq!(Token::BooleanLiteral(false).to_string(), "false");
    }

    #[test]
    fn test_token_clone() {
        let token = Token::Identifier("test".to_string());
        let cloned = token.clone();
        assert_eq!(token, cloned);
    }

    #[test]
    fn test_is_keyword() {
        assert!(is_keyword("SELECT"));
        assert!(is_keyword("select"));
        assert!(!is_keyword("users"));
        assert!(is_keyword("TRUE"));
        assert!(is_keyword("null"));
    }

    #[test]
    fn test_is_keyword_various() {
        // Data type keywords
        assert!(is_keyword("INTEGER"));
        assert!(is_keyword("TEXT"));
        assert!(is_keyword("FLOAT"));
        assert!(is_keyword("BOOLEAN"));
        assert!(is_keyword("BLOB"));

        // Boolean keywords
        assert!(is_keyword("TRUE"));
        assert!(is_keyword("FALSE"));

        // Logical operators
        assert!(is_keyword("AND"));
        assert!(is_keyword("OR"));
        assert!(is_keyword("NOT"));

        // Non-keyword
        assert!(!is_keyword("users"));
        assert!(!is_keyword("foo"));
        assert!(!is_keyword("bar123"));
    }

    #[test]
    fn test_token_from_keyword() {
        assert_eq!(from_keyword("SELECT"), Some(Token::Select));
        assert_eq!(from_keyword("INSERT"), Some(Token::Insert));
        assert_eq!(from_keyword("UNKNOWN"), None);
        assert_eq!(from_keyword("select"), Some(Token::Select));
        assert_eq!(from_keyword("TRUE"), Some(Token::BooleanLiteral(true)));
    }

    #[test]
    fn test_token_from_keyword_data_type() {
        assert_eq!(from_keyword("DATA"), Some(Token::Data));
        assert_eq!(from_keyword("TYPE"), Some(Token::Type));
        assert_eq!(from_keyword("data"), Some(Token::Data));
        assert_eq!(from_keyword("type"), Some(Token::Type));
    }

    #[test]
    fn test_token_display_data_type_keywords() {
        assert_eq!(Token::Data.to_string(), "DATA");
        assert_eq!(Token::Type.to_string(), "TYPE");
    }

    #[test]
    fn test_from_keyword_various() {
        // DML keywords
        assert_eq!(from_keyword("FROM"), Some(Token::From));
        assert_eq!(from_keyword("WHERE"), Some(Token::Where));
        assert_eq!(from_keyword("UPDATE"), Some(Token::Update));
        assert_eq!(from_keyword("SET"), Some(Token::Set));
        assert_eq!(from_keyword("DELETE"), Some(Token::Delete));

        // DDL keywords
        assert_eq!(from_keyword("CREATE"), Some(Token::Create));
        assert_eq!(from_keyword("TABLE"), Some(Token::Table));
        assert_eq!(from_keyword("DROP"), Some(Token::Drop));
        assert_eq!(from_keyword("ALTER"), Some(Token::Alter));
        assert_eq!(from_keyword("INDEX"), Some(Token::Index));

        // Transaction keywords
        assert_eq!(from_keyword("BEGIN"), Some(Token::Begin));
        assert_eq!(from_keyword("COMMIT"), Some(Token::Commit));
        assert_eq!(from_keyword("ROLLBACK"), Some(Token::Rollback));

        // Data types
        assert_eq!(from_keyword("INTEGER"), Some(Token::Integer));
        assert_eq!(from_keyword("TEXT"), Some(Token::Text));
        assert_eq!(from_keyword("FLOAT"), Some(Token::Float));
        assert_eq!(from_keyword("BOOLEAN"), Some(Token::Boolean));
        assert_eq!(from_keyword("BLOB"), Some(Token::Blob));
        assert_eq!(from_keyword("NULL"), Some(Token::Null));

        // Boolean literals
        assert_eq!(from_keyword("FALSE"), Some(Token::BooleanLiteral(false)));

        // Logical operators
        assert_eq!(from_keyword("AND"), Some(Token::And));
        assert_eq!(from_keyword("OR"), Some(Token::Or));
        assert_eq!(from_keyword("NOT"), Some(Token::Not));

        // Case insensitive
        assert_eq!(from_keyword("Select"), Some(Token::Select));
        assert_eq!(from_keyword("FROM"), Some(Token::From));

        // Non-keywords return None
        assert_eq!(from_keyword("users"), None);
        assert_eq!(from_keyword("foo"), None);
    }

    #[test]
    fn test_token_debug() {
        let token = Token::Identifier("test".to_string());
        let debug_str = format!("{:?}", token);
        assert!(debug_str.contains("Identifier"));
    }

    #[test]
    fn test_token_display_dml_keywords() {
        assert_eq!(Token::Merge.to_string(), "MERGE");
        assert_eq!(Token::Using.to_string(), "USING");
        assert_eq!(Token::On.to_string(), "ON");
        assert_eq!(Token::Of.to_string(), "OF");
        assert_eq!(Token::When.to_string(), "WHEN");
        assert_eq!(Token::Matched.to_string(), "MATCHED");
    }

    #[test]
    fn test_token_display_ddl_keywords() {
        assert_eq!(Token::Add.to_string(), "ADD");
        assert_eq!(Token::Column.to_string(), "COLUMN");
        assert_eq!(Token::Rename.to_string(), "RENAME");
        assert_eq!(Token::To.to_string(), "TO");
        assert_eq!(Token::Default.to_string(), "DEFAULT");
    }

    #[test]
    fn test_token_display_constraint_keywords() {
        assert_eq!(Token::Foreign.to_string(), "FOREIGN");
        assert_eq!(Token::References.to_string(), "REFERENCES");
        assert_eq!(Token::Unique.to_string(), "UNIQUE");
        assert_eq!(Token::Check.to_string(), "CHECK");
        assert_eq!(Token::Constraint.to_string(), "CONSTRAINT");
        assert_eq!(Token::Cascade.to_string(), "CASCADE");
        assert_eq!(Token::Restrict.to_string(), "RESTRICT");
    }

    #[test]
    fn test_token_display_subquery_keywords() {
        assert_eq!(Token::Exists.to_string(), "EXISTS");
        assert_eq!(Token::In.to_string(), "IN");
        assert_eq!(Token::Is.to_string(), "IS");
        assert_eq!(Token::All.to_string(), "ALL");
        assert_eq!(Token::Any.to_string(), "ANY");
        assert_eq!(Token::Some.to_string(), "SOME");
    }

    #[test]
    fn test_token_display_conditional() {
        assert_eq!(Token::If.to_string(), "IF");
        assert_eq!(Token::Case.to_string(), "CASE");
        assert_eq!(Token::Then.to_string(), "THEN");
        assert_eq!(Token::Else.to_string(), "ELSE");
    }

    #[test]
    fn test_token_display_cte() {
        assert_eq!(Token::With.to_string(), "WITH");
        assert_eq!(Token::Recursive.to_string(), "RECURSIVE");
        assert_eq!(Token::As.to_string(), "AS");
    }

    #[test]
    fn test_token_display_aggregate() {
        assert_eq!(Token::Count.to_string(), "COUNT");
        assert_eq!(Token::Sum.to_string(), "SUM");
        assert_eq!(Token::Avg.to_string(), "AVG");
        assert_eq!(Token::Min.to_string(), "MIN");
        assert_eq!(Token::Max.to_string(), "MAX");
    }

    #[test]
    fn test_token_display_join() {
        assert_eq!(Token::Join.to_string(), "JOIN");
        assert_eq!(Token::Inner.to_string(), "INNER");
        assert_eq!(Token::Left.to_string(), "LEFT");
        assert_eq!(Token::Right.to_string(), "RIGHT");
        assert_eq!(Token::Full.to_string(), "FULL");
        assert_eq!(Token::Cross.to_string(), "CROSS");
        assert_eq!(Token::Outer.to_string(), "OUTER");
        assert_eq!(Token::Natural.to_string(), "NATURAL");
    }

    #[test]
    fn test_token_display_window_func_keywords() {
        assert_eq!(Token::Window.to_string(), "WINDOW");
        assert_eq!(Token::Partition.to_string(), "PARTITION");
        assert_eq!(Token::Partitions.to_string(), "PARTITIONS");
        assert_eq!(Token::Range.to_string(), "RANGE");
        assert_eq!(Token::List.to_string(), "LIST");
        assert_eq!(Token::Over.to_string(), "OVER");
        assert_eq!(Token::Between.to_string(), "BETWEEN");
        assert_eq!(Token::Unbounded.to_string(), "UNBOUNDED");
        assert_eq!(Token::Preceding.to_string(), "PRECEDING");
        assert_eq!(Token::Following.to_string(), "FOLLOWING");
        assert_eq!(Token::Current.to_string(), "CURRENT");
        assert_eq!(Token::Row.to_string(), "ROW");
        assert_eq!(Token::Rows.to_string(), "ROWS");
    }

    #[test]
    fn test_token_display_admin_keywords() {
        assert_eq!(Token::Analyze.to_string(), "ANALYZE");
        assert_eq!(Token::Truncate.to_string(), "TRUNCATE");
        assert_eq!(Token::Replace.to_string(), "REPLACE");
        assert_eq!(Token::Ignore.to_string(), "IGNORE");
        assert_eq!(Token::No.to_string(), "NO");
        assert_eq!(Token::Action.to_string(), "ACTION");
        assert_eq!(Token::AutoIncrement.to_string(), "AUTO_INCREMENT");
        assert_eq!(Token::Write.to_string(), "WRITE");
        assert_eq!(Token::Only.to_string(), "ONLY");
        assert_eq!(Token::SqlCalcFoundRows.to_string(), "SQL_CALC_FOUND_ROWS");
    }

    #[test]
    fn test_token_display_mysql_keywords() {
        assert_eq!(Token::Duplicate.to_string(), "DUPLICATE");
        assert_eq!(Token::Database.to_string(), "DATABASE");
        assert_eq!(Token::Use.to_string(), "USE");
        assert_eq!(Token::View.to_string(), "VIEW");
        assert_eq!(Token::HighPriority.to_string(), "HIGH_PRIORITY");
        assert_eq!(Token::SqlCache.to_string(), "SQL_CACHE");
        assert_eq!(Token::SqlNoCache.to_string(), "SQL_NO_CACHE");
        assert_eq!(Token::Convert.to_string(), "CONVERT");
    }

    #[test]
    fn test_token_display_aggregate_set() {
        assert_eq!(Token::Grouping.to_string(), "GROUPING");
        assert_eq!(Token::Rollup.to_string(), "ROLLUP");
        assert_eq!(Token::Cube.to_string(), "CUBE");
        assert_eq!(Token::Group.to_string(), "GROUP");
        assert_eq!(Token::By.to_string(), "BY");
        assert_eq!(Token::Having.to_string(), "HAVING");
        assert_eq!(Token::Order.to_string(), "ORDER");
        assert_eq!(Token::Limit.to_string(), "LIMIT");
        assert_eq!(Token::Offset.to_string(), "OFFSET");
        assert_eq!(Token::Distinct.to_string(), "DISTINCT");
        assert_eq!(Token::Union.to_string(), "UNION");
        assert_eq!(Token::Intersect.to_string(), "INTERSECT");
        assert_eq!(Token::Except.to_string(), "EXCEPT");
        assert_eq!(Token::AsOf.to_string(), "ASOF");
        assert_eq!(Token::Fulltext.to_string(), "FULLTEXT");
    }

    #[test]
    fn test_token_display_trigger() {
        assert_eq!(Token::Trigger.to_string(), "TRIGGER");
        assert_eq!(Token::Before.to_string(), "BEFORE");
        assert_eq!(Token::After.to_string(), "AFTER");
        assert_eq!(Token::ForEach.to_string(), "FOR EACH");
        assert_eq!(Token::Each.to_string(), "EACH");
    }

    #[test]
    fn test_token_display_functions() {
        assert_eq!(Token::Nulls.to_string(), "NULLS");
        assert_eq!(Token::First.to_string(), "FIRST");
        assert_eq!(Token::Last.to_string(), "LAST");
        assert_eq!(Token::Asc.to_string(), "ASC");
        assert_eq!(Token::Desc.to_string(), "DESC");
        assert_eq!(Token::Like.to_string(), "LIKE");
        assert_eq!(Token::Escape.to_string(), "ESCAPE");
        assert_eq!(Token::Maxvalue.to_string(), "MAXVALUE");
        assert_eq!(Token::Minvalue.to_string(), "MINVALUE");
        assert_eq!(Token::Date.to_string(), "DATE");
        // V312-64: new keyword tokens added for #4662/#4663.
        assert_eq!(Token::Instead.to_string(), "INSTEAD");
        assert_eq!(Token::Vacuum.to_string(), "VACUUM");
        assert_eq!(Token::Reindex.to_string(), "REINDEX");
        assert_eq!(Token::Substring.to_string(), "SUBSTRING");
        assert_eq!(Token::Position.to_string(), "POSITION");
        assert_eq!(Token::Interval.to_string(), "INTERVAL");
    }

    #[test]
    fn test_token_display_transaction_keywords() {
        assert_eq!(Token::Transaction.to_string(), "TRANSACTION");
        assert_eq!(Token::Work.to_string(), "WORK");
        assert_eq!(Token::Savepoint.to_string(), "SAVEPOINT");
        assert_eq!(Token::Release.to_string(), "RELEASE");
        assert_eq!(Token::Start.to_string(), "START");
        assert_eq!(Token::Isolation.to_string(), "ISOLATION");
        assert_eq!(Token::Level.to_string(), "LEVEL");
        assert_eq!(Token::Serializable.to_string(), "SERIALIZABLE");
        assert_eq!(Token::Repeatable.to_string(), "REPEATABLE");
        assert_eq!(Token::Read.to_string(), "READ");
        assert_eq!(Token::Committed.to_string(), "COMMITTED");
        assert_eq!(Token::Uncommitted.to_string(), "UNCOMMITTED");
    }

    #[test]
    fn test_token_display_stored_proc() {
        assert_eq!(Token::Call.to_string(), "CALL");
        assert_eq!(Token::Procedure.to_string(), "PROCEDURE");
        assert_eq!(Token::End.to_string(), "END");
        assert_eq!(Token::Prepare.to_string(), "PREPARE");
        assert_eq!(Token::Execute.to_string(), "EXECUTE");
        assert_eq!(Token::Deallocate.to_string(), "DEALLOCATE");
    }

    #[test]
    fn test_token_display_show() {
        assert_eq!(Token::Show.to_string(), "SHOW");
        assert_eq!(Token::Describe.to_string(), "DESCRIBE");
        assert_eq!(Token::Role.to_string(), "ROLE");
        assert_eq!(Token::Roles.to_string(), "ROLES");
        assert_eq!(Token::Parent.to_string(), "PARENT");
        assert_eq!(Token::Grants.to_string(), "GRANTS");
        assert_eq!(Token::For.to_string(), "FOR");
    }

    #[test]
    fn test_token_display_complex_ops() {
        assert_eq!(Token::JsonArrow.to_string(), "->");
        assert_eq!(Token::JsonArrowText.to_string(), "->>");
    }

    #[test]
    fn test_token_display_syntax_alt() {
        assert_eq!(Token::SingleQuote.to_string(), "'");
        assert_eq!(Token::Star.to_string(), "*");
        assert_eq!(Token::Eof.to_string(), "EOF");
        assert_eq!(Token::NotEqual.to_string(), "<>");
        assert_eq!(Token::LessEqual.to_string(), "<=");
        assert_eq!(Token::GreaterEqual.to_string(), ">=");
    }

    #[test]
    fn test_token_boolean_literal_display() {
        assert_eq!(Token::BooleanLiteral(true).to_string(), "true");
        assert_eq!(Token::BooleanLiteral(false).to_string(), "false");
    }
}
