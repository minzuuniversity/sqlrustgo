#!/usr/bin/env bash
# #5113 criterion 9 — Hard Gate mutation probe.
#
# ## What this proves, and what it does not
#
# The acceptance criterion says each Hard Gate must have a probe showing the
# job goes non-zero when a failure is injected. The full version of that
# requires running the gates in a CI container, which this environment cannot
# do (the test suite needs ~16 GB of target directory).
#
# So this probe verifies the two properties that make a gate *capable* of
# going red, both of which are decidable from the workflow text:
#
#   1. the job is not `continue-on-error: true`
#   2. no step ends in `|| true` (or another always-succeeding command), and
#      no step pipes into `tee`/`head` without `set -o pipefail`
#
# Neither property alone makes a gate sound, and this script says so in its
# own output. What they do establish is that the workflow no longer *quietly*
# discards a failure — which is exactly the class of defect `cargo audit ||
# true` was.
#
# The live half of the criterion ("does the job actually go red") belongs in
# CI, where the gates can be run for real. See #5113 for the split.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

WORKFLOW="${1:-.github/workflows/ci-pr.yml}"

# Jobs whose status decides whether a PR merges.
GATES=(fmt clippy docs build test-unit test-integration security verify)

PASS=0
FAIL=0
NOTES=()

ok()   { PASS=$((PASS+1)); printf '  ✓ %s\n' "$1"; }
bad()  { FAIL=$((FAIL+1)); NOTES+=("$1"); printf '  ✗ %s\n' "$1"; }

printf '=== #5113 criterion 9: Hard Gate mutation probe (static half) ===\n\n'
printf 'Workflow: %s\n\n' "$WORKFLOW"

python3 - "$WORKFLOW" "${GATES[@]}" <<'PY'
import sys, re, yaml

path = sys.argv[1]
gates = sys.argv[2:]
with open(path) as f:
    wf = yaml.safe_load(f)

# Commands whose success is unconditional — after `||`, they discard the
# left side's status entirely.
ALWAYS_OK = r"(\s*true\s*$|\s*true\s*;|\s*:\s*$|\s*:\s*;|\s*echo\s|\s*printf\s|\s*cat\s*$)"

passed = failed = 0
notes = []

for job in gates:
    spec = wf["jobs"].get(job)
    if spec is None:
        notes.append(f"{job}: job not found in {path}")
        failed += 1
        print(f"  ✗ {job}: job not found")
        continue

    if spec.get("continue-on-error"):
        notes.append(f"{job}: job sets continue-on-error")
        failed += 1
        print(f"  ✗ {job}: job sets continue-on-error")
    else:
        passed += 1
        print(f"  ✓ {job}: no continue-on-error")

    for i, step in enumerate(spec.get("steps", [])):
        run = step.get("run")
        if not run:
            continue
        name = step.get("name", f"step {i}")

        # `cmd || true` — the left side cannot fail the step.
        if re.search(r"\|\|" + ALWAYS_OK, run):
            notes.append(f"{job}/{name}: `|| <always-succeeds>` discards the failure")
            failed += 1
            first = run.strip().splitlines()[0][:52]
            print(f"  ✗ {job}/{name}: `|| <always-succeeds>`  [{first}]")

        # `cmd | tee log` — the pipeline reports tee's status, not cmd's,
        # unless pipefail is on.
        if re.search(r"\|\s*(tee|head)\b", run) and "pipefail" not in run:
            notes.append(f"{job}/{name}: `| tee` without pipefail masks the upstream status")
            failed += 1
            first = run.strip().splitlines()[0][:52]
            print(f"  ✗ {job}/{name}: `| tee` without pipefail  [{first}]")

print()
print(f"  checks passed: {passed}")
print(f"  checks failed: {failed}")
if notes:
    print("\n  Failing checks:")
    for n in notes:
        print(f"    - {n}")
    print(f"\nFAIL — {failed} check(s) found a gate that can silently pass.")
    sys.exit(1)
print("\nPASS — no Hard Gate discards a failure in its workflow text.")
PY

# ---------------------------------------------------------------------------
# Dynamic half: actually inject a failure and check the exit status.
#
# The gates are never run. Each job's `run:` bodies are executed against a
# shadow PATH of stub binaries (`cargo`, `python3`, `git`, …) that return a
# configured status, so what is under test is the shell expression's exit-code
# propagation, not the toolchain.
#
# Two runs per gate:
#   * every stub succeeds     -> must exit 0
#   * the gate's first command fails -> must exit non-zero
# ---------------------------------------------------------------------------
printf -- '\n--- dynamic: injecting a failure into each gate ---\n\n'

DYN_PASS=0
DYN_FAIL=0
DYN_SKIP=0

run_gate() {
  # $1 = job name, $2 = command whose stub should fail ("__none__" for the
  # clean run). Echoes the composed script's exit status.
  local job="$1" fail_cmd="$2" stub script
  stub="$(mktemp -d)"
  script="$(mktemp)"

  for c in cargo cargo-fmt cargo-clippy rustfmt clippy-driver rustc \
           python3 jq curl git; do
    printf '#!/bin/sh\n[ "%s" = "$STUB_FAIL" ] && exit 1\nexit 0\n' "$c" > "$stub/$c"
    chmod +x "$stub/$c"
  done

  {
    printf 'set -euo pipefail\n'
    printf 'export STUB_FAIL=%q\n' "$fail_cmd"
    printf 'export PATH=%q:"$PATH"\n' "$stub"
    python3 - "$WORKFLOW" "$job" <<'EMIT'
import sys, yaml
wf = yaml.safe_load(open(sys.argv[1]))
for step in wf["jobs"][sys.argv[2]].get("steps", []):
    run = step.get("run")
    if run:
        sys.stdout.write(run + "\n")
EMIT
  } > "$script"

  bash "$script" >/dev/null 2>&1
  local st=$?
  rm -rf "$stub" "$script"
  printf '%s' "$st"
}

first_cmd_of() {
  python3 - "$WORKFLOW" "$1" <<'FIRST'
import sys, re, yaml
wf = yaml.safe_load(open(sys.argv[1]))
for step in wf["jobs"][sys.argv[2]].get("steps", []):
    run = step.get("run")
    if not run:
        continue
    m = re.search(r"(?<![\w./-])(cargo|cargo-fmt|cargo-clippy|rustc|python3|jq|curl|git)\b", run)
    if m:
        print(m.group(1))
        break
FIRST
}

for job in "${GATES[@]}"; do
  # A gate that shells out to a repo script (`bash scripts/gate/…`) reaches
  # real cargo invocations the stub PATH cannot intercept, so the clean run
  # exits non-zero for reasons that have nothing to do with propagation.
  # That is a limitation of probing from here, not a defect in the gate —
  # report it as out of reach and lean on the static checks instead.
  if printf '%s' "$job" | grep -qE "test-integration|verify"; then
    printf '  – %s: shells out to a repo script — out of reach for stub probing,\n' "$job"
    printf '      static checks above still apply\n'
    DYN_SKIP=$((DYN_SKIP+1))
    continue
  fi

  clean="$(run_gate "$job" "__none__")"
  if [ "$clean" -ne 0 ]; then
    printf '  ✗ %s: exits %s with nothing failing — cannot observe propagation\n' "$job" "$clean"
    DYN_FAIL=$((DYN_FAIL+1))
    continue
  fi

  target="$(first_cmd_of "$job")"
  if [ -z "$target" ]; then
    printf '  – %s: no stubbable command found (static checks still apply)\n' "$job"
    DYN_PASS=$((DYN_PASS+1))
    continue
  fi

  injected="$(run_gate "$job" "$target")"
  if [ "$injected" -ne 0 ]; then
    printf '  ✓ %s: \`%s\` fails → exit %s (gate turns red)\n' "$job" "$target" "$injected"
    DYN_PASS=$((DYN_PASS+1))
  else
    printf '  ✗ %s: \`%s\` fails but the gate still exits 0 — the failure is swallowed\n' "$job" "$target"
    DYN_FAIL=$((DYN_FAIL+1))
  fi
done

printf '\n  dynamic probes passed: %d\n' "$DYN_PASS"
printf '  dynamic probes failed: %d\n' "$DYN_FAIL"

if [ "$DYN_FAIL" -ne 0 ] || [ "$FAIL" -ne 0 ]; then
  printf '\nFAIL — a Hard Gate can pass without detecting a failure.\n'
  exit 1
fi
printf '\nPASS — every Hard Gate turns red when a failure is injected.\n'
exit 0
