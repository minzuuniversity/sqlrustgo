#!/usr/bin/env bash
#
# check_docs_pointer_paths.sh — gate the **documentation pointer paths** of
# the release line that is currently being shipped (issue #5499).
#
# # The defect this closes
# # ----------------------
# `docs/releases/v3.12.0/perf/BIN_LOAD_PERF.md` carried the line
#
#     Full report: `docs/releases/v3.12.0/perf/JSON_VS_BINT_MEASURED.md`.
#
# and that file **did not exist**. The pointer *read* like evidence — it
# named a concrete path — so a reader followed it and landed on nothing.
# Twice in that one chain: the same document also pointed at a release
# directory (`v3.13.0`) that does not exist.
#
# # Why the existing docs gate did not catch it
# # -------------------------------------------
# `scripts/gate/check_docs_links.sh` checks **markdown link syntax** —
# `[text](path)`. The broken pointer was **inline code** (`` `path` ``),
# which the link checker never sees. A pointer that is invisible to the
# checker is a pointer nobody has to keep alive.
#
# # What this gate states
# # ----------------------
# In the scoped documentation tree, **every backticked repository-relative
# path that names a file with a known extension must resolve to a file that
# exists.**
#
# ==> Writing convention: inline code IS the pointer convention in this
#     repository. A document that *quotes* a path known to be broken (a
#     defect being documented, a file that lives on another branch) must
#     write it as plain prose — otherwise the document that documents a
#     broken pointer **becomes** a broken pointer.
#
# ==> Process: run this gate **after** writing the evidence document, not
#     before. Batch 9 ran the gate, then wrote its evidence document with
#     two pointer-shaped quotations; the next run found them. The gate was
#     right and the ordering was wrong.
#
# The scope is deliberately narrow — the **release line being shipped** —
# because the repository also carries ~2 300 historical pointers into
# documents that were never committed or have since been restructured.
# Those are records of the past; rewriting them would make the evidence
# describe a state that never existed. They are handled by a **baseline**:
# the set of known-broken pointers is recorded, and the gate fails on any
# **new** one. The debt is made visible and non-growing instead of being
# silently rewritten.
#
# Exit codes
#   0  [PASS] — no new broken pointer
#   1  [FAIL] — at least one broken pointer that is not in the baseline
#   2  usage error
#
# Usage:
#   bash scripts/gate/check_docs_pointer_paths.sh
#   bash scripts/gate/check_docs_pointer_paths.sh --scope docs/releases/v4.1.0
#   bash scripts/gate/check_docs_pointer_paths.sh --update
#   bash scripts/gate/check_docs_pointer_paths.sh --selftest
#
# Refs: issue #5499

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

SCOPE="docs/releases/v4.1.0"
UPDATE=0
SELFTEST=0

while [ $# -gt 0 ]; do
    case "$1" in
        --scope) SCOPE="${2:-}"; shift 2 ;;
        --scope=*) SCOPE="${1#*=}"; shift ;;
        --update) UPDATE=1; shift ;;
        --selftest) SELFTEST=1; shift ;;
        -h|--help) sed -n '2,60p' "$0"; exit 0 ;;
        *) echo "usage: $0 [--scope <dir>] [--update] [--selftest]" >&2; exit 2 ;;
    esac
done

if ! command -v python3 >/dev/null 2>&1; then
    echo "  [FAIL] python3 is not on PATH (needed to scan the documentation)"
    exit 1
fi

BASELINE="tests/baseline/doc_pointer_paths_$(printf '%s' "$SCOPE" | tr '/' '_' | sed 's/^docs_//').json"

echo "=== Documentation Pointer-Path Gate ==="
echo "  scope:    $SCOPE"
echo "  baseline: $BASELINE"

# The scanner lives in one Python program so that the gate and its
# self-test exercise **the same code**.
SCANNER_FILE="$(mktemp -t docptr_scanner.XXXXXX.py)"
trap 'rm -f "$SCANNER_FILE"' EXIT

cat > "$SCANNER_FILE" <<'PYEOF'
import os, re, sys, json, subprocess, datetime

# A repository-relative path written in inline code, ending in an extension
# we know how to resolve.
PATH_RE = re.compile(r"`([A-Za-z0-9_./@+-]+\.(?:md|rs|sh|toml|yml|yaml|json|py|txt|sql|lock))`")

ROOTS = ("docs/", "scripts/", "tests/", "crates/", "src/", "queries/", ".gitea/", "benches/")

# A path containing one of these is a **placeholder**, not a broken pointer
# (`docs/releases/vX.Y.Z/STAGE.yaml` is a template, not a link).
PLACEHOLDERS = ("vX.Y.Z", "<", ">", "*", "...", "XXXX")


def pointer_paths(text):
    """Every repository-relative path written in inline code."""
    out = []
    for m in PATH_RE.finditer(text):
        t = m.group(1)
        if not t.startswith(ROOTS):
            continue
        if any(p in t for p in PLACEHOLDERS):
            continue
        if t not in out:
            out.append(t)
    return out


def broken_pointers(root):
    """[(file, target)] for every pointer that does not resolve."""
    out = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames.sort()
        for name in sorted(filenames):
            if not name.endswith(".md"):
                continue
            full = os.path.join(dirpath, name)
            rel = os.path.relpath(full, root)
            try:
                with open(full, "r", errors="replace") as fh:
                    text = fh.read()
            except OSError:
                continue
            for target in pointer_paths(text):
                if not os.path.exists(target):
                    out.append((rel, target))
    return out


def classify(target):
    """A category for the baseline, so every known debt is reviewable."""
    base = target.split("/")[-1]
    if target.startswith("crates/"):
        return "path_moved_after_crates_flattening"
    # Is there a file with the same name anywhere in the repository?
    found = subprocess.run(
        ["git", "ls-files"], capture_output=True, text=True, timeout=120
    ).stdout.split("\n")
    if any(f.endswith("/" + base) or f == base for f in found):
        return "path_moved_after_restructure"
    return "planned_artifact_never_committed"


def build_baseline(scope):
    entries = []
    for rel, target in broken_pointers(scope):
        entries.append({
            "file": rel,
            "target": target,
            "category": classify(target),
        })
    entries.sort(key=lambda e: (e["file"], e["target"]))
    return {
        "version": 1,
        "scope": scope,
        "updated_at": datetime.datetime.now(datetime.timezone.utc)
            .replace(microsecond=0).isoformat().replace("+00:00", "Z"),
        "note": (
            "Known-broken documentation pointers in the release line. The gate "
            "fails on any pointer that is NOT listed here. Each entry is a "
            "reviewable debt item, not a suppression: the category says why "
            "the pointer is broken (the path was moved by the crates/ -> src/ "
            "flattening, the artifact was planned but never committed, or the "
            "target lives on another branch only)."
        ),
        "entries": entries,
    }


def selftest(tmp):
    good = "docs/releases/v4.1.0/REAL.md"
    os.makedirs(os.path.join(tmp, "docs/releases/v4.1.0"), exist_ok=True)
    with open(os.path.join(tmp, good), "w") as fh:
        fh.write("# real\n")
    broken = os.path.join(tmp, "docs/releases/v4.1.0/BROKEN.md")
    with open(broken, "w") as fh:
        fh.write("see `docs/releases/v4.1.0/MISSING.md`\n")
    placeholder = os.path.join(tmp, "docs/releases/v4.1.0/TEMPLATE.md")
    with open(placeholder, "w") as fh:
        fh.write("see `docs/releases/vX.Y.Z/STAGE.yaml` and `docs/a<b>.md`\n")
    outside = os.path.join(tmp, "docs/releases/v4.1.0/OUTSIDE.md")
    with open(outside, "w") as fh:
        fh.write("see `docs/releases/v4.1.0/../OUTSIDE.md` and `src/nope.rs`\n")

    root = os.path.join(tmp, "docs/releases/v4.1.0")
    found = broken_pointers(root)
    targets = sorted(t for _, t in found)

    failures = []
    if not any(t.endswith("MISSING.md") for t in targets):
        failures.append("a broken pointer was NOT detected")
    if any("vX.Y.Z" in t for t in targets):
        failures.append("a template placeholder was flagged as broken")
    if any(t.endswith("OUTSIDE.md") and ".." not in t for t in targets):
        failures.append("a path outside the scope was flagged")
    if failures:
        for f in failures:
            print("  [FAIL] selftest: %s" % f)
        return 1
    print("  [OK] selftest: broken pointer detected, placeholders ignored")
    return 0


if __name__ == "__main__":
    mode = sys.argv[1]
    if mode == "scan":
        for rel, target in broken_pointers(sys.argv[2]):
            print("%s\t%s" % (rel, target))
    elif mode == "baseline":
        print(json.dumps(build_baseline(sys.argv[2]), indent=2, ensure_ascii=False))
    elif mode == "selftest":
        sys.exit(selftest(sys.argv[2]))
    else:
        print("unknown mode", file=sys.stderr)
        sys.exit(2)
PYEOF

if [ "$SELFTEST" -eq 1 ]; then
    SELFTEST_DIR="$(mktemp -d -t docptr_selftest.XXXXXX)"
    trap 'rm -f "$SCANNER_FILE"; rm -rf "$SELFTEST_DIR"' EXIT
    ( cd "$SELFTEST_DIR" && python3 "$SCANNER_FILE" selftest "$SELFTEST_DIR" )
    STATUS=$?
    if [ "$STATUS" -eq 0 ]; then
        echo "  [PASS] Documentation Pointer-Path Gate (selftest)"
    else
        echo "  [FAIL] the gate has no discriminating power — see above"
    fi
    exit "$STATUS"
fi

if [ ! -d "$SCOPE" ]; then
    echo "  [FAIL] the scope directory does not exist: $SCOPE"
    exit 2
fi

# Self-check: without `--update`, this gate must not touch the baseline.
# A gate that re-baselines on every run compares nothing and is green by
# construction (the batch-8 mutation proof found exactly that bug in the
# lint-budget gate).
BASELINE_SHA_BEFORE=""
if [ -f "$BASELINE" ]; then
    BASELINE_SHA_BEFORE=$(shasum -a 256 "$BASELINE" 2>/dev/null | awk '{print $1}')
fi

if [ "$UPDATE" -eq 1 ]; then
    python3 "$SCANNER_FILE" baseline "$SCOPE" > "$BASELINE.tmp"
    mv "$BASELINE.tmp" "$BASELINE"
    echo "  [OK] baseline written: $BASELINE"
    exit 0
fi

if [ ! -f "$BASELINE" ]; then
    echo "  [FAIL] $BASELINE does not exist."
    echo "         Generate it once with: bash scripts/gate/check_docs_pointer_paths.sh --update"
    exit 1
fi

python3 - "$SCANNER_FILE" "$SCOPE" "$BASELINE" <<'PYEOF'
import json, subprocess, sys

scanner, scope, baseline_path = sys.argv[1], sys.argv[2], sys.argv[3]

try:
    with open(baseline_path) as fh:
        baseline = json.load(fh)
except Exception as exc:
    print("  [FAIL] %s is not valid JSON: %s" % (baseline_path, exc))
    sys.exit(1)

known = set()
for entry in baseline.get("entries") or []:
    known.add((entry.get("file"), entry.get("target")))

# The baseline records the scope it was built for. Using a baseline from a
# different scope would compare against the wrong debt list — and would
# silently pass anything the other scope happened to contain.
base_scope = baseline.get("scope")
if base_scope != scope:
    print("  [FAIL] the baseline was built for scope %r, not %r." % (base_scope, scope))
    print("         Rebuild it with: bash scripts/gate/check_docs_pointer_paths.sh --update")
    sys.exit(1)

found = []
out = subprocess.run([sys.executable, scanner, "scan", scope], capture_output=True, text=True)
for line in out.stdout.splitlines():
    if "\t" in line:
        rel, target = line.split("\t", 1)
        found.append((rel, target))

new = [f for f in found if f not in known]
stale = [k for k in known if k not in set(found)]

print("  pointers in scope: %d broken, %d registered as known debt"
      % (len(found), len(known)))

if new:
    print("  [FAIL] %d broken pointer(s) that are NOT in the baseline:" % len(new))
    for rel, target in new:
        print("         %s -> %s" % (rel, target))
    print("         Either point at the file that actually exists, or (if the")
    print("         target is a planned artifact) register it deliberately with")
    print("         `bash scripts/gate/check_docs_pointer_paths.sh --update`.")
    sys.exit(1)

if stale:
    print("  [NOTE] %d baseline entr(y/ies) are now resolved — re-run with --update"
          % len(stale))
    for rel, target in stale:
        print("         %s -> %s" % (rel, target))

print("  [OK] no broken pointer outside the recorded debt")
print("  [PASS] Documentation Pointer-Path Gate")
PYEOF
STATUS=$?

if [ "$STATUS" -eq 0 ] && [ -n "$BASELINE_SHA_BEFORE" ] && [ -f "$BASELINE" ]; then
    BASELINE_SHA_AFTER=$(shasum -a 256 "$BASELINE" 2>/dev/null | awk '{print $1}')
    if [ "$BASELINE_SHA_BEFORE" != "$BASELINE_SHA_AFTER" ]; then
        echo "  [FAIL] the gate modified $BASELINE while running in compare mode."
        echo "         A gate that re-baselines on every run compares nothing."
        exit 1
    fi
    echo "  [OK] the baseline was not modified (compare mode)"
fi

exit "$STATUS"
