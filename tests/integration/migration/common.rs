// Resolve mod common; for tests/integration/migration/*.rs
// Point to the actual implementation in tests/common/mod.rs
// (PR #3807 migrated wal_tx_contract_test.rs from tests/ but didn't replicate this re-export)
#[path = "../../common/mod.rs"]
pub mod common;

pub use common::MySqlTestClient;
