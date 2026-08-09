#!/usr/bin/env bash
# =============================================================================
# V312-19 / ISSUE #3906 — check_r2_invariants.sh
# =============================================================================
# Drives the R2.1-R2.8 architectural invariant checks and emits a
# unified report at:
#   docs/releases/v3.12.0/evidence/arch_invariants/R2_INVARIANTS_REPORT.md
#
# For each R2.N (1..8), the script either invokes an existing check or
# emits a stub that exits 0 with `TODO: implement R2.N` in stdout. The
# honest-gap policy (per the v3.12 governance rule) is preferred over
# fabricating passing checks.
#
# See:
#   openspec/changes/v312-19-sql-corpus-arch-invariant-reviewer-gate/
#   ISSUE #3906
# =============================================================================
set -uo pipefail

TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
SOURCE_AGENT="${SOURCE_AGENT:-minimax}"
SOURCE_RUN="${SOURCE_RUN:-minimax-v312-19-r2-$(git rev-parse --short HEAD 2>/dev/null || echo nohead)}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT_DIR="${ROOT}/docs/releases/v3.12.0/evidence/arch_invariants"
REPORT="${OUT_DIR}/R2_INVARIANTS_REPORT.md"
mkdir -p "${OUT_DIR}"

emit_header() {
    cat > "${REPORT}" <<EOF
# v3.12.0 R2 Architectural Invariants Report

- source_agent: \`${SOURCE_AGENT}\`
- source_run: \`${SOURCE_RUN}\`
- timestamp: \`${TIMESTAMP}\`
- branch: \`$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)\`
- commit: \`$(git rev-parse HEAD 2>/dev/null || echo unknown)\`

| check | status | stdout_sha256 | exit_code |
|-------|--------|---------------|-----------|
EOF
}

# Each R2.N may correspond to a real script or a stub. The real
# script names follow the existing v3.11.0 naming convention.
declare -A R2_SCRIPTS=(
    ["R2.1"]="check_arch2_no_bypass.sh"
    ["R2.2"]="check_arch3_no_bypass.sh"
    ["R2.3"]="check_arch_invariants.sh"
    ["R2.4"]="check_arch_sem_debt.sh"
)

run_real_or_stub() {
    local r2="$1"
    local script_name="${R2_SCRIPTS[$r2]:-}"
    local stdout_file="${OUT_DIR}/${r2}.stdout"
    local exit_code
    local status

    if [ -n "${script_name}" ] && [ -f "${ROOT}/scripts/gate/${script_name}" ]; then
        if bash "${ROOT}/scripts/gate/${script_name}" > "${stdout_file}" 2>&1; then
            status="pass"
        else
            status="fail"
        fi
        exit_code=$?
    else
        # Stub: honest-gap, not a fabricated pass.
        {
            echo "TODO: implement ${r2}"
            echo ""
            echo "This is a stub. The v3.12.0 release must either:"
            echo "  1. Land the real ${r2} check, or"
            echo "  2. Mark this R2.N as deferred with owner + expiry"
            echo "     in a follow-up issue."
        } > "${stdout_file}"
        status="stub"
        exit_code=0
    fi

    local sha
    sha="$(sha256sum "${stdout_file}" | awk '{print $1}')"
    cat >> "${REPORT}" <<EOF
| ${r2} | ${status} | ${sha} | ${exit_code} |
EOF
    echo "==> ${r2}: ${status} (sha=${sha:0:12}, exit=${exit_code})"
}

emit_header
for r2 in R2.1 R2.2 R2.3 R2.4 R2.5 R2.6 R2.7 R2.8; do
    run_real_or_stub "${r2}"
done

# Footer
REPORT_SHA="$(sha256sum "${REPORT}" | awk '{print $1}')"
cat >> "${REPORT}" <<EOF

---

<!-- report_sha256: ${REPORT_SHA} -->
EOF

echo "==> R2 invariants report written: ${REPORT}"
exit 0
