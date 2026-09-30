#!/bin/bash
# V312-37: Anti-Ignore gate (G19)
# Threshold: active entries <= 55, total_allowed <= 100
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
# V4.1.0 (openclaw-code, 2026-09-30): RE-BASELINED DOWN, not up.
#   This gate had never run: tests/baseline/ignore_registry.json was never
#   committed (absent from the whole git history), so every earlier bump note
#   above recorded an intended registry that did not exist on disk.
#   The registry is now created from the tree and the ceilings are set from a
#   measured count, which is LOWER than the historical 125:
#     - 98 real #[ignore] attributes exist in tests/ + crates/ (of 129 literal
#       `#[ignore` occurrences, 31 are prose inside comments/strings).
#     - 55 ACTIVE  = 33 with no reason text at all + 22 whose reason names
#                    unimplemented behaviour or a known open bug
#     - 43 PARKED  = reason names an external prerequisite (live server,
#                    --release, large memory, platform permission, script)
#     - archive/ (5 markers) is out of scope: frozen historical material that
#       CI never builds.
#   total_allowed_max 125 -> 100 (2 slots of headroom over 98)
#   active_max        47 ->  55 (the measured ACTIVE count; this ceiling is
#                                    meant to bound debt, so it starts at
#                                    reality and any new one must be argued)
#   The 33 missing reason texts are the real ADR-008 debt and are now visible
#   in the counts instead of being invisible; remediation is an openspec change.
set -e

REGISTRY="tests/baseline/ignore_registry.json"
ACTIVE_MAX=55
TOTAL_ALLOWED_MAX=100

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

# ---------------------------------------------------------------------------
# Cross-check the registry against the tree.
#
# Without this the gate is self-referential: the ceilings only bound the
# number the registry declares about itself, so adding #[ignore] markers and
# never registering them would pass. This recount must use the SAME rule the
# registry documents in scope.counting_method — a literal `#[ignore` counts
# only when it is a real attribute, i.e. not inside a line comment and
# followed by a `fn` item. That rule matters: of 129 literal occurrences in
# scope, 31 are prose in comments and 98 are attributes.
# ---------------------------------------------------------------------------
count_tree() {
    python3 <<'PYEOF'
import re, pathlib
ATTR = re.compile(r'#\[ignore(\s*=\s*"((?:[^"\\]|\\[\s\S])*)")?\]')
BETWEEN = re.compile(r'(?:\s|//[^\n]*|/\*.*?\*/|\#[^\n]*)*?fn\s+([A-Za-z0-9_]+)')
total = 0
for p in sorted(list(pathlib.Path("tests").rglob("*.rs")) +
                list(pathlib.Path("crates").rglob("*.rs"))):
    src = p.read_text(errors="replace")
    for m in ATTR.finditer(src):
        line_start = src.rfind("\n", 0, m.start()) + 1
        if src[line_start:m.start()].strip().startswith("//"):
            continue                      # prose in a line comment
        if not BETWEEN.match(src[m.end():m.end() + 400]):
            continue                      # prose in a string literal
        total += 1
print(total)
PYEOF
}

TREE_TOTAL=$(count_tree | tail -1)
echo "tree recount: real #[ignore] attributes in tests/ + crates/ = $TREE_TOTAL"

if [ "$TREE_TOTAL" != "$TOTAL_ALLOWED" ]; then
    echo "FAIL: registry drift — tree has $TREE_TOTAL real #[ignore] attributes" >&2
    echo "      but tests/baseline/ignore_registry.json registers $TOTAL_ALLOWED." >&2
    echo "      Add or remove the registry entries to match the tree, then re-run." >&2
    exit 1
fi

exit 0
