#!/usr/bin/env bash
# check_release_artifacts.sh — are the artifacts this stage demands actually there?
#
# WHY THIS FILE EXISTS
# ---------------------
# Issue #5439: `docs/releases/v4.1.0/` was missing 4 of the 5 GA
# `required_files` and 4 of the 6 GA `doc_artifacts`, and **nothing in the
# repository noticed**. The reason was structural:
#
#   * `check_evidence_binding.sh` walks the `.md` files that **exist**
#     (`find "$RELEASE_DIR" -name "*.md"`) — a missing file cannot be
#     bound to evidence, and a file that is not there is not walked at
#     all;
#   * `doc_artifacts` were listed in `STAGE_CONFIG.yaml` but **no code
#     ever read them**;
#   * three of the five GA `required_files` carried a `(state=GA)`
#     annotation glued onto the path, which made them unresolvable — so
#     even the list that did exist could not be checked.
#
# ⇒ This gate is the missing "required document list" the issue asks for.
# It reads the **same** list the driver reads (the SSOT), applies the
# **same** resolution rules (scripts/gate/stage_config_common.sh), and
# fails when an artifact is missing.
#
# WHAT IT DOES NOT DO
# -------------------
# It does not create the artifacts. A missing GA artifact is reported as
# missing — it is never faked, never stubbed, and "skip" is never
# reported as "pass".
#
# MODES
# -----
#   default          check the version's **current** stage (read from
#                    docs/releases/<ver>/STAGE.yaml). Missing artifacts ⇒ FAIL.
#   --stage <S>      check one specific stage instead.
#   --all-stages     check every stage; missing artifacts in ANY stage ⇒ FAIL.
#                    (Used when promoting, where the next stage's
#                    artifacts must be present too.)
#
# In every mode the gate additionally **discloses** the artifacts that
# the *later* stages still need. Those are not failures — the version has
# not reached that stage — but they must be visible, because "we are
# nowhere near GA" should be a fact somebody can read, not a fact that has
# to be re-derived by hand.
#
# Usage:
#   bash scripts/gate/check_release_artifacts.sh --version v4.1.0
#   bash scripts/gate/check_release_artifacts.sh --version v4.1.0 --stage GA
#   bash scripts/gate/check_release_artifacts.sh --selftest
#
# Exit codes:
#   0 = PASS (every artifact of the checked stage exists)
#   1 = FAIL (at least one required artifact is missing)
#   2 = usage / environment error
#
# Refs: #5439 (missing GA artifacts must be detectable), #5437 (one
#       criterion), #5440 (the SSOT needs an executor).

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/gate/stage_config_common.sh
source "${SCRIPT_DIR}/stage_config_common.sh"

VERSION=""
STAGE=""
ALL_STAGES=false
SELFTEST=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --version)    VERSION="$2"; shift 2 ;;
        --stage)      STAGE="$2"; shift 2 ;;
        --all-stages) ALL_STAGES=true; shift ;;
        --selftest)   SELFTEST=true; shift ;;
        -h|--help)    grep -E '^#( |!)' "$0" | head -45; exit 0 ;;
        *) echo "Unknown arg: $1" >&2; exit 2 ;;
    esac
done

# --- selftest ---------------------------------------------------------------
#
# The gate must tell "every artifact present" from "one artifact missing".
# The selftest builds a synthetic release directory and a synthetic stage
# config and asserts both answers.
run_selftest() {
    local tmp rc=0
    tmp="$(mktemp -d "${TMPDIR:-/tmp}/release_artifacts_selftest.XXXXXX")"

    cat >"$tmp/cfg.yaml" <<'YAMLEOF'
framework:
  stages: [ALPHA, GA]
stages:
  ALPHA:
    required_files:
      - "docs/releases/v{VER}/STAGE.yaml"
    doc_artifacts:
      - "CHANGELOG.md"
  GA:
    required_files:
      - "docs/releases/v{VER}/GA_GATE_REPORT.md"
    doc_artifacts:
      - "POST_GA_PLAN.md"
YAMLEOF

    mkdir -p "$tmp/release/v4.1.0"
    # The clean case: everything the ALPHA stage demands exists.
    printf 'x\n' >"$tmp/release/v4.1.0/STAGE.yaml"
    printf 'x\n' >"$tmp/release/v4.1.0/CHANGELOG.md"

    # check_stage_artifacts <config> <version> <stage> <release-dir-root>
    # The release-dir-root lets the selftest point the resolution at a
    # synthetic tree instead of the repository.
    local out rc_clean rc_missing
    out="$(check_stage_artifacts "$tmp/cfg.yaml" v4.1.0 ALPHA "$tmp/release" 2>&1)"; rc_clean=$?
    if [[ $rc_clean -eq 0 ]] && printf '%s' "$out" | grep -q "OK.*STAGE.yaml"; then
        echo "  [OK]   complete artifact set -> PASS"
    else
        echo "  [FAIL] complete artifact set -> FAIL (rc=$rc_clean)"
        printf '%s\n' "$out" | sed 's/^/         /'
        rc=1
    fi

    # Remove one artifact and the gate must notice.
    rm -f "$tmp/release/v4.1.0/CHANGELOG.md"
    out="$(check_stage_artifacts "$tmp/cfg.yaml" v4.1.0 ALPHA "$tmp/release" 2>&1)"; rc_missing=$?
    if [[ $rc_missing -ne 0 ]] && printf '%s' "$out" | grep -q "MISS.*CHANGELOG.md"; then
        echo "  [OK]   missing artifact -> FAIL (and named)"
    else
        echo "  [FAIL] missing artifact -> not detected (rc=$rc_missing)"
        printf '%s\n' "$out" | sed 's/^/         /'
        rc=1
    fi

    rm -rf "$tmp"
    return $rc
}

# --- the check --------------------------------------------------------------
#
# check_stage_artifacts <config> <version> <stage> [release-root]
#
# release-root overrides the directory the version's release files are
# resolved against; it exists so the selftest can work on a synthetic
# tree. Empty = the repository.
check_stage_artifacts() {
    local cfg="$1" ver="$2" stage="$3" root="${4:-}"
    local kind entry resolved rc=0
    while IFS=$'\x1f' read -r kind entry; do
        [[ -z "${kind:-}" || -z "${entry:-}" ]] && continue
        case "$kind" in
            required_file) resolved="$(stage_resolve_required_file "$entry" "$ver")" ;;
            doc_artifact) resolved="$(stage_resolve_doc_artifact "$entry" "$ver")" ;;
            *) continue ;;
        esac
        if [[ -n "$root" ]]; then
            resolved="${resolved/#docs\/releases\//$root/}"
        fi
        if [[ -f "$resolved" ]]; then
            printf 'OK\t%s\t%s\n' "$kind" "$resolved"
        else
            printf 'MISS\t%s\t%s\n' "$kind" "$resolved"
            rc=1
        fi
    done < <(stage_artifact_entries "$cfg" "$stage")
    return $rc
}

if [[ "$SELFTEST" == true ]]; then
    echo "=== release artifacts gate — selftest ==="
    if run_selftest; then
        echo "SELFTEST PASS"
        exit 0
    fi
    echo "SELFTEST FAIL"
    exit 1
fi

# --- main -------------------------------------------------------------------

if [[ -z "$VERSION" ]]; then
    # Derive the version from the current branch (develop/vX.Y.Z) or, as a
    # fallback, from the release directories present in the tree.
    branch="$(git symbolic-ref --short HEAD 2>/dev/null || echo "")"
    if [[ "$branch" =~ ^(develop|beta|rc|release|hotfix)/v(.+)$ ]]; then
        VERSION="v${BASH_REMATCH[2]}"
    else
        VERSION="v4.1.0"
    fi
fi

if [[ -z "$STAGE" ]]; then
    STAGE="$(stage_current_stage "$VERSION")"
    if [[ -z "$STAGE" || "$STAGE" == "unknown" ]]; then
        echo "ERROR: cannot determine the current stage of $VERSION" >&2
        echo "       (docs/releases/${VERSION}/STAGE.yaml has no current_stage)" >&2
        exit 2
    fi
fi

CONFIG="${STAGE_CONFIG_PATH_DEFAULT}"
if [[ ! -f "$CONFIG" ]]; then
    echo "ERROR: stage config not found: $CONFIG" >&2
    exit 2
fi

echo "=== Release Artifacts Gate ==="
echo "version: $VERSION"
echo "stage:   $STAGE"
echo ""

# The stages that come after the current one — their artifacts are
# disclosed, not enforced.
ALL_STAGE_NAMES=()
while IFS= read -r _s; do
    [[ -n "$_s" ]] && ALL_STAGE_NAMES+=("$_s")
done < <(stage_stage_names "$CONFIG")
current_idx=-1
for i in "${!ALL_STAGE_NAMES[@]}"; do
    if [[ "${ALL_STAGE_NAMES[$i]}" == "$STAGE" ]]; then
        current_idx=$i
        break
    fi
done

rc=0
if [[ "$ALL_STAGES" == true ]]; then
    echo "--- All stages (blocking) ---"
    for s in "${ALL_STAGE_NAMES[@]}"; do
        out="$(check_stage_artifacts "$CONFIG" "$VERSION" "$s")"; src=$?
        printf '%s\n' "$out" | sed "s/^/  [$s] /"
        [[ $src -ne 0 ]] && rc=1
    done
else
    echo "--- $STAGE stage (blocking) ---"
    out="$(check_stage_artifacts "$CONFIG" "$VERSION" "$STAGE")"; src=$?
    printf '%s\n' "$out" | sed 's/^/  /'
    [[ $src -ne 0 ]] && rc=1
fi

# Disclosure: what the later stages still need.
if [[ "$ALL_STAGES" != true && $current_idx -ge 0 ]]; then
    later=("${ALL_STAGE_NAMES[@]:$((current_idx + 1))}")
    if [[ ${#later[@]} -gt 0 ]]; then
        echo ""
        echo "--- Later stages (disclosure only — not enforced at $STAGE) ---"
        for s in "${later[@]}"; do
            out="$(check_stage_artifacts "$CONFIG" "$VERSION" "$s")"
            n_miss=$(printf '%s\n' "$out" | grep -c '^MISS	' || true)
            n_ok=$(printf '%s\n' "$out" | grep -c '^OK	' || true)
            echo "  $s: $n_ok present, $n_miss missing"
            printf '%s\n' "$out" | grep '^MISS	' | sed 's/^/    /' || true
        done
    fi
fi

echo ""
echo "----------------------------------------"
if [[ $rc -eq 0 ]]; then
    echo "  RESULT: PASS"
else
    echo "  RESULT: FAIL — required artifacts are missing"
fi
echo "----------------------------------------"
exit $rc
