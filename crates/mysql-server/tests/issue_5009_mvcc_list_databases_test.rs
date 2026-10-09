//! #5009 / #5015 regression: `MvccStorage::list_databases` must forward
//! to the inner engine, and the wire path must be able to see it.
//!
//! # The defect this pins
//!
//! `StorageEngine::list_databases` has a trait default of `Ok(vec![])`.
//! A wrapper that forgets the forward silently answers with that default
//! — no error, no warning — and `SHOW DATABASES` reports nothing. This
//! is the #4974 "wrapper drops the method" shape, one layer up:
//! `WalStorage` forwards it, `MvccStorage` did not.
//!
//! The shipped server wraps `FileStorage` in `MvccStorage` before it ever
//! becomes a `BoxStorageEngine` (`crates/mysql-server/src/lib.rs`,
//! V400-MVCC-ENABLE), so the dropped forward sat on the production read
//! path of `SHOW DATABASES` for every database the user created.
//!
//! # Why the REPL test cannot cover this
//!
//! `issue_5009_show_databases_server_test.rs` spawns `repl`, which builds
//! a plain `MemoryStorage` — it never constructs an `MvccStorage`. A
//! mutation to `MvccStorage::list_databases` survives that test by
//! construction. It is still worth keeping: it is the only test that
//! covers the *binary's* argument parsing and REPL dispatch. But it is
//! structurally blind to the MVCC layer, and claiming otherwise would be
//! the same overstatement the mutation audit exists to remove.
//!
//! This file uses `start_ephemeral`, which does build the real
//! FileStorage → MvccStorage → WalStorage → `Box<dyn StorageEngine>`
//! chain and serves it over the wire, so a mutation at any layer of that
//! chain turns these tests red.

use sqlrustgo_mysql_client::{MySqlConnection, ResultSet};
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
use std::net::SocketAddr;

fn start_server() -> sqlrustgo_mysql_server::testing::EphemeralHandle {
    // `storage: None` is the shipped default: FileStorage + WAL, wrapped
    // in MvccStorage. Do not set it to Some("binary") — that selects
    // BinaryTableStorage, which has its own forward and would make this
    // test blind to the MVCC layer in the same way.
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

fn connect(port: u16) -> MySqlConnection {
    let addr: SocketAddr = format!("127.0.0.1:{}", port)
        .parse()
        .expect("invalid socket addr");
    MySqlConnection::connect(&addr, "tester", "tester", "").expect("connect")
}

/// `SHOW DATABASES` as a list of names, in the order the server returned.
fn show_databases(conn: &mut MySqlConnection) -> Vec<String> {
    match conn.execute("SHOW DATABASES").expect("SHOW DATABASES") {
        ResultSet::Select { rows, .. } => rows.iter().filter_map(|r| r.first().cloned()).collect(),
        other => panic!("SHOW DATABASES must return a result set, got {other:?}"),
    }
}

/// The forward itself: a database created over the MVCC-backed wire path
/// must be enumerable through that same path.
#[test]
fn mvcc_wrapped_storage_enumerates_a_created_database() {
    let handle = start_server();
    let mut conn = connect(handle.port);

    conn.execute("CREATE DATABASE shop")
        .expect("CREATE DATABASE shop");
    conn.execute("CREATE DATABASE blog")
        .expect("CREATE DATABASE blog");

    let listed = show_databases(&mut conn);
    for want in ["shop", "blog"] {
        assert!(
            listed.iter().any(|d| d == want),
            "#5009: `{want}` must be enumerated through the MVCC-backed \
             storage; SHOW DATABASES returned {listed:?}. With the \
             MvccStorage::list_databases forward removed, the trait \
             default answers with an empty list and this is [] plus \
             whatever the engine synthesizes."
        );
    }
}

/// A dropped forward must also be caught on the negative side: a
/// database that was dropped must not linger in the enumeration.
#[test]
fn dropped_database_disappears_from_the_mvcc_enumeration() {
    let handle = start_server();
    let mut conn = connect(handle.port);

    conn.execute("CREATE DATABASE shop")
        .expect("CREATE DATABASE shop");
    conn.execute("CREATE DATABASE blog")
        .expect("CREATE DATABASE blog");
    conn.execute("DROP DATABASE blog")
        .expect("DROP DATABASE blog");

    let listed = show_databases(&mut conn);
    assert!(
        !listed.iter().any(|d| d == "blog"),
        "#5009: dropped `blog` must not be listed; got {listed:?}"
    );
    assert!(
        listed.iter().any(|d| d == "shop"),
        "#5009: surviving `shop` must still be listed; got {listed:?}"
    );
}

/// `SHOW TABLES FROM <db>` validates its argument through
/// `ensure_database_known`, which reads the *same* `list_databases()` the
/// forward feeds. So the dropped forward breaks the second consumer too:
/// a database that really exists gets reported as unknown.
///
/// This is the assertion that distinguishes the two layers. `CREATE
/// DATABASE` does not consult the enumeration at all (a duplicate create
/// succeeds on the MVCC path whether or not the forward exists), so a
/// duplicate-create test would pin nothing.
#[test]
fn show_tables_from_a_created_database_is_not_reported_unknown() {
    let handle = start_server();
    let mut conn = connect(handle.port);

    conn.execute("CREATE DATABASE shop")
        .expect("CREATE DATABASE shop");
    let rs = conn.execute("SHOW TABLES FROM shop");
    assert!(
        !matches!(rs, Ok(ResultSet::Error { .. })),
        "#5009: `shop` was just created over this same connection, so \
         `SHOW TABLES FROM shop` must not reject it. With the \
         MvccStorage::list_databases forward removed, ensure_database_known \
         sees an empty list and answers \"Unknown database: shop\". Got {rs:?}"
    );
}
