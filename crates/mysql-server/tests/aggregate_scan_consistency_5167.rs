//! #5167 acceptance: `COUNT(*)` / range scans / aggregates must agree
//! with point lookups.
//!
//! # The reported symptom
//!
//! ```text
//! SELECT COUNT(*) FROM t;                        -- 0   ✗
//! SELECT SUM(k) FROM t;                          -- NULL ✗
//! SELECT id FROM t LIMIT 3;                      -- empty ✗
//! SELECT COUNT(*) FROM t WHERE id=2;             -- 1   ✓ point lookup normal
//! ```
//!
//! Reads were reported as returning an empty set while point lookups on the
//! same table hit every sampled row — a scan/point disagreement, which
//! would silently corrupt any downstream count, pagination, or report.
//!
//! # What this file establishes
//!
//! The symptom does not reproduce on `develop/v4.1.0`, in-process or over
//! the wire, at 3 rows or at 10000. These tests are kept as the standing
//! acceptance criterion from the issue: they run the full matrix (aggregate
//! vs. range vs. point, with and without concurrent transactions, and with
//! rows split across the insert buffer) so that if the scan path ever
//! regresses to "sees only one of the two row stores", one of them turns
//! red with the actual numbers rather than a downstream count quietly
//! reporting 0.
//!
//! The engine was never at fault — `ExecutionEngine` over `MemoryStorage`
//! computes all of these correctly, which is why the first bisect step
//! was in-process rather than over the wire.

use sqlrustgo_mysql_client::{MySqlConnection, ResultSet};
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};

fn start() -> sqlrustgo_mysql_server::testing::EphemeralHandle {
    start_ephemeral(EphemeralConfig {
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
    })
    .expect("ephemeral server starts")
}

fn connect(h: &sqlrustgo_mysql_server::testing::EphemeralHandle) -> MySqlConnection {
    let addr = format!("127.0.0.1:{}", h.port).parse().expect("addr");
    MySqlConnection::connect(&addr, "root", "", "test").expect("connect")
}

/// Single-column scalar as a string, or a panic naming the query.
fn scalar(c: &mut MySqlConnection, sql: &str) -> String {
    match c.execute(sql) {
        Ok(ResultSet::Select { rows, .. }) => rows
            .first()
            .and_then(|r| r.first())
            .cloned()
            .unwrap_or_else(|| panic!("{sql}: no rows")),
        Ok(other) => panic!("{sql}: expected a result set, got {other:?}"),
        Err(e) => panic!("{sql}: query failed: {e}"),
    }
}

fn row_count(c: &mut MySqlConnection, sql: &str) -> usize {
    match c.execute(sql) {
        Ok(ResultSet::Select { rows, .. }) => rows.len(),
        Ok(other) => panic!("{sql}: expected a result set, got {other:?}"),
        Err(e) => panic!("{sql}: query failed: {e}"),
    }
}

/// Seed the 3-row table from the issue report.
fn seed_small(c: &mut MySqlConnection) {
    c.execute("CREATE DATABASE d1").expect("create db");
    c.execute("USE d1").expect("use");
    c.execute("CREATE TABLE t(id INT PRIMARY KEY, k INT)")
        .expect("create");
    c.execute("INSERT INTO t VALUES (1,10),(2,20),(3,30)")
        .expect("insert");
}

/// The issue's exact reproduction block.
#[test]
fn aggregates_match_point_lookups_on_a_fresh_server() {
    let h = start();
    let mut c = connect(&h);
    seed_small(&mut c);

    assert_eq!(
        scalar(&mut c, "SELECT COUNT(*) FROM t"),
        "3",
        "#5167: COUNT(*) must see every row"
    );
    assert_eq!(
        scalar(&mut c, "SELECT SUM(k) FROM t"),
        "60",
        "#5167: SUM must aggregate every row"
    );
    assert_eq!(
        row_count(&mut c, "SELECT id FROM t LIMIT 3"),
        3,
        "#5167: scan must return rows"
    );
    assert_eq!(
        scalar(&mut c, "SELECT COUNT(*) FROM t WHERE id=2"),
        "1",
        "#5167: the point lookup that already worked must keep working"
    );
}

/// Range and OR predicates, which the issue reported as 0.
#[test]
fn range_and_or_predicates_count_every_matching_row() {
    let h = start();
    let mut c = connect(&h);
    seed_small(&mut c);

    assert_eq!(
        scalar(&mut c, "SELECT COUNT(*) FROM t WHERE id>=1 AND id<=3"),
        "3"
    );
    assert_eq!(
        scalar(&mut c, "SELECT COUNT(*) FROM t WHERE id=1 OR id=2"),
        "2"
    );
    assert_eq!(scalar(&mut c, "SELECT COUNT(*) FROM t WHERE k>0"), "3");
    assert_eq!(scalar(&mut c, "SELECT COUNT(*) FROM t WHERE id>3"), "0");
}

/// The scale at which the issue was observed. Batched inserts cross the
/// insert-buffer threshold repeatedly, so rows end up split between
/// `tables.rows` and `insert_buffer` — the state in which a scan that
/// reads only one of the two would report a short count.
#[test]
fn counts_are_correct_when_rows_span_the_insert_buffer() {
    let h = start();
    let mut c = connect(&h);

    c.execute("CREATE DATABASE d1").expect("create db");
    c.execute("USE d1").expect("use");
    c.execute("CREATE TABLE t(id INT PRIMARY KEY, k INT)")
        .expect("create");
    for start_id in (1..=10_000).step_by(500) {
        let vals: Vec<String> = (start_id..(start_id + 500).min(10_001))
            .map(|i| format!("({i},{i})"))
            .collect();
        c.execute(&format!("INSERT INTO t VALUES {}", vals.join(",")))
            .expect("insert batch");
    }

    // The issue sampled these ids and reported every point lookup hitting
    // while scans saw nothing.
    let mut point_hits = 0;
    for id in [
        1, 50, 100, 500, 1000, 2000, 2716, 2717, 3000, 5000, 9000, 9999, 10_000,
    ] {
        if row_count(&mut c, &format!("SELECT k FROM t WHERE id={id}")) == 1 {
            point_hits += 1;
        }
    }
    assert_eq!(
        point_hits, 13,
        "#5167: every sampled id must be reachable by point lookup"
    );

    assert_eq!(
        scalar(&mut c, "SELECT COUNT(*) FROM t"),
        "10000",
        "#5167: a scan must see rows in `tables.rows` AND `insert_buffer`"
    );
    assert_eq!(scalar(&mut c, "SELECT SUM(k) FROM t"), "50005000");
    assert_eq!(row_count(&mut c, "SELECT id FROM t LIMIT 3"), 3);
}

/// The issue's acceptance criterion names concurrency explicitly. A second
/// connection running an overlapping range read must not change what the
/// first one sees.
#[test]
fn counts_are_stable_while_another_connection_reads() {
    let h = start();
    let mut writer = connect(&h);
    seed_small(&mut writer);

    // Each connection has its own current database (#5025), so a fresh
    // connection starts in `default` and cannot see `d1.t` until it
    // selects the database itself. That scoping is correct behaviour, not
    // the #5167 defect — an unqualified `t` here answers "Table not
    // found", which is why this reader has to `USE d1` first.
    let mut reader = connect(&h);
    reader.execute("USE d1").expect("reader USE d1");

    for _ in 0..20 {
        assert_eq!(
            scalar(&mut reader, "SELECT COUNT(*) FROM t"),
            "3",
            "#5167: a concurrent reader must not perturb the aggregate"
        );
        assert_eq!(scalar(&mut reader, "SELECT SUM(k) FROM t"), "60");
        assert_eq!(row_count(&mut reader, "SELECT id FROM t LIMIT 3"), 3);
    }
}
