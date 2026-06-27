//! F-32: mysqladmin Equivalent
//!
//! **Issue**: #2831 (F-32 mysqladmin equivalent)
//! **Source**: v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (F-32)
//! **Change**: openspec/changes/f-32-mysqladmin
//!
//! Implements 8 mysqladmin subcommands as an in-test command dispatcher:
//! ping, status, version, processlist, kill, reload, flush-logs, variables.
//!
//! Note: Real CLI binary is deferred to v3.9.0 (separate crate).

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct Connection {
    pub id: u64,
    pub user: String,
    pub host: String,
    pub db: String,
    pub command: String,
    pub time: u64,
    pub state: String,
    pub info: String,
}

#[derive(Debug, Clone)]
pub struct SystemVariable {
    pub name: String,
    pub value: String,
}

pub struct MysqlAdmin {
    start_time: SystemTime,
    connections: Arc<Mutex<HashMap<u64, Connection>>>,
    next_id: Arc<AtomicU64>,
    total_queries: Arc<AtomicU64>,
    slow_queries: Arc<AtomicU64>,
    log_flushed: Arc<AtomicBool>,
    grants_reloaded: Arc<AtomicBool>,
    variables: Arc<Mutex<HashMap<String, String>>>,
}

impl MysqlAdmin {
    pub fn new() -> Self {
        let mut vars = HashMap::new();
        vars.insert("version".to_string(), "3.8.0".to_string());
        vars.insert("max_connections".to_string(), "151".to_string());
        vars.insert("datadir".to_string(), "./data".to_string());
        vars.insert("port".to_string(), "3306".to_string());

        Self {
            start_time: SystemTime::now(),
            connections: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(AtomicU64::new(1)),
            total_queries: Arc::new(AtomicU64::new(0)),
            slow_queries: Arc::new(AtomicU64::new(0)),
            log_flushed: Arc::new(AtomicBool::new(false)),
            grants_reloaded: Arc::new(AtomicBool::new(false)),
            variables: Arc::new(Mutex::new(vars)),
        }
    }

    /// Register a new connection (for processlist testing).
    pub fn add_connection(&self, user: &str, host: &str, db: &str) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let conn = Connection {
            id,
            user: user.to_string(),
            host: host.to_string(),
            db: db.to_string(),
            command: "Sleep".to_string(),
            time: 0,
            state: "".to_string(),
            info: "".to_string(),
        };
        self.connections.lock().unwrap().insert(id, conn);
        id
    }

    /// Dispatch a mysqladmin command.
    pub fn dispatch(&self, cmd: &str, args: &[&str]) -> String {
        match cmd {
            "ping" => self.ping(),
            "status" => self.status(),
            "version" => self.version(),
            "processlist" => self.processlist(),
            "kill" => {
                if args.is_empty() {
                    return "ERROR: kill requires a connection id".to_string();
                }
                match args[0].parse::<u64>() {
                    Ok(id) => self.kill(id),
                    Err(_) => format!("ERROR: invalid id '{}'", args[0]),
                }
            }
            "reload" => self.reload(),
            "flush-logs" => self.flush_logs(),
            "variables" => self.variables(),
            other => format!("ERROR: unknown command '{}'", other),
        }
    }

    pub fn ping(&self) -> String {
        "mysqld is alive".to_string()
    }

    pub fn status(&self) -> String {
        let uptime = SystemTime::now()
            .duration_since(self.start_time)
            .unwrap_or_default()
            .as_secs();
        let threads = self.connections.lock().unwrap().len();
        let questions = self.total_queries.load(Ordering::SeqCst);
        let slow = self.slow_queries.load(Ordering::SeqCst);
        format!(
            "Uptime: {}  Threads: {}  Questions: {}  Slow queries: {}",
            uptime, threads, questions, slow
        )
    }

    pub fn version(&self) -> String {
        "sqlrustgo  v3.8.0".to_string()
    }

    pub fn processlist(&self) -> String {
        let conns = self.connections.lock().unwrap();
        let mut out = String::from("Id\tUser\tHost\tdb\tCommand\tTime\tState\tInfo\n");
        for c in conns.values() {
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                c.id, c.user, c.host, c.db, c.command, c.time, c.state, c.info
            ));
        }
        out
    }

    pub fn kill(&self, id: u64) -> String {
        let mut conns = self.connections.lock().unwrap();
        match conns.remove(&id) {
            Some(_) => format!("Killed connection {}", id),
            None => format!("ERROR: connection {} not found", id),
        }
    }

    pub fn reload(&self) -> String {
        self.grants_reloaded.store(true, Ordering::SeqCst);
        "Reload complete".to_string()
    }

    pub fn flush_logs(&self) -> String {
        self.log_flushed.store(true, Ordering::SeqCst);
        "Log files flushed".to_string()
    }

    pub fn variables(&self) -> String {
        let vars = self.variables.lock().unwrap();
        let mut out = String::new();
        for (k, v) in vars.iter() {
            out.push_str(&format!("{}\t{}\n", k, v));
        }
        out
    }

    pub fn set_variable(&self, name: &str, value: &str) {
        self.variables
            .lock()
            .unwrap()
            .insert(name.to_string(), value.to_string());
    }

    pub fn is_log_flushed(&self) -> bool {
        self.log_flushed.load(Ordering::SeqCst)
    }

    pub fn is_grants_reloaded(&self) -> bool {
        self.grants_reloaded.load(Ordering::SeqCst)
    }

    pub fn inc_query(&self) {
        self.total_queries.fetch_add(1, Ordering::SeqCst);
    }

    pub fn inc_slow(&self) {
        self.slow_queries.fetch_add(1, Ordering::SeqCst);
    }
}

impl Default for MysqlAdmin {
    fn default() -> Self {
        Self::new()
    }
}

#[test]
fn test_ping() {
    let admin = MysqlAdmin::new();
    assert_eq!(admin.dispatch("ping", &[]), "mysqld is alive");
}

#[test]
fn test_status() {
    let admin = MysqlAdmin::new();
    admin.inc_query();
    admin.inc_query();
    admin.inc_slow();
    let s = admin.dispatch("status", &[]);
    assert!(s.contains("Uptime:"));
    assert!(s.contains("Threads:"));
    assert!(s.contains("Questions: 2"));
    assert!(s.contains("Slow queries: 1"));
}

#[test]
fn test_version() {
    let admin = MysqlAdmin::new();
    let v = admin.dispatch("version", &[]);
    assert!(v.contains("sqlrustgo"));
    assert!(v.contains("3.8.0"));
}

#[test]
fn test_processlist() {
    let admin = MysqlAdmin::new();
    let id1 = admin.add_connection("alice", "192.168.1.1", "test_db");
    let id2 = admin.add_connection("bob", "192.168.1.2", "test_db");
    let pl = admin.dispatch("processlist", &[]);
    assert!(pl.contains("alice"));
    assert!(pl.contains("bob"));
    assert!(pl.contains(&format!("{}", id1)));
    assert!(pl.contains(&format!("{}", id2)));
}

#[test]
fn test_kill() {
    let admin = MysqlAdmin::new();
    let id = admin.add_connection("alice", "host", "db");
    let result = admin.dispatch("kill", &[&format!("{}", id)]);
    assert!(result.contains("Killed connection"));
    // Verify connection is gone
    let pl = admin.dispatch("processlist", &[]);
    assert!(!pl.contains(&format!("{}", id)));
}

#[test]
fn test_kill_invalid_id() {
    let admin = MysqlAdmin::new();
    let result = admin.dispatch("kill", &["99999"]);
    assert!(result.contains("not found"));
}

#[test]
fn test_reload() {
    let admin = MysqlAdmin::new();
    assert!(!admin.is_grants_reloaded());
    let result = admin.dispatch("reload", &[]);
    assert!(result.contains("Reload complete"));
    assert!(admin.is_grants_reloaded());
}

#[test]
fn test_flush_logs() {
    let admin = MysqlAdmin::new();
    assert!(!admin.is_log_flushed());
    let result = admin.dispatch("flush-logs", &[]);
    assert!(result.contains("flushed"));
    assert!(admin.is_log_flushed());
}

#[test]
fn test_variables() {
    let admin = MysqlAdmin::new();
    let v = admin.dispatch("variables", &[]);
    assert!(v.contains("version"));
    assert!(v.contains("3.8.0"));
    assert!(v.contains("max_connections"));
    assert!(v.contains("port"));
    // Test setting a variable
    admin.set_variable("new_var", "test_value");
    let v2 = admin.dispatch("variables", &[]);
    assert!(v2.contains("new_var"));
    assert!(v2.contains("test_value"));
}

#[test]
fn test_dispatch_unknown_command() {
    let admin = MysqlAdmin::new();
    let result = admin.dispatch("nonexistent", &[]);
    assert!(result.contains("unknown command"));
}

#[test]
fn test_dispatch_kill_missing_arg() {
    let admin = MysqlAdmin::new();
    let result = admin.dispatch("kill", &[]);
    assert!(result.contains("requires"));
}
