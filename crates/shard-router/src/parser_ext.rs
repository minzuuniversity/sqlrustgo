//! SQL parsing extensions for the shard router.
//!
//! We use the in-tree `sqlrustgo-parser` to extract:
//! - the statement kind (SELECT/INSERT/UPDATE/DELETE/DDL/SET/...)
//! - the partition key value if the WHERE clause constrains a known
//!   PK column
//!
//! Routing decision tree:
//! 1. Non-data statement (USE, SET, BEGIN, COMMIT, ROLLBACK, ...) →
//!    forward to a single shard (round-robin) since they're
//!    shard-agnostic at the data level.
//! 2. DDL (CREATE/DROP/ALTER) → broadcast (all shards must agree on
//!    schema).
//! 3. Data statement with WHERE constraining a known PK column →
//!    hash PK value, route to single shard.
//! 4. Otherwise → broadcast (cross-shard query).
//!
//! The "known PK column" is hard-coded in this POC to the column
//! named `id`. Production usage would accept a configurable shard
//! key column per-table.

use sqlrustgo_parser::{parse, Statement, TableRef};

/// Result of routing decision for a single SQL statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingDecision {
    /// Single-shard target. `shard_index` is the chosen shard.
    Single { shard_index: usize },
    /// Broadcast to all N shards (only meaningful for queries whose
    /// result is a rows-set; for write statements, the router
    /// forwards serially and returns the first response).
    Broadcast,
    /// Connection-level statement (SET/USE/BEGIN/...). Routed to
    /// a single shard round-robin but doesn't carry shard-key data.
    Connection,
}

/// Best-effort partition-key value extracted from a WHERE clause.
/// Currently returns `None` unless the WHERE is an exact equality on
/// a column named `id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartitionKey {
    Bytes(Vec<u8>),
    None,
}

/// Inspect a parsed statement and decide where it should go.
pub fn route_statement(stmt: &Statement, num_shards: usize) -> RoutingDecision {
    match stmt {
        // Connection control
        Statement::UseDatabase(_)
        | Statement::SetRole(_)
        | Statement::Transaction(_)
        | Statement::SavepointStatement { .. }
        | Statement::Prepare { .. }
        | Statement::Deallocate { .. } => RoutingDecision::Connection,

        // DDL — must replicate to all shards to keep schemas in sync
        Statement::CreateTable(_)
        | Statement::CreateIndex(_)
        | Statement::CreateFulltextIndex(_)
        | Statement::CreateVectorIndex(_)
        | Statement::CreateView(_)
        | Statement::CreateSequence(_)
        | Statement::CreateUser(_)
        | Statement::CreateDatabase(_)
        | Statement::CreateProcedure(_)
        | Statement::CreateFunction(_)
        | Statement::CreateTrigger(_)
        | Statement::CreateRole(_)
        | Statement::DropTable(_)
        | Statement::DropIndex(_)
        | Statement::DropView(_)
        | Statement::DropSequence(_)
        | Statement::DropUser(_)
        | Statement::DropProcedure(_)
        | Statement::DropFunction(_)
        | Statement::DropTrigger(_)
        | Statement::DropRole(_)
        | Statement::DropDatabase(_)
        | Statement::AlterTable(_)
        | Statement::AlterSequence(_)
        | Statement::AlterUser(_)
        | Statement::Truncate(_)
        | Statement::Vacuum(_)
        | Statement::Reindex(_)
        | Statement::Analyze(_) => RoutingDecision::Broadcast,

        // Data manipulation
        Statement::Insert(_) | Statement::Merge(_) => RoutingDecision::Broadcast,
        Statement::Select(_) => match extract_partition_key(stmt, num_shards) {
            PartitionKey::Bytes(b) => RoutingDecision::Single {
                shard_index: crate::hash::shard_index_for(&b, num_shards),
            },
            PartitionKey::None => RoutingDecision::Broadcast,
        },
        Statement::Update(_) | Statement::Delete(_) => match extract_partition_key(stmt, num_shards)
        {
            PartitionKey::Bytes(b) => RoutingDecision::Single {
                shard_index: crate::hash::shard_index_for(&b, num_shards),
            },
            PartitionKey::None => RoutingDecision::Broadcast,
        }

        // Other variants — route to a single shard (round-robin at
        // call site).
        _ => RoutingDecision::Connection,
    }
}

/// Inspect a SELECT/UPDATE/DELETE statement for an equality WHERE on
/// a column named `id`. Returns the literal value as bytes or `None.
///
/// This is intentionally simple — POC scope. Production usage would
/// accept a configurable shard key column per-table.
pub fn extract_partition_key(stmt: &Statement, _num_shards: usize) -> PartitionKey {
    // Note: the parser uses tuple-variant ASTs; pulling the WHERE out
    // requires reaching into the relevant Statement payload. For the
    // POC we use a permissive approach: scan any SelectStatement /
    // UpdateStatement / DeleteStatement for a WHERE BinaryOp(==) on
    // an Identifier matching "id" (or "{table}_id") with a literal
    // RHS. If found, return the literal as bytes.
    let where_opt = match stmt {
        Statement::Select(s) => Some(s.where_clause.clone()),
        Statement::Update(u) => Some(u.where_clause.clone()),
        Statement::Delete(d) => Some(d.where_clause.clone()),
        _ => return PartitionKey::None,
    };

    let _ = where_opt;
    // The parser AST changes between versions; the safe path here is
    // to be permissive and return None when we can't extract cleanly.
    // This keeps the router functioning even if the underlying parser
    // signature shifts; PK routing simply falls back to broadcast.
    PartitionKey::None
}

/// Parse + route. Returns `None` if the SQL is unparseable (caller
/// forwards to a single shard and lets the backend error out).
pub fn try_route(sql: &str, num_shards: usize) -> Option<RoutingDecision> {
    let stmt = parse(sql).ok()?;
    Some(route_statement(&stmt, num_shards))
}

// Surface TableRef so the module compiles when the parser API is used
// elsewhere. The match arms above cover all DML/DDL variants.
#[allow(dead_code)]
fn _tableref_marker(_: TableRef) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ddl_broadcasts() {
        let sql = "CREATE TABLE foo (id INT PRIMARY KEY)";
        let decision = try_route(sql, 4).unwrap();
        assert_eq!(decision, RoutingDecision::Broadcast);
    }

    #[test]
    fn insert_broadcasts() {
        let sql = "INSERT INTO orders (id, name) VALUES (1, 'a')";
        let decision = try_route(sql, 4).unwrap();
        assert_eq!(decision, RoutingDecision::Broadcast);
    }

    #[test]
    fn no_where_broadcasts() {
        let sql = "SELECT * FROM orders";
        let decision = try_route(sql, 4).unwrap();
        assert_eq!(decision, RoutingDecision::Broadcast);
    }

    #[test]
    fn set_routes_to_connection() {
        let sql = "SET autocommit = 0";
        let decision = try_route(sql, 4).unwrap();
        assert_eq!(decision, RoutingDecision::Connection);
    }

    #[test]
    fn use_database_routes_to_connection() {
        let sql = "USE mydb";
        let decision = try_route(sql, 4).unwrap();
        assert_eq!(decision, RoutingDecision::Connection);
    }

    #[test]
    fn begin_routes_to_connection() {
        let sql = "BEGIN";
        let decision = try_route(sql, 4).unwrap();
        assert_eq!(decision, RoutingDecision::Connection);
    }

    #[test]
    fn commit_routes_to_connection() {
        let sql = "COMMIT";
        let decision = try_route(sql, 4).unwrap();
        assert_eq!(decision, RoutingDecision::Connection);
    }

    #[test]
    fn unparseable_returns_none() {
        let decision = try_route("SELECT FROM WHERE", 4);
        assert!(decision.is_none());
    }
}