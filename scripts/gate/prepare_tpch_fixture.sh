#!/usr/bin/env bash
#
# scripts/gate/prepare_tpch_fixture.sh
#
# Runtime-generated TPC-H fixture (hermetic test input) for issue #5379.
#
# The TPC-H .tbl files are NEVER committed to the repository (AGENTS.md /
# .gitignore: "*.tbl" is a forbidden commit extension). This script is the
# single entry point used by both CI and local runs to materialize the
# fixture on demand, so that a test can never depend on a stale, dangling
# or hand-edited data directory.
#
# It delegates the actual data generation to the repository's existing
# deterministic generator (scripts/gate/generate_tpch_sf.py), which seeds
# `random` explicitly and therefore produces byte-identical output for a
# given (sf, seed) pair.
#
# Usage:
#   bash scripts/gate/prepare_tpch_fixture.sh --out <DIR> [--sf 0.01] [--seed 42]
#
# Options:
#   --out <DIR>    Directory to receive the 8 .tbl files + the manifest.
#                  (required)
#   --sf <FLOAT>   TPC-H scale factor. Default: 0.01
#   --seed <INT>   RNG seed passed to the generator. Default: 42
#   -h, --help     Show this help.
#
# Output files (in <DIR>):
#   region.tbl nation.tbl supplier.tbl customer.tbl
#   part.tbl partsupp.tbl orders.tbl lineitem.tbl
#   tpch_fixture_manifest.json
#
# The manifest records the generator, a SHA-256 fingerprint of the generator
# file, the seed, the scale factor, the generation timestamp and the ACTUAL
# row count of every table (measured with `wc -l` on the generated files —
# never copied from the generator's theoretical numbers).
#
# Idempotency:
#   Re-running with the same (--out, --sf, --seed) **and the same generator
#   file** prints "[SKIP] fixture already present" and exits 0 without
#   regenerating. A different (--sf, --seed), a changed generator, or a
#   missing/!complete .tbl set triggers a regeneration.
#
# Exit codes:
#   0  fixture is present (either freshly generated or already present)
#   1  usage error, missing generator, or generation failure
#
# Requires: bash >= 3.2 (macOS default), python3 (the generator is Python).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

GENERATOR="${REPO_ROOT}/scripts/gate/generate_tpch_sf.py"
MANIFEST_NAME="tpch_fixture_manifest.json"

# The 8 TPC-H tables, in the canonical order used by the manifest.
TABLES="region nation supplier customer part partsupp orders lineitem"

OUT_DIR=""
SF="0.01"
SEED="42"

TMP_DIR=""

err() { printf 'ERROR: %s\n' "$*" >&2; }
info() { printf '%s\n' "$*"; }

usage() {
    cat <<'USAGE'
Usage: bash scripts/gate/prepare_tpch_fixture.sh --out <DIR> [--sf 0.01] [--seed 42]

  --out <DIR>    target directory for the 8 .tbl files + manifest (required)
  --sf <FLOAT>   TPC-H scale factor (default: 0.01)
  --seed <INT>   RNG seed for the generator (default: 42)
  -h, --help     this message
USAGE
}

cleanup() {
    if [[ -n "${TMP_DIR}" && -d "${TMP_DIR}" ]]; then
        rm -rf -- "${TMP_DIR}"
    fi
}

need_value() {
    # $1 = flag name, $2 = number of remaining args
    if [[ "$2" -lt 2 ]]; then
        err "option $1 requires a value"
        usage >&2
        exit 1
    fi
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --out)   need_value "$1" $#; OUT_DIR="$2"; shift 2 ;;
        --out=*) OUT_DIR="${1#--out=}"; shift ;;
        --sf)    need_value "$1" $#; SF="$2"; shift 2 ;;
        --sf=*)  SF="${1#--sf=}"; shift ;;
        --seed)  need_value "$1" $#; SEED="$2"; shift 2 ;;
        --seed=*) SEED="${1#--seed=}"; shift ;;
        -h|--help) usage; exit 0 ;;
        *) err "unknown argument: $1"; usage >&2; exit 1 ;;
    esac
done

# ---------------------------------------------------------------------------
# Argument validation
# ---------------------------------------------------------------------------
if [[ -z "${OUT_DIR}" ]]; then
    err "--out <DIR> is required"
    usage >&2
    exit 1
fi

# Plain decimal only: no exponent, no sign. Keeps the manifest's "sf" field
# canonical so that the idempotency comparison is a plain string compare.
if ! [[ "${SF}" =~ ^[0-9]+([.][0-9]+)?$ ]]; then
    err "invalid --sf '${SF}': expected a positive decimal number (e.g. 0.01)"
    exit 1
fi
if ! [[ "${SEED}" =~ ^[0-9]+$ ]]; then
    err "invalid --seed '${SEED}': expected a non-negative integer"
    exit 1
fi

# Canonical scale factor: always contain a '.' so the manifest stores a
# float-looking number. "1" -> "1.0", "0.010" -> "0.010" (the generator
# parses it as a float either way).
case "${SF}" in
    *.*) SF_CANON="${SF}" ;;
    *)   SF_CANON="${SF}.0" ;;
esac

if [[ "${SF_CANON%%.*}" =~ ^0+$ && "${SF_CANON#*.}" =~ ^0+$ ]]; then
    err "invalid --sf '${SF}': scale factor must be greater than zero"
    exit 1
fi

# ---------------------------------------------------------------------------
# Preflight
# ---------------------------------------------------------------------------
if ! command -v python3 >/dev/null 2>&1; then
    err "python3 not on PATH (required to run ${GENERATOR})"
    exit 1
fi
if [[ ! -f "${GENERATOR}" ]]; then
    err "generator not found: ${GENERATOR}"
    exit 1
fi

MANIFEST_PATH="${OUT_DIR}/${MANIFEST_NAME}"

# ---------------------------------------------------------------------------
# Generator fingerprint
# ---------------------------------------------------------------------------
# The idempotency check below compares (sf, seed) and the presence of the
# eight .tbl files. That is not enough: the *generator itself* can change
# while (sf, seed) stay the same, and then the directory holds a fixture that
# the current generator would never produce — the script would print
# "[SKIP] fixture already present" and every test would silently run against
# stale data. (This is not hypothetical: the fixture's text columns were
# reworked in issue #5379 while sf and seed stayed at 0.01 / 42.)
#
# The manifest therefore records a SHA-256 of the generator file, and a
# directory is only reused when that fingerprint still matches.
# ---------------------------------------------------------------------------
generator_sha256() {
    if command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | awk '{print $1}'
    elif command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    else
        echo "UNAVAILABLE"
    fi
}

GEN_SHA256="$(generator_sha256 "${GENERATOR}")"

# ---------------------------------------------------------------------------
# Idempotency check
# ---------------------------------------------------------------------------
if [[ -f "${MANIFEST_PATH}" ]]; then
    MANIFEST_STATE=$(python3 - "${MANIFEST_PATH}" "${SF_CANON}" "${SEED}" "${GEN_SHA256}" <<'PYEOF'
import json
import os
import sys

manifest_path, want_sf, want_seed, want_gen = sys.argv[1:5]
tables = ["region", "nation", "supplier", "customer",
          "part", "partsupp", "orders", "lineitem"]

try:
    with open(manifest_path) as fh:
        data = json.load(fh)
except Exception as exc:
    print("UNREADABLE\t0\t%s" % exc)
    sys.exit(0)

if not isinstance(data, dict):
    print("UNREADABLE\t0\tmanifest is not a JSON object")
    sys.exit(0)

have_sf = data.get("sf")
have_seed = data.get("seed")
have_gen = data.get("generator_sha256")

try:
    sf_match = str(float(have_sf)) == str(float(want_sf))
except (TypeError, ValueError):
    sf_match = False
try:
    seed_match = int(have_seed) == int(want_seed)
except (TypeError, ValueError):
    seed_match = False

gen_match = (want_gen == "UNAVAILABLE") or (have_gen == want_gen)

out_dir = os.path.dirname(os.path.abspath(manifest_path))
missing_tbl = [t for t in tables
               if not os.path.isfile(os.path.join(out_dir, t + ".tbl"))]

if sf_match and seed_match and gen_match and not missing_tbl:
    print("MATCH\t0\t")
else:
    reasons = []
    if not sf_match:
        reasons.append("sf differs (manifest=%r requested=%r)" % (have_sf, want_sf))
    if not seed_match:
        reasons.append("seed differs (manifest=%r requested=%r)" % (have_seed, want_seed))
    if not gen_match:
        reasons.append("generator changed (manifest=%s current=%s)"
                       % (str(have_gen)[:12], want_gen[:12]))
    if missing_tbl:
        reasons.append("missing tables: %s" % ",".join(missing_tbl))
    print("DIFF\t%d\t%s" % (len(reasons), "; ".join(reasons)))
PYEOF
    )
    MANIFEST_KIND="${MANIFEST_STATE%%	*}"
    MANIFEST_REASON="${MANIFEST_STATE#*	}"
    MANIFEST_REASON="${MANIFEST_REASON#*	}"
    if [[ "${MANIFEST_KIND}" == "MATCH" ]]; then
        info "[SKIP] fixture already present"
        info "  dir:      ${OUT_DIR}"
        info "  sf:       ${SF_CANON}"
        info "  seed:     ${SEED}"
        info "  manifest: ${MANIFEST_PATH}"
        exit 0
    fi
    info "[REGEN] regenerating fixture: ${MANIFEST_REASON}"
fi

# ---------------------------------------------------------------------------
# Generate into a temporary directory first, then publish.
# Publishing only after a complete, verified generation keeps a failed run
# from leaving a half-written fixture behind.
# ---------------------------------------------------------------------------
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/tpch-fixture.XXXXXX")"
trap cleanup EXIT

info "[GEN] sf=${SF_CANON} seed=${SEED} tmp=${TMP_DIR}"

if ! python3 "${GENERATOR}" --sf "${SF_CANON}" --output "${TMP_DIR}" --seed "${SEED}"; then
    err "generator failed: ${GENERATOR} --sf ${SF_CANON} --seed ${SEED}"
    exit 1
fi

for table in ${TABLES}; do
    if [[ ! -f "${TMP_DIR}/${table}.tbl" ]]; then
        err "generator did not produce ${table}.tbl in ${TMP_DIR}"
        exit 1
    fi
done

# ---------------------------------------------------------------------------
# Publish into the target directory
# ---------------------------------------------------------------------------
if ! mkdir -p "${OUT_DIR}"; then
    err "cannot create output directory: ${OUT_DIR}"
    exit 1
fi
OUT_DIR="$(cd "${OUT_DIR}" && pwd)"

for table in ${TABLES}; do
    if ! mv -f "${TMP_DIR}/${table}.tbl" "${OUT_DIR}/${table}.tbl"; then
        err "cannot publish ${table}.tbl into ${OUT_DIR}"
        exit 1
    fi
done

# Actual row counts (wc -l), not the generator's theoretical numbers.
COUNT_PAIRS=""
ROW_TOTAL=0
for table in ${TABLES}; do
    rows=$(wc -l < "${OUT_DIR}/${table}.tbl" | tr -d '[:space:]')
    if ! [[ "${rows}" =~ ^[0-9]+$ ]]; then
        err "cannot count rows of ${OUT_DIR}/${table}.tbl"
        exit 1
    fi
    ROW_TOTAL=$((ROW_TOTAL + rows))
    if [[ -z "${COUNT_PAIRS}" ]]; then
        COUNT_PAIRS="\"${table}\": ${rows}"
    else
        COUNT_PAIRS="${COUNT_PAIRS}, \"${table}\": ${rows}"
    fi
done

GENERATED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

FIXTURE_OUT_DIR="${OUT_DIR}" \
FIXTURE_SF="${SF_CANON}" \
FIXTURE_SEED="${SEED}" \
FIXTURE_GENERATED_AT="${GENERATED_AT}" \
FIXTURE_GEN_SHA256="${GEN_SHA256}" \
FIXTURE_COUNTS="{${COUNT_PAIRS}}" \
python3 - <<'PYEOF'
import json
import os

tables = ["region", "nation", "supplier", "customer",
          "part", "partsupp", "orders", "lineitem"]
counts = json.loads(os.environ["FIXTURE_COUNTS"])

data = {
    "generator": "scripts/gate/generate_tpch_sf.py",
    "generator_sha256": os.environ["FIXTURE_GEN_SHA256"],
    "seed": int(os.environ["FIXTURE_SEED"]),
    "sf": float(os.environ["FIXTURE_SF"]),
    "generated_at": os.environ["FIXTURE_GENERATED_AT"],
    "row_counts": {t: int(counts[t]) for t in tables},
}

manifest_path = os.path.join(os.environ["FIXTURE_OUT_DIR"], "tpch_fixture_manifest.json")
with open(manifest_path, "w") as fh:
    json.dump(data, fh, indent=2, ensure_ascii=False)
    fh.write("\n")
PYEOF

info "[OK] fixture generated: ${OUT_DIR}"
info "  tables:   8"
info "  rows:     ${ROW_TOTAL} total"
info "  manifest: ${OUT_DIR}/${MANIFEST_NAME}"
