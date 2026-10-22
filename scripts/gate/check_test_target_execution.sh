#!/usr/bin/env bash
#
# scripts/gate/check_test_target_execution.sh
#
# CI execution-completeness gate for issue #5379.
#
# The problem this gate closes: `cargo test` is fail-fast by default. When
# the first test target fails, cargo stops and the remaining declared test
# targets never run — but the CI summary only counts `test result:` lines
# (i.e. test *binaries*), so a run that executed 64 of 446 declared targets
# still reported "Tests: passed=70 failed=1" and looked green.
#
# This gate takes the cargo test log(s) and the Cargo.toml and asserts the
# hard truth: EVERY `[[test]]` target declared in Cargo.toml must have
# actually been executed. Anything that did not run is a FAIL — there is no
# "skip" branch that turns a missing target into a pass.
#
# Usage:
#   bash scripts/gate/check_test_target_execution.sh \
#       --cargo-toml <path> --log <cargo-test-log> [--log <log2> ...] [--json <out>]
#
# Options:
#   --cargo-toml <path>  Cargo.toml to read the `[[test]]` declarations from
#                        (required)
#   --log <path>         cargo test log to parse. May be given more than
#                        once; all logs are merged into one executed set.
#   --json <path>        Optional: write a machine-readable result there.
#   -h, --help           Show this help.
#
# Log parsing:
#   cargo prints one line per executed test binary:
#       Running tests/foo.rs (target/debug/deps/foo-hash)
#   The ANSI colour codes sit BETWEEN "Running" and the path, e.g.
#       ESC[1m ESC[92m     Running ESC[0m tests/foo.rs (target/...)
#   so the log is first stripped of ANSI sequences (which preserves the line
#   numbering) and only then matched. Matching the raw log would silently
#   find nothing and the gate would wrongly report "0 executed".
#
#   `Running unittests src/lib.rs` / `Running unittests src/main.rs` are the
#   lib/bin unit-test binaries. They are NOT declared as `[[test]]` targets,
#   so they are not "missing" — they are reported separately as
#   extra_binaries.
#
# Exit codes:
#   0  PASS — every declared test target was executed
#   1  FAIL — at least one declared test target was never executed
#   2  usage error / a required input file is missing
#
# Requires: bash >= 3.2 (macOS default), python3, sed, grep.

set -uo pipefail

CARGO_TOML=""
LOGS=()
JSON_OUT=""

color_red()   { printf '\033[0;31m%s\033[0m' "$*"; }
color_green() { printf '\033[0;32m%s\033[0m' "$*"; }
color_yellow() { printf '\033[0;33m%s\033[0m' "$*"; }

step() { echo ""; echo "=== $* ==="; }
pass() { echo "  $(color_green PASS): $*"; }
fail() { echo "  $(color_red FAIL): $*"; }
warn() { echo "  $(color_yellow WARN): $*"; }

usage() {
    cat <<'USAGE'
Usage: bash scripts/gate/check_test_target_execution.sh \
         --cargo-toml <path> --log <cargo-test-log> [--log <log> ...] [--json <out>]

  --cargo-toml <path>  Cargo.toml holding the [[test]] declarations (required)
  --log <path>         cargo test log to parse (repeatable)
  --json <path>        optional machine-readable result file
  -h, --help           this message
USAGE
}

usage_error() {
    printf 'ERROR: %s\n' "$*" >&2
    usage >&2
    exit 2
}

need_value() {
    # $1 = flag name, $2 = number of remaining args
    if [[ "$2" -lt 2 ]]; then
        usage_error "option $1 requires a value"
    fi
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --cargo-toml)   need_value "$1" $#; CARGO_TOML="$2"; shift 2 ;;
        --cargo-toml=*) CARGO_TOML="${1#--cargo-toml=}"; shift ;;
        --log)          need_value "$1" $#; LOGS+=("$2"); shift 2 ;;
        --log=*)        LOGS+=("${1#--log=}"); shift ;;
        --json)         need_value "$1" $#; JSON_OUT="$2"; shift 2 ;;
        --json=*)       JSON_OUT="${1#--json=}"; shift ;;
        -h|--help)      usage; exit 0 ;;
        *)              usage_error "unknown argument: $1" ;;
    esac
done

# ---------------------------------------------------------------------------
# Preflight
# ---------------------------------------------------------------------------
if [[ -z "${CARGO_TOML}" ]]; then
    usage_error "--cargo-toml <path> is required"
fi
if [[ ! -f "${CARGO_TOML}" ]]; then
    usage_error "Cargo.toml not found: ${CARGO_TOML}"
fi
if [[ "${#LOGS[@]}" -eq 0 ]]; then
    usage_error "at least one --log <cargo-test-log> is required"
fi
for log_file in "${LOGS[@]}"; do
    if [[ ! -f "${log_file}" ]]; then
        usage_error "log file not found: ${log_file}"
    fi
done
if ! command -v python3 >/dev/null 2>&1; then
    usage_error "python3 not on PATH"
fi

CARGO_TOML_ABS="$(cd "$(dirname "${CARGO_TOML}")" && pwd)/$(basename "${CARGO_TOML}")"
PACKAGE_ROOT="$(cd "$(dirname "${CARGO_TOML}")" && pwd)"

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/test-target-exec.XXXXXX")"
trap 'rm -rf -- "${TMP_DIR}"' EXIT

# ---------------------------------------------------------------------------
# 1. Declared targets — every test target in the **whole workspace**
# ---------------------------------------------------------------------------
step "Step 1/3: collect declared test targets (cargo metadata)"

# #5515: this step used to hand-scan the [[test]] sections of ONE
# Cargo.toml — the root manifest. That made the gate structurally blind to
# the workspace's member packages: `cargo metadata` reports **773** test
# targets across 26 packages, of which the root manifest's [[test]]
# sections name **472**. The other **301** (`crates/*`) were invisible to
# the gate, so it kept reporting PASS while a third of the test suite had
# never been executed by anything.
#
# The single source of truth for "which test targets exist" is cargo
# itself. `cargo metadata --no-deps` enumerates every target of every
# workspace member, including the ones cargo **auto-discovers** (a file
# under `tests/` with no explicit [[test]] entry) — a hand-rolled scan of
# [[test]] sections cannot see those at all.
#
# The path cargo reports is the authoritative one (it is the path the test
# binary is compiled from), so the comparison against the `Running ...`
# lines in the log becomes an exact match instead of a guess.
#
# `cargo metadata` is used rather than `tomllib` because the CI images do
# not guarantee python >= 3.11, and because metadata is the only answer
# that includes auto-discovered targets.

METADATA_FILE="${TMP_DIR}/metadata.json"
if ! cargo metadata --no-deps --format-version 1 > "${METADATA_FILE}" 2>/dev/null; then
    fail "cargo metadata failed — cannot establish which test targets exist"
    exit 2
fi

python3 - "${METADATA_FILE}" > "${TMP_DIR}/declared.tsv" <<'PYEOF'
import json
import os
import sys

with open(sys.argv[1]) as fh:
    meta = json.load(fh)

workspace_root = meta.get("workspace_root") or os.getcwd()
members = set(meta.get("workspace_members") or [])

declared = []
for pkg in meta.get("packages") or []:
    # Only workspace members count: a dependency's test targets are not
    # this repository's test suite.
    if pkg.get("id") not in members:
        continue
    pkg_dir = pkg.get("manifest_path") or ""
    pkg_dir = os.path.dirname(pkg_dir)
    for target in pkg.get("targets") or []:
        kinds = target.get("kind") or []
        if "test" not in kinds:
            continue
        src = target.get("src_path") or ""
        if os.path.isabs(src):
            rel = os.path.relpath(src, workspace_root)
        else:
            rel = src
        rel = rel.replace(os.sep, "/")
        declared.append((target.get("name") or "", rel))

declared.sort()
for name, path in declared:
    print("%s\t%s" % (name, path))
PYEOF

DECLARED_RAW_COUNT=$(wc -l < "${TMP_DIR}/declared.tsv" | tr -d '[:space:]')
if [[ "${DECLARED_RAW_COUNT}" -eq 0 ]]; then
    fail "cargo metadata reported no test targets — cannot verify execution"
    exit 2
fi
pass "declared test targets (whole workspace): ${DECLARED_RAW_COUNT}"

# Report the per-package breakdown, because a gate that only prints a
# total can hide a blind spot behind a healthy-looking number.
python3 - "${TMP_DIR}/declared.tsv" <<'PYEOF'
import collections
import sys

counts = collections.Counter()
with open(sys.argv[1]) as fh:
    for line in fh:
        line = line.rstrip("\n")
        if not line:
            continue
        _name, path = line.split("\t", 1)
        # crates/<pkg>/... -> the package directory
        parts = path.split("/")
        if len(parts) >= 2 and parts[0] in ("crates", "tools"):
            counts["%s/%s" % (parts[0], parts[1])] += 1
        else:
            counts["<root package>"] += 1

for key in sorted(counts):
    print("    %-40s %d" % (key, counts[key]))
PYEOF

# ---------------------------------------------------------------------------
# 2. Executed targets — the `Running <path>` lines of the cargo test log
# ---------------------------------------------------------------------------
step "Step 2/3: parse cargo test log(s)"

# ESC is built with printf because the ANSI escape must survive both BSD sed
# (macOS) and GNU sed; the \x1b escape is not portable in BSD regexes.
ESC=$(printf '\033')

log_index=0
for log_file in "${LOGS[@]}"; do
    log_index=$((log_index + 1))
    # Strip ANSI first (one line in -> one line out, so `grep -n` below still
    # reports the ORIGINAL line numbers of the log), then select the lines
    # that mention "Running". Matching the raw log would miss every line,
    # because the colour codes sit between "Running" and the path.
    sed "s/${ESC}\[[0-9;]*m//g" "${log_file}" \
        | grep -nE 'Running[[:space:]]+' \
        > "${TMP_DIR}/running_${log_index}.txt" || true
    hits=$(wc -l < "${TMP_DIR}/running_${log_index}.txt" | tr -d '[:space:]')
    echo "  log: ${log_file} (${hits} 'Running' line(s))"
done

# ---------------------------------------------------------------------------
# 3. Compare and report
# ---------------------------------------------------------------------------
step "Step 3/3: compare declared vs executed"

python3 - "${TMP_DIR}" "${PACKAGE_ROOT}" "${CARGO_TOML_ABS}" "${JSON_OUT}" <<'PYEOF'
import json
import os
import re
import sys

tmp_dir, package_root, cargo_toml, json_out = sys.argv[1:5]

# #5515: the old pattern required the binary path to start with `target/`,
# i.e. it only matched cargo's **relative** output form:
#
#     Running tests/foo.rs (target/debug/deps/foo-abc123)
#
# Current cargo prints an **absolute** path:
#
#     Running tests/foo.rs (/abs/workspace/target/debug/deps/foo-abc123)
#
# so the pattern matched **nothing** and the gate reported every declared
# target as "never executed" — measured on a full regression log that ran
# 472/472 root-package targets: the gate said "472 of 472 declared test
# targets never ran".
#
# The target path is what follows `Running ` up to the opening parenthesis;
# the binary path inside the parentheses is irrelevant to the comparison.
RUNNING_RE = re.compile(r'Running\s+(.*?)\s+\(')


def norm(path):
    """Canonical form of a target path for comparison."""
    p = path.strip()
    while p.startswith("./"):
        p = p[2:]
    p = re.sub(r'/+', '/', p)
    if p.endswith("/"):
        p = p[:-1]
    if os.path.isabs(p):
        root = package_root.rstrip("/") + "/"
        if p.startswith(root):
            p = p[len(root):]
    return p


# --- declared -------------------------------------------------------------
declared = []
with open(os.path.join(tmp_dir, "declared.tsv")) as fh:
    for line in fh:
        line = line.rstrip("\n")
        if not line:
            continue
        name, path = line.split("\t", 1)
        declared.append((name, path))

declared_by_path = {}
for name, path in declared:
    declared_by_path.setdefault(norm(path), {"name": name, "path": path})

# --- executed -------------------------------------------------------------
executed = {}          # normalized path -> {"path", "log", "line"}
binaries = []          # lib/bin unit-test binaries
running_lines = 0

for fname in sorted(os.listdir(tmp_dir)):
    if not fname.startswith("running_") or not fname.endswith(".txt"):
        continue
    log_index = int(fname[len("running_"):-len(".txt")])
    with open(os.path.join(tmp_dir, fname), errors="replace") as fh:
        for raw in fh:
            raw = raw.rstrip("\n")
            m = re.match(r"^(\d+):(.*)$", raw)
            if not m:
                continue
            line_no = int(m.group(1))
            content = m.group(2)
            mr = RUNNING_RE.search(content)
            if not mr:
                continue
            running_lines += 1
            target = mr.group(1).strip()
            if target.startswith("unittests "):
                # lib/bin unit-test binary: not a [[test]] target.
                binaries.append({"target": target, "log_index": log_index,
                                 "line": line_no})
                continue
            p = norm(target)
            if p not in executed:
                executed[p] = {"path": p, "log_index": log_index, "line": line_no}

# --- diff -----------------------------------------------------------------
missing = []
for key in sorted(declared_by_path):
    if key not in executed:
        missing.append(declared_by_path[key])

undeclared = []
for key in sorted(executed):
    if key not in declared_by_path:
        undeclared.append(executed[key])

extra_binaries = []
seen_bin = set()
for b in binaries:
    if b["target"] in seen_bin:
        continue
    seen_bin.add(b["target"])
    extra_binaries.append(b)

status = "FAIL" if missing else "PASS"

summary = {
    "status": status,
    "cargo_toml": cargo_toml,
    "declared_count": len(declared),
    "executed_count": len(executed),
    "running_lines": running_lines,
    "extra_binaries_count": len(extra_binaries),
    "extra_binaries": [b["target"] for b in extra_binaries],
    "missing_count": len(missing),
    "missing": missing,
    "executed_but_undeclared_count": len(undeclared),
    "executed_but_undeclared": undeclared,
}

with open(os.path.join(tmp_dir, "summary.json"), "w") as fh:
    json.dump(summary, fh, indent=2, ensure_ascii=False)
    fh.write("\n")

with open(os.path.join(tmp_dir, "missing.tsv"), "w") as fh:
    for entry in missing:
        fh.write("%s\t%s\n" % (entry["name"], entry["path"]))

with open(os.path.join(tmp_dir, "undeclared.tsv"), "w") as fh:
    for entry in undeclared:
        fh.write("%s\n" % entry["path"])

with open(os.path.join(tmp_dir, "counts.txt"), "w") as fh:
    fh.write("%d %d %d %d %d\n" % (
        len(declared), len(executed), len(extra_binaries), len(missing),
        len(undeclared)))

if json_out:
    with open(json_out, "w") as fh:
        json.dump(summary, fh, indent=2, ensure_ascii=False)
        fh.write("\n")
PYEOF

if [[ ! -f "${TMP_DIR}/counts.txt" ]]; then
    fail "analysis failed"
    exit 2
fi

read -r DECLARED_TOTAL EXECUTED_TOTAL EXTRA_BINARIES MISSING_COUNT UNDECLARED_COUNT < "${TMP_DIR}/counts.txt"

echo ""
echo "  declared test targets:   ${DECLARED_TOTAL}"
echo "  executed test targets:   ${EXECUTED_TOTAL}"
echo "  extra lib/bin binaries:  ${EXTRA_BINARIES}"
echo "  missing test targets:    ${MISSING_COUNT}"
echo "  executed but undeclared: ${UNDECLARED_COUNT}"

if [[ "${EXTRA_BINARIES}" -gt 0 ]]; then
    echo ""
    echo "  lib/bin unit-test binaries (not [[test]] targets, not counted as missing):"
    python3 - "${TMP_DIR}/summary.json" <<'PYEOF'
import json
import sys
with open(sys.argv[1]) as fh:
    data = json.load(fh)
for target in data.get("extra_binaries", []):
    print("    %s" % target)
PYEOF
fi

if [[ "${MISSING_COUNT}" -gt 0 ]]; then
    echo ""
    echo "  missing test targets (declared in Cargo.toml but never executed):"
    while IFS=$'\t' read -r miss_name miss_path; do
        [[ -z "${miss_path}" ]] && continue
        fail "target '${miss_name}' was declared but never executed (path: ${miss_path})"
    done < "${TMP_DIR}/missing.tsv"
fi

if [[ "${UNDECLARED_COUNT}" -gt 0 ]]; then
    echo ""
    warn "${UNDECLARED_COUNT} executed target(s) are not declared as [[test]] in Cargo.toml:"
    while IFS= read -r extra_path; do
        [[ -z "${extra_path}" ]] && continue
        echo "    ${extra_path}"
    done < "${TMP_DIR}/undeclared.tsv"
fi

echo ""
if [[ "${MISSING_COUNT}" -eq 0 ]]; then
    echo "Test target execution gate PASS: all ${DECLARED_TOTAL} declared test targets were executed."
    exit 0
fi

echo "Test target execution gate FAIL: ${MISSING_COUNT} of ${DECLARED_TOTAL} declared test targets never ran."
if [[ -n "${JSON_OUT}" ]]; then
    echo "  machine-readable result: ${JSON_OUT}"
fi
exit 1
