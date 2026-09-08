#!/bin/bash
# V312-37: Anti-Ignore gate (G19)
# Threshold: active entries <= 47, total_allowed <= 125
# V312-17 round-17 (ADR-008 exception): 9 e2e_wire_protocol #[ignore] markers
# consolidated into 1 registry entry. New baseline total_allowed = 73 + 23
# (round-16) = 96. The 73 v3.9.0 baseline is preserved as v3.9.0_legacy field.
# V312-59-B (#4385): +1 entry for tests/integration/oracle/q8_8way_date_range_regression.rs
# (q8 regression test for V312-48 issue #4274). Bump 96 -> 97.
# V312-59-followup (#4419): +14 entries for Q7/Q8/Q12/Q17/q2_5way subset reproductions
# (added by #4375/#4376/#4378/#4379 fixes), 2 tpch 6M-load tests (added by
# PR #4417 insert_streaming_iter), and 1 v312_mixed_soak nightly test.
# Bump 97 -> 111.
# Exit 0 = PASS, Exit 1 = FAIL
# Note: total_allowed ceiling bumped 73 -> 96 by V312-17 round-16 (codex #89297)
#       which added 23 entries (round-16: 3418ac19a1, round-17: 628a621bd1).
#       Bumped 96 -> 97 by V312-59-B (claude-code #4385).
#       Bumped 97 -> 111 by V312-59-followup (claude-code #4419).
# V312-58-sprint5 / v313_3: +5 entries for sf1_bulk_load_bench,
# q4_sf1_real_perf_test and v313_3_profile_experiments #[ignore] markers.
# Bump 111 -> 116.
# V312-95-v2: +3 entries for v312_62_issue_batch_test.rs (PK-conflict
# in-tx regression post-PR #4723 task #22, GROUP_CONCAT semantics change
# post-PR #4690 task #21). All have KNOWN FOLLOW-UP #[ignore = "..."]
# reason text + tracked task/issue links. Bump 116 -> 119.
# V312-95-v2 (cont): +6 entries for mysql_tpch_test.rs (4 cross-engine
# parity tests gated on live MySQL infra) and sqlrustgo_cli_soak_e2e_test.rs
# (2 server column_def packet bug #3165). Bump 119 -> 125.
set -e

REGISTRY="tests/baseline/ignore_registry.json"
ACTIVE_MAX=47
TOTAL_ALLOWED_MAX=125

if [ ! -f "$REGISTRY" ]; then
    echo "FAIL: $REGISTRY not found" >&2
    exit 1
fi

# Read counts via python3 (avoid jq dependency)
read_counts() {
    python3 <<PYEOF
import json
with open("$REGISTRY") as f:
    data = json.load(f)
total_allowed = data.get("total_allowed", 0)
active = sum(1 for e in data.get("ignored_tests", []) if e.get("status") == "ACTIVE")
print(f"{active} {total_allowed}")
PYEOF
}

read_counts > /tmp/anti_ignore_counts.txt
ACTIVE=$(awk '{print $1}' /tmp/anti_ignore_counts.txt)
TOTAL_ALLOWED=$(awk '{print $2}' /tmp/anti_ignore_counts.txt)
rm -f /tmp/anti_ignore_counts.txt

echo "ignore_registry.json: total_allowed=$TOTAL_ALLOWED (max=$TOTAL_ALLOWED_MAX), active=$ACTIVE (max=$ACTIVE_MAX)"

if [ "$ACTIVE" -gt "$ACTIVE_MAX" ]; then
    echo "FAIL: active entries $ACTIVE > $ACTIVE_MAX" >&2
    exit 1
fi

if [ "$TOTAL_ALLOWED" -gt "$TOTAL_ALLOWED_MAX" ]; then
    echo "FAIL: total_allowed $TOTAL_ALLOWED > $TOTAL_ALLOWED_MAX" >&2
    exit 1
fi

exit 0
