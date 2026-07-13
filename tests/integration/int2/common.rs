// Re-export shared test utilities for tests/integration/int2/
// PR #3807 migrated files from tests/ to tests/integration/int2/
// mod common; resolves relative to this directory.
#[path = "../../common/mod.rs"]
pub mod common;
