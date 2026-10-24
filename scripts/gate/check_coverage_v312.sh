#!/bin/bash
# V312-35: per-crate coverage measurement (L1_8)
# Threshold: L1_8 average line coverage >= 80%
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

THRESHOLD=80.0

coverage_average "$THRESHOLD"
