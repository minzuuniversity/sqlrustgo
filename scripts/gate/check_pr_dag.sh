#!/usr/bin/env bash
# =============================================================================
# check_pr_dag.sh — alias for verify_pr_dag.sh (issue #5457)
# =============================================================================
# Purpose: Provide the gate under the SECOND name the repository already
#          refers to, so that both citations resolve to a real script.
# Coverage: GATE_CONDITIONS.md BE5 / B-F5 (cites verify_pr_dag.sh)
#           docs/releases/v3.8.0/plans/DEVELOPMENT_PLAN.md:92
#           docs/releases/v3.8.0/alpha/ALPHA_STAGE_REVIEW.md:81
#           (both cite check_pr_dag.sh)
# Principle: P1 (有计划必有实现 — a cited artifact must exist)
# Verify:   execs verify_pr_dag.sh, forwarding all arguments and the exit code.
#
# Why two names:
#   The repository refers to this gate under two different names:
#     docs/governance/GATE_CONDITIONS.md:103  -> scripts/gate/verify_pr_dag.sh
#     docs/governance/GATE_CONDITIONS.md:129  -> scripts/gate/verify_pr_dag.sh
#     docs/releases/v3.8.0/plans/DEVELOPMENT_PLAN.md:92
#                                           -> scripts/gate/check_pr_dag.sh
#     docs/releases/v3.8.0/alpha/ALPHA_STAGE_REVIEW.md:81
#                                           -> scripts/gate/check_pr_dag.sh
#   Both were missing. This PR lands ONE implementation and resolves BOTH
#   citations, rather than leaving one of them broken.
#
# Exit codes: identical to verify_pr_dag.sh (0 = PASS, 1 = FAIL, 2 = DRIFT).
# =============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET="$SCRIPT_DIR/verify_pr_dag.sh"

if [ ! -f "$TARGET" ]; then
    echo "❌ $TARGET not found — the alias cannot delegate."
    echo "   Cannot judge — DRIFT, not PASS."
    exit 2
fi

exec bash "$TARGET" "$@"
