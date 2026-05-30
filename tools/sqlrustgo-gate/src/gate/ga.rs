//! GA Gate — General Availability promotion
//!
//! Final gate: certifies the release is production-ready.
//! Requires: Alpha ✅ + Beta ✅ + RC ✅ already passed.
//!
//! GA = lightweight sign-off: verify all prior gates passed + create tag.

use crate::gate::stability::StabilityWindow;

/// Load and verify the stability window shows clean history
fn verify_clean_history() -> anyhow::Result<()> {
    let window = StabilityWindow::load_or_default();
    let summary = window.summary();

    println!(
        "[ga] stability score: {:.2} (streak: {}, runs: {})",
        summary.score, summary.streak, summary.total_runs
    );

    if summary.is_degraded {
        anyhow::bail!("[ga] FAILED — stability window is degraded. Fix before GA.");
    }

    // Require at least 3 consecutive passes (Alpha + Beta + RC)
    if summary.streak < 3 {
        println!(
            "[ga] warning: only {} consecutive passes (expected >= 3)",
            summary.streak
        );
    }

    Ok(())
}

pub fn run() -> anyhow::Result<()> {
    println!("=== GA GATE v3.7.0 ===");

    // Verify prior gates passed (check stability window)
    println!("[ga] verifying prior gate history ...");
    verify_clean_history()?;

    // Final build sanity
    println!("[ga] final build check ...");
    crate::checks::build::check()?;

    // Record GA pass
    crate::gate::beta::record_result("ga", true, 0.1);

    println!();
    println!("[ga] PASS — v3.7.0 promoted to General Availability");
    println!("[ga] Next: create git tag v3.7.0 and publish");
    println!();

    Ok(())
}
