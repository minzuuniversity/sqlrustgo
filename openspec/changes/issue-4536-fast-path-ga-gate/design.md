# Design: --fast-path flag for check_ga_v3.12.0.sh (Issue #4536)

## Overview

Add a CLI flag to `scripts/gate/check_ga_v3.12.0.sh` that lets local
developers skip the heavy BETA stage execution while preserving
full backward compatibility for CI.

## Behavior matrix

| Invocation | BETA behavior | RC / GA / thresholds_override |
|------------|---------------|--------------------------------|
| `bash check_ga_v3.12.0.sh` (no flag) | **heavy** (legacy default) | fast-path (existing) |
| `bash check_ga_v3.12.0.sh --full`   | heavy | fast-path (existing) |
| `bash check_ga_v3.12.0.sh --fast-path` | **fast-path only** | fast-path (existing) |
| `bash check_ga_v3.12.0.sh --help`   | print usage + exit 0 | n/a |

## Implementation sketch

### Step 1: add CLI parser block

Insert after line 49 (color vars) and before line 50 (counters):

```bash
# CLI flags
MODE="full"  # default: backward-compatible
while [[ $# -gt 0 ]]; do
    case "$1" in
        --fast-path) MODE="fast"; shift ;;
        --full)      MODE="full"; shift ;;
        --help|-h)
            sed -n '2,28p' "$0" | sed 's/^# \{0,1\}//'
            echo ""
            echo "Modes:"
            echo "  (default | --full)   run heavy BETA stage (CI parity)"
            echo "  --fast-path          skip BETA heavy run (local-dev only)"
            exit 0
            ;;
        *)
            echo "[ERROR] unknown arg: $1" >&2
            echo "  try --help for usage" >&2
            exit 2
            ;;
    esac
done
```

### Step 2: refactor `run_beta_gate()` to branch on `$MODE`

Inside `run_beta_gate()` (lines 99-125), wrap the heavy execution
in an `if` branch:

```bash
run_beta_gate() {
    log_step "BETA" "v3.12.0 BETA promotion_to_BETA_requires (40/40)"
    local beta_script="$SCRIPT_DIR/check_beta_v3.12.0.sh"
    if [ ! -f "$beta_script" ]; then
        log_fail "BETA gate script missing: $beta_script"
        BETA_BLOCKERS=$((BETA_BLOCKERS + 40))
        return
    fi

    # Fast-path: syntax check only (analogous to RC / GA fast-path at
    # lines 162-170). Heavy execution is delegated to CI.
    if [ "$MODE" = "fast" ]; then
        if bash -n "$beta_script" 2>/dev/null; then
            log_pass "BETA gate (fast-path: script syntax OK)"
            BETA_PASS=40
            # Mark evidence as fast-path so reviewers can tell.
            BETA_EVIDENCE_HASH="fast-path-no-evidence"
        else
            log_fail "BETA gate (fast-path: script syntax error)"
            BETA_BLOCKERS=$((BETA_BLOCKERS + 40))
        fi
        return
    fi

    # Heavy path (default / --full): run the BETA gate script
    local beta_log="$EVIDENCE_DIR/ga_beta_gate_$(date +%Y%m%d_%H%M%S).log"
    if bash "$beta_script" > "$beta_log" 2>&1; then
        log_pass "BETA gate (exit 0): see $beta_log"
        BETA_PASS=40
        BETA_EVIDENCE_HASH=$(evidence_hash "$beta_log")
    else
        local pass_lines fail_lines warn_lines
        pass_lines=$(grep -c "^\s*\[PASS\]" "$beta_log" 2>/dev/null || echo "0")
        fail_lines=$(grep -c "^\s*\[FAIL\]" "$beta_log" 2>/dev/null || echo "0")
        warn_lines=$(grep -c "^\s*\[WARN\]" "$beta_log" 2>/dev/null || echo "0")
        BETA_PASS=$pass_lines
        BETA_BLOCKERS=$fail_lines
        log_warn "BETA gate partial: PASS=$pass_lines FAIL=$fail_lines WARN=$warn_lines (see $beta_log)"
        BETA_EVIDENCE_HASH=$(evidence_hash "$beta_log")
    fi
}
```

### Step 3: header banner + hint

Insert in `main()` (line 337) before the stage calls, after the
opening banner:

```bash
if [ "$MODE" = "full" ]; then
    log_warn "Running heavy BETA stage (cargo build + clippy + fmt). Use --fast-path on dev laptops to skip."
fi
```

### Step 4: JSON report field

Add a top-level `"mode"` field to the JSON report (lines 383-424):

```json
"mode": "fast-path"   # or "full"
```

so reviewers can tell whether the verdict came from a fast-path
or heavy run.

## Non-design

- We do **not** refactor `check_beta_v3.12.0.sh` to split into
  `check_beta_v3.12.0.sh` (full) and `check_beta_v3.12.0_fast.sh`
  (syntax-only). Issue #4536 §"建议" mentions this as an option
  but it's a larger change; the `--fast-path` aggregator flag
  achieves the UX goal without touching the BETA script.
- We do **not** change the default behavior — `--full` is the
  default to preserve CI parity. The issue body explicitly asks
  for backward compatibility.

## Test matrix (verification on dev laptop)

| Step | Command | Expected |
|------|---------|----------|
| 1 | `bash scripts/gate/check_ga_v3.12.0.sh --help` | exit 0, prints usage |
| 2 | `time bash scripts/gate/check_ga_v3.12.0.sh --fast-path` | exit 0/1, **<30s** wall clock |
| 3 | `time bash scripts/gate/check_ga_v3.12.0.sh` (default = full) | exit 0/1, >3 min wall clock |
| 4 | `cat docs/releases/v3.12.0/evidence/v312-59/ga_gate_report.json \| python3 -m json.tool` | new `"mode": "fast-path"` or `"full"` field |
| 5 | `bash -n scripts/gate/check_ga_v3.12.0.sh` | exit 0 (sanity check) |

## Failure modes

| Mode | 缓解 |
|------|------|
| `--fast-path` accidentally used in CI | JSON report `"mode": "fast-path"` field flags it; CI gate in `.gitea/workflows/` can reject fast-path verdicts |
| `--fast-path` syntax check passes but BETA actually broken | Fast-path is dev-UX only; CI continues to run full BETA via `check_beta_v3.12.0.sh` directly, not via the aggregator |
| Counter logic drifts from heavy path | Same counters (`BETA_PASS=40`, `BETA_BLOCKERS=0`) in both branches; downstream JSON shape unchanged |