#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$REPO_ROOT"

COMMIT="$(git rev-parse --short=10 HEAD)"
TIMESTAMP="$(date +%Y%m%d_%H%M%S)"
OUT_DIR="${OUT_DIR:-docs/releases/v3.12.0/coverage-baseline/current_${COMMIT}_${TIMESTAMP}}"
TIMEOUT_SECONDS="${COVERAGE_TIMEOUT_SECONDS:-120}"

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

mkdir -p "$OUT_DIR/logs"

if ! command -v cargo-llvm-cov >/dev/null 2>&1 && ! cargo llvm-cov --version >/dev/null 2>&1; then
  echo "ERROR: cargo-llvm-cov is not installed or not available in PATH" >&2
  exit 2
fi

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

echo "summary: $summary_md"
echo "json: $summary_json"
