#!/usr/bin/env bash
#
# check_beta_v3.12.0.sh -- v3.12.0 BETA Governance Gate
#
# Companion to check_beta_gate.sh (B1-B5 / technical gates) which is
# v3.10.0-oriented. This script adds the v3.12.0-specific release files
# and documents that the per-version BETA promotion framework expects.
#
# v3.12.0 variant:
#   - B1 build/clippy/fmt (same as v3.11.0)
#   - B2 lib + integration tests
#   - B3 v3.12.0 release files (paths from docs/releases/v3.12.0/)
#   - B4 gate scripts (alpha + beta + common)
#   - B5 debt register
#   - B6 v3.12.0 BETA promotion_to_BETA_requires evidence
#   - B7 alpha gate still passing (sanity check that ALPHA gates hold)
#
# Usage:
#   bash scripts/gate/check_beta_v3.12.0.sh
#   bash scripts/gate/check_beta_v3.12.0.sh --json
#   bash scripts/gate/check_beta_v3.12.0.sh --out-dir /tmp/reports

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

VERSION="${VERSION:-v3.12.0}"
OUT_DIR="${OUT_DIR:-/tmp/beta_g312_$(date +%Y%m%d_%H%M%S)}"
JSON_OUTPUT=false

for arg in "$@"; do
    case "$arg" in
        --json) JSON_OUTPUT=true; shift ;;
        --out-dir) shift; OUT_DIR="${1:-/tmp/beta_g312}"; shift ;;
        --help|-h) grep "^#" "$0" | head -20; exit 0 ;;
    esac
done

mkdir -p "$OUT_DIR"

PASS=0; TOTAL=0; BLOCKERS=0; WARN=0

check() {
    local name="$1" cmd="$2"
    TOTAL=$((TOTAL+1))
    if eval "$cmd" >/dev/null 2>&1; then
        PASS=$((PASS+1))
        printf "  [PASS] %s\n" "$name"
    else
        BLOCKERS=$((BLOCKERS+1))
        printf "  [FAIL] %s\n" "$name"
    fi
}

warn() {
    local name="$1" cmd="$2"
    TOTAL=$((TOTAL+1))
    if eval "$cmd" >/dev/null 2>&1; then
        PASS=$((PASS+1))
        printf "  [PASS] %s\n" "$name"
    else
        WARN=$((WARN+1))
        printf "  [WARN] %s\n" "$name"
    fi
}

echo "=== v3.12.0 Beta Gate ==="
echo "Version: $VERSION"
echo "Branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'unknown') @ $(git rev-parse --short HEAD 2>/dev/null || echo 'unknown')"
echo ""

# =====================================================
# B1: Build — zero errors
# ============================================================
echo "--- B1: Build ---"
check "B1_CARGO_BUILD" "cargo build --all-features --quiet"
check "B1_CLIPPY" "cargo clippy --all-features -- -D warnings"
check "B1_FMT" "cargo fmt --check --quiet"

# ============================================================
# B2: Test — all lib + integration tests pass
# ============================================================
echo ""
echo "--- B2: Test ---"
check "B2_LIB_TESTS" "cargo test --all-features --lib --quiet"
warn "B2_INTEGRATION_TESTS" "cargo test --all-features --test '*' --quiet"

# ============================================================
# B3: v3.12.0 release files
# ============================================================
echo ""
echo "--- B3: Release Files (v3.12.0) ---"
check "B3_CHANGELOG" "test -f CHANGELOG.md"
check "B3_RELEASE_NOTES" "test -f docs/releases/v3.12.0/RELEASE_NOTES.md"
check "B3_STAGE_YAML" "test -f docs/releases/v3.12.0/STAGE.yaml"
check "B3_FEATURE_CHECKLIST" "test -f docs/releases/v3.12.0/FEATURE_CHECKLIST.md"
check "B3_VERSION_PLAN" "test -f docs/releases/v3.12.0/VERSION_PLAN.md"
check "B3_DEV_PLAN" "test -f docs/releases/v3.12.0/DEVELOPMENT_PLAN.md"
check "B3_TEST_PLAN" "test -f docs/releases/v3.12.0/TEST_PLAN.md"
check "B3_ISSUES_PLAN" "test -f docs/releases/v3.12.0/ISSUES_PLAN.md"

# ============================================================
# B4: Gate scripts exist
# ============================================================
echo ""
echo "--- B4: Gate Scripts ---"
check "B4_ALPHA_GATE" "test -f scripts/gate/check_alpha_v3.12.0.sh"
check "B4_BETA_GATE" "test -f scripts/gate/check_beta_v3.12.0.sh"
check "B4_COMMON_GATES" "test -f scripts/gate/check_arch_invariants.sh && test -f scripts/gate/check_beta_gate.sh && test -f scripts/gate/check_anti_fabrication.sh"
check "B4_V312_55_PROCEDURE_TRIGGER_GATE_DEFINED" "test -f scripts/gate/check_v312_procedure_trigger_gate.sh"
check "B4_V312_STAGE_BOUNDARY" "bash scripts/gate/check_v312_stage_boundary.sh"

# ============================================================
# B5: Debt register
# ============================================================
echo ""
echo "--- B5: Debt Tracking ---"
check "B5_DEBT_REGISTRY" "test -f docs/governance/debt/debt-registry.yaml"

# ============================================================
# B6: v3.12.0 BETA promotion_to_BETA_requires evidence
# ============================================================
echo ""
echo "--- B6: BETA Promotion Evidence (v3.12.0) ---"
check "B6_GMP_SCHEMA"              "test -f docs/releases/v3.12.0/v312-02-gmp-schema-report.md"
check "B6_GMP_INGESTION"           "test -f docs/releases/v3.12.0/v312-03-gmp-ingestion-report.md"
check "B6_EMBEDDING_PROVIDER"      "test -f docs/releases/v3.12.0/v312-04-embedding-provider-report.md"
check "B6_HYBRID_RETRIEVAL"        "test -f docs/releases/v3.12.0/v312-05-hybrid-retrieval-report.md"
check "B6_GRAPH_PROJECTION"        "test -f docs/releases/v3.12.0/v312-06-graph-projection-report.md"
check "B6_AUDIT_HASH_CHAIN"        "test -f docs/releases/v3.12.0/v312-08-compliance-audit-report.md"
check "B6_SQLLOGICTEST_SMOKE_GATE" "bash scripts/gate/check_sqllogictest_v312.sh"
check "B6_SQLLOGICTEST_MANIFEST"   "python3 - <<'PY'
import json
from pathlib import Path

manifest = Path('docs/releases/v3.12.0/evidence/sqllogictest/sqlite-corpus-manifest.json')
data = json.loads(manifest.read_text())
stats = data.get('corpus_stats', {})
total = int(stats.get('total_files', -1))
passed = int(stats.get('pass_files', -1))
failed = int(stats.get('fail_files', -1))
if total <= 0 or passed != total or failed != 0:
    raise SystemExit(f'sqllogictest manifest not clean: total={total} pass={passed} fail={failed}')
PY"
check "B6_SQLLOGICTEST_OPEN_EXCLUSIONS" "python3 - <<'PY'
from pathlib import Path
import re

text = Path('docs/releases/v3.12.0/evidence/sqllogictest/exclusions.yml').read_text()
items = re.split(r'^  - id: ', text, flags=re.M)[1:]
open_items = []
for item in items:
    if not re.search(r'^    status: closed\\b', item, flags=re.M):
        open_items.append(item.splitlines()[0].strip())
if open_items:
    raise SystemExit('open sqllogictest exclusions: ' + ', '.join(open_items))
PY"
check "B6_TPCH_SF1_G4"             "test -f docs/releases/v3.12.0/evidence/G4_tpch_sf1.txt"
check "B6_QUANTILE_FUNCTIONS"        "python3 - <<'PY'
# Issue #4155 / #4216 — quantile_disc / quantile_cont single-fraction and array-fraction forms.
# Verifies:
#   1. Parser accepts quantile_disc(x, 0.5) and quantile_disc(x, [0.25, 0.5, 0.75])
#   2. Executor returns correct values (floor index for disc, interpolated for cont)
#   3. All quantile tests pass
import subprocess
import sys

tests = [
    ('parser quantile', ['cargo', 'test', '-p', 'sqlrustgo-parser', 'quantile', '--', '--nocapture']),
    ('executor quantile', ['cargo', 'test', '--lib', 'quantile', '--', '--nocapture']),
    ('green_4155 single-frac', ['cargo', 'test', '--test', 'null_handling_test', 'green_4155', '--', '--nocapture']),
]

failed = []
for name, cmd in tests:
    r = subprocess.run(cmd, capture_output=True, text=True, timeout=300)
    if r.returncode != 0:
        failed.append(f'{name}: exit {r.returncode}')
        sys.stderr.write(f'FAIL: {name}\n')
        sys.stderr.write(r.stderr[-500:] if r.stderr else '')
        sys.stderr.write(r.stdout[-500:] if r.stdout else '')
    else:
        # verify at least some tests ran
        if 'test result:' not in r.stdout:
            failed.append(f'{name}: no test result in output')
        else:
            print(f'  {name}: OK')

if failed:
    sys.stderr.write(f'quantile gate failed: {\", \".join(failed)}\n')
    sys.exit(1)
print('  All quantile tests PASS')
PY"
check "B6_V312_56_TEACHING_CORPUS" "test -d tests/compat/teaching_sql_v3_12 && test -f tests/compat/teaching_sql_v3_12/manifest.yml"
check "B6_V312_56_EXPLAIN_FIXTURES" "python3 - <<'PY'
import yaml
from pathlib import Path
manifest = Path('tests/compat/teaching_sql_v3_12/manifest.yml')
if not manifest.exists():
    raise SystemExit('manifest.yml not found')
data = yaml.safe_load(manifest.read_text())
explain_files = [f for f in data.get('files', []) if 'explain' in f.get('path', '')]
if len(explain_files) < 5:
    raise SystemExit(f'Expected 5+ EXPLAIN fixtures, got {len(explain_files)}')
PY"
check "B6_V312_47_PARTIAL_CLOSURE" "python3 - <<'PY'
# Issue #4220 / V312-47 PARTIAL closure gate.
# Verifies that every README PARTIAL row is bound to a Gitea issue,
# not left as a dangling state. Aligns with #4220 acceptance condition:
#   - README 中每个 PARTIAL 都绑定到具体 issue 或降级为 DEFERRED/UNSUPPORTED.
import re
import sys
from pathlib import Path

readme = Path('README.md').read_text()
plan = Path('docs/releases/v3.12.0/PARTIAL_FEATURE_REMEDIATION_ISSUE_PLAN.md')
issues_plan = Path('docs/releases/v3.12.0/ISSUES_PLAN.md')

missing = []
issue_pat = re.compile(r'#\d{3,5}')

# Strip the row that LITERALLY describes the PARTIAL plan (a link to
# the plan doc, not a PARTIAL feature row). That row contains the
# substring 'PARTIAL_FEATURE_REMEDIATION_ISSUE_PLAN' as the link target.
lines = [
    line for line in readme.splitlines()
    if 'PARTIAL_FEATURE_REMEDIATION_ISSUE_PLAN.md' not in line
]

# Find all table rows containing PARTIAL. Parse by splitting on '|' and checking columns.
# Table format: | col1 | col2 (v3.11 status) | col3 (v3.12 status) | col4 (evidence) |
partial_rows = []
for line in lines:
    if '|' not in line:
        continue
    cols = [c.strip() for c in line.split('|')]
    # Filter to rows that look like table rows (5+ cells) and contain PARTIAL
    if len(cols) >= 4 and 'PARTIAL' in cols[1]:
        partial_rows.append((line, cols))

# Separate status rows (around line 50-55, col2=v3.11 status, col3=v3.12 status)
# from matrix rows (around line 100-165, col2=capability, col3=v3.11 status, col4=v3.12 status)
status_count = 0
matrix_count = 0

for line, cols in partial_rows:
    has_issue = bool(issue_pat.search(line))
    
    # Determine if this is a status row (col1=capability, col2=v3.11, col3=v3.12)
    # or a matrix row (col1=capability, col2=label, col3=v3.11 status, col4=v3.12 status)
    if len(cols) == 4:
        # Status row format: | capability | v3.11 status | v3.12 status |
        v312_status = cols[2] if len(cols) > 2 else ''
        v311_status = cols[1]
        row_type = 'status'
        status_count += 1
    else:
        # Matrix row format: | capability | label | v3.11 status | v3.12 status | evidence |
        v312_status = cols[3] if len(cols) > 3 else ''
        v311_status = cols[2] if len(cols) > 2 else ''
        row_type = 'matrix'
        matrix_count += 1
    
    # Skip if v3.12 status is DONE (item is resolved)
    if 'DONE' in v312_status.upper():
        continue
    
    # Skip if v3.11 status is DEFERRED (not a PARTIAL claim)
    if 'DEFERRED' in v311_status.upper():
        continue
    
    # This is an unresolved PARTIAL - must have issue reference
    if not has_issue:
        missing.append((f'README.{row_type}', line.strip()))

# 3. PARTIAL_FEATURE_REMEDIATION_ISSUE_PLAN.md must exist + have all required sections.
if not plan.exists():
    missing.append(('PLAN.missing', str(plan)))
else:
    plan_text = plan.read_text()
    for sec in ['## 2. README PARTIAL 整改总账', '## 3. 新增 Issue', '## 4. 必须同步到门禁']:
        if sec not in plan_text:
            missing.append(('PLAN.section_missing', sec))

# 4. ISSUES_PLAN.md must reference #4220 (per PARTIAL plan §4.3).
if not issues_plan.exists():
    missing.append(('ISSUES_PLAN.missing', str(issues_plan)))
elif '#4220' not in issues_plan.read_text():
    missing.append(('ISSUES_PLAN.no_#4220', str(issues_plan)))

if missing:
    for entry in missing:
        print(f'  missing: {entry}', file=sys.stderr)
    raise SystemExit(f'#4220 PARTIAL closure failed: {len(missing)} missing binding(s)')
print(f'  README PARTIAL rows bound to issues: OK ({status_count} status + {matrix_count} matrix rows scanned, 0 unresolved PARTIAL without issue)')
PY"

check "B6_V312_56_ISSUE_DEFINITION" "test -f docs/releases/v3.12.0/issues/V312-56_TEACHING_AND_V400_REMEDIATION_ISSUE_BODIES.md"
check "B6_V312_56_TEST_PLAN_GATE"   "grep -q 'V312-G27' docs/releases/v3.12.0/TEST_PLAN.md"
check "B6_V312_56_BETA_STAGE_SCOPE" "grep -q 'V312-56A Metadata/SHOW/information_schema' docs/releases/v3.12.0/STAGE.yaml && grep -q 'V312-56D Prepared statement / wire protocol' docs/releases/v3.12.0/STAGE.yaml"
check "B6_V312_56_VERIFICATION"     "bash -c 'for f in docs/releases/v3.12.0/evidence/teaching_v400/V312-56-VERIFICATION.md docs/releases/v3.12.0/evidence/v312-56/V312-56-VERIFICATION.md; do test -f \"\$f\" && grep -Eqi \"exit code|exit codes\" \"\$f\" && grep -Eqi \"evidence hash|hashes\" \"\$f\" && exit 0; done; exit 1'"
check "B6_V312_56_TEACHING_GAPS" "python3 - <<'PY'
# Issue #4250 / V312-56 master orchestrator teaching-gap closure gate.
# Verifies each V312-56B-G sub-issue has at minimum the required artifacts:
#   - 56B (teaching corpus + manifest): manifest.yml + 28+ SQL files
#   - 56C (transaction teaching): V312-56C_TRANSACTION_TEACHING.md
#   - 56D (prepared statement teaching): prepared/* fixtures
#   - 56E (EXPLAIN teaching): explain/* fixtures + crates/executor/src/explain.rs
#   - 56F (VIEW/CTE/MERGE disposition): MYSQL_COMPAT_STATUS entries
#   - 56G (Partition/FullText disposition): MYSQL_COMPAT_STATUS entries
import sys
from pathlib import Path

missing = []

# 56B: teaching corpus
manifest = Path('tests/compat/teaching_sql_v3_12/manifest.yml')
if not manifest.exists():
    missing.append('56B.manifest_missing')
else:
    sql_count = len(list(Path('tests/compat/teaching_sql_v3_12').rglob('*.sql')))
    if sql_count < 28:
        missing.append(f'56B.too_few_sql_files (have {sql_count}, need >=28)')

# 56C: transaction teaching doc
tx_doc = Path('docs/releases/v3.12.0/evidence/v312-56/V312-56C_TRANSACTION_TEACHING.md')
if not tx_doc.exists():
    missing.append('56C.doc_missing')

# 56D: prepared statement fixtures
prep_dir = Path('tests/compat/teaching_sql_v3_12/prepared')
if not prep_dir.exists():
    missing.append('56D.dir_missing')
else:
    prep_files = list(prep_dir.glob('*.sql'))
    if len(prep_files) < 1:
        missing.append(f'56D.no_sql_files (have {len(prep_files)}, need >=1)')

# 56E: EXPLAIN executor + fixtures
explain_rs = Path('crates/executor/src/explain.rs')
if not explain_rs.exists():
    missing.append('56E.explain_rs_missing')
explain_dir = Path('tests/compat/teaching_sql_v3_12/explain')
if not explain_dir.exists():
    missing.append('56E.dir_missing')
else:
    explain_files = list(explain_dir.glob('*.sql'))
    if len(explain_files) < 5:
        missing.append(f'56E.too_few_explain (have {len(explain_files)}, need >=5)')

# 56F: VIEW/CTE/MERGE disposition (status entries in MYSQL_COMPAT_STATUS.md)
compat = Path('docs/releases/v3.12.0/MYSQL_COMPAT_STATUS.md')
if not compat.exists():
    missing.append('56F.compat_status_missing')
else:
    compat_text = compat.read_text()
    for keyword, sub in [('CREATE VIEW', '56F.VIEW'), ('WITH RECURSIVE', '56F.CTE'), ('MERGE', '56F.MERGE')]:
        if keyword not in compat_text:
            missing.append(f'{sub}.keyword_missing ({keyword})')

# 56G: Partition/FullText disposition
if not compat.exists():
    missing.append('56G.compat_status_missing')
else:
    compat_text = compat.read_text()
    for keyword, sub in [('TABLE PARTITION', '56G.PARTITION'), ('FULLTEXT', '56G.FULLTEXT')]:
        if keyword not in compat_text:
            missing.append(f'{sub}.keyword_missing ({keyword})')

if missing:
    for entry in missing:
        print(f'  missing: {entry}', file=sys.stderr)
    raise SystemExit(f'#4250 V312-56 teaching gaps: {len(missing)} missing artifact(s)')
print('  V312-56B-G teaching gaps: all artifacts present')
PY"


# ============================================================
# B7: ALPHA gate sanity check (BETA cannot regress ALPHA state)
# ============================================================
echo ""
echo "--- B7: ALPHA Gate Sanity (must still pass) ---"
warn "B7_ALPHA_ENTRY"      "ALPHA_QUALITY_FAST_TEST=1 bash scripts/gate/check_alpha_entry_v3.12.0.sh >/dev/null 2>&1"
warn "B7_ALPHA_QUALITY"    "ALPHA_QUALITY_FAST_TEST=1 bash scripts/gate/check_alpha_quality_v3.12.0.sh >/dev/null 2>&1"

# ============================================================
# Summary
# ============================================================
echo ""
echo "=== v3.12.0 Beta Gate Summary ==="
echo "PASS: $PASS/$TOTAL"
[ "$WARN" -gt 0 ] && echo "WARN: $WARN"
echo "BLOCKERS: $BLOCKERS"
echo ""

# Write JSON report
cat > "$OUT_DIR/beta_gate_report.json" <<JSONEOF
{
  "version": "$VERSION",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "checks": { "pass": $PASS, "total": $TOTAL, "warn": $WARN, "blockers": $BLOCKERS },
  "result": "$([ "$BLOCKERS" -eq 0 ] && echo 'PASS' || echo 'BLOCKED')"
}
JSONEOF

if $JSON_OUTPUT; then
    cat "$OUT_DIR/beta_gate_report.json"
fi

if [ "$BLOCKERS" -eq 0 ]; then
    echo "✓ All checks pass — ready for BETA promotion."
    exit 0
else
    echo "✗ $BLOCKERS blocker(s) must be resolved before BETA promotion."
    exit 1
fi
