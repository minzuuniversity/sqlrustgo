#!/usr/bin/env bash
# A01 gate: every committed audit run must pass evidence_hash verify (Issue #5223).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUNS_DIR="${REPO_ROOT}/audit/v4.1.0/runs"
TOOL="${REPO_ROOT}/audit/v4.1.0/audit_run.py"

if [[ ! -d "${RUNS_DIR}" ]]; then
    echo "PASS: no audit runs committed yet (audit/v4.1.0/runs absent)"
    exit 0
fi

shopt -s nullglob
manifests=("${RUNS_DIR}"/*/manifest.yaml)
if [[ ${#manifests[@]} -eq 0 ]]; then
    echo "PASS: runs/ exists but contains no manifest.yaml"
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
