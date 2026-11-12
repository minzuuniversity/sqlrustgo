//! #5099 — `TlsStream::read` must not busy-spin on EOF.
//!
//! Trigger (both conditions are required, and both are easy to get wrong):
//!
//! 1. **The connection must be TLS.** `TlsStream` is only built on the
//!    SSL-upgrade path (`lib.rs:6375`). A plain socket takes the non-TLS
//!    branch and never reaches the loop — a raw-socket reproducer does not
//!    reproduce this bug at all (measured: 0% CPU on the unfixed binary).
//! 2. **The peer must send a clean FIN.** On EOF with no pending writes,
//!    rustls's `complete_io` returns `Ok((0, 0))` from its final match arm
//!    while `wants_read()` still holds. The old `Ok(_) => {}` arm read that
//!    as "progress" and looped again — forever, at 100% CPU per worker.
//!    An RST instead produces an error the loop propagates and unwinds.
//!
//! Measured on the unfixed binary with this exact reproducer: **842% CPU**
//! across 16 workers. With the fix: under 1%.
//!
//! The client is pymysql rather than a hand-rolled rustls client: getting a
//! client into the TLS command loop takes an exactly-shaped SSLRequest and
//! handshake, and hand-rolling it would be a second untested implementation
//! of the very thing under test. The server is this crate's own binary, so
//! the assertion is still end-to-end.

use std::net::TcpStream;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Open N authenticated TLS connections, then clean-FIN each one.
const REPRO_SCRIPT: &str = r#"
import os, socket, ssl, sys, time, glob

# #5712: find pymysql without hardcoding one interpreter's site-packages.
# PYTHONPATH (inherited from the environment) still wins, so a venv works.
for pattern in (
    os.path.expanduser("~/.local/lib/python3.*/site-packages"),
    os.path.expanduser("~/Library/Python/3.*/lib/python/site-packages"),
):
    for path in sorted(glob.glob(pattern)):
        if path not in sys.path:
            sys.path.insert(0, path)

try:
    import pymysql
except ImportError:
    print("FATAL: pymysql not installed — pip install pymysql (or point "
          "PYTHONPATH at a site-packages that has it)", file=sys.stderr)
    sys.exit(2)

PORT = int(sys.argv[1])
N = int(sys.argv[2])
HOLD = int(sys.argv[3])

ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
ctx.check_hostname = False
ctx.verify_mode = ssl.CERT_NONE

held = []
for _ in range(N):
    held.append(pymysql.connect(host="127.0.0.1", port=PORT, user="root",
                                autocommit=True, ssl=ctx, connect_timeout=15))

sys.stdout.write("%d\n" % len(held))
sys.stdout.flush()

# Clean FIN: the server's read_tls now sees Ok(0) = EOF on a connection that
# is otherwise intact. This is the state that spun.
for c in held:
    try:
        c._sock.shutdown(socket.SHUT_WR)
    except Exception:
        pass

time.sleep(HOLD)
"#;

/// Accumulated CPU seconds for a process.
///
/// Spinning threads burn CPU while doing nothing, so asserting *accumulated
/// CPU* is the honest check. Sampling `ps` wall-clock would be flaky on a
/// loaded CI box; accumulated CPU time is not.
///
/// #5712: this used to read `/proc/<pid>/stat`, which only exists on Linux
/// — on macOS the test died in the measurement helper and reported a
/// platform red that had nothing to do with the behaviour under test.
/// Linux reads `/proc`; macOS asks `ps -o time=` (which reports *used* CPU,
/// not wall clock).
fn cpu_seconds(pid: u32) -> f64 {
    if cfg!(target_os = "linux") {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat"))
            .unwrap_or_else(|e| panic!("read /proc/{pid}/stat: {e}"));
        // utime/stime are fields 14/15, after the possibly space-containing comm.
        let after_comm = stat.rsplit_once(')').expect("comm field").1;
        let f: Vec<&str> = after_comm.split_whitespace().collect();
        let ticks: f64 = f[11].parse().unwrap_or(0.0) + f[12].parse().unwrap_or(0.0);
        ticks / 100.0 // _SC_CLK_TCK is 100 on Linux
    } else if cfg!(target_os = "macos") {
        let out = std::process::Command::new("ps")
            .args(["-o", "time=", "-p", &pid.to_string()])
            .output()
            .unwrap_or_else(|e| panic!("run ps for pid {pid}: {e}"));
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        parse_ps_time(&text).unwrap_or_else(|| {
            panic!("cannot read the accumulated CPU time of pid {pid} from `ps`: {text:?}")
        })
    } else {
        panic!(
            "cpu_seconds is implemented for Linux (/proc/<pid>/stat) and macOS \
             (`ps -o time=`); this platform needs its own implementation"
        )
    }
}

/// Parse the `[[HH:]MM:]SS.ss` shape `ps -o time=` prints.
fn parse_ps_time(text: &str) -> Option<f64> {
    if text.is_empty() {
        return None;
    }
    let mut total = 0.0;
    for part in text.split(':') {
        total = total * 60.0 + part.parse::<f64>().ok()?;
    }
    Some(total)
}

fn wait_ready(port: u16, timeout: Duration) {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("server on port {port} never became ready");
}

#[test]
fn tls_eof_does_not_spin_cpu() {
    let port: u16 = 39_087;
    let dir = std::env::temp_dir().join(format!("tls_eof_{port}"));
    let _ = std::fs::remove_dir_all(&dir);
    let bin = env!("CARGO_BIN_EXE_sqlrustgo-mysql-server");

    let mut child = std::process::Command::new(bin)
        .args([
            "serve",
            "--host",
            "127.0.0.1",
            "--port",
            &port.to_string(),
            "--data-dir",
            dir.to_str().unwrap(),
            "--server-threads",
            "16",
            "--max-connections",
            "128",
            "--log-level",
            "error",
            "--storage",
            "file",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn server");
    let pid = child.id();
    wait_ready(port, Duration::from_secs(30));
    std::thread::sleep(Duration::from_millis(500));

    let script: PathBuf = std::env::temp_dir().join("tls_eof_repro_5099.py");
    std::fs::write(&script, REPRO_SCRIPT).expect("write repro script");

    let baseline = cpu_seconds(pid);
    let hold_secs = 8;
    let out = std::process::Command::new("python3")
        .arg(&script)
        .arg(port.to_string())
        .arg("16") // connections == worker count, so every worker spins
        .arg(hold_secs.to_string())
        .output()
        .expect("run reproducer");

    let held = String::from_utf8_lossy(&out.stdout);
    let burned = cpu_seconds(pid) - baseline;

    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_file(&script);

    assert!(
        held.trim() == "16",
        "reproducer failed to open 16 TLS connections (stdout {:?}, stderr {:?})",
        held.trim(),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        burned < 20.0,
        "server burned {burned:.1}s of CPU while 16 authenticated TLS \
         connections sat at a clean FIN. The unfixed binary burns >400s \
         here: the read loop spins instead of reporting EOF."
    );
}
