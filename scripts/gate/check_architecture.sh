#!/usr/bin/env bash
# check_architecture.sh — GA-P1 Architecture Gate
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PASS=0; FAIL=0
echo "=== GA-P1 Architecture Gate ==="
[ -f docs/releases/v3.11.0/ARCHITECTURE.md ] && echo "  [PASS] ARCHITECTURE.md" && PASS=$((PASS+1)) || { echo "  [FAIL] missing"; FAIL=$((FAIL+1)); }
grep -q "Clustered Index" docs/releases/v3.11.0/ARCHITECTURE.md && echo "  [PASS] Clustered Index" && PASS=$((PASS+1)) || { echo "  [FAIL]"; FAIL=$((FAIL+1)); }
grep -q "Adaptive Hash Index" docs/releases/v3.11.0/ARCHITECTURE.md && echo "  [PASS] AHI" && PASS=$((PASS+1)) || { echo "  [FAIL]"; FAIL=$((FAIL+1)); }
grep -q "Hash Semi Join" docs/releases/v3.11.0/ARCHITECTURE.md && echo "  [PASS] Hash Semi Join" && PASS=$((PASS+1)) || { echo "  [FAIL]"; FAIL=$((FAIL+1)); }
grep -q "Hash Anti Join" docs/releases/v3.11.0/ARCHITECTURE.md && echo "  [PASS] Hash Anti Join" && PASS=$((PASS+1)) || { echo "  [FAIL]"; FAIL=$((FAIL+1)); }
[ -x scripts/gate/check_architecture.sh ] && echo "  [PASS] gate executable" && PASS=$((PASS+1)) || { echo "  [FAIL]"; FAIL=$((FAIL+1)); }
echo "PASS: $PASS, FAIL: $FAIL"
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
