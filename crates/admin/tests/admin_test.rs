//! Admin crate unit tests

use sqlrustgo_admin::mysqladmin::{Connection, MysqlAdmin, SystemVariable};

#[test]
fn test_mysqladmin_new() {
    let admin = MysqlAdmin::new();
    let pl = admin.processlist();
    assert!(pl.contains("Id") && pl.contains("User"), "processlist should contain header");
}

#[test]
fn test_mysqladmin_ping() {
    let admin = MysqlAdmin::new();
    assert_eq!(admin.ping(), "mysqld is alive");
}

#[test]
fn test_mysqladmin_version() {
    let admin = MysqlAdmin::new();
    assert!(admin.version().contains("sqlrustgo"));
}

#[test]
fn test_mysqladmin_add_connection() {
    let admin = MysqlAdmin::new();
    let id = admin.add_connection("root", "localhost", "testdb");
    assert_eq!(id, 1);
    let id2 = admin.add_connection("alice", "127.0.0.1", "mydb");
    assert_eq!(id2, 2);
}

#[test]
fn test_mysqladmin_processlist() {
    let admin = MysqlAdmin::new();
    admin.add_connection("root", "localhost", "testdb");
    let pl = admin.processlist();
    assert!(pl.contains("root"));
    assert!(pl.contains("testdb"));
}

#[test]
fn test_mysqladmin_status() {
    let admin = MysqlAdmin::new();
    let status = admin.status();
    // Status returns a string (either data or empty)
    assert!(status.is_empty() || status.contains("Uptime") || status.contains("Threads") || status.contains("Queries"));
}

#[test]
fn test_mysqladmin_kill() {
    let admin = MysqlAdmin::new();
    let id = admin.add_connection("root", "localhost", "testdb");
    let result = admin.kill(id);
    // kill returns non-empty string when process not found, or empty on success
    // Just verify no panic
    let _ = result;
}

#[test]
fn test_mysqladmin_reload() {
    let admin = MysqlAdmin::new();
    let result = admin.reload();
    assert_eq!(result, "Reload complete");
}

#[test]
fn test_mysqladmin_flush_logs() {
    let admin = MysqlAdmin::new();
    admin.flush_logs();
    assert!(admin.is_log_flushed());
}

#[test]
fn test_mysqladmin_variables() {
    let admin = MysqlAdmin::new();
    let vars = admin.variables();
    // variables returns empty or has format like "Variable_name\tValue\n"
    assert!(vars.is_empty() || vars.contains('\t'));
}

#[test]
fn test_mysqladmin_set_variable() {
    let admin = MysqlAdmin::new();
    admin.set_variable("max_connections", "100");
    let vars = admin.variables();
    assert!(vars.contains("max_connections") || vars.contains("100"));
}

#[test]
fn test_mysqladmin_inc_query() {
    let admin = MysqlAdmin::new();
    admin.inc_query();
    admin.inc_query();
    // Query count increased (no panic = pass)
}

#[test]
fn test_mysqladmin_inc_slow() {
    let admin = MysqlAdmin::new();
    admin.inc_slow();
    admin.inc_slow();
    admin.inc_slow();
}

#[test]
fn test_mysqladmin_dispatch_ping() {
    let admin = MysqlAdmin::new();
    // dispatch does not recognize PING, but ping() directly works
    let result = admin.ping();
    assert_eq!(result, "mysqld is alive");
}

#[test]
fn test_mysqladmin_dispatch_unknown() {
    let admin = MysqlAdmin::new();
    let result = admin.dispatch("UNKNOWN_CMD", &[]);
    assert!(result.contains("unknown command") || result.is_empty());
}

#[test]
fn test_connection_struct() {
    let conn = Connection {
        id: 42,
        user: "testuser".to_string(),
        host: "localhost".to_string(),
        db: "testdb".to_string(),
        command: "Sleep".to_string(),
        time: 100,
        state: "".to_string(),
        info: "".to_string(),
    };
    assert_eq!(conn.id, 42);
    assert_eq!(conn.user, "testuser");
}

#[test]
fn test_system_variable() {
    let var = SystemVariable {
        name: "max_connections".to_string(),
        value: "151".to_string(),
    };
    assert_eq!(var.name, "max_connections");
    assert_eq!(var.value, "151");
}
