## Why

`scripts/gate/check_g13_stability.sh` invokes G7 and checks for success with:

```bash
G7_RESULT=$(bash scripts/gate/check_p13_soak_test.sh 2>&1 | tail -3 || true)
if echo "$G7_RESULT" | grep -q "PASS"; then
    echo "  [4/7] ✅ PASS: G7 Soak gate (unit-level) PASS"
else
    echo "  ❌ FAIL: G7 Soak gate did not pass"
    exit 1
fi
```

The grep targets the literal string `PASS` in `tail -3` of G7's output. But G7's final lines are:

```
=== G7 Gate: PASS ===
P1-3 (#3175) Soak Test: Layer 1 (harness) + Layer 2/3 (mysqlslap scripts) verified

Next steps for full soak:
  1. bash scripts/soak/prepare_sf01_data.sh
  ...
```

`tail -3` captures the **"Next steps"** block, which contains no `PASS`. So G7 always reports FAIL to G13, even when G7 itself passed (11/11). The G13 gate has been reporting a false negative on every run since this heuristic was added.

## What Changes

- Replace the grep heuristic with **exit-code inspection**: capture G7's actual exit code with `set +e; bash ...; G7_EXIT=$?; set -e` and check `$G7_EXIT -eq 0`. This is the canonical "did the previous command succeed" pattern in shell.
- Print G7's full output on failure (not just the last 3 lines) so the operator can debug what actually went wrong.
- Add a self-test: `check_g13_stability.sh` invokes G7 directly and verifies the exit-code path; a mock G7 with `exit 0` makes G13 pass, `exit 1` makes G13 fail.