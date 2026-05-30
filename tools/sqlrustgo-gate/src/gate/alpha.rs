//! Alpha Gate: "Architecture can compile + core path does not break"
//!
//! Stage semantics: API may change, executor may be重构不稳定, telemetry链路尚完整
//!
//! Alpha = lightweight — coverage / integration / performance NOT required here.
//! Those belong to Beta.

use crate::checks;

pub fn run() -> anyhow::Result<()> {
    println!("=== ALPHA GATE v3.7.0 ===");

    checks::build::check()?;
    checks::test::unit()?;
    checks::clippy::check()?;

    // Parser smoke: core SQL parsing must not regress
    checks::sql_corpus::smoke()?;

    println!();
    println!("[alpha] PASS — core compilation + unit tests + clippy OK");
    println!();

    Ok(())
}
