#!/usr/bin/env bash
#
# check_test_target_lint_budget.sh — gate the lint debt of the **test** targets.
#
# #5465: the CI clippy step was
#
#     cargo clippy --all-features -- -D warnings
#
# which checks the library and the binaries, and **not** the 446 test
# targets. Every lint in `tests/` was therefore invisible to the gate —
# including a deny-by-default false positive
# (`clippy::approx_constant` in `v312_62_issue_batch_test.rs`) that only
# shows up under `--all-targets`.
#
# Adding `--all-targets` to the existing step is not an option on its own:
# the test tree carries a real lint backlog, and turning the whole CI red
# over it would stop the pipeline for a reason nobody can act on.
#
# So the debt is made **visible, quantified and non-growing** instead:
#
# 1. `--all-targets` must **compile**: any error is a hard failure. A test
#    target that does not compile is a test target nobody runs, and that is
#    a defect, not debt.
# 2. The number of lint warnings per lint name must not **exceed** the
#    recorded baseline. New warnings are red; the existing backlog is a
#    known, dated, reviewable quantity that can be paid down over time.
# 3. A lint name that is not in the baseline at all is red — otherwise the
#    debt could be traded from one lint to another.
#
# The baseline lives in `tests/baseline/test_target_lint_budget.json` and is
# regenerated **explicitly**:
#
#     bash scripts/gate/check_test_target_lint_budget.sh --update
#
# #5508: the scope above is the **root package** only. In a workspace whose
# root is also a package, cargo's default members are that one package — so
# `crates/*` (44 members, 301 test targets) had never been linted at all. The
# same rule is now available for the whole workspace:
#
#     bash scripts/gate/check_test_target_lint_budget.sh --scope workspace
#
# The rule is implemented once and parameterised by scope: same compile-must-
# pass rule, same "per lint name, not above the baseline" rule, same
# "a lint name absent from the baseline is red" rule, same "never re-baseline
# in compare mode" self-check. The workspace scope has its own baseline file
# (`tests/baseline/workspace_lint_budget.json`), because the two scopes have
# very different amounts of debt and one number for both would hide that.
#
# The baseline is **keyed by toolchain** (format version 2). The lint set a
# clippy produces is a property of the toolchain, and the CI runner image
# does not pin one: two runs an hour apart on the same label reported
# `rust-clippy/rust-1.95.0` and `rust-clippy/rust-1.91.0` respectively — the
# clippy diagnostic URL carries the toolchain version, which is how the drift
# was noticed. A baseline measured with one toolchain therefore cannot be
# compared against another:
#
#   * when the running toolchain has its **own** baseline entry, that entry is
#     compared strictly (per-lint names and counts, stale entries included);
#   * when it has none, the **union** of all recorded entries is used (per
#     lint name, the largest count seen in any toolchain) and the run says
#     so loudly. The comparison is still a real one — a lint name nobody has
#     ever recorded is red, and a count above every recorded maximum is red —
#     but a lint that simply does not exist in this toolchain is not counted
#     as "the baseline entry went stale".
#
# Either way the gate never re-baselines itself in compare mode, and the
# baseline records the toolchain every entry was measured with.
#
# Exit codes:
#   0  PASS — no compile error, no lint above the baseline
#   1  FAIL — see the message
#   2  usage error
#
# Refs: issue #5465 (root package), issue #5508 (whole workspace)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

SCOPE="root"
BASELINE="tests/baseline/test_target_lint_budget.json"
CARGO_SCOPE_ARGS=""
UPDATE=0

while [ $# -gt 0 ]; do
    case "$1" in
        --update) UPDATE=1 ;;
        --scope)
            shift
            case "${1:-}" in
                root)
                    SCOPE="root"
                    BASELINE="tests/baseline/test_target_lint_budget.json"
                    CARGO_SCOPE_ARGS=""
                    ;;
                workspace)
                    SCOPE="workspace"
                    BASELINE="tests/baseline/workspace_lint_budget.json"
                    CARGO_SCOPE_ARGS="--workspace"
                    ;;
                *) echo "usage: $0 [--scope root|workspace] [--update]" >&2; exit 2 ;;
            esac
            ;;
        --scope=*)
            case "${1#--scope=}" in
                root)
                    SCOPE="root"; BASELINE="tests/baseline/test_target_lint_budget.json"; CARGO_SCOPE_ARGS=""
                    ;;
                workspace)
                    SCOPE="workspace"; BASELINE="tests/baseline/workspace_lint_budget.json"; CARGO_SCOPE_ARGS="--workspace"
                    ;;
                *) echo "usage: $0 [--scope root|workspace] [--update]" >&2; exit 2 ;;
            esac
            ;;
        -h|--help) awk 'NR > 1 { if ($0 !~ /^#/) exit; sub(/^# ?/, ""); print }' "$0"; exit 0 ;;
        *) echo "usage: $0 [--scope root|workspace] [--update]" >&2; exit 2 ;;
    esac
    shift
done

if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    else
        echo "  [FAIL] cargo is not on PATH"
        exit 1
    fi
fi

if ! command -v python3 >/dev/null 2>&1; then
    echo "  [FAIL] python3 is not on PATH (needed to read cargo's JSON output)"
    exit 1
fi

echo "=== Test-Target Lint Budget Gate ==="
echo "  scope:    ${SCOPE}"
echo "  baseline: $BASELINE"

# The lint set a toolchain produces is a property of the toolchain, not only of
# the source. CI runs the pinned runner toolchain (rustc/clippy 1.95.0), so a
# baseline generated on a different version compares apples to oranges. The
# toolchain can therefore be pinned explicitly, and the recorded version is
# checked against the running one below.
CLIPPY_TOOLCHAIN="${SQLRUSTGO_CLIPPY_TOOLCHAIN:-}"
if [ -n "$CLIPPY_TOOLCHAIN" ]; then
    CARGO_CMD=(cargo "+${CLIPPY_TOOLCHAIN}")
    echo "  toolchain: pinned to ${CLIPPY_TOOLCHAIN}"
else
    CARGO_CMD=(cargo)
    echo "  toolchain: default (cargo on PATH)"
fi

# Self-check: without `--update`, this gate must not touch the baseline.
# The gate exists to keep the lint debt from growing; a gate that rewrites
# the baseline on every run compares nothing and is green by construction.
# (That is exactly the bug the batch-8 mutation proof found: the shell
# passed "0" for "no update", Python read it as a truthy string, and every
# run silently re-baselined instead of comparing.)
BASELINE_SHA_BEFORE=""
if [ -f "$BASELINE" ]; then
    BASELINE_SHA_BEFORE=$(sha256sum "$BASELINE" | awk '{print $1}')
fi

RAW="$(mktemp -t lint_budget.XXXXXX.json)"
trap 'rm -f "$RAW"' EXIT

# `--message-format=json` gives one JSON object per line, so the output can
# be read incrementally and a truncated run still yields partial data.
echo "  running: ${CARGO_CMD[*]} clippy ${CARGO_SCOPE_ARGS} --all-features --all-targets --message-format=json"
"${CARGO_CMD[@]}" clippy ${CARGO_SCOPE_ARGS} --all-features --all-targets --message-format=json >"$RAW" 2>/dev/null
CLIPPY_STATUS=$?

python3 - "$RAW" "$BASELINE" "$UPDATE" "$CLIPPY_STATUS" "$CLIPPY_TOOLCHAIN" "$SCOPE" <<'PYEOF'
import json, os, re, sys, subprocess, datetime

raw_path, baseline_path, update_arg, clippy_status = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
toolchain = sys.argv[5] if len(sys.argv) > 5 else ""
scope = sys.argv[6] if len(sys.argv) > 6 else "root"

# `--update` is passed as the string "1" or "0". The comparison must be an
# exact string comparison: a bare truthiness test would treat the "0" that
# the shell passes for "no update" as **true** (every non-empty string is
# truthy in Python), and the gate would then rewrite the baseline instead
# of comparing against it — silently green, every time.
#
# That is exactly the bug this gate is supposed to prevent, and it was
# found by the batch-8 mutation proof (M4): the gate reported success
# while the comparison never ran. The truthiness form is now rejected by
# the gate's own self-test.
update_mode = update_arg == "1"

errors = []
by_lint = {}
total = 0

with open(raw_path, "r", errors="replace") as fh:
    for line in fh:
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            obj = json.loads(line)
        except Exception:
            continue
        if obj.get("reason") != "compiler-message":
            continue
        msg = obj.get("message") or {}
        level = msg.get("level")
        code = ((msg.get("code") or {}) or {}).get("code")
        if level == "error":
            errors.append((code or "<error>", (msg.get("message") or "").strip()))
            continue
        if level != "warning" or not code:
            continue
        by_lint[code] = by_lint.get(code, 0) + 1
        total += 1

def rustc_version():
    # The version must come from the **same** toolchain the clippy run used.
    # Asking the default `rustc` would record 1.97 while the lints were
    # produced by 1.95, and the baseline would claim a toolchain it was not
    # measured with.
    cmd = ["cargo"]
    if toolchain:
        cmd.append("+" + toolchain)
    cmd.append("--version")
    try:
        out = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        return out.stdout.strip()
    except Exception:
        return "unknown"

def parse_version(rustc_str):
    """`cargo 1.95.0 (f2d3ce0bd 2026-03-21)` -> `1.95.0`."""
    m = re.search(r"cargo (\d+\.\d+\.\d+)", rustc_str or "")
    return m.group(1) if m else ""

now_rustc = rustc_version()
now_version = parse_version(now_rustc)

# --- compile errors are always fatal -------------------------------------
if errors:
    print("  [FAIL] the test targets do not compile under --all-targets:")
    seen = set()
    for code, text in errors:
        key = (code, text[:120])
        if key in seen:
            continue
        seen.add(key)
        print(f"         {code}: {text[:200]}")
    if update_mode:
        print("         (--update does not write a baseline for a tree that does not compile)")
    sys.exit(1)

print(f"  compile: OK (no errors)")
print(f"  lint warnings: {total} across {len(by_lint)} lint name(s)")

# --- baseline ------------------------------------------------------------
if update_mode:
    # Format version 2: the baseline is keyed by toolchain. An entry for
    # another toolchain is **preserved** — a toolchain bump must not throw
    # away the measurement of the toolchain it replaced.
    toolchains = {}
    if os.path.exists(baseline_path):
        try:
            with open(baseline_path) as fh:
                existing = json.load(fh)
            if isinstance(existing.get("toolchains"), dict):
                toolchains = existing["toolchains"]
        except Exception:
            toolchains = {}
    key = now_version or "unknown"
    toolchains[key] = {
        "rustc": now_rustc,
        "total": total,
        "by_lint": dict(sorted(by_lint.items())),
    }
    payload = {
        "version": 2,
        "updated_at": datetime.datetime.now(datetime.timezone.utc)
            .replace(microsecond=0).isoformat().replace("+00:00", "Z"),
        "toolchains": toolchains,
    }
    with open(baseline_path, "w") as fh:
        json.dump(payload, fh, indent=2, ensure_ascii=False, sort_keys=False)
        fh.write("\n")
    print(f"  [OK] baseline written: {baseline_path} (toolchain={key}, total={total})")
    sys.exit(0)

try:
    with open(baseline_path) as fh:
        baseline = json.load(fh)
except FileNotFoundError:
    print(f"  [FAIL] {baseline_path} does not exist.")
    print("         Generate it once with: bash scripts/gate/check_test_target_lint_budget.sh --update")
    sys.exit(1)
except Exception as exc:
    print(f"  [FAIL] {baseline_path} is not valid JSON: {exc}")
    sys.exit(1)

# How the baseline is compared depends on whether the running toolchain has
# an entry of its own.
#
# The lint set a toolchain produces is a property of the toolchain, not only
# of the source, and the CI runner image does not pin one: two runs an hour
# apart on the same runner label reported `rust-clippy/rust-1.95.0` and
# `rust-clippy/rust-1.91.0` (the clippy diagnostic URL carries the toolchain
# version). Comparing a 1.91 run against a 1.95 baseline would call every
# lint the older toolchain does not know about "the baseline entry went
# stale" — a false red that has nothing to do with the code.
#
# So:
#   * own entry  -> strict comparison (names, counts, and stale entries);
#   * no entry   -> the union of all recorded entries (per lint name, the
#                   largest count any toolchain recorded). The comparison is
#                   still real: a lint name nobody ever recorded is red, a
#                   count above every recorded maximum is red. Only the
#                   "the baseline entry went stale" rule is relaxed, because
#                   a lint that does not exist in this toolchain is not a
#                   regression.
#
# The baseline stays honest because every entry records the toolchain it was
# measured with, and the mode that was used is printed.
comparison_mode = "strict"
base_by_lint = {}
base_total = None
base_rustc = None

entries = baseline.get("toolchains")
if isinstance(entries, dict) and entries:
    if now_version and now_version in entries:
        entry = entries[now_version]
        base_by_lint = entry.get("by_lint") or {}
        base_total = entry.get("total")
        base_rustc = entry.get("rustc")
        comparison_mode = "strict"
    else:
        union = {}
        for name, entry in entries.items():
            for code, count in (entry.get("by_lint") or {}).items():
                union[code] = max(union.get(code, 0), count)
        base_by_lint = union
        base_total = sum(union.values())
        base_rustc = "union of: " + ", ".join(sorted(entries.keys()))
        comparison_mode = "union"
elif isinstance(baseline.get("by_lint"), dict):
    # Legacy single-toolchain baseline (format version 1).
    base_by_lint = baseline.get("by_lint") or {}
    base_total = baseline.get("total")
    base_rustc = baseline.get("rustc")
    comparison_mode = "legacy"

if not isinstance(base_by_lint, dict):
    print(f"  [FAIL] {baseline_path} has no usable `by_lint` map")
    sys.exit(1)

if comparison_mode == "union":
    print(f"  [NOTE] no baseline entry for the running toolchain ({now_rustc or 'unknown'});")
    print(f"         comparing against the union of the recorded toolchains")
    print(f"         ({base_rustc}). A lint name that no recorded toolchain ever")
    print(f"         produced is still red, and a count above every recorded")
    print(f"         maximum is still red. Record this toolchain explicitly with:")
    print(f"           SQLRUSTGO_CLIPPY_TOOLCHAIN={now_version} \\")
    print(f"             bash scripts/gate/check_test_target_lint_budget.sh --scope {scope} --update")
elif base_rustc and now_rustc != "unknown" and base_rustc != now_rustc:
    print(f"  [NOTE] toolchain mismatch:")
    print(f"         baseline entry was measured with: {base_rustc}")
    print(f"         this run used:                    {now_rustc}")
    print(f"         The lint set is a property of the toolchain, so the comparison")
    print(f"         below is approximate. Re-generate the baseline entry with:")
    print(f"           SQLRUSTGO_CLIPPY_TOOLCHAIN={now_version} \\")
    print(f"             bash scripts/gate/check_test_target_lint_budget.sh --scope {scope} --update")

violations = []
for code, count in sorted(by_lint.items()):
    allowed = base_by_lint.get(code)
    if allowed is None:
        violations.append(f"{code}: {count} warning(s) — this lint is NOT in the baseline")
    elif count > allowed:
        violations.append(f"{code}: {count} warning(s) — baseline allows {allowed}")

if comparison_mode != "union":
    for code, allowed in sorted(base_by_lint.items()):
        if code not in by_lint:
            violations.append(f"{code}: 0 warning(s) now, baseline allowed {allowed} (fine, but the baseline entry is stale)")

if violations:
    print(f"  [FAIL] {len(violations)} lint budget violation(s):")
    for v in violations:
        print(f"         {v}")
    print("         Pay the debt down, or (deliberately) re-baseline with --update.")
    sys.exit(1)

print(f"  [OK] no lint exceeds the baseline (total {total} <= {base_total}, comparison={comparison_mode})")
print("  [PASS] Test-Target Lint Budget Gate")
PYEOF
GATE_STATUS=$?

# The self-check: in compare mode the baseline must be byte-identical
# afterwards. If it changed, the gate re-baselined instead of comparing,
# and its verdict means nothing.
if [ "$UPDATE" -eq 0 ] && [ -n "$BASELINE_SHA_BEFORE" ] && [ -f "$BASELINE" ]; then
    BASELINE_SHA_AFTER=$(sha256sum "$BASELINE" | awk '{print $1}')
    if [ "$BASELINE_SHA_BEFORE" != "$BASELINE_SHA_AFTER" ]; then
        echo "  [FAIL] the gate modified $BASELINE while running in compare mode."
        echo "         A gate that re-baselines on every run compares nothing and is"
        echo "         green by construction."
        exit 1
    fi
    echo "  [OK] the baseline was not modified (compare mode)"
fi

if [ "$GATE_STATUS" -ne 0 ]; then
    exit "$GATE_STATUS"
fi
exit 0
