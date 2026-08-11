#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$REPO_ROOT"

usage() {
  cat <<'EOF'
Usage:
  scripts/gate/check_v312_coverage_baseline.sh [--check-config] [--enforce-stage alpha|beta|rc|ga]

Modes:
  default             Generate per-crate coverage baseline artifacts.
  --check-config      Fast governance/configuration check; no coverage run.
  --enforce-stage     Generate baseline and enforce stage-specific thresholds.

The canonical measurement command is:
  cargo llvm-cov -p <crate> --all-features --tests --ignore-run-fail --json --summary-only
EOF
}

CHECK_CONFIG_ONLY=0
ENFORCE_STAGE=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    --check-config)
      CHECK_CONFIG_ONLY=1
      ;;
    --enforce-stage)
      ENFORCE_STAGE="${2:-}"
      if [ -z "$ENFORCE_STAGE" ]; then
        echo "ERROR: --enforce-stage requires alpha|beta|rc|ga" >&2
        exit 2
      fi
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "ERROR: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

COMMIT="$(git rev-parse --short=10 HEAD)"
TIMESTAMP="$(date +%Y%m%d_%H%M%S)"
OUT_DIR="${OUT_DIR:-docs/releases/v3.12.0/coverage-baseline/current_${COMMIT}_${TIMESTAMP}}"
TIMEOUT_SECONDS="${COVERAGE_TIMEOUT_SECONDS:-120}"
FRAMEWORK_DOC="docs/releases/v3.12.0/COMPREHENSIVE_TEST_FRAMEWORK_AND_COVERAGE_BASELINE.md"

TRACKED_CRATES=(
  sqlrustgo-parser
  sqlrustgo-executor
  sqlrustgo-storage
  sqlrustgo-planner
  sqlrustgo-optimizer
  sqlrustgo-mysql-client
  sqlrustgo-mysql-server
  sqlrustgo-gmp
  sqlrustgo-rag
  sqlrustgo-vector
  sqlrustgo-catalog
  sqlrustgo-transaction
  sqlrustgo-tools
  sqlrustgo-admin
  sqlrustgo-server
  sqlrustgo-sql-corpus
)

check_config() {
  local missing=0

  if [ ! -f "$FRAMEWORK_DOC" ]; then
    echo "ERROR: coverage framework document missing: $FRAMEWORK_DOC" >&2
    missing=1
  fi

  if ! grep -q "cargo llvm-cov -p <crate> --all-features --tests --ignore-run-fail --json --summary-only" "$FRAMEWORK_DOC" 2>/dev/null; then
    echo "ERROR: framework doc does not declare the canonical coverage command" >&2
    missing=1
  fi

  if ! grep -q "L0.*单元测试" "$FRAMEWORK_DOC" 2>/dev/null || \
     ! grep -q "L1.*集成测试" "$FRAMEWORK_DOC" 2>/dev/null || \
     ! grep -q "L2.*E2E" "$FRAMEWORK_DOC" 2>/dev/null || \
     ! grep -q "L3.*per-crate 覆盖率" "$FRAMEWORK_DOC" 2>/dev/null || \
     ! grep -q "L4.*性能测试" "$FRAMEWORK_DOC" 2>/dev/null || \
     ! grep -q "L5.*SOAK" "$FRAMEWORK_DOC" 2>/dev/null; then
    echo "ERROR: framework doc does not describe the required L0-L5 test layers" >&2
    missing=1
  fi

  if [ ! -x "scripts/gate/check_v312_coverage_baseline.sh" ]; then
    echo "ERROR: coverage baseline script is missing or not executable" >&2
    missing=1
  fi

  if ! command -v cargo >/dev/null 2>&1; then
    echo "ERROR: cargo is not available in PATH" >&2
    missing=1
  fi

  if ! command -v cargo-llvm-cov >/dev/null 2>&1 && ! cargo llvm-cov --version >/dev/null 2>&1; then
    echo "ERROR: cargo-llvm-cov is not installed or not available in PATH" >&2
    missing=1
  fi

  if [ "$missing" -ne 0 ]; then
    exit 1
  fi

  printf 'coverage framework config: PASS\n'
  printf 'tracked crates: %s\n' "${#TRACKED_CRATES[@]}"
}

if [ "$CHECK_CONFIG_ONLY" -eq 1 ]; then
  check_config
  exit 0
fi

check_config
mkdir -p "$OUT_DIR/logs"

summary_json="$OUT_DIR/summary.json"
summary_md="$OUT_DIR/summary.md"

python3 - "$summary_json" <<'PY'
import json, sys
with open(sys.argv[1], "w", encoding="utf-8") as f:
    json.dump([], f)
PY

for crate in "${TRACKED_CRATES[@]}"; do
  json_path="$OUT_DIR/${crate}.json"
  log_path="$OUT_DIR/logs/${crate}.log"
  echo "=== coverage: $crate ==="
  set +e
  timeout "$TIMEOUT_SECONDS" cargo llvm-cov \
    -p "$crate" \
    --all-features \
    --tests \
    --ignore-run-fail \
    --json \
    --summary-only \
    --output-path "$json_path" \
    >"$log_path" 2>&1
  rc=$?
  set -e

  python3 - "$summary_json" "$crate" "$json_path" "$log_path" "$rc" "$TIMEOUT_SECONDS" <<'PY'
import json, os, sys
summary_path, crate, json_path, log_path, rc, timeout_s = sys.argv[1:]
rc = int(rc)
row = {
    "crate": crate,
    "status": "ok" if rc == 0 and os.path.exists(json_path) else "timeout_or_fail",
    "returncode": rc,
    "json": json_path if os.path.exists(json_path) else "",
    "log": log_path,
    "line_pct": None,
    "lines": "",
    "functions": "",
    "test_health": "unknown",
}
if os.path.exists(json_path):
    try:
        data = json.load(open(json_path, encoding="utf-8"))
        totals = data["data"][0].get("totals") or data["data"][0].get("summary")
        lines = totals.get("lines", {})
        funcs = totals.get("functions", {})
        row["line_pct"] = lines.get("percent") or lines.get("percent_covered")
        row["lines"] = f'{lines.get("covered", "?")}/{lines.get("count", "?")}'
        row["functions"] = f'{funcs.get("covered", "?")}/{funcs.get("count", "?")}'
    except Exception as exc:
        row["status"] = "json_parse_error"
        row["error"] = repr(exc)
try:
    log = open(log_path, encoding="utf-8", errors="replace").read()
    if "test result: FAILED" in log or "target failed" in log or "error: test failed" in log:
        row["test_health"] = "report-only-failure"
    elif rc == 124:
        row["test_health"] = "timeout"
    elif rc == 0:
        row["test_health"] = "pass-or-no-test-failure-detected"
    else:
        row["test_health"] = "command-failed"
except FileNotFoundError:
    row["test_health"] = "missing-log"
rows = json.load(open(summary_path, encoding="utf-8"))
rows.append(row)
with open(summary_path, "w", encoding="utf-8") as f:
    json.dump(rows, f, ensure_ascii=False, indent=2)
PY
done

python3 - "$summary_json" "$summary_md" "$COMMIT" "$TIMESTAMP" <<'PY'
import json, sys
summary_json, summary_md, commit, ts = sys.argv[1:]
rows = json.load(open(summary_json, encoding="utf-8"))
with open(summary_md, "w", encoding="utf-8") as f:
    f.write(f"# v3.12.0 per-crate coverage baseline\n\n")
    f.write(f"- commit: `{commit}`\n")
    f.write(f"- generated_at: `{ts}`\n")
    f.write("- command: `cargo llvm-cov -p <crate> --all-features --tests --ignore-run-fail --json --summary-only`\n\n")
    f.write("| Crate | Line% | Lines | Functions | Test health | JSON | Log |\n")
    f.write("|---|---:|---:|---:|---|---|---|\n")
    for r in rows:
        pct = "" if r["line_pct"] is None else f'{r["line_pct"]:.2f}%'
        f.write(f'| {r["crate"]} | {pct} | {r["lines"]} | {r["functions"]} | {r["test_health"]} | `{r["json"]}` | `{r["log"]}` |\n')
PY

if [ -n "$ENFORCE_STAGE" ]; then
  python3 - "$summary_json" "$ENFORCE_STAGE" <<'PY'
import json, sys

summary_json, stage = sys.argv[1:]
rows = json.load(open(summary_json, encoding="utf-8"))

thresholds = {
    "alpha": {
        "default": 0.0,
        "required_health": [],
    },
    "beta": {
        "default": 70.0,
        "overrides": {"sqlrustgo-gmp": 78.0},
        "required_health": [],
    },
    "rc": {
        "default": 80.0,
        "overrides": {
            "sqlrustgo-mysql-client": 78.0,
            "sqlrustgo-mysql-server": 72.0,
        },
        "required_health": [
            "sqlrustgo-parser",
            "sqlrustgo-executor",
            "sqlrustgo-storage",
            "sqlrustgo-planner",
            "sqlrustgo-optimizer",
            "sqlrustgo-mysql-client",
            "sqlrustgo-mysql-server",
            "sqlrustgo-gmp",
            "sqlrustgo-rag",
            "sqlrustgo-vector",
            "sqlrustgo-catalog",
            "sqlrustgo-transaction",
        ],
    },
    "ga": {
        "default": 80.0,
        "overrides": {},
        "required_health": [
            "sqlrustgo-parser",
            "sqlrustgo-executor",
            "sqlrustgo-storage",
            "sqlrustgo-planner",
            "sqlrustgo-optimizer",
            "sqlrustgo-mysql-client",
            "sqlrustgo-mysql-server",
            "sqlrustgo-gmp",
            "sqlrustgo-rag",
            "sqlrustgo-vector",
            "sqlrustgo-catalog",
            "sqlrustgo-transaction",
        ],
    },
}

if stage not in thresholds:
    print(f"ERROR: unknown enforce stage: {stage}", file=sys.stderr)
    sys.exit(2)

policy = thresholds[stage]
failed = []
for row in rows:
    crate = row["crate"]
    pct = row.get("line_pct")
    threshold = policy.get("overrides", {}).get(crate, policy["default"])
    if pct is None:
        failed.append(f"{crate}: missing coverage percentage")
    elif pct < threshold:
        failed.append(f"{crate}: {pct:.2f}% < {threshold:.2f}%")
    if crate in policy["required_health"] and row.get("test_health") != "pass-or-no-test-failure-detected":
        failed.append(f"{crate}: unhealthy coverage run ({row.get('test_health')})")

if failed:
    print("coverage enforcement: FAIL")
    for item in failed:
        print(f"- {item}")
    sys.exit(1)

print(f"coverage enforcement: PASS ({stage})")
PY
fi

echo "summary: $summary_md"
echo "json: $summary_json"
