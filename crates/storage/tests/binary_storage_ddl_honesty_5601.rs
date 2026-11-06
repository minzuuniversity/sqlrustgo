//! #5601：binary 存储下的 DDL 不得「报成功但没生效」。
//!
//! 修复前的实测（run `20261106-cap-smoke-002`，`audit/v4.1.0/binmode_probe.py`）：
//!
//! * `CREATE DATABASE probeb` 返回 **rc=0**，但 `SHOW DATABASES` 里只有 `default`
//!   ⇒ 报成功却零效果（假成功）；随后 `CREATE TABLE smoke.t` 才报
//!   `BinaryTableStorage has no database namespace`。
//! * `CREATE TRIGGER` 返回 **rc=0**，服务端日志里才有自认行
//!   `TriggerExecutor::new: storage has no WAL enabled — triggers will be silently skipped`，
//!   INSERT 时触发器从未执行，`DROP TRIGGER` 再说「not found」——
//!   客户端可观测到的自相矛盾。
//!
//! 修复：`BinaryTableStorage` 对这四条 DDL 明确报错（没有命名空间 / 没有 WAL
//! ⇒ 没有触发器），不再返回 `Ok(())` 假装成功。
//!
//! 判据：四条 DDL **必须**返回错误；返回成功即视为缺陷。

use sqlrustgo_storage::engine::{StorageEngine, TriggerEvent, TriggerInfo, TriggerTiming};
use sqlrustgo_storage::BinaryTableStorage;

fn storage(dir: &std::path::Path) -> BinaryTableStorage {
    BinaryTableStorage::new(dir.to_path_buf()).expect("binary storage opens")
}

#[test]
fn create_database_must_fail_instead_of_reporting_success() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut bin = storage(dir.path());
    let err = bin
        .create_database("probeb")
        .expect_err("CREATE DATABASE must be rejected in binary mode");
    let msg = err.to_string();
    assert!(
        msg.contains("no database namespace"),
        "错误信息必须说明原因（没有库命名空间），got: {}",
        msg
    );
}

#[test]
fn drop_database_must_fail_instead_of_reporting_success() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut bin = storage(dir.path());
    let err = bin
        .drop_database("probeb")
        .expect_err("DROP DATABASE must be rejected in binary mode");
    assert!(
        err.to_string().contains("no database namespace"),
        "got: {}",
        err
    );
}

#[test]
fn create_trigger_must_fail_instead_of_reporting_success() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut bin = storage(dir.path());
    let info = TriggerInfo {
        name: "t1".to_string(),
        table_name: "orders".to_string(),
        timing: TriggerTiming::Before,
        event: TriggerEvent::Insert,
        body: "SET NEW.v = 1".to_string(),
        update_columns: Some(Vec::new()),
        original_sql: String::new(),
    };
    let err = bin
        .create_trigger(info)
        .expect_err("CREATE TRIGGER must be rejected in binary mode");
    let msg = err.to_string();
    assert!(
        msg.contains("does not support triggers"),
        "错误信息必须说明原因（binary 模式没有 WAL ⇒ 没有触发器），got: {}",
        msg
    );
}

#[test]
fn drop_trigger_must_fail_instead_of_reporting_success() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut bin = storage(dir.path());
    let err = bin
        .drop_trigger("t1")
        .expect_err("DROP TRIGGER must be rejected in binary mode");
    assert!(
        err.to_string().contains("does not support triggers"),
        "got: {}",
        err
    );
}

#[test]
fn list_databases_still_reports_only_the_default_namespace() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = storage(dir.path());
    let dbs = bin.list_databases().expect("list_databases");
    assert!(
        dbs.is_empty() || dbs == vec!["default".to_string()],
        "binary 模式没有库命名空间，列表不应凭空多出库：{:?}",
        dbs
    );
}
