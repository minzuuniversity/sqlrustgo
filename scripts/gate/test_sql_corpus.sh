#!/usr/bin/env bash
# =============================================================================
# V312-19 / ISSUE #3906 — test_sql_corpus.sh
# =============================================================================
# Walks scripts/gate/corpus_manifest.yaml, runs each target, captures
# stdout+exit, SHA256 the log, and writes
# docs/releases/v3.12.0/evidence/sql_corpus/ALL_TARGETS_REPORT.md with
# one row per target.
#
# Each row carries: target | cases | pass | fail | skipped | status |
# evidence_hash | timestamp | source_run.
#
# Status values: pass (exit 0), fail (exit non-zero), missing
# (allow_missing=false and command not found), deferred (min_cases
# floor not met, or allow_missing=true and command failed).
#
# See: openspec/changes/v312-19-sql-corpus-arch-invariant-reviewer-gate
# =============================================================================
set -uo pipefail

TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
SOURCE_AGENT="${SOURCE_AGENT:-minimax}"
SOURCE_RUN="${SOURCE_RUN:-minimax-v312-19-corpus-$(git rev-parse --short HEAD 2>/dev/null || echo nohead)}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MANIFEST="${ROOT}/scripts/gate/corpus_manifest.yaml"
OUT_DIR="${ROOT}/docs/releases/v3.12.0/evidence/sql_corpus"
REPORT="${OUT_DIR}/ALL_TARGETS_REPORT.md"
LOG_DIR="${OUT_DIR}/logs"

mkdir -p "${OUT_DIR}" "${LOG_DIR}"

# ---- YAML parse (no python dep, so keep it simple) --------------------------
# corpus_manifest.yaml format (one row per target):
#   - name: <id>
#     command: <shell command>
#     evidence_dir: <path under docs/releases/v3.12.0/evidence/>
#     min_cases: <int>
#     allow_missing: <bool>  (optional)
parse_manifest() {
    # Stream the YAML into key/value pairs. The format is constrained
    # to the schema documented in corpus_manifest.yaml; we do not
    # attempt a full YAML parse.
    awk '
        /^[[:space:]]*-[[:space:]]*name:/ {
            if (name != "") emit()
            name = $0
            sub(/^[[:space:]]*-[[:space:]]*name:[[:space:]]*/, "", name)
            command = ""; evidence_dir = ""; min_cases = "0"; allow_missing = "false"
            next
        }
        /^[[:space:]]*command:/ {
            command = $0
            sub(/^[[:space:]]*command:[[:space:]]*/, "", command)
            next
        }
        /^[[:space:]]*evidence_dir:/ {
            evidence_dir = $0
            sub(/^[[:space:]]*evidence_dir:[[:space:]]*/, "", evidence_dir)
            next
        }
        /^[[:space:]]*min_cases:/ {
            min_cases = $0
            sub(/^[[:space:]]*min_cases:[[:space:]]*/, "", min_cases)
            next
        }
        /^[[:space:]]*allow_missing:/ {
            allow_missing = $0
            sub(/^[[:space:]]*allow_missing:[[:space:]]*/, "", allow_missing)
            next
        }
        /^[[:space:]]*targets:/ { next }
        /^[[:space:]]*-/ { next }
        /^[[:space:]]*$/ { next }
        END { if (name != "") emit() }
        function emit() {
            # Trim quotes if present.
            gsub(/^["'\'']|["'\'']$/, "", command)
            gsub(/^["'\'']|["'\'']$/, "", evidence_dir)
            gsub(/^["'\'']|["'\'']$/, "", min_cases)
            gsub(/^["'\'']|["'\'']$/, "", allow_missing)
            printf("%s\t%s\t%s\t%s\t%s\n", name, command, evidence_dir, min_cases, allow_missing)
            name = ""
        }
    ' "$MANIFEST"
}

# ---- emit header ----------------------------------------------------------
emit_header() {
    cat > "${REPORT}" <<EOF
# v3.12.0 SQL Corpus — All-Targets Report

- source_agent: \`${SOURCE_AGENT}\`
- source_run: \`${SOURCE_RUN}\`
- timestamp: \`${TIMESTAMP}\`
- branch: \`$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)\`
- commit: \`$(git rev-parse HEAD 2>/dev/null || echo unknown)\`

| target | cases | pass | fail | skipped | status | evidence_hash | timestamp | source_run |
|--------|-------|------|------|---------|--------|---------------|-----------|------------|
EOF
}

# ---- run one target -------------------------------------------------------
run_target() {
    local name="$1"
    local cmd="$2"
    local evidence_dir="$3"
    local min_cases="$4"
    local allow_missing="$5"
    local log="${LOG_DIR}/${name}.log"
    local start_ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    local status="pass"
    local cases=0
    local pass=0
    local fail=0
    local skipped=0
    local sha="(no log)"

    if [ -z "${cmd}" ]; then
        status="missing"
        if [ "${allow_missing}" != "true" ]; then
            echo "FAIL: target ${name} has empty command and allow_missing=false"
            return 1
        fi
    else
        # Run the target command. We do NOT use set -e so a non-zero
        # exit is recorded as `fail`, not abort the whole runner.
        (
            cd "${ROOT}"
            bash -c "${cmd}"
        ) > "${log}" 2>&1
        local rc=$?
        if [ "${rc}" -ne 0 ]; then
            if [ "${allow_missing}" = "true" ]; then
                status="deferred"
            else
                status="fail"
            fi
        fi
    fi

    # Best-effort: count cases / pass / fail from the log. Most
    # corpus commands emit a recognizable pattern; for now we use a
    # generic grep that looks for `test result: ok` / `tests passed`
    # and counts test cases from `^test ... ok` lines.
    if [ -f "${log}" ]; then
        cases=$(grep -cE '^test .* \.\.\. ok' "${log}" 2>/dev/null | head -1)
        cases=${cases:-0}
        pass=${cases}
        fail=$(grep -cE '^test .* \.\.\. FAILED' "${log}" 2>/dev/null | head -1)
        fail=${fail:-0}
        skipped=$(grep -cE '^test .* \.\.\. ignored' "${log}" 2>/dev/null | head -1)
        skipped=${skipped:-0}
        sha="$(sha256sum "${log}" 2>/dev/null | awk '{print $1}')"
    fi

    # If status is still pass but the manifest says min_cases and
    # the actual cases is below, mark deferred.
    if [ "${status}" = "pass" ] && [ "${min_cases}" != "0" ] && [ "${cases}" -lt "${min_cases}" ]; then
        status="deferred"
    fi

    # Record the row.
    cat >> "${REPORT}" <<EOF
| ${name} | ${cases} | ${pass} | ${fail} | ${skipped} | ${status} | ${sha} | ${start_ts} | ${SOURCE_RUN} |
EOF
    echo "  ${name}: ${status} (cases=${cases}, pass=${pass}, fail=${fail}, sha=${sha:0:12})"
}

# ---- main -----------------------------------------------------------------
emit_header
parse_manifest | while IFS=$'\t' read -r name cmd evidence_dir min_cases allow_missing; do
    [ -z "${name}" ] && continue
    run_target "${name}" "${cmd}" "${evidence_dir}" "${min_cases}" "${allow_missing}"
done

# ---- footer ---------------------------------------------------------------
REPORT_SHA="$(sha256sum "${REPORT}" 2>/dev/null | awk '{print $1}')"
cat >> "${REPORT}" <<EOF

---

<!-- report_sha256: ${REPORT_SHA} -->
EOF

echo ""
echo "==> ALL_TARGETS_REPORT written: ${REPORT} (sha=${REPORT_SHA:0:12})"
echo "    per-target logs: ${LOG_DIR}/"
exit 0
