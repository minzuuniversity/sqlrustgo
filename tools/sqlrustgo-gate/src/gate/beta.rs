//! Beta Gate: "Quality main battlefield"
//!
//! Stage semantics: API is stabilizing, executor is stable, telemetry链路完整
//!
//! Beta = heavy validation — coverage, integration, SQL corpus, performance regression.

use crate::checks;

pub fn run() -> anyhow::Result<()> {
    println!("=== BETA GATE v3.7.0 ===");

    checks::build::check()?;
    checks::test::unit()?;
    checks::test::integration()?;
    checks::clippy::check()?;
    checks::coverage::check(50.0)?;
    checks::sql_corpus::full()?;
    checks::integration::run()?;
    checks::perf::tpch_regression(5.0)?;

    println!();
    println!("[beta] PASS — build + tests + coverage + SQL corpus + integration + perf regression OK");
    println!();

    Ok(())
}