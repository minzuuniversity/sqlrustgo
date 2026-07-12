#!/usr/bin/env bash
# scripts/gate/check_tpch_sf1.sh
#
# G4: TPC-H SF=1.0 gate for v3.10.0 / Issue #3732.
# Runs the canonical SF=1 baseline wrapper against official dbgen row counts.

set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

SF1_DIR="${TPCH_SF1_DIR:-${SF1_DIR:-/tmp/tpch-sf1}}"

exec bash scripts/tpch_sf1_baseline.sh --sf1-dir "$SF1_DIR" "${@:--dry-run}"
