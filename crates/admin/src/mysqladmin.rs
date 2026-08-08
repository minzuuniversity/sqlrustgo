use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

#[allow(dead_code)]
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

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SystemVariable {
    pub name: String,
    pub value: String,
}

#[allow(dead_code)]
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

#[allow(dead_code)]
impl MysqlAdmin {
    pub fn new() -> Self {
        let mut vars = HashMap::new();
        vars.insert("version".to_string(), "3.10.0".to_string());
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
        "sqlrustgo  v3.10.0".to_string()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn admin() -> MysqlAdmin {
        MysqlAdmin::new()
    }

    #[test]
    fn test_new_has_default_variables() {
        let admin = admin();
        let vars = admin.variables();
        assert!(vars.contains("version\t3.10.0"));
        assert!(vars.contains("max_connections\t151"));
        assert!(vars.contains("port\t3306"));
    }

    #[test]
    fn test_ping() {
        assert_eq!(admin().ping(), "mysqld is alive");
    }

    #[test]
    fn test_version() {
        assert_eq!(admin().version(), "sqlrustgo  v3.10.0");
    }

    #[test]
    fn test_add_connection_returns_incremental_id() {
        let a = admin();
        let id1 = a.add_connection("root", "localhost", "testdb");
        let id2 = a.add_connection("alice", "127.0.0.1", "db2");
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    #[test]
    fn test_processlist_empty() {
        let out = admin().processlist();
        assert!(out.starts_with("Id\tUser\tHost\tdb\tCommand\tTime\tState\tInfo\n"));
    }

    #[test]
    fn test_processlist_with_connections() {
        let a = admin();
        a.add_connection("root", "localhost", "testdb");
        a.add_connection("alice", "127.0.0.1", "db2");
        let out = a.processlist();
        assert!(out.contains("root\tlocalhost\ttestdb"));
        assert!(out.contains("alice\t127.0.0.1\tdb2"));
    }

    #[test]
    fn test_kill_existing_connection() {
        let a = admin();
        let id = a.add_connection("root", "localhost", "testdb");
        assert_eq!(a.kill(id), format!("Killed connection {}", id));
        assert!(!a.processlist().contains("root"));
    }

    #[test]
    fn test_kill_nonexistent_connection() {
        let a = admin();
        assert_eq!(a.kill(999), "ERROR: connection 999 not found");
    }

    #[test]
    fn test_reload() {
        let a = admin();
        assert!(!a.is_grants_reloaded());
        assert_eq!(a.reload(), "Reload complete");
        assert!(a.is_grants_reloaded());
    }

    #[test]
    fn test_flush_logs() {
        let a = admin();
        assert!(!a.is_log_flushed());
        assert_eq!(a.flush_logs(), "Log files flushed");
        assert!(a.is_log_flushed());
    }

    #[test]
    fn test_set_and_get_variable() {
        let a = admin();
        a.set_variable("max_connections", "200");
        let vars = a.variables();
        assert!(vars.contains("max_connections\t200"));
    }

    #[test]
    fn test_inc_query() {
        let a = admin();
        a.inc_query();
        a.inc_query();
        a.inc_query();
        let out = a.status();
        assert!(out.contains("Questions: 3"));
    }

    #[test]
    fn test_inc_slow() {
        let a = admin();
        a.inc_slow();
        a.inc_slow();
        let out = a.status();
        assert!(out.contains("Slow queries: 2"));
    }

    #[test]
    fn test_status_contains_uptime_threads() {
        let out = admin().status();
        assert!(out.starts_with("Uptime:"));
        assert!(out.contains("Threads: 0"));
        assert!(out.contains("Questions: 0"));
    }

    #[test]
    fn test_dispatch_ping() {
        assert_eq!(admin().dispatch("ping", &[]), "mysqld is alive");
    }

    #[test]
    fn test_dispatch_version() {
        assert_eq!(admin().dispatch("version", &[]), "sqlrustgo  v3.10.0");
    }

    #[test]
    fn test_dispatch_unknown_command() {
        let out = admin().dispatch("foobar", &[]);
        assert!(out.contains("ERROR: unknown command 'foobar'"));
    }

    #[test]
    fn test_dispatch_kill_missing_arg() {
        let out = admin().dispatch("kill", &[]);
        assert_eq!(out, "ERROR: kill requires a connection id");
    }

    #[test]
    fn test_dispatch_kill_invalid_arg() {
        let out = admin().dispatch("kill", &["not-a-number"]);
        assert!(out.contains("ERROR: invalid id 'not-a-number'"));
    }

    #[test]
    fn test_dispatch_kill_valid() {
        let a = admin();
        let id = a.add_connection("root", "localhost", "testdb");
        assert_eq!(
            a.dispatch("kill", &[&id.to_string()]),
            format!("Killed connection {}", id)
        );
    }

    #[test]
    fn test_dispatch_reload() {
        let a = admin();
        assert_eq!(a.dispatch("reload", &[]), "Reload complete");
        assert!(a.is_grants_reloaded());
    }

    #[test]
    fn test_dispatch_flush_logs() {
        let a = admin();
        assert_eq!(a.dispatch("flush-logs", &[]), "Log files flushed");
        assert!(a.is_log_flushed());
    }

    #[test]
    fn test_dispatch_variables() {
        let out = admin().dispatch("variables", &[]);
        assert!(out.contains("version\t3.10.0"));
    }
}
