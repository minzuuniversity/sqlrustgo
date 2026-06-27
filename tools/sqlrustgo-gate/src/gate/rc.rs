//! RC Gate: "Release candidate — publish lock"
//!
//! Stage semantics: API locked, no more breaking changes, all quality bars final.
//!
//! RC = strictest — full suite + coverage raised to 60% + TPC-H strict SLO.

use crate::checks;

pub fn run() -> anyhow::Result<()> {
    println!("=== RC GATE v3.7.0 ===");

    checks::build::check()?;
    checks::test::unit()?;
    checks::integration::subset()?;
    checks::clippy::check_strict()?;
    checks::coverage::check(60.0)?;
    checks::sql_corpus::full()?;
    checks::perf::tpch_strict()?;

    println!();
    println!("[rc] PASS — all checks passed, API locked, release candidate ready");
    println!();

    Ok(())
}
