#!/usr/bin/env bash
#
# scripts/gate/check_tpch_known_differences.sh
#
# Registry gate for the TPC-H "known differences" table (issue #5379).
#
# A TPC-H difference against the Oracle reference is only acceptable when it
# is *registered*: an issue to track it, a named owner, and a date from
# which the difference has been known. This gate enforces that discipline:
#
#   1. the registry is valid JSON and has all top-level fields
#   2. every entry has all required fields with the right types
#   3. `issue` is non-empty      (no unowned differences)
#   4. `owner` is non-empty      (no unowned differences)
#   5. `case_id` is unique across the whole registry
#   6. the number of entries does not exceed --max-entries
#      (default 1 => the registry holds the single difference that is
#       registered today; known differences may not grow silently)
#   7. `since` is a YYYY-MM-DD date
#   8. an entry of kind `row_count` whose `actual_row_count` equals its
#      `oracle_row_count` is stale — the difference no longer exists and
#      must be removed. (An entry of kind `cell_values` legitimately has
#      equal row counts: the difference is in the cells.)
#
# Usage:
#   bash scripts/gate/check_tpch_known_differences.sh [--registry <path>] [--max-entries <N>]
#
# Options:
#   --registry <path>    registry file
#                        (default: tests/baseline/tpch_known_differences.json)
#   --max-entries <N>    maximum number of registered entries
#                        (default: 1)
#   -h, --help           Show this help.
#
# Exit codes:
#   0  PASS — the registry is valid and within the entry limit
#   1  FAIL — at least one rule is violated
#   2  usage error / a required input file is missing
#
# Requires: bash >= 3.2 (macOS default), python3 (JSON parsing; jq is NOT a
# dependency of this repository).

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

REGISTRY="${REPO_ROOT}/tests/baseline/tpch_known_differences.json"
MAX_ENTRIES=1

color_red()   { printf '\033[0;31m%s\033[0m' "$*"; }
color_green() { printf '\033[0;32m%s\033[0m' "$*"; }
color_yellow() { printf '\033[0;33m%s\033[0m' "$*"; }

step() { echo ""; echo "=== $* ==="; }
pass() { echo "  $(color_green PASS): $*"; }
fail() { echo "  $(color_red FAIL): $*"; }
warn() { echo "  $(color_yellow WARN): $*"; }

usage() {
    cat <<'USAGE'
Usage: bash scripts/gate/check_tpch_known_differences.sh \
         [--registry <path>] [--max-entries <N>]

  --registry <path>    registry file
                       (default: tests/baseline/tpch_known_differences.json)
  --max-entries <N>    maximum number of registered entries (default: 1)
  -h, --help           this message
USAGE
}

usage_error() {
    printf 'ERROR: %s\n' "$*" >&2
    usage >&2
    exit 2
}

need_value() {
    if [[ "$2" -lt 2 ]]; then
        usage_error "option $1 requires a value"
    fi
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --registry)     need_value "$1" $#; REGISTRY="$2"; shift 2 ;;
        --registry=*)   REGISTRY="${1#--registry=}"; shift ;;
        --max-entries)  need_value "$1" $#; MAX_ENTRIES="$2"; shift 2 ;;
        --max-entries=*) MAX_ENTRIES="${1#--max-entries=}"; shift ;;
        -h|--help)      usage; exit 0 ;;
        *)              usage_error "unknown argument: $1" ;;
    esac
done

if ! [[ "${MAX_ENTRIES}" =~ ^[0-9]+$ ]]; then
    usage_error "invalid --max-entries '${MAX_ENTRIES}': expected a non-negative integer"
fi
if [[ ! -f "${REGISTRY}" ]]; then
    usage_error "registry not found: ${REGISTRY}"
fi
if ! command -v python3 >/dev/null 2>&1; then
    usage_error "python3 not on PATH"
fi

REGISTRY_ABS="$(cd "$(dirname "${REGISTRY}")" && pwd)/$(basename "${REGISTRY}")"

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/tpch-known-diff.XXXXXX")"
trap 'rm -rf -- "${TMP_DIR}"' EXIT

step "Known-differences registry check"

echo "  registry:    ${REGISTRY_ABS}"
echo "  max_entries: ${MAX_ENTRIES}"

# ---------------------------------------------------------------------------
# Validate the registry
# ---------------------------------------------------------------------------
python3 - "${REGISTRY_ABS}" "${MAX_ENTRIES}" "${TMP_DIR}" <<'PYEOF'
import json
import re
import sys

registry_path, max_entries_raw, tmp_dir = sys.argv[1], sys.argv[2], sys.argv[3]
max_entries = int(max_entries_raw)

violations = []
stats = {"total": 0, "limit": max_entries}

DATE_RE = re.compile(r"^\d{4}-\d{2}-\d{2}$")

REQUIRED_STRING_FIELDS = ["case_id", "target", "query_id", "kind", "issue",
                          "owner", "since", "detail"]
REQUIRED_INT_FIELDS = ["expected_row_count", "actual_row_count", "oracle_row_count"]

# A difference is either about the number of rows or about the values inside
# the rows. The two are checked differently: a `row_count` difference whose
# two counts are equal has disappeared (rule 8), while a `cell_values`
# difference legitimately has equal counts on both sides — the difference is
# in the cells, not in the row count.
ALLOWED_KINDS = ("row_count", "cell_values")


def is_int(value):
    # bool is a subclass of int in Python — it must NOT pass as an integer.
    return isinstance(value, int) and not isinstance(value, bool)


try:
    with open(registry_path) as fh:
        raw = fh.read()
except OSError as exc:
    violations.append("registry file cannot be read: %s" % exc)
    raw = None

data = None
if raw is not None:
    try:
        data = json.loads(raw)
    except ValueError as exc:
        violations.append("registry is not valid JSON: %s" % exc)

if data is not None:
    if not isinstance(data, dict):
        violations.append("registry top level must be a JSON object, got %s"
                          % type(data).__name__)
        data = None

if isinstance(data, dict):
    for field in ("version", "updated_at", "entries"):
        if field not in data:
            violations.append("top-level field '%s' is missing" % field)
    if "version" in data and not isinstance(data["version"], int):
        violations.append("top-level field 'version' must be an integer, got %s"
                          % type(data["version"]).__name__)
    if "updated_at" in data and not isinstance(data["updated_at"], str):
        violations.append("top-level field 'updated_at' must be a string, got %s"
                          % type(data["updated_at"]).__name__)
    if "entries" in data and not isinstance(data["entries"], list):
        violations.append("top-level field 'entries' must be an array, got %s"
                          % type(data["entries"]).__name__)

entries = data.get("entries") if isinstance(data, dict) else None
if isinstance(entries, list):
    stats["total"] = len(entries)
    seen_case_ids = {}
    for index, entry in enumerate(entries):
        label = "entries[%d]" % index
        if not isinstance(entry, dict):
            violations.append("%s: entry must be a JSON object, got %s"
                              % (label, type(entry).__name__))
            continue
        case_id = entry.get("case_id")
        if isinstance(case_id, str) and case_id:
            label = "entries[%d] (%s)" % (index, case_id)
        for field in REQUIRED_STRING_FIELDS:
            if field not in entry:
                violations.append("%s: required field '%s' is missing" % (label, field))
            elif not isinstance(entry[field], str):
                violations.append("%s: field '%s' must be a string, got %s"
                                  % (label, field, type(entry[field]).__name__))
        for field in REQUIRED_INT_FIELDS:
            if field not in entry:
                violations.append("%s: required field '%s' is missing" % (label, field))
            elif not is_int(entry[field]):
                violations.append("%s: field '%s' must be an integer, got %s"
                                  % (label, field, type(entry[field]).__name__))
        # rule 3: no unowned difference
        if "issue" in entry and isinstance(entry["issue"], str) and not entry["issue"].strip():
            violations.append("%s: 'issue' must not be empty (no unowned differences)"
                              % label)
        # rule 4: no unowned difference
        if "owner" in entry and isinstance(entry["owner"], str) and not entry["owner"].strip():
            violations.append("%s: 'owner' must not be empty (no unowned differences)"
                              % label)
        # rule 4b: the difference must be described
        if "detail" in entry and isinstance(entry["detail"], str) and not entry["detail"].strip():
            violations.append("%s: 'detail' must not be empty (a difference that is not "
                              "described cannot be reviewed)" % label)
        # rule 4c: the target must be named ("*" means every target)
        if "target" in entry and isinstance(entry["target"], str) and not entry["target"].strip():
            violations.append("%s: 'target' must not be empty (use '*' for a difference "
                              "that applies to every target)" % label)
        # rule 4d: the kind must be one of the known kinds
        kind = entry.get("kind")
        if isinstance(kind, str) and kind not in ALLOWED_KINDS:
            violations.append("%s: 'kind' must be one of %s, got '%s'"
                              % (label, "/".join(ALLOWED_KINDS), kind))
        # rule 5: globally unique case_id
        if isinstance(case_id, str) and case_id:
            if case_id in seen_case_ids:
                violations.append("%s: duplicate case_id '%s' (first seen at entries[%d])"
                                  % (label, case_id, seen_case_ids[case_id]))
            else:
                seen_case_ids[case_id] = index
        # rule 7: since must be a YYYY-MM-DD date
        since = entry.get("since")
        if isinstance(since, str) and since and not DATE_RE.match(since):
            violations.append("%s: 'since' must be formatted YYYY-MM-DD, got '%s'"
                              % (label, since))
        # rule 8: a difference that no longer differs must be unregistered.
        # Only a `row_count` difference is judged this way: a
        # `cell_values` difference has the same number of rows on both sides
        # by definition, and its cells are what differ.
        actual = entry.get("actual_row_count")
        oracle = entry.get("oracle_row_count")
        entry_kind = entry.get("kind", "row_count")
        if (is_int(actual) and is_int(oracle) and actual == oracle
                and entry_kind == "row_count"):
            violations.append("%s: the difference has disappeared "
                              "(actual_row_count == oracle_row_count == %d); "
                              "remove this entry from the registry" % (label, actual))

    # rule 6: the registry may not grow silently
    if len(entries) > max_entries:
        violations.append("registry holds %d entries but the limit is %d "
                          "(known differences may not grow silently)"
                          % (len(entries), max_entries))

with open(tmp_dir + "/violations.txt", "w") as fh:
    for v in violations:
        fh.write(v + "\n")

with open(tmp_dir + "/stats.txt", "w") as fh:
    fh.write("%d %d %d\n" % (stats["total"], stats["limit"], len(violations)))
PYEOF

if [[ ! -f "${TMP_DIR}/stats.txt" ]]; then
    fail "registry validation failed to run"
    exit 2
fi

read -r TOTAL_ENTRIES LIMIT_ENTRIES VIOLATION_COUNT < "${TMP_DIR}/stats.txt"

echo ""
echo "  total entries:  ${TOTAL_ENTRIES}"
echo "  limit:          ${LIMIT_ENTRIES}"
echo "  violations:     ${VIOLATION_COUNT}"

if [[ "${VIOLATION_COUNT}" -gt 0 ]]; then
    echo ""
    echo "  violations:"
    while IFS= read -r violation; do
        [[ -z "${violation}" ]] && continue
        fail "${violation}"
    done < "${TMP_DIR}/violations.txt"
    echo ""
    echo "Known-differences registry FAIL: ${VIOLATION_COUNT} violation(s)."
    exit 1
fi

pass "registry is valid (${TOTAL_ENTRIES} entr(y/ies), limit ${LIMIT_ENTRIES})"
echo ""
echo "Known-differences registry PASS."
exit 0
