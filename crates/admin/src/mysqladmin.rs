use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

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
