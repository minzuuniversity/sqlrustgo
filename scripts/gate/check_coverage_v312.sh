#!/bin/bash
# V312-35: per-crate coverage measurement (L1_8)
# Threshold: the SSOT's per-stage bar —
#   docs/governance/STAGE_CONFIG.yaml → thresholds → COVERAGE_MIN_<STAGE>
#   (ALPHA 50 / BETA 75 / RC 80 / GA 85); this script has **no constant of
#   its own**. Which stage a given invocation judges is stated by its
#   caller (SQLRUSTGO_COVERAGE_STAGE); unset means GA, the strictest bar.
# Exit 0 = PASS, Exit 1 = FAIL
#
# #5579: the measurement this script used to own now lives in
# scripts/gate/common.sh, because the RC/GA gate's A5 criterion measured the
# same eight crates with a *different* implementation — and that one quietly
# dropped any crate it could not measure from the average. The two gates
# therefore reported different numbers for the same quantity, and the RC/GA
# one reported a number computed over a silently reduced sample.
#
# One rule, one implementation:
#
#   * the crate set (the canonical L1_8 set)            — common.sh
#   * the per-crate feature derivation                   — common.sh
#   * the environment-dependent test exclusions          — common.sh
#   * the report column (Lines, not Regions)             — common.sh
#   * the fail-closed verdict                            — common.sh
#
# This script keeps only what is specific to the L1_8 criterion: the
# threshold and the fact that this gate exists.
#
# The verdict itself fails closed in both directions:
#
#   * a crate that produces no coverage table  => FAIL, naming the crate
#   * every crate measured but the average is under the threshold => FAIL
#
# The first case is not hypothetical. storage's crash-consistency tests need a
# real `sqlrustgo-mysql-server` binary; measure the crate without the feature
# derivation and the environment-dependent exclusions and it produces NO
# coverage table at all, and the average silently becomes an average over
# seven crates. The report prints the sample size ("measured 8/8 crates") so
# a reduced sample is visible in the report itself and not only in the exit
# status.
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

# The coverage measurement — one rule, one implementation.
# shellcheck source=scripts/gate/common.sh
source "$SCRIPT_DIR/common.sh"
# The threshold reader — one rule, one implementation (#5581).
# shellcheck source=scripts/gate/stage_config_common.sh
source "$SCRIPT_DIR/stage_config_common.sh"

# Which stage's threshold this invocation judges.
#
#   SQLRUSTGO_COVERAGE_STAGE=alpha|beta|rc|ga   — explicit
#   (unset)                                    — GA (the strictest bar in the SSOT)
#
# #5581: this used to be a hardcoded `THRESHOLD=80.0`, while the SSOT
# (docs/governance/STAGE_CONFIG.yaml → thresholds) says ALPHA 50 / BETA 75 /
# RC 80 / GA 85. This script is **GA's own `required_gates` entry**, so a GA
# release was being judged at 80 while the SSOT said 85 — and the
# `docs/releases/v4.1.0/GA_GATE_REPORT.md` line "门槛来源：STAGE_CONFIG.yaml
# → constants" was describing a threshold this script never applied.
#
# WHY THE DEFAULT IS GA AND NOT "THE CURRENT STAGE"
# -------------------------------------------------
# The obvious default — "the release's current stage" — would have been
# **ALPHA = 50** for v4.1.0 (docs/releases/v4.1.0/STAGE.yaml → current_stage).
# That is *looser* than the 80 this script used to enforce, so every caller
# that forgot to state its stage would have **silently loosened** the gate.
# A refactor must never weaken a gate by accident, so the default is the
# **strictest** bar in the SSOT: a caller that forgets to say which stage it
# is judging still gets the hardest bar, never the easiest.
#
# Every call site in this repository states its stage explicitly:
#
#   .gitea/workflows/ci.yml              → ga    (the GA coverage criterion)
#   .github/workflows/ci-pr.yml          → ga    (same criterion, public mirror)
#   scripts/gate/check_alpha_quality_v4.10.sh → alpha (the ALPHA quality gate)
#   scripts/gate/check_v312_gate_thresholds.sh → rc (v3.12.0 declares 80)
#
# GA = 85 is not a new decision: docs/releases/v4.1.0/GA_GATE_REPORT.md §G3
# already states "GA 按 85 判" (and `STAGE.yaml` → coverage_target_for_ga
# says the same). This script simply applies it.
COVERAGE_STAGE="${SQLRUSTGO_COVERAGE_STAGE:-ga}"

THRESHOLD="$(stage_coverage_min "$COVERAGE_STAGE")" || {
    echo "FAIL: no coverage threshold for stage '${COVERAGE_STAGE}' in the SSOT" >&2
    echo "      (docs/governance/STAGE_CONFIG.yaml → thresholds → COVERAGE_MIN_<STAGE>)" >&2
    exit 1
}

echo "Coverage criterion: stage=${COVERAGE_STAGE} threshold=${THRESHOLD}%"
echo

coverage_average "$THRESHOLD"
