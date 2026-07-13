// Resolve mod common; for tests/integration/*.rs
// PR #3807 migrated files from tests/ to tests/integration/
// mod common; resolves relative to this directory.
#[path = "../common/mod.rs"]
pub mod common;
