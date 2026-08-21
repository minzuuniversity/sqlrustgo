# Design — V312-59-E Thresholds Override Gate

## Architecture

```
┌────────────────────────────────────────────────────────────┐
│  scripts/gate/check_beta_v3.12.0.sh                        │
│    ├─ B1_CLIPPY                                             │
│    ├─ B2_INTEGRATION_TEST                                   │
│    ├─ ...                                                   │
│    └─ B8_THRESHOLDS_OVERRIDE  ← NEW                        │
│         ├─ check_v312_gate_thresholds.sh                   │
│         └─ check_v312_stage_yaml_sync.sh                   │
└────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────────┐
│  check_v312_gate_thresholds.sh                             │
│    Section 1: 13 boolean/numeric field checks              │
│      • parse STAGE.yaml → all 13 fields non-empty & valid   │
│    Section 2: 8 executable gate checks                     │
│      • COVERAGE_MIN_PER_CRATE → check_coverage_v312.sh     │
│      • GMP_AUDIT_TAMPER_TEST → check_audit_hash_chain.sh    │
│      • GMP_RETRIEVAL_CITATION → check_hybrid_retrieval.sh   │
│      • MIXED_SOAK_HOURS → grep SOAK report for >=168h      │
│      • SQLLOGICTEST_SMOKE → B6_SQLLOGICTEST_SMOKE_GATE     │
│      • SQLLOGICTEST_SELECTED → B6_SQLLOGICTEST_SELECTED     │
│      • TPCH_SF1_CORRECTNESS → B6_TPCH_SF1_G4 22/22 check   │
│      • MYSQL_WIRE_E2E → check_mysql_wire_e2e.sh (NEW) │
│      • LOAD_DATA_BULK_IMPORT → check_load_data_bulk.sh     │
│      • CRASH_RECOVERY → check_crash_recovery.sh (NEW) │
│      • UPGRADE_DOWNGRADE → check_upgrade_downgrade.sh NEW │
│      • BUSTUBX_EDU_SQLITE_CLI → check_bustubx_edu_cli.sh   │
│      • GMP_CORPUS_UNCLASSIFIED → grep unclassified_count≤0 │
└────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────────┐
│  check_v312_stage_yaml_sync.sh                             │
│    • parse STAGE.yaml thresholds_override section          │
│    • parse check_v312_gate_thresholds.sh declared fields   │
│    • check: every gate field declared in STAGE.yaml        │
│    • check: every STAGE.yaml field checked by gate script  │
│    • FAIL on any drift                                     │
└────────────────────────────────────────────────────────────┘
```

## Decisions

### D1: Single composite script vs N independent scripts

**Decision**: Single composite script `check_v312_gate_thresholds.sh` with sections, not N independent scripts.

**Rationale**:
- Issue #4388 calls out 13 fields, but only 8 are independently executable. The rest are boolean/numeric checks that parse STAGE.yaml.
- Composite keeps the B8_THRESHOLDS_OVERRIDE invocation atomic (one PASS/FAIL).
- Each section can be invoked independently via `bash check_v312_gate_thresholds.sh --section COVERAGE` for debugging.

### D2: STAGE.yaml parsing approach

**Decision**: Use `yq` (yq-go from homebrew) for STAGE.yaml parsing.

**Rationale**:
- STAGE.yaml is well-formed YAML, not arbitrary text. yq is already in the project toolchain (used by other gate scripts).
- Avoid regex parsing fragility (issue precedent in #3887 case).
- Fall back to `python3 -c "import yaml; ..."` if yq unavailable.

### D3: Pass/fail contract

**Decision**: Each section outputs `[SECTION_NAME] PASS|FAIL` line, exit code 0 only if all PASS.

**Rationale**:
- Parsable by both humans and CI scrapers.
- Consistent with existing `B1_CLIPPY`, `B2_INTEGRATION_TEST` style in check_beta_v3.12.0.sh.

### D4: Sub-scripts ownership

**Decision**: Wire E2E / Crash Recovery / Upgrade-Downgrade get NEW sub-scripts only if they don't already exist. Reuse existing ones where possible.

**Rationale**:
- Avoid script proliferation.
- The three NEW gates (#4388 E-1) are placeholders — actual scripts may already exist under different names.

### D5: STAGE.yaml change scope

**Decision**: Add explicit references to `B8_THRESHOLDS_OVERRIDE` in `promotion_to_RC_requires` and `promotion_to_GA_requires` lists.

**Rationale**:
- Make the gate an explicit promotion gate (not implicit).
- Aligns with existing pattern of `B1_CLIPPY` etc. being referenced.

## File-by-file plan

### `scripts/gate/check_v312_gate_thresholds.sh` (new)

Length: ~250 lines bash.
- `set -euo pipefail`
- 13 field declarations at top (`FIELD_*` associative array)
- Function `_check_field()`: assert STAGE.yaml field meets expected value
- Function `_check_executable_gate()`: invoke and parse PASS/FAIL of inner script
- 13 section blocks, each outputting `[N/13 SECTION_NAME] PASS|FAIL`
- Final summary: `13/13 PASS` or `12/13 PASS, 1 FAIL`

### `scripts/gate/check_v312_stage_yaml_sync.sh` (new)

Length: ~80 lines bash.
- yq query STAGE.yaml `thresholds_override` keys → set A
- grep `check_v312_gate_thresholds.sh` declared fields → set B
- diff: `comm -23 <(echo A | sort) <(echo B | sort)` → must be empty
- diff: `comm -13 <(echo A | sort) <(echo B | sort)` → warn if non-empty

### `scripts/gate/check_beta_v3.12.0.sh` (modify)

Length: +15 lines.
- Add new section `B8_THRESHOLDS_OVERRIDE` matching pattern of B1/B2.
- Call `bash scripts/gate/check_v312_gate_thresholds.sh`.
- Call `bash scripts/gate/check_v312_stage_yaml_sync.sh`.
- Increment gate count from 40 → 41.

### `docs/releases/v3.12.0/STAGE.yaml` (modify)

Length: +2 lines (text refs).
- In `promotion_to_RC_requires`: append `- "B8_THRESHOLDS_OVERRIDE: 13/13 boolean + executable gates PASS"`
- In `promotion_to_GA_requires`: append `- "B8_THRESHOLDS_OVERRIDE: 13/13 PASS at GA cut time"`

### `docs/releases/v3.12.0/evidence/v312-59-e/EVIDENCE.md` (new)

Length: ~100 lines markdown.
- For each of 13 fields: STAGE.yaml line number + gate script + last run output + verdict.

## Test plan

- `bash scripts/gate/check_v312_gate_thresholds.sh` → expect 13/13 PASS (or documented FAIL with reason)
- `bash scripts/gate/check_v312_stage_yaml_sync.sh` → expect PASS
- `bash scripts/gate/check_beta_v3.12.0.sh` → expect 41/41 PASS (was 40/40)

## Risk

- **R1**: Some new sub-gate scripts may not yet exist (e.g., wire E2E standalone). Mitigation: section reports "INFRASTRUCTURE_MISSING" with explicit remediation issue link.
- **R2**: STAGE.yaml parsing edge cases (multi-line values, anchors). Mitigation: use `yq` first, fallback to `python3 -c "import yaml; ..."`.
- **R3**: Gate count increment from 40 → 41 may break external dashboards. Mitigation: search dashboards before bumping, update after.