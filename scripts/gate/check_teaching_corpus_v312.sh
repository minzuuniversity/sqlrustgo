#!/usr/bin/env bash
# =============================================================================
# V312-56B / ISSUE #4252 — Teaching SQL corpus structural gate
# =============================================================================
# Validates the structural integrity of tests/compat/teaching_sql_v3_12/:
#   1. Every file in the corpus is listed in manifest.yml (and vice-versa)
#   2. Every .sql file declares `# name:` and `# expect:` headers
#   3. FAIL/SKIP entries carry issue_link / owner / expiry in the manifest
#   4. The oracles block declares at least SQLite (and optionally MySQL /
#      PostgreSQL)
#   5. The corpus does not contain any "ignored" comments that would
#      silently bypass the gate.
#
# The actual SQLite-vs-SQLRustGo row-by-row oracle comparison is delegated
# to the teaching_corpus_test integration test (which uses the engine
# directly) and to the Round-21 / #4218 follow-up. This gate keeps the
# structural contract enforceable without booting the full REPL.
# =============================================================================
set -o pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

CORPUS_DIR="$ROOT/tests/compat/teaching_sql_v3_12"
MANIFEST="$CORPUS_DIR/manifest.yml"
OUT_DIR="$ROOT/docs/releases/v3.12.0/evidence/teaching_corpus"
LOG_DIR="$ROOT/docs/releases/v3.12.0/logs"
REPORT="$OUT_DIR/teaching-corpus-report.md"
SUMMARY="$OUT_DIR/teaching-corpus-summary.json"

mkdir -p "$OUT_DIR" "$LOG_DIR"

TS="$(date +%Y%m%d_%H%M%S)"
COMMIT="$(git rev-parse --short=10 HEAD 2>/dev/null || echo unknown)"
LOG="$LOG_DIR/teaching_corpus_${COMMIT}_${TS}.log"

# ---- flag parsing ----
# --with-oracle additionally runs the row-by-row SQLite-vs-SQLRustGo
# comparison test (tests/integration/sql/teaching_corpus_oracle_test.rs)
# and folds its pass/fail counts into the JSON summary. Default: off
# to keep the structural gate fast.
WITH_ORACLE=0
for arg in "$@"; do
  case "$arg" in
    --with-oracle) WITH_ORACLE=1 ;;
    --help|-h)
      echo "usage: $0 [--with-oracle]"
      exit 0
      ;;
    *) echo "unknown argument: $arg" >&2; exit 2 ;;
  esac
done

PASS=0
FAIL=0
WARN=0
ORACLE_RUNS=0
ORACLE_PASS=0
ORACLE_FAIL=0
ORACLE_SKIPPED=0
ORACLE_DIFF_DIR="$OUT_DIR/oracle_diffs"

record_pass() { echo "[PASS] $1" | tee -a "$LOG"; PASS=$((PASS+1)); }
record_fail() { echo "[FAIL] $1" | tee -a "$LOG"; FAIL=$((FAIL+1)); }
record_warn() { echo "[WARN] $1" | tee -a "$LOG"; WARN=$((WARN+1)); }

echo "=== SQLRustGo v3.12 Teaching Corpus Structural Gate ===" | tee "$LOG"
echo "timestamp: $(date -Iseconds)" | tee -a "$LOG"
echo "commit: $(git rev-parse HEAD 2>/dev/null || echo unknown)" | tee -a "$LOG"
echo "corpus_dir: $CORPUS_DIR" | tee -a "$LOG"
echo "manifest: $MANIFEST" | tee -a "$LOG"
echo | tee -a "$LOG"

# --- 1. Manifest existence ---
if [ ! -f "$MANIFEST" ]; then
  record_fail "manifest missing: $MANIFEST"
  write_report_and_exit 1
fi
record_pass "manifest exists"

# --- 2. Top-level keys ---
for key in 'version:' 'owner:' 'stage:' 'oracles:' 'files:'; do
  if grep -q "^${key}" "$MANIFEST"; then
    record_pass "manifest key '$key' present"
  else
    record_fail "manifest missing top-level key '$key'"
  fi
done

# --- 3. Oracle block declares sqlite ---
if awk '/^oracles:/{f=1; next} f && /^[[:alpha:]_-]+:/{exit} f && /sqlite/{found=1} END{exit !found}' "$MANIFEST"; then
  record_pass "oracles block declares sqlite (V312-56B minimum)"
else
  record_fail "oracles block missing sqlite declaration"
fi

# --- 4. Manifest ↔ filesystem bijection ---
SCRATCH="$(mktemp -d)"
trap 'rm -rf "$SCRATCH"' EXIT

# paths declared in manifest (paths are quoted-free strings in our schema)
manifest_paths="$(awk '
  /^files:/{f=1; next}
  f && /^[[:alpha:]_-]+:/{exit}
  f && /^[[:space:]]*- path:/{
    sub(/^[[:space:]]*- path:[[:space:]]*/, "");
    gsub(/^"|"$/, "");
    gsub(/^'\''|'\''$/, "");
    print
  }
' "$MANIFEST" | sort -u)"

# paths on disk
disk_paths="$(find "$CORPUS_DIR" -type f -name '*.sql' \
  | sed "s|^$CORPUS_DIR/||" | sort -u)"

missing="$(comm -23 <(printf '%s\n' "$manifest_paths") <(printf '%s\n' "$disk_paths") | grep .)"
extra="$(comm -13 <(printf '%s\n' "$manifest_paths") <(printf '%s\n' "$disk_paths") | grep .)"

if [ -z "$missing" ]; then
  record_pass "manifest covers every .sql file on disk"
else
  record_fail "manifest references files missing on disk: $(echo "$missing" | tr '\n' ' ')"
fi
if [ -z "$extra" ]; then
  record_pass "every .sql file on disk is listed in manifest"
else
  record_fail "files on disk not in manifest: $(echo "$extra" | tr '\n' ' ')"
fi

# --- 5. Per-file headers ---
missing_name=0
missing_expect=0
while IFS= read -r f; do
  [ -n "$f" ] || continue
  rel="${f#$CORPUS_DIR/}"
  if ! grep -q '^[[:space:]]*#[[:space:]]*name:' "$f"; then
    record_fail "$rel: missing '# name:' header"
    missing_name=$((missing_name + 1))
  fi
  if ! grep -q '^[[:space:]]*#[[:space:]]*expect:' "$f"; then
    record_fail "$rel: missing '# expect:' header (PASS/FAIL/SKIP)"
    missing_expect=$((missing_expect + 1))
  fi
done < <(find "$CORPUS_DIR" -type f -name '*.sql')
if [ "$missing_name" -eq 0 ]; then
  record_pass "every file has '# name:' header"
fi
if [ "$missing_expect" -eq 0 ]; then
  record_pass "every file has '# expect:' header"
fi

# --- 6. FAIL/SKIP governance fields ---
tbd_or_missing=0
while IFS= read -r line; do
  rel="$(echo "$line" | sed 's/^[[:space:]]*- path:[[:space:]]*//')"
  rel="${rel%\"}"; rel="${rel#\"}"; rel="${rel%\'}"; rel="${rel#\'}"
  # find the block under this entry
  block="$(awk -v target="^  - path: ${rel}\$" '
    $0 ~ target {capture=1; print; next}
    capture && /^[[:space:]]*- path:/ {exit}
    capture {print}
  ' "$MANIFEST")"
  expect="$(echo "$block" | awk -F': *' '/^[[:space:]]*expected:/{print $2; exit}')"
  case "$expect" in
    FAIL|SKIP)
      for field in issue_link owner expiry; do
        val="$(echo "$block" | awk -v f="$field" -F': *' '
          tolower($1) ~ "^[[:space:]]*" f ":" {sub(/^[[:space:]]*/, "", $2); print $2; exit}
        ')"
        if [ -z "$val" ] || [ "$val" = "TBD" ]; then
          record_fail "$rel: FAIL/SKIP entry missing $field (or still 'TBD')"
          tbd_or_missing=$((tbd_or_missing + 1))
        fi
      done
      ;;
  esac
done < <(grep "^[[:space:]]*- path:" "$MANIFEST")
if [ "$tbd_or_missing" -eq 0 ]; then
  record_pass "FAIL/SKIP entries carry issue_link / owner / expiry"
fi

# --- 7. No silent #[ignore] ---
ignore_hits="$(find "$CORPUS_DIR" -type f -name '*.sql' -exec grep -l -E '^[[:space:]]*#[[:space:]]*ignore' {} +)"
if [ -z "$ignore_hits" ]; then
  record_pass "no silent #[ignore] markers in corpus"
else
  record_fail "found #[ignore] markers (teaching corpus disallows silent skip): $ignore_hits"
fi

# --- 8. FAIL/SKIP count (informational) ---
fail_count="$(grep -c '^[[:space:]]*- path:' "$MANIFEST" \
  | awk '{n=$0; getline; while($0 ~ /^[[:space:]]*[a-z_]+:/ && $0 !~ /^[[:space:]]*- /){getline; if($0 ~ /^[[:space:]]*- path:/){break}; if($0 ~ /^[[:space:]]*expected:[[:space:]]*(FAIL|SKIP)/){c++}} print c+0}')"
record_warn "FAIL/SKIP entries: $(grep -E '^[[:space:]]*expected:[[:space:]]*(FAIL|SKIP)' "$MANIFEST" | wc -l | tr -d ' ')"

# --- 9. Optional SQLite-vs-SQLRustGo row oracle (--with-oracle) ---
if [ "$WITH_ORACLE" -eq 1 ]; then
  echo "[oracle] --with-oracle enabled, running teaching_corpus_oracle_test ..." | tee -a "$LOG"
  ORACLE_LOG="$LOG_DIR/oracle_${COMMIT}_${TS}.log"
  if cargo test --test teaching_corpus_oracle_test --all-features -- --nocapture >"$ORACLE_LOG" 2>&1; then
    record_pass "teaching_corpus_oracle_test (cargo)"
  else
    record_fail "teaching_corpus_oracle_test (cargo): see $ORACLE_LOG"
  fi
  # Count rowset entries actually compared (vs plan_shape /
  # dialect_only skipped). Each entry has exactly one
  # `oracle_mode:` line under `files:`, so a simple grep on the
  # unique value counts cleanly without hand-rolled YAML parsing.
  ORACLE_RUNS="$(grep -cE '^[[:space:]]+oracle_mode:[[:space:]]*"rowset"' "$MANIFEST" || true)"
  ORACLE_RUNS="${ORACLE_RUNS:-0}"
  ORACLE_SKIPPED_PLAN="$(grep -cE '^[[:space:]]+oracle_mode:[[:space:]]*"plan_shape"' "$MANIFEST" || true)"
  ORACLE_SKIPPED_DIALECT="$(grep -cE '^[[:space:]]+oracle_mode:[[:space:]]*"dialect_only"' "$MANIFEST" || true)"
  ORACLE_SKIPPED=$(( ${ORACLE_SKIPPED_PLAN:-0} + ${ORACLE_SKIPPED_DIALECT:-0} ))
  # Parse test stdout to count passed/failed files (cargo prints
  # `test ... ok` or `FAILED` for each). The test aggregates all
  # rowsets into one assertion so a pass=fail=0 unless explicitly
  # invoking the --list flag (we don't), so map pass/fail to a single
  # boolean.
  ORACLE_PASS=$(grep -cE '^test test_teaching_corpus_oracle_rowset_matches_sqlite \.\.\. ok' "$ORACLE_LOG" || true)
  ORACLE_PASS="${ORACLE_PASS:-0}"
  ORACLE_FAIL=$(grep -cE '^test test_teaching_corpus_oracle_rowset_matches_sqlite \.\.\. FAILED' "$ORACLE_LOG" || true)
  ORACLE_FAIL="${ORACLE_FAIL:-0}"
  record_pass "oracle_diffs dir: $ORACLE_DIFF_DIR"
fi

LOG_HASH="$(sha256sum "$LOG" 2>/dev/null | cut -d' ' -f1 || echo unavailable)"

cat > "$REPORT" <<EOF
# SQLRustGo v3.12 Teaching Corpus Structural Report

> **provenance:** generated_by=check_teaching_corpus_v312.sh, generated_at=$(date -Iseconds), commit=$(git rev-parse HEAD 2>/dev/null || echo unknown), source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, evidence_hash=$LOG_HASH, log=$LOG

| Field | Value |
|---|---|
| source_agent | ${SOURCE_AGENT:-local} |
| source_run | ${SOURCE_RUN:-check_teaching_corpus_v312} |
| timestamp | $(date -Iseconds) |
| commit | $(git rev-parse HEAD 2>/dev/null || echo unknown) |
| log | $LOG |
| evidence_hash | $LOG_HASH |
| gate_status | $([ "$FAIL" -eq 0 ] && echo PASS || echo FAIL) |
| pass | $PASS |
| fail | $FAIL |
| warn | $WARN |
| with_oracle | $WITH_ORACLE |
| oracle_runs | ${ORACLE_RUNS:-n/a} |
| oracle_pass | ${ORACLE_PASS:-n/a} |
| oracle_fail | ${ORACLE_FAIL:-n/a} |
| oracle_skipped | ${ORACLE_SKIPPED:-n/a} |
| oracle_diff_dir | ${ORACLE_DIFF_DIR:-(disabled)} |

## Scope

This is the V312-56B structural gate. It enforces the manifest contract:

1. Every file in \`$CORPUS_DIR\` is listed in \`manifest.yml\` (and vice-versa)
2. Every \`.sql\` file declares \`# name:\` and \`# expect:\` markers
3. FAIL/SKIP entries carry \`issue_link\` / \`owner\` / \`expiry\` in
   \`manifest.yml\`
4. The oracles block declares at least SQLite (MySQL/PostgreSQL optional)
5. No silent \`# ignore\` markers

The actual SQLite-vs-SQLRustGo row-by-row oracle comparison is delegated
to \`cargo test --test teaching_corpus_test\` (manifest integrity) and
to \`cargo test --test teaching_corpus_oracle_test\` (per-file row-set
parity vs SQLite goldens under \`docs/releases/v3.12.0/evidence/teaching_corpus/golden/\`).
The oracle test runs only when this gate is invoked with
\`--with-oracle\`; pass/fail counts are then folded into the JSON
summary and oracle diffs land in \`docs/releases/v3.12.0/evidence/teaching_corpus/oracle_diffs/\`.

## Boundary

- This report covers the teaching corpus only. The smoke corpus
  (\`crates/sqlrustgo_sqllogictest/testdata\`) and the SQLite official
  corpus are tracked under \`check_sqllogictest_v312.sh\` / V312-11 / V312-24.
EOF

cat > "$SUMMARY" <<JSON
{
  "timestamp": "$(date -Iseconds)",
  "commit": "$(git rev-parse HEAD 2>/dev/null || echo unknown)",
  "corpus_dir": "$CORPUS_DIR",
  "pass": $PASS,
  "fail": $FAIL,
  "warn": $WARN,
  "evidence_hash": "$LOG_HASH"$(if [ "$WITH_ORACLE" -eq 1 ]; then printf ',\n  "oracle_runs": %s,\n  "oracle_pass": %s,\n  "oracle_fail": %s,\n  "oracle_skipped": %s,\n  "oracle_diff_dir": "%s"' "$ORACLE_RUNS" "$ORACLE_PASS" "$ORACLE_FAIL" "$ORACLE_SKIPPED" "$ORACLE_DIFF_DIR"; fi)
}
JSON

echo | tee -a "$LOG"
echo "report: $REPORT"
echo "summary: $SUMMARY"

if [ "$FAIL" -eq 0 ]; then
  exit 0
fi
exit 1
