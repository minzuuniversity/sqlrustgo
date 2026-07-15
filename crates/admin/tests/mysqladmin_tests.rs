//! mysqladmin module tests
//!
//! Tests: MysqlAdmin, Connection, SystemVariable

use sqlrustgo_admin::mysqladmin::{Connection, MysqlAdmin, SystemVariable};

fn new_admin() -> MysqlAdmin {
    MysqlAdmin::new()
}

// ============ Connection tests ============

#[test]
fn test_connection_debug() {
    let conn = Connection {
        id: 1,
        user: "root".to_string(),
        host: "localhost".to_string(),
        db: "test".to_string(),
        command: "Query".to_string(),
        time: 5,
        state: "".to_string(),
        info: "SELECT 1".to_string(),
    };
    let debug = format!("{:?}", conn);
    assert!(debug.contains("Connection"));
    assert!(debug.contains("root"));
}

#[test]
fn test_connection_clone() {
    let conn = Connection {
        id: 42,
        user: "admin".to_string(),
        host: "127.0.0.1".to_string(),
        db: "prod".to_string(),
        command: "Sleep".to_string(),
        time: 100,
        state: "".to_string(),
        info: "".to_string(),
    };
    let c2 = conn.clone();
    assert_eq!(c2.id, 42);
    assert_eq!(c2.user, "admin");
}

// ============ SystemVariable tests ============

#[test]
fn test_system_variable_debug() {
    let var = SystemVariable {
        name: "max_connections".to_string(),
        value: "151".to_string(),
    };
    let debug = format!("{:?}", var);
    assert!(debug.contains("SystemVariable"));
    assert!(debug.contains("max_connections"));
}

#[test]
fn test_system_variable_clone() {
    let var = SystemVariable {
        name: "port".to_string(),
        value: "3306".to_string(),
    };
    let v2 = var.clone();
    assert_eq!(v2.name, "port");
    assert_eq!(v2.value, "3306");
}

// ============ MysqlAdmin::new tests ============

#[test]
fn test_mysql_admin_new() {
    let admin = MysqlAdmin::new();
    let status = admin.status();
    assert!(status.contains("Uptime"));
}

#[test]
fn test_mysql_admin_default() {
    let admin = MysqlAdmin::default();
    let status = admin.status();
    assert!(status.contains("Uptime"));
}

// ============ MysqlAdmin::ping tests ============

#[test]
fn test_mysql_admin_ping() {
    let admin = MysqlAdmin::new();
    assert_eq!(admin.ping(), "mysqld is alive");
}

// ============ MysqlAdmin::add_connection tests ============

#[test]
fn test_add_connection_single() {
    let admin = MysqlAdmin::new();
    let id = admin.add_connection("root", "localhost", "test");
    assert_eq!(id, 1);
}

#[test]
fn test_add_connection_multiple() {
    let admin = MysqlAdmin::new();
    let id1 = admin.add_connection("root", "localhost", "db1");
    let id2 = admin.add_connection("admin", "127.0.0.1", "db2");
    assert!(id1 != id2);
}

// ============ MysqlAdmin::dispatch tests ============

#[test]
fn test_dispatch_processlist() {
    let admin = MysqlAdmin::new();
    admin.add_connection("root", "localhost", "test");
    let out = admin.dispatch("processlist", &[]);
    assert!(out.contains("root") || out.contains("Id"));
}

#[test]
fn test_dispatch_status() {
    let admin = MysqlAdmin::new();
    let out = admin.dispatch("status", &[]);
    assert!(out.contains("Uptime"));
}

#[test]
fn test_dispatch_version() {
    let admin = MysqlAdmin::new();
    let out = admin.dispatch("version", &[]);
    assert!(out.contains("3.10.0") || out.contains("sqlrustgo"));
}

#[test]
fn test_dispatch_ping() {
    let admin = MysqlAdmin::new();
    assert_eq!(admin.dispatch("ping", &[]), "mysqld is alive");
}

#[test]
fn test_dispatch_unknown() {
    let admin = MysqlAdmin::new();
    let out = admin.dispatch("unknown_cmd", &[]);
    assert!(out.contains("ERROR") && out.contains("unknown command"));
}

#[test]
fn test_dispatch_variables() {
    let admin = MysqlAdmin::new();
    let out = admin.dispatch("variables", &[]);
    assert!(out.contains("max_connections") || out.contains("version"));
}

#[test]
fn test_dispatch_flush_logs() {
    let admin = MysqlAdmin::new();
    let out = admin.dispatch("flush-logs", &[]);
    assert!(out.contains("Log") || out.contains("flushed") || out.contains("OK"));
}

#[test]
fn test_dispatch_kill() {
    let admin = MysqlAdmin::new();
    let id = admin.add_connection("root", "localhost", "test");
    let out = admin.dispatch("kill", &[&id.to_string()]);
    assert!(out.contains("Killed") || out.contains("Connection"));
}

#[test]
fn test_dispatch_reload() {
    let admin = MysqlAdmin::new();
    let out = admin.dispatch("reload", &[]);
    assert!(out.contains("Reload") || out.contains("OK"));
}

// ============ MysqlAdmin::status tests ============

#[test]
fn test_status_contains_uptime() {
    let admin = MysqlAdmin::new();
    let status = admin.status();
    assert!(status.contains("Uptime"));
}

#[test]
fn test_status_contains_threads() {
    let admin = MysqlAdmin::new();
    let status = admin.status();
    assert!(status.contains("Threads"));
}

#[test]
fn test_status_format() {
    let admin = MysqlAdmin::new();
    admin.inc_query();
    admin.inc_query();
    let status = admin.status();
    assert!(status.contains("Questions"));
    assert!(status.contains("Slow queries"));
}

// ============ MysqlAdmin::version tests ============

#[test]
fn test_version_value() {
    let admin = MysqlAdmin::new();
    let v = admin.version();
    assert!(v.contains("3.10.0") || v.contains("sqlrustgo"));
}

// ============ MysqlAdmin::processlist tests ============

#[test]
fn test_processlist_empty() {
    let admin = MysqlAdmin::new();
    let pl = admin.processlist();
    // Empty list should show header or be empty
    assert!(pl.is_empty() || pl.contains("Id"));
}

#[test]
fn test_processlist_with_connections() {
    let admin = MysqlAdmin::new();
    admin.add_connection("user1", "host1", "db1");
    admin.add_connection("user2", "host2", "db2");
    let pl = admin.processlist();
    assert!(pl.contains("user1") || pl.contains("user2"));
}

// ============ MysqlAdmin::kill tests ============

#[test]
fn test_kill_connection() {
    let admin = MysqlAdmin::new();
    let id = admin.add_connection("root", "localhost", "test");
    let result = admin.kill(id);
    assert!(result.contains("Killed") || result.contains("Connection"));
}

#[test]
fn test_kill_nonexistent() {
    let admin = MysqlAdmin::new();
    let result = admin.kill(99999);
    assert!(result.contains("ERROR") || result.contains("not found"));
}

// ============ MysqlAdmin::reload tests ============

#[test]
fn test_reload() {
    let admin = MysqlAdmin::new();
    let result = admin.reload();
    assert!(result.contains("Reload") || result.contains("OK"));
    assert!(admin.is_grants_reloaded());
}

// ============ MysqlAdmin::flush_logs tests ============

#[test]
fn test_flush_logs() {
    let admin = MysqlAdmin::new();
    let result = admin.flush_logs();
    assert!(result.contains("Log") || result.contains("flushed") || result.contains("OK"));
    assert!(admin.is_log_flushed());
}

// ============ MysqlAdmin::variables tests ============

#[test]
fn test_variables_contains_port() {
    let admin = MysqlAdmin::new();
    let vars = admin.variables();
    assert!(vars.contains("port"));
    assert!(vars.contains("3306"));
}

#[test]
fn test_variables_contains_max_connections() {
    let admin = MysqlAdmin::new();
    let vars = admin.variables();
    assert!(vars.contains("max_connections"));
    assert!(vars.contains("151"));
}

#[test]
fn test_variables_contains_version() {
    let admin = MysqlAdmin::new();
    let vars = admin.variables();
    assert!(vars.contains("version"));
}

// ============ MysqlAdmin::set_variable tests ============

#[test]
fn test_set_variable() {
    let admin = MysqlAdmin::new();
    admin.set_variable("wait_timeout", "28800");
    let vars = admin.variables();
    assert!(vars.contains("wait_timeout"));
}

// ============ MysqlAdmin::is_log_flushed tests ============

#[test]
fn test_is_log_flushed_initially_false() {
    let admin = MysqlAdmin::new();
    assert!(!admin.is_log_flushed());
}

#[test]
fn test_is_log_flushed_after_flush() {
    let admin = MysqlAdmin::new();
    admin.flush_logs();
    assert!(admin.is_log_flushed());
}

// ============ MysqlAdmin::is_grants_reloaded tests ============

#[test]
fn test_is_grants_reloaded_initially_false() {
    let admin = MysqlAdmin::new();
    assert!(!admin.is_grants_reloaded());
}

#[test]
fn test_is_grants_reloaded_after_reload() {
    let admin = MysqlAdmin::new();
    admin.reload();
    assert!(admin.is_grants_reloaded());
}

// ============ MysqlAdmin::inc_query tests ============

#[test]
fn test_inc_query() {
    let admin = MysqlAdmin::new();
    admin.inc_query();
    admin.inc_query();
    admin.inc_query();
}

// ============ MysqlAdmin::inc_slow tests ============

#[test]
fn test_inc_slow() {
    let admin = MysqlAdmin::new();
    admin.inc_slow();
    admin.inc_slow();
}
