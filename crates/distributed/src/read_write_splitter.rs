//! Read/Write Splitter — classifies SQL statements and routes them to
//! primary or replica nodes.
//!
//! # API surface
//!
//! - [`QueryClass`] — `Read` or `Write` enum.
//! - [`ReadWriteSplitter`] — owns the local node id and exposes
//!   [`ReadWriteSplitter::classify`] (parse + classify a SQL string) and
//!   [`ReadWriteSplitter::route_simple`] (parse + classify + answer
//!   "should this go to the primary?").
//! - [`classify_statement`] — pure function on a parsed
//!   [`sqlrustgo_parser::Statement`]; no I/O, easy to unit test.
//!
//! # Classification rule (SEM-1 / #3172)
//!
//! `SAVEPOINT` / `ROLLBACK TO SAVEPOINT` / `RELEASE SAVEPOINT` all
//! modify per-transaction undo-log state on the primary node, so the
//! splitter MUST classify `Statement::SavepointStatement { .. }` as
//! [`QueryClass::Write`]. Sending these to a read replica would route
//! the undo-log mutation away from the primary and corrupt replication.
//!
//! The arm below is the canonical source for that rule. It is asserted
//! by `tests/integration/transaction/sem1_savepoint_test.rs`.

use sqlrustgo_parser::{parse, Statement};

/// Whether a query is read-only or writes through the primary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryClass {
    /// Read-only — safe to route to a replica.
    Read,
    /// Writes through the primary.
    Write,
}

impl QueryClass {
    /// True iff this class is [`QueryClass::Read`].
    pub fn is_read(&self) -> bool {
        matches!(self, QueryClass::Read)
    }

    /// True iff this class is [`QueryClass::Write`].
    pub fn is_write(&self) -> bool {
        matches!(self, QueryClass::Write)
    }
}

/// Result of [`ReadWriteSplitter::route_simple`] — the query class plus
/// whether the local node should serve as the primary for the call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteDecision {
    pub query_class: QueryClass,
    pub is_primary: bool,
}

/// Errors returned by the splitter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SplitterError {
    /// The SQL string failed to parse.
    ParseError(String),
}

impl std::fmt::Display for SplitterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SplitterError::ParseError(msg) => write!(f, "parse error: {}", msg),
        }
    }
}

impl std::error::Error for SplitterError {}

/// Pure classification: given a parsed `Statement`, return its
/// [`QueryClass`]. See the module-level docs for the SAVEPOINT rule.
///
/// # Coverage
///
/// Every variant of [`sqlrustgo_parser::Statement`] is enumerated below.
/// Adding a new variant to the AST must add a match arm here; the
/// `match` is intentionally non-exhaustive (no `_ => ...`) so future
/// additions fail to compile and force an explicit routing decision.
pub fn classify_statement(statement: &Statement) -> QueryClass {
    match statement {
        // ---- Read-only ----------------------------------------------------
        Statement::Select(_) => QueryClass::Read,
        Statement::Explain(_) => QueryClass::Read,
        Statement::Show(_) => QueryClass::Read,
        Statement::Describe(_) => QueryClass::Read,
        Statement::Call(_) => QueryClass::Read,
        Statement::Union(_) => QueryClass::Read,
        // V310-06 PR2: set-op chains also read from both sides —
        // route them to a replica just like UNION.
        Statement::Intersect(_) => QueryClass::Read,
        Statement::Except(_) => QueryClass::Read,
        Statement::WithSelect(_) => QueryClass::Read,
        // WITH-clause followed by DML: recurse on the DML body.
        Statement::WithDml(with_dml) => classify_statement(&with_dml.body),
        Statement::Values(_) => QueryClass::Read,
        Statement::ShowRoles => QueryClass::Read,
        Statement::ShowGrantsFor(_) => QueryClass::Read,
        Statement::Execute { .. } => QueryClass::Read,

        // ---- DML writes ---------------------------------------------------
        Statement::Insert(_) => QueryClass::Write,
        Statement::Update(_) => QueryClass::Write,
        Statement::Delete(_) => QueryClass::Write,
        Statement::Merge(_) => QueryClass::Write,

        // ---- DDL writes ---------------------------------------------------
        Statement::CreateTable(_) => QueryClass::Write,
        Statement::CreateIndex(_) => QueryClass::Write,
        Statement::CreateView(_) => QueryClass::Write,
        Statement::DropTable(_) => QueryClass::Write,
        Statement::DropIndex(_) => QueryClass::Write,
        Statement::DropView(_) => QueryClass::Write,
        Statement::CreateSequence(_) => QueryClass::Write,
        Statement::DropSequence(_) => QueryClass::Write,
        Statement::AlterSequence(_) => QueryClass::Write,
        Statement::Truncate(_) => QueryClass::Write,
        Statement::Analyze(_) => QueryClass::Write,
        Statement::AlterTable(_) => QueryClass::Write,
        Statement::AlterUser(_) => QueryClass::Write,
        Statement::CreateUser(_) => QueryClass::Write,
        Statement::DropUser(_) => QueryClass::Write,
        Statement::CreateProcedure(_) => QueryClass::Write,
        Statement::DropProcedure(_) => QueryClass::Write,
        Statement::CreateFunction(_) => QueryClass::Write,
        Statement::DropFunction(_) => QueryClass::Write,
        Statement::CreateTrigger(_) => QueryClass::Write,
        Statement::DropTrigger(_) => QueryClass::Write,
        Statement::CreateRole(_) => QueryClass::Write,
        Statement::DropRole(_) => QueryClass::Write,
        Statement::CreateDatabase(_) => QueryClass::Write,
        Statement::DropDatabase(_) => QueryClass::Write,
        Statement::UseDatabase(_) => QueryClass::Write,
        Statement::GrantRole(_) => QueryClass::Write,
        Statement::RevokeRole(_) => QueryClass::Write,
        Statement::SetRole(_) => QueryClass::Write,

        // ---- Permissions --------------------------------------------------
        Statement::Grant(_) => QueryClass::Write,
        Statement::Revoke(_) => QueryClass::Write,

        // ---- Transaction control -----------------------------------------
        // BEGIN / COMMIT / ROLLBACK all mutate tx state on the primary.
        Statement::Transaction(_) => QueryClass::Write,
        // SEM-1 (#3172): SAVEPOINT/ROLLBACK TO SAVEPOINT/RELEASE
        // SAVEPOINT modify per-tx undo log; treat as Write so the
        // distributed router sends them to the primary.
        Statement::SavepointStatement { .. } => QueryClass::Write,

        // ---- Prepared statements -----------------------------------------
        Statement::Prepare { .. } => QueryClass::Write,
        Statement::Deallocate { .. } => QueryClass::Write,

        // ---- Connection control ------------------------------------------
        Statement::Kill { .. } => QueryClass::Write,
    }
}

/// Read/write splitter. In this drop it only tracks the local node id;
/// the full shard-router plumbing (replica selection, consistency
/// inference, shard key extraction) is reintroduced with the v3.13+
/// distributed-routing track.
#[derive(Debug, Clone)]
pub struct ReadWriteSplitter {
    /// Local node id (1 for single-node setups).
    local_node_id: u64,
}

impl ReadWriteSplitter {
    /// Create a splitter that considers node `local_node_id` to be the
    /// local primary. Mirrors the constructor used by
    /// `crates/server/src/openclaw_endpoints.rs::execute_sql`.
    pub fn new(local_node_id: u64) -> Self {
        Self { local_node_id }
    }

    /// Local node id.
    pub fn local_node_id(&self) -> u64 {
        self.local_node_id
    }

    /// Parse `sql` and return its [`QueryClass`] plus a debug string of
    /// the AST. Errors surface as [`SplitterError::ParseError`].
    pub fn classify(&self, sql: &str) -> Result<(QueryClass, String), SplitterError> {
        let statement = parse(sql).map_err(|e| SplitterError::ParseError(format!("{:?}", e)))?;
        let query_class = classify_statement(&statement);
        Ok((query_class, format!("{:?}", statement)))
    }

    /// Parse `sql` and answer "which node should serve this?" — returns
    /// the [`QueryClass`] and a flag indicating whether the local node
    /// is the primary for the call.
    ///
    /// In single-node setups (which is all this crate supports today),
    /// every query goes to the local primary — `is_primary` is always
    /// `true` regardless of [`QueryClass`]. The flag is preserved so the
    /// caller signature matches the v3.11 deleted-crate archive and the
    /// T-27 routing helper in `crates/server/src/openclaw_endpoints.rs`.
    pub fn route_simple(&self, sql: &str) -> Result<(QueryClass, bool), SplitterError> {
        let (query_class, _) = self.classify(sql)?;
        Ok((query_class, true))
    }

    /// Pure-function form of [`Self::route_simple`] for callers that
    /// already hold a parsed [`Statement`].
    pub fn decide(&self, statement: &Statement) -> RouteDecision {
        RouteDecision {
            query_class: classify_statement(statement),
            is_primary: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classify_sql(sql: &str) -> QueryClass {
        let stmt = parse(sql).expect("test SQL must parse");
        classify_statement(&stmt)
    }

    #[test]
    fn classify_select_as_read() {
        assert_eq!(classify_sql("SELECT 1"), QueryClass::Read);
    }

    #[test]
    fn classify_insert_as_write() {
        assert_eq!(classify_sql("INSERT INTO t VALUES (1)"), QueryClass::Write);
    }

    #[test]
    fn classify_update_as_write() {
        assert_eq!(classify_sql("UPDATE t SET a = 1"), QueryClass::Write);
    }

    #[test]
    fn classify_delete_as_write() {
        assert_eq!(classify_sql("DELETE FROM t"), QueryClass::Write);
    }

    #[test]
    fn classify_savepoint_as_write_sem1() {
        // SEM-1 (#3172): SAVEPOINT must be Write — the primary owns the
        // per-tx undo log.
        assert_eq!(
            classify_sql("SAVEPOINT sp1"),
            QueryClass::Write,
            "SAVEPOINT must be classified as Write (SEM-1 / #3172)"
        );
        assert_eq!(
            classify_sql("ROLLBACK TO SAVEPOINT sp1"),
            QueryClass::Write,
            "ROLLBACK TO SAVEPOINT must be classified as Write (SEM-1 / #3172)"
        );
        assert_eq!(
            classify_sql("RELEASE SAVEPOINT sp1"),
            QueryClass::Write,
            "RELEASE SAVEPOINT must be classified as Write (SEM-1 / #3172)"
        );
    }

    #[test]
    fn classify_transaction_as_write() {
        assert_eq!(classify_sql("BEGIN"), QueryClass::Write);
        assert_eq!(classify_sql("COMMIT"), QueryClass::Write);
        assert_eq!(classify_sql("ROLLBACK"), QueryClass::Write);
    }

    #[test]
    fn classify_show_describe_as_read() {
        assert_eq!(classify_sql("SHOW TABLES"), QueryClass::Read);
        assert_eq!(classify_sql("DESCRIBE t"), QueryClass::Read);
    }

    #[test]
    fn splitter_route_simple_returns_class_and_primary() {
        let splitter = ReadWriteSplitter::new(1);
        let (class, is_primary) = splitter.route_simple("SELECT 1").unwrap();
        assert_eq!(class, QueryClass::Read);
        assert!(is_primary, "single-node setup routes everything to primary");

        let (class, is_primary) = splitter.route_simple("SAVEPOINT sp1").unwrap();
        assert_eq!(class, QueryClass::Write);
        assert!(is_primary);
    }
}
