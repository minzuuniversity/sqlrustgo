#!/usr/bin/env bash
# A01 gate: every committed audit run must pass verify (Issue #5223).
#
# Scans both run layouts:
#   legacy      audit/v4.1.0/runs/<run_id>/manifest.yaml   (schema_version 1)
#   iaia410/v1  audit/v4.1.0/<run_id>/manifest.yaml        (SSOT section 5)
# `runs/` itself holds no manifest.yaml, so the two globs cannot double-count.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
AUDIT_DIR="${REPO_ROOT}/audit/v4.1.0"
RUNS_DIR="${AUDIT_DIR}/runs"
TOOL="${AUDIT_DIR}/audit_run.py"

shopt -s nullglob
manifests=("${RUNS_DIR}"/*/manifest.yaml "${AUDIT_DIR}"/*/manifest.yaml)
if [[ ${#manifests[@]} -eq 0 ]]; then
    echo "PASS: no audit runs committed yet"
    exit 0
fi

fail=0
for mf in "${manifests[@]}"; do
    run_id="$(basename "$(dirname "${mf}")")"
    if python3 "${TOOL}" verify --run-id "${run_id}"; then
        :
    else
        echo "FAIL: audit run ${run_id} failed evidence verify" >&2
        fail=1
    fi
done

if [[ "${fail}" -ne 0 ]]; then
    exit 1
fi
echo "PASS: ${#manifests[@]} audit run(s) verified"
