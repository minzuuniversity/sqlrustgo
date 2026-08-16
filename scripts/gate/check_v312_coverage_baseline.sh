#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$REPO_ROOT"

DOC_PATH="docs/releases/v3.12.0/COMPREHENSIVE_TEST_FRAMEWORK_AND_COVERAGE_BASELINE.md"
BASELINE_DIR="docs/releases/v3.12.0/coverage-baseline"
COMMIT="$(git rev-parse --short=10 HEAD)"
TIMESTAMP="$(date +%Y%m%d_%H%M%S)"
OUT_DIR="${OUT_DIR:-${BASELINE_DIR}/current_${COMMIT}_${TIMESTAMP}}"
TIMEOUT_SECONDS="${COVERAGE_TIMEOUT_SECONDS:-120}"
MODE="run"
ENFORCE_STAGE=""
SUMMARY_OVERRIDE=""

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

usage() {
  cat <<'EOF'
Usage:
  bash scripts/gate/check_v312_coverage_baseline.sh
  bash scripts/gate/check_v312_coverage_baseline.sh --check-config
  bash scripts/gate/check_v312_coverage_baseline.sh --enforce-stage <alpha|beta|rc|ga> [--summary <summary.json>]

Notes:
  --check-config is the lightweight Alpha/RC smoke path. It validates that the
  v3.12.0 L0-L5 framework, canonical command, tracked crate set, and saved
  baseline shape are configured; it does not run coverage.

  --enforce-stage without --summary runs the per-crate coverage collection first,
  then applies stage thresholds. Use --summary for reviewing an existing saved
  baseline.
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --check-config)
      MODE="check-config"
      shift
      ;;
    --enforce-stage)
      MODE="enforce"
      ENFORCE_STAGE="${2:-}"
      if [ -z "$ENFORCE_STAGE" ]; then
        echo "ERROR: --enforce-stage requires alpha|beta|rc|ga" >&2
        exit 2
      fi
      shift 2
      ;;
    --summary)
      SUMMARY_OVERRIDE="${2:-}"
      if [ -z "$SUMMARY_OVERRIDE" ]; then
        echo "ERROR: --summary requires a path" >&2
        exit 2
      fi
      shift 2
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
done

check_config() {
  local failures=0

  echo "=== v3.12.0 Coverage Framework Config Check ==="
  echo "Branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown) @ $(git rev-parse --short HEAD 2>/dev/null || echo unknown)"

  if [ -f "$DOC_PATH" ]; then
    echo "[PASS] framework doc exists: $DOC_PATH"
  else
    echo "[FAIL] framework doc missing: $DOC_PATH"
    failures=$((failures + 1))
  fi

  if [ -f "$DOC_PATH" ] && grep -q 'cargo llvm-cov -p <crate> --all-features --tests --ignore-run-fail --json --summary-only' "$DOC_PATH"; then
    echo "[PASS] canonical coverage command documented"
  else
    echo "[FAIL] canonical coverage command missing from framework doc"
    failures=$((failures + 1))
  fi

  if [ -f "$DOC_PATH" ] && grep -q 'L0' "$DOC_PATH" && grep -q 'L5' "$DOC_PATH"; then
    echo "[PASS] L0-L5 test layering documented"
  else
    echo "[FAIL] L0-L5 test layering missing from framework doc"
    failures=$((failures + 1))
  fi

  if [ -f "$DOC_PATH" ] && grep -q '再次检查，评审所有 open ISSUE' "$DOC_PATH"; then
    echo "[FAIL] framework doc contains pasted issue-review contamination"
    failures=$((failures + 1))
  else
    echo "[PASS] no known pasted issue-review contamination"
  fi

  if [ "${#TRACKED_CRATES[@]}" -eq 16 ]; then
    echo "[PASS] tracked crate count: 16"
  else
    echo "[FAIL] tracked crate count is ${#TRACKED_CRATES[@]}, expected 16"
    failures=$((failures + 1))
  fi

  local latest_summary
  latest_summary="$(find "$BASELINE_DIR" -path '*/summary.json' -type f 2>/dev/null | sort | tail -1 || true)"
  if [ -n "$latest_summary" ]; then
    python3 - "$latest_summary" "${TRACKED_CRATES[@]}" <<'PY'
import json, sys
path = sys.argv[1]
expected = sys.argv[2:]
rows = json.load(open(path, encoding="utf-8"))
seen = {r.get("crate") for r in rows}
missing = [c for c in expected if c not in seen]
required = {"crate", "status", "returncode", "json", "log", "line_pct", "lines", "functions", "test_health"}
field_missing = []
for row in rows:
    miss = sorted(required - set(row))
    if miss:
        field_missing.append(f"{row.get('crate', '<unknown>')}:{','.join(miss)}")
if missing or field_missing:
    if missing:
        print("[FAIL] latest summary missing crates:", ", ".join(missing))
    if field_missing:
        print("[FAIL] latest summary missing fields:", "; ".join(field_missing[:8]))
    raise SystemExit(1)
print(f"[PASS] latest summary shape ok: {path}")
PY
  else
    echo "[WARN] no saved coverage summary found under $BASELINE_DIR"
  fi

  if [ "$failures" -eq 0 ]; then
    echo "STATUS: CONFIG PASS"
    return 0
  fi
  echo "STATUS: CONFIG FAIL ($failures blockers)"
  return 1
}

enforce_summary() {
  local stage="$1"
  local summary="$2"

  python3 - "$stage" "$summary" "${TRACKED_CRATES[@]}" <<'PY'
import json, statistics, sys
stage = sys.argv[1].lower()
summary = sys.argv[2]
expected = sys.argv[3:]
rows = json.load(open(summary, encoding="utf-8"))
by = {r.get("crate"): r for r in rows}
missing = [c for c in expected if c not in by]
if missing:
    print("FAIL: missing tracked crates:", ", ".join(missing))
    raise SystemExit(1)

debt_issues = {
    "sqlrustgo-parser": "#3904",
    "sqlrustgo-mysql-server": "#3904/#4223",
    "sqlrustgo-mysql-client": "#3904",
    "sqlrustgo-vector": "#4225",
    "sqlrustgo-sql-corpus": "#4224",
    "sqlrustgo-admin": "#3904",
}

def pct(crate):
    value = by[crate].get("line_pct")
    return None if value is None else float(value)

def health(crate):
    return by[crate].get("test_health", "unknown")

failures = []
warnings = []

for crate in expected:
    row = by[crate]
    line = pct(crate)
    bad_health = health(crate) != "pass-or-no-test-failure-detected"
    if stage == "alpha":
        if line is None or line < 75.0 or bad_health:
            if crate not in debt_issues:
                failures.append(f"{crate}: alpha debt has no linked issue")
            else:
                warnings.append(f"{crate}: tracked alpha debt -> {debt_issues[crate]} (line={line}, health={health(crate)})")

if stage == "beta":
    p0 = ["sqlrustgo-parser", "sqlrustgo-executor", "sqlrustgo-storage", "sqlrustgo-planner",
          "sqlrustgo-optimizer", "sqlrustgo-mysql-client", "sqlrustgo-mysql-server",
          "sqlrustgo-gmp", "sqlrustgo-rag", "sqlrustgo-vector"]
    values = [pct(c) for c in p0 if pct(c) is not None]
    if values and statistics.mean(values) < 80.0:
        failures.append(f"L1/P0 average below 80: {statistics.mean(values):.2f}%")
    for c in p0:
        line = pct(c)
        if line is None or line < 70.0:
            failures.append(f"{c}: beta P0 line coverage below 70 or missing ({line})")
    if pct("sqlrustgo-gmp") is None or pct("sqlrustgo-gmp") < 78.0:
        failures.append("sqlrustgo-gmp: beta target below 78")

if stage == "rc":
    rc_targets = {
        "sqlrustgo-parser": 80.0,
        "sqlrustgo-executor": 80.0,
        "sqlrustgo-storage": 80.0,
        "sqlrustgo-planner": 80.0,
        "sqlrustgo-optimizer": 80.0,
        "sqlrustgo-catalog": 80.0,
        "sqlrustgo-transaction": 80.0,
        "sqlrustgo-gmp": 80.0,
        "sqlrustgo-rag": 80.0,
        "sqlrustgo-mysql-client": 78.0,
        "sqlrustgo-mysql-server": 72.0,
    }
    for c, target in rc_targets.items():
        line = pct(c)
        if line is None or line < target:
            failures.append(f"{c}: rc line coverage {line} < {target}")
    for c in expected:
        if health(c) != "pass-or-no-test-failure-detected":
            failures.append(f"{c}: rc disallows report-only/failed coverage health ({health(c)})")

if stage == "ga":
    production = ["sqlrustgo-parser", "sqlrustgo-executor", "sqlrustgo-storage", "sqlrustgo-planner",
                  "sqlrustgo-optimizer", "sqlrustgo-catalog", "sqlrustgo-transaction",
                  "sqlrustgo-gmp", "sqlrustgo-rag", "sqlrustgo-server", "sqlrustgo-mysql-client"]
    for c in production:
        line = pct(c)
        if line is None or line < 80.0:
            failures.append(f"{c}: ga production crate line coverage {line} < 80")
    for c in expected:
        if health(c) != "pass-or-no-test-failure-detected":
            failures.append(f"{c}: ga disallows report-only/failed coverage health ({health(c)})")

if stage not in {"alpha", "beta", "rc", "ga"}:
    failures.append(f"unknown stage: {stage}")

print(f"summary: {summary}")
print(f"stage: {stage}")
for item in warnings:
    print("WARN:", item)
if failures:
    for item in failures:
        print("FAIL:", item)
    raise SystemExit(1)
print("STATUS: ENFORCE PASS")
PY
}

if [ "$MODE" = "check-config" ]; then
  check_config
  exit $?
fi

mkdir -p "$OUT_DIR/logs"

if ! command -v cargo-llvm-cov >/dev/null 2>&1 && ! cargo llvm-cov --version >/dev/null 2>&1; then
  echo "ERROR: cargo-llvm-cov is not installed or not available in PATH" >&2
  exit 2
fi

summary_json="$OUT_DIR/summary.json"
summary_md="$OUT_DIR/summary.md"

if [ "$MODE" = "enforce" ] && [ -n "$SUMMARY_OVERRIDE" ]; then
  enforce_summary "$ENFORCE_STAGE" "$SUMMARY_OVERRIDE"
  exit $?
fi

python3 - "$summary_json" <<'PY'
import json, sys
with open(sys.argv[1], "w", encoding="utf-8") as f:
    json.dump([], f)
PY

for crate in "${TRACKED_CRATES[@]}"; do
  json_path="$OUT_DIR/${crate}.json"
  log_path="$OUT_DIR/logs/${crate}.log"
  echo "=== coverage: $crate ==="
  started_at="$(date +%s)"
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
  finished_at="$(date +%s)"
  run_seconds=$((finished_at - started_at))

  python3 - "$summary_json" "$crate" "$json_path" "$log_path" "$rc" "$TIMEOUT_SECONDS" "$run_seconds" <<'PY'
import json, os, sys
summary_path, crate, json_path, log_path, rc, timeout_s, run_seconds = sys.argv[1:]
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
    "run_seconds": int(run_seconds),
    "ignored_count": None,
    "failed_count": None,
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
    import re
    ignored = [int(x) for x in re.findall(r"(\d+) ignored", log)]
    failed = [int(x) for x in re.findall(r"(\d+) failed", log)]
    row["ignored_count"] = sum(ignored) if ignored else 0
    row["failed_count"] = sum(failed) if failed else 0
    if rc == 124:
        row["test_health"] = "timeout"
    elif "test result: FAILED" in log or "target failed" in log or "error: test failed" in log:
        row["test_health"] = "report-only-failure"
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
    f.write("| Crate | Line% | Lines | Functions | Seconds | Ignored | Failed | Test health | JSON | Log |\n")
    f.write("|---|---:|---:|---:|---:|---:|---:|---|---|---|\n")
    for r in rows:
        pct = "" if r["line_pct"] is None else f'{r["line_pct"]:.2f}%'
        f.write(f'| {r["crate"]} | {pct} | {r["lines"]} | {r["functions"]} | {r.get("run_seconds", "")} | {r.get("ignored_count", "")} | {r.get("failed_count", "")} | {r["test_health"]} | `{r["json"]}` | `{r["log"]}` |\n')
PY

echo "summary: $summary_md"
echo "json: $summary_json"

if [ "$MODE" = "enforce" ]; then
  enforce_summary "$ENFORCE_STAGE" "$summary_json"
fi
