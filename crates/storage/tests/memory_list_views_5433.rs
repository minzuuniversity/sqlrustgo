//! Regression test: `MemoryStorage::list_views` must see the views that
//! `create_view` stored.
//!
//! Issue #5433 — `CREATE VIEW` succeeded, `has_view` and `get_view` found the
//! view, but `sqlite_master` listed **no view at all**: `sqlite_master` builds
//! its rows from `storage.list_views(db)`, and that call always returned an
//! empty list.
//!
//! Root cause: the key separator drifted. `scoped_key` joins the database and
//! the object name with `DB_SEP` (`\u{1}`), but `list_views` stripped a
//! hand-spelled `"{db}\0"` prefix — a **NUL**, not `DB_SEP`. The strip never
//! matched, so the list was empty. `FileStorage::list_views` had exactly the
//! same defect and was fixed by #5350 ("split on the real separator instead of
//! hand-spelling it"); `MemoryStorage` was the twin that was left behind.
//!
//! This test pins the contract at the storage layer, so that a future change
//! to the key format cannot silently empty the list again — the failure mode
//! is invisible from the outside (every view lookup still works; only the
//! introspection list comes out empty).

use sqlrustgo_storage::engine::DEFAULT_DATABASE;
use sqlrustgo_storage::{MemoryStorage, StorageEngine, ViewInfo};

fn view(name: &str) -> ViewInfo {
    ViewInfo::with_original_sql(
        name.to_string(),
        vec!["id".to_string()],
        format!("SELECT id FROM {}", name),
        format!("CREATE VIEW {} AS SELECT id FROM base", name),
    )
}

#[test]
fn memory_storage_list_views_sees_created_views() {
    let mut s = MemoryStorage::new();
    assert!(
        s.list_views(DEFAULT_DATABASE).is_empty(),
        "a fresh store has no views"
    );

    s.create_view(DEFAULT_DATABASE, view("v_base")).unwrap();

    let names = s.list_views(DEFAULT_DATABASE);
    assert_eq!(
        names,
        vec!["v_base".to_string()],
        "list_views must return the view create_view stored"
    );
}

#[test]
fn memory_storage_list_views_is_scoped_to_its_database() {
    let mut s = MemoryStorage::new();
    s.create_database("d1").unwrap();
    s.create_view("d1", view("v_d1")).unwrap();

    assert_eq!(
        s.list_views("d1"),
        vec!["v_d1".to_string()],
        "the view belongs to d1"
    );
    assert!(
        s.list_views(DEFAULT_DATABASE).is_empty(),
        "and must not leak into the default database"
    );
}

#[test]
fn memory_storage_list_views_agrees_with_has_view_and_get_view() {
    let mut s = MemoryStorage::new();
    s.create_view(DEFAULT_DATABASE, view("v_a")).unwrap();
    s.create_view(DEFAULT_DATABASE, view("v_b")).unwrap();

    let mut listed = s.list_views(DEFAULT_DATABASE);
    listed.sort();
    assert_eq!(listed, vec!["v_a".to_string(), "v_b".to_string()]);

    // The three accessors must agree — a view that `has_view` reports but
    // `list_views` omits is precisely the #5433 failure.
    for name in &listed {
        assert!(
            s.has_view(DEFAULT_DATABASE, name),
            "{} must be present",
            name
        );
        assert!(
            s.get_view(DEFAULT_DATABASE, name).is_some(),
            "{} must be retrievable",
            name
        );
    }
}

#[test]
fn memory_storage_drop_view_removes_it_from_the_list() {
    let mut s = MemoryStorage::new();
    s.create_view(DEFAULT_DATABASE, view("v_tmp")).unwrap();
    assert_eq!(s.list_views(DEFAULT_DATABASE).len(), 1);

    s.drop_view(DEFAULT_DATABASE, "v_tmp").unwrap();
    assert!(
        s.list_views(DEFAULT_DATABASE).is_empty(),
        "a dropped view must not linger in the list"
    );
}
