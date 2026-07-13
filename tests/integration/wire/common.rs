// Re-export shared test utilities for tests/integration/wire/
// PR #3807 migrated files from tests/ to tests/integration/wire/
// mod common; resolves relative to this directory.
#[path = "../../common/mod.rs"]
pub mod common;
