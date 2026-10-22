#!/usr/bin/env bash
# =============================================================================
# check_gate_script_refs.sh — Gate Criterion Script Reference Existence Gate
# =============================================================================
# Purpose: Every script path cited by a GATE CRITERION document must exist
#          and be executable in the repository.
# Coverage: GATE_CONDITIONS.md, RELEASE_GATE_CHECKLIST.md, STAGE_CONFIG.yaml,
#           GATE_CI_CD.md, STAGE_CONTROL_FRAMEWORK.md, INDEX.md,
#           RC_TO_GA_GATE_CHECKLIST.md, AGENTS.md
# Principle: P1 (有计划必有实现 — a cited artifact must exist) /
#            ADR-001 G-01 (Claim ≠ Evidence) / ADR-008 Policy 2
# Verify:   extracts every `scripts/**.sh` citation from the criterion
#           documents and checks each one against the filesystem.
#
# Why this gate exists (issues #5450 / #5457 / #5458):
#   Gate criterion documents cited scripts that did not exist. The
#   consequence was threefold and none of it self-healed:
#     1. The criterion was NOT EXECUTABLE — running it produced
#        "No such file or directory", never PASS and never FAIL.
#     2. Historical gate reports had already recorded the criterion as
#        "PASS", and there was NO reproducible way to re-verify it:
#          docs/releases/v4.0.0/BETA_GATE_REPORT.md:19
#          docs/releases/v3.11.0/BETA_GATE_REPORT.md:38,58
#     3. The criterion table had been accepted without an existence check.
#
#   This script is the root fix: it makes the defect DETECTABLE THE MOMENT
#   IT APPEARS, rather than surfacing months later as an unauditable
#   historical claim.
#
# Scope discipline (why not "scan everything")
# ---------------------------------------------
#   A repository-wide scan finds ~90 distinct missing script citations.
#   Most of them live in HISTORICAL records (`docs/releases/v3.x.0/**`,
#   `evidence/**`) that faithfully describe what was true at the time.
#   Failing on those would mean trading one fake green for a permanent red
#   — a net loss.
#
#   So the scope is deliberately: SSOT criterion documents (live documents
#   whose paths must be runnable) + a committed baseline for KNOWN debt.
#   That is the same pattern the repository already uses for document
#   pointers: scripts/gate/check_docs_pointer_paths.sh together with
#   tests/baseline/doc_pointer_paths_releases_v4.1.0.json.
#
# Baseline semantics
# ------------------
#   missing AND in baseline    => WARN  (known debt, tracked, with a reason)
#   missing AND NOT in baseline=> FAIL  (a NEW instance of the defect)
#   exists                    => PASS
#
#   Every baseline entry carries a reason. A baseline entry without a
#   reason is a hidden problem, which is the opposite of this gate's
#   purpose.
#
# Exit codes:
#   0 = PASS — no un-baselined missing script
#   1 = FAIL — at least one missing script that is not in the baseline
#   2 = ENV_ERROR — the baseline file or every criterion document is missing
#
# DRIFT is NOT PASS (ADR-006 P14 / scripts/gate/check_drift_not_pass.sh).
# =============================================================================

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

# --- ADR-014 5 evidence fields -------------------------------------------------
SOURCE_AGENT="${SOURCE_AGENT:-opencode-z440}"
SOURCE_RUN="${SOURCE_RUN:-opencode-z440-20261022-gate-refs}"
TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

# --- Criterion documents (SSOT — live documents) ------------------------------
CRITERION_DOCS=(
    "docs/governance/GATE_CONDITIONS.md"
    "docs/governance/RELEASE_GATE_CHECKLIST.md"
    "docs/governance/STAGE_CONFIG.yaml"
    "docs/governance/GATE_CI_CD.md"
    "docs/governance/STAGE_CONTROL_FRAMEWORK.md"
    "docs/governance/INDEX.md"
    "docs/governance/RC_TO_GA_GATE_CHECKLIST.md"
    "AGENTS.md"
)

BASELINE="tests/baseline/gate_script_refs_baseline.json"

# This gate deliberately has NO --update-baseline mode. The baseline is a
# DEBT LEDGER: every entry names the file:line that cites the missing
# script and says what to do about it. A tool that rewrote it automatically
# would let debt disappear without anyone deciding to retire it.
#
# The previous version of the baseline file advertised
#   "Regenerate with: bash scripts/gate/check_gate_script_refs.sh --update-baseline"
# while the script had no argument parsing at all — the flag was silently
# ignored and the command just re-printed the report. An advertised
# capability that does not exist is an AFP Type A claim. The flag is now
# rejected with an explanation instead of being ignored.
for arg in "$@"; do
    if [ "$arg" = "--update-baseline" ]; then
        echo "❌ --update-baseline is not supported, by design."
        echo "   The baseline is a debt ledger whose entries name the citing"
        echo "   file:line and say what to do about them. Add an entry by"
        echo "   hand after verifying the citation yourself."
        exit 2
    fi
done

# --- house-style counters (see check_anti_fabrication.sh) ----------------------
ERRORS=0
WARNINGS=0
PASSED=0

log_info()  { echo "[INFO] $*"; }
log_pass()  { PASSED=$((PASSED + 1)); echo "[PASS] $*"; }
log_warn()  { WARNINGS=$((WARNINGS + 1)); echo "[WARN] $*"; }
log_error() { ERRORS=$((ERRORS + 1)); echo "[ERROR] $*"; }

echo "=== Gate Criterion Script Reference Existence Check ==="
echo "  source_agent: $SOURCE_AGENT"
echo "  source_run:   $SOURCE_RUN"
echo "  timestamp:    $TIMESTAMP"
echo "  baseline:     $BASELINE"
echo

# --- Phase 1: baseline availability -------------------------------------------
if [ ! -f "$BASELINE" ]; then
    echo "❌ baseline not found: $BASELINE"
    echo "   Without a baseline, a missing script cannot be told apart from"
    echo "   known debt. Cannot judge — DRIFT, not PASS."
    exit 2
fi

# --- Phase 2: document availability -------------------------------------------
DOCS_OK=0
for doc in "${CRITERION_DOCS[@]}"; do
    if [ -f "$doc" ]; then
        DOCS_OK=$((DOCS_OK + 1))
    else
        log_warn "criterion document not found: $doc"
    fi
done

if [ "$DOCS_OK" -eq 0 ]; then
    echo "❌ No criterion document could be read — cannot judge."
    exit 2
fi
log_info "criterion documents readable: $DOCS_OK / ${#CRITERION_DOCS[@]}"
echo

# --- Phase 3: extract cited script paths --------------------------------------
# Regex covers BOTH `scripts/gate/x.sh` and `scripts/x.sh`. The narrower
# `scripts/gate/`-only pattern misses issue #5450's instances, whose
# missing scripts live directly under `scripts/`.
CITED=$(grep -hoE '(\./)?scripts/[A-Za-z0-9_./-]+\.sh' "${CRITERION_DOCS[@]}" 2>/dev/null \
    | sed 's|^\./||' | sort -u)

if [ -z "$CITED" ]; then
    echo "❌ No script citations extracted — the extraction is broken."
    exit 2
fi

CITED_COUNT=$(printf '%s\n' "$CITED" | grep -c .)
log_info "distinct script paths cited: $CITED_COUNT"
echo

# --- Sourced libraries ----------------------------------------------------------
# Gate files that are `source`d by a driver rather than executed. They MUST
# NOT carry the executable bit, so requiring it would be a false alarm.
#
# The list is explicit: inferring "is this sourced?" by grepping is exactly
# the kind of judgment this gate exists to avoid.
SOURCED_LIBRARIES=(
    "scripts/gate/stage_config_common.sh"
)

is_sourced_library() {
    local candidate="$1"
    local lib
    for lib in "${SOURCED_LIBRARIES[@]}"; do
        [ "$candidate" = "$lib" ] && return 0
    done
    return 1
}

# --- Phase 4: existence check -------------------------------------------------
EXISTS=0
NOT_EXEC=0
MISSING=0
BASELINED=0
NEW_MISSING=0
NEW_MISSING_LIST=()

while IFS= read -r path; do
    [ -z "$path" ] && continue
    if [ -f "$path" ]; then
        # A cited script that exists but is NOT executable is unusable when
        # it is meant to be RUN. But some gate files are LIBRARIES that
        # are `source`d by a driver (e.g. scripts/gate/stage_config_common.sh,
        # sourced by check_stage.sh) — those must NOT be executable, and
        # requiring it would be a false alarm.
        #
        # The distinction is observable: a file that is sourced elsewhere
        # is not invoked as a command. This list is explicit rather than
        # inferred, because "infer it from a grep" is exactly the kind of
        # judgment this gate exists to avoid.
        if [ -x "$path" ]; then
            EXISTS=$((EXISTS + 1))
        elif is_sourced_library "$path"; then
            EXISTS=$((EXISTS + 1))
        else
            NOT_EXEC=$((NOT_EXEC + 1))
            NEW_MISSING_LIST+=("$path (exists but is not executable)")
            log_error "$path — cited by a criterion document, exists, but is NOT executable (mode $(stat -c '%a' "$path" 2>/dev/null || echo '?')). The criterion cannot be executed."
        fi
        continue
    fi

    MISSING=$((MISSING + 1))

    # Is it known debt?
    reason=$(python3 - "$BASELINE" "$path" <<'PY'
import json, sys
baseline_path, target = sys.argv[1], sys.argv[2]
try:
    with open(baseline_path) as fh:
        data = json.load(fh)
except Exception:
    sys.exit(0)
entry = (data.get("known_missing") or {}).get(target)
if entry:
    sys.stdout.write(entry if isinstance(entry, str) else entry.get("reason", ""))
PY
)
    if [ -n "$reason" ]; then
        BASELINED=$((BASELINED + 1))
        log_warn "$path — missing, but registered in the baseline as known debt: $reason"
    else
        NEW_MISSING=$((NEW_MISSING + 1))
        NEW_MISSING_LIST+=("$path")
        log_error "$path — cited by a criterion document but does not exist, and is NOT in the baseline"
    fi
done <<< "$CITED"

# --- Phase 5: verdict ----------------------------------------------------------
echo
echo "--- Summary ---"
echo "  cited paths:        $CITED_COUNT"
echo "  exist + executable: $EXISTS"
echo "  exist, not exec:    $NOT_EXEC"
echo "  missing (total):    $MISSING"
echo "  missing (baselined):$BASELINED"
echo "  missing (NEW):      $NEW_MISSING"
echo "  errors:             $ERRORS"
echo "  warnings:           $WARNINGS"

if [ "$NEW_MISSING" -gt 0 ]; then
    echo
    echo "❌ FAIL: ${NEW_MISSING} referenced script(s) that cannot be executed"
    for m in "${NEW_MISSING_LIST[@]}"; do
        echo "     - $m"
    done
    echo
    echo "A criterion that references a missing script is NOT EXECUTABLE."
    echo "Either implement the script, remove the criterion, or — if the debt is"
    echo "already known — register it in $BASELINE with a reason."
    exit 1
fi

if [ "$ERRORS" -gt 0 ]; then
    echo
    echo "❌ FAIL: $ERRORS error(s) detected."
    exit 1
fi

echo
echo "✅ PASS: all $EXISTS existing citations verified (present AND executable); $BASELINED known-debt entries reported as warnings."
exit 0
