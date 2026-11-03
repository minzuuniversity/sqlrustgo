//! #5514 — the idle-connection reaper must distinguish a **dead** peer
//! from a peer that is simply **thinking for a long time**.
//!
//! ## The defect
//!
//! `read_idle_timeout_secs()` parsed `SQLRUSTGO_IDLE_TIMEOUT_SECS` as
//! `u64` and then filtered it with `|n| *n >= 1`, so a configured `0`
//! fell through to the default `30`:
//!
//! ```text
//! SQLRUSTGO_IDLE_TIMEOUT_SECS=0  ⇒  30 s   (not "disabled")
//! ```
//!
//! There was therefore **no way to switch the reaper off**. A client
//! that computes a long task between two statements — a long
//! transaction, a call to an external system, a bulk computation — was
//! disconnected after 30 s of silence, and the symptom arrived at the
//! client as a transport error
//! (`read packet header: failed to fill whole buffer`), not as a
//! server-side rejection. That is how #5464 and #5415 were both
//! mis-attributed.
//!
//! ## The fix
//!
//! * `SQLRUSTGO_IDLE_TIMEOUT_SECS=0` now means **disabled** (MySQL's
//!   `wait_timeout=0` semantics): no idle read timeout is armed and the
//!   reaper reaps nothing.
//! * A client that sends a heartbeat (`COM_PING`) refreshes the idle
//!   timer, so a connection that is alive but busy is kept — the
//!   reaper only ever closes connections that stopped talking.
//!
//! ## The three cases this file pins
//!
//! | test | scenario | expected |
//! |---|---|---|
//! | `a_heartbeat_keeps_the_connection_alive` | client pings during a long idle | the connection survives |
//! | `a_silent_client_is_reaped` | client stops talking | the connection is closed (#4682 must not regress) |
//! | `zero_disables_the_idle_timeout` | `SQLRUSTGO_IDLE_TIMEOUT_SECS=0` | the connection survives **past the 30 s default** |
//!
//! ## Why the third test sleeps for 33 seconds
//!
//! The old code mapped `0` onto the **30 s** default, so the only way to
//! observe the difference between "disabled" and "the old 0 → 30
//! mapping" from the outside is to outlast 30 s. Anything shorter would
//! pass under the old code too, and would therefore not pin anything.
//! The unit test `read_idle_timeout_zero_disables_the_reaper` in
//! `crates/mysql-server/src/lib.rs` pins the mapping itself; this test
//! pins the observable consequence.

use sqlrustgo_mysql_client::MySqlConnection;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
use std::net::SocketAddr;
use std::time::Duration;

/// The env vars are **process-global**, and cargo runs the tests of one
/// binary in parallel threads. Without this lock two tests could read
/// each other's threshold — a flake that looks exactly like a broken
/// feature.
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Sets the two env vars and restores them when the guard is dropped.
struct IdleEnv {
    _guard: std::sync::MutexGuard<'static, ()>,
    saved: Vec<(&'static str, Option<String>)>,
}

impl IdleEnv {
    fn set(idle_timeout: &str, reaper_interval: &str) -> Self {
        let guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let vars = [
            ("SQLRUSTGO_IDLE_TIMEOUT_SECS", idle_timeout),
            ("SQLRUSTGO_REAPER_INTERVAL_SECS", reaper_interval),
        ];
        let saved = vars
            .iter()
            .map(|(k, _)| (*k, std::env::var(k).ok()))
            .collect();
        for (k, v) in vars {
            std::env::set_var(k, v);
        }
        Self {
            _guard: guard,
            saved,
        }
    }
}

impl Drop for IdleEnv {
    fn drop(&mut self) {
        for (k, v) in self.saved.drain(..) {
            match v {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
    }
}

/// Boot a fresh ephemeral server with an empty catalog.
fn start_server() -> sqlrustgo_mysql_server::testing::EphemeralHandle {
    let config = EphemeralConfig {
        data_dir: None,
        port: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        bulk_insert_rows_per_flush: 10_000,
        load_infile_dir: None,
        server_threads: 8,
        storage: None,
        slow_query_log: None,
        metrics_port: None,
        wal_sync_mode_override: None,
    };
    start_ephemeral(config).expect("ephemeral server starts")
}

/// Start the idle reaper **once per process**.
///
/// #5514: `run_server_v2` (the production path) spawns the reaper, but
/// `start_ephemeral` — the test path — does not. Without this, the
/// heartbeat test would pass even if the heartbeat stopped refreshing the
/// idle timer, because the socket read timeout is re-armed by *every*
/// successful read: a client that talks is never idle, whether or not
/// the reaper's tracker knows about it.
///
/// The reaper thread is process-wide and reads the threshold from the
/// environment on every cycle, so one instance is enough for all three
/// tests; the `IdleEnv` guard makes sure only one of them runs at a
/// time.
fn ensure_reaper_running() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        sqlrustgo_mysql_server::spawn_idle_connection_reaper();
    });
}

fn connect(port: u16) -> MySqlConnection {
    let addr: SocketAddr = format!("127.0.0.1:{}", port)
        .parse()
        .expect("invalid socket addr");
    MySqlConnection::connect(&addr, "tester", "tester", "").expect("connect")
}

/// A client that keeps sending `COM_PING` while it is "thinking" must not
/// be disconnected — even though its idle time is far past the threshold.
///
/// The threshold is 2 s and the heartbeat is every 300 ms, so the total
/// idle time (~4 s) is twice the threshold: without the heartbeat the
/// connection would be long gone.
#[test]
fn a_heartbeat_keeps_the_connection_alive() {
    let _env = IdleEnv::set("2", "1");
    ensure_reaper_running();
    let handle = start_server();
    let mut conn = connect(handle.port);

    for round in 1..=12 {
        std::thread::sleep(Duration::from_millis(300));
        conn.ping().unwrap_or_else(|e| {
            panic!("round {round}: the heartbeat must keep the connection alive: {e}")
        });
    }

    // And the connection is still usable afterwards.
    conn.execute("SELECT 1")
        .expect("the connection must still answer queries");
}

/// The other half of the contract: a client that **disappears** must
/// still be reaped, or #4682 comes back (CLOSE_WAIT zombies and a
/// server that waits 600 s on a dead socket).
#[test]
fn a_silent_client_is_reaped() {
    let _env = IdleEnv::set("2", "1");
    ensure_reaper_running();
    let handle = start_server();
    let mut conn = connect(handle.port);

    // Say nothing for well past the 2 s threshold.
    std::thread::sleep(Duration::from_secs(5));

    let err = conn
        .execute("SELECT 1")
        .expect_err("a client that stopped talking must have been reaped");
    assert!(
        !err.to_string().is_empty(),
        "the error must describe the closed connection"
    );
}

/// `SQLRUSTGO_IDLE_TIMEOUT_SECS=0` disables the idle timeout and the
/// reaper: the connection must survive **past the 30 s default** that
/// the old code fell back to.
///
/// See the file header for why the sleep is 33 s.
#[test]
fn zero_disables_the_idle_timeout() {
    let _env = IdleEnv::set("0", "1");
    ensure_reaper_running();
    let handle = start_server();
    let mut conn = connect(handle.port);

    std::thread::sleep(Duration::from_secs(33));

    conn.execute("SELECT 1")
        .expect("with SQLRUSTGO_IDLE_TIMEOUT_SECS=0 the connection must not be reaped");
}
