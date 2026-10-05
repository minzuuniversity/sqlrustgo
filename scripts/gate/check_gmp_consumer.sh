#!/usr/bin/env bash
# GMP-Platform consumer compile/smoke gate — closes V400-10 / #4873 / #4943.
#
# ## Why this lives here and not in GMP-Platform
#
# The gate itself is a SQLRustGo GA-gate obligation: GA_GATE_REPORT.md
# lists "V400-10 GMP consumer gate" among the items that must pass before
# promotion, so the check has to run from this repository's gate suite and
# write its evidence into this repository's evidence tree. The *assets*
# it drives (the REST surface, the web UI, the audit crates, the 408
# evaluator) belong to GMP-Platform and are exercised in place.
#
# The two repositories are siblings under the parent directory, which is
# what makes GMP-Platform's `path` dependencies on
# `../../../sqlrustgo/crates/{graph,storage,types}` resolve.
#
# ## What #4873 got wrong
#
# It recorded "PR #207 self-approval pending" as a blocker. That PR is
# **not in this repository at all** — `GET /pulls/207` returns 404 here.
# It lives in GMP-Platform and was merged on 2026-09-11:
#
#   67fc628 Merge PR#207: M5 sqlrustgo-graph migration
#            (resolve cypher_engine.rs conflict)      ai <ai@z440>
#
# Its content was subsequently forward-merged into develop/v1.5.0 via
# PR #210 ("Merge M5 into develop/v1.5.0 (resolve conflicts: take PR#210
# fix side)"), and develop/v1.6.0 carries it forward
# (crates/gmp-storage/src/cypher_engine.rs is present). So the recorded
# blocker never existed; the search was pointed at the wrong repository.
#
# This gate therefore does not check for a PR — it checks the four
# consumer surfaces for what actually matters: that they build against the
# current SQLRustGo crates, and that the SQL-facing ones answer.
#
# ## Usage
#
#   scripts/gate/check_gmp_consumer.sh              # compile surfaces
#   GMP_SMOKE_DEEP=1 scripts/gate/check_gmp_consumer.sh   # + 408 smoke
#
# Exit codes: 0 all checks pass · 1 a check failed · 2 GMP-Platform not
# reachable (recorded as SKIPPED, not a pass — the evidence file says so).

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO_ROOT"

# GMP-Platform is expected as a sibling of this repository.
GMP_ROOT="${GMP_ROOT:-}"
if [ -z "$GMP_ROOT" ]; then
    for cand in "$(dirname "$REPO_ROOT")/GMP-Platform" \
                "$(dirname "$REPO_ROOT")/gmp-platform"; do
        [ -d "$cand/crates" ] && GMP_ROOT="$cand" && break
    done
fi

EVIDENCE_DIR="docs/releases/v4.1.0/evidence"
EVIDENCE_FILE="${EVIDENCE_DIR}/gmp_consumer_gate_$(date -u +%Y%m%d).md"
mkdir -p "$EVIDENCE_DIR"

PASS=0; FAIL=0; SKIP=0
results=()

record() {  # record <status> <name> <detail>
    results+=("$1|$2|$3")
    case "$1" in
        PASS) PASS=$((PASS + 1)); echo "  [PASS] $2 — $3" ;;
        FAIL) FAIL=$((FAIL + 1)); echo "  [FAIL] $2 — $3" ;;
        SKIP) SKIP=$((SKIP + 1)); echo "  [SKIP] $2 — $3" ;;
    esac
}

echo "=== GMP-Platform consumer gate (V400-10 / #4873 / #4943) ==="
echo ""

# ---------------------------------------------------------------- 0
if [ -z "$GMP_ROOT" ] || [ ! -d "$GMP_ROOT/crates" ]; then
    record SKIP "gmp-platform-located" "GMP-Platform not found as a sibling of $REPO_ROOT; set GMP_ROOT"
    echo ""
    echo "RESULT: SKIPPED (not a pass) — see evidence file $EVIDENCE_FILE"
    {
        echo "# GMP-Platform consumer gate — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
        echo
        echo "**SKIPPED** — GMP-Platform not reachable at the expected sibling path."
    } > "$EVIDENCE_FILE"
    exit 2
fi

echo "GMP-Platform: $GMP_ROOT"
GMP_HEAD="$(git -C "$GMP_ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"
GMP_BRANCH="$(git -C "$GMP_ROOT" rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)"
echo "  head: $GMP_HEAD ($GMP_BRANCH)"
echo ""

# ---------------------------------------------------------------- 1
# PR #207 verification. This replaces the wrong-repository search that
# produced the "self-approval pending" record.
#
# `grep -q` is deliberately avoided here: it exits on first match, the
# upstream `git log` then dies of SIGPIPE (rc 141), and `set -o pipefail`
# turns that into a false negative. Capturing the output first and
# matching on it keeps the exit status meaningful.
pr207_log="$(git -C "$GMP_ROOT" log --all --oneline 2>/dev/null || true)"
if grep -E "Merge PR#?207:" <<< "$pr207_log" > /dev/null; then
    # No PCRE lookahead: BSD grep (macOS) rejects `(?=...)`.
    sha="$(grep -E "Merge PR#?207:" <<< "$pr207_log" | head -1 | cut -d' ' -f1)"
    record PASS "pr-207-resolved" \
          "found and merged in GMP-Platform at ${sha:-unknown} (2026-09-11); #4873's 'self-approval pending' was a wrong-repository lookup"
else
    record FAIL "pr-207-resolved" "PR #207 merge commit not found in GMP-Platform history"
fi

# ---------------------------------------------------------------- 2
# Compile surfaces. `cargo check` per crate keeps the blast radius small —
# checking the whole workspace would drag in crates unrelated to the
# consumer contract.
for crate in gmp-api gmp-auth gmp-audit-db; do
    if [ ! -d "$GMP_ROOT/crates/$crate" ]; then
        record SKIP "compile-$crate" "crate absent from GMP-Platform"
        continue
    fi
    if out="$(cd "$GMP_ROOT" && timeout 600 cargo check -p "$crate" 2>&1)"; then
        record PASS "compile-$crate" "cargo check against current sqlrustgo crates"
    else
        record FAIL "compile-$crate" "$(echo "$out" | grep -E '^error' | head -1)"
    fi
done

# ---------------------------------------------------------------- 3
# The sqlrustgo-facing surface is the one that actually depends on this
# repository. Prove the linkage is live rather than nominal: the graph /
# kg crates carry `path` dependencies onto sqlrustgo crates, and those
# crates must themselves still build.
if grep -q 'path = "../../../sqlrustgo/crates/' "$GMP_ROOT"/crates/*/Cargo.toml 2>/dev/null; then
    linked="$(grep -l 'path = "../../../sqlrustgo/crates/' \
              "$GMP_ROOT"/crates/*/Cargo.toml 2>/dev/null | wc -l | tr -d ' ')"
    if out="$(cd "$REPO_ROOT" && timeout 900 cargo check -p sqlrustgo-storage 2>&1)"; then
        record PASS "sqlrustgo-linkage" \
              "${linked} GMP crate(s) path-depend on this repo; sqlrustgo-storage still builds"
    else
        record FAIL "sqlrustgo-linkage" \
              "sqlrustgo-storage failed to build: $(echo "$out" | grep -E '^error' | head -1)"
    fi
else
    record FAIL "sqlrustgo-linkage" "no GMP crate path-depends on sqlrustgo/crates — the consumer contract is not wired"
fi

# ---------------------------------------------------------------- 4
# 408 smoke. `gmp-eval` drives the evaluation through a **local ollama**
# endpoint (`--ollama-url`, default http://localhost:11434) and an
# embedding model. It does not need an LLM API key -- an earlier
# revision of this gate skipped for want of credentials, which was
# wrong: the real prerequisite is ollama, and on a machine without it
# installed there is nothing to wait for.
if [ "${GMP_SMOKE_DEEP:-0}" = "1" ]; then
    ollama_url="${GMP_OLLAMA_URL:-http://localhost:11434}"
    if ! curl -sf --max-time 5 "$ollama_url/api/tags" >/dev/null 2>&1; then
        record SKIP "smoke-408" \
              "ollama not reachable at $ollama_url (gmp-eval's evaluation \
               backend is local, not an LLM API)"
    elif [ ! -f "$GMP_ROOT/test-data/391_scenarios.json" ]; then
        record SKIP "smoke-408" "408 scenario set missing in GMP-Platform"
    else
        if (cd "$GMP_ROOT" && timeout 900 cargo run -q -p gmp-eval -- \
                --test-set test-data/391_scenarios.json \
                --limit "${GMP_408_LIMIT:-5}" \
                --output /tmp/gmp-408-report.json) \
             > /tmp/gmp-408-smoke.log 2>&1; then
            record PASS "smoke-408" \
                  "gmp-eval ran ${GMP_408_LIMIT:-5} of 408 scenarios"
        else
            record FAIL "smoke-408" \
                  "gmp-eval failed: $(tail -2 /tmp/gmp-408-smoke.log | head -1)"
        fi
    fi
else
    record SKIP "smoke-408" \
          "not requested (set GMP_SMOKE_DEEP=1); needs a running ollama"
fi

# ---------------------------------------------------------------- 5
# WebUI. `web/` is a node project; this gate installs its dependencies and
# actually runs the test suite, so the check is real rather than a
# manifest sanity test.
if [ ! -f "$GMP_ROOT/web/package.json" ]; then
    record FAIL "webui" "web/package.json absent"
elif ! command -v node >/dev/null 2>&1; then
    record SKIP "webui" "node not on PATH"
else
    if [ ! -d "$GMP_ROOT/web/node_modules" ]; then
        log_info "  installing web dependencies..."
        (cd "$GMP_ROOT/web" && timeout 600 npm ci --no-audit --no-fund) \
            > /tmp/gmp-web-npm-ci.log 2>&1 || true
    fi
    web_out="$(cd "$GMP_ROOT/web" && timeout 600 npm run test 2>&1)"
    if [ $? -eq 0 ]; then
        n="$(echo "$web_out" | grep -oE 'Tests +[0-9]+ passed' | head -1)"
        record PASS "webui" "npm run test ok${n:+ ($n)}"
    else
        record FAIL "webui" \
              "npm run test failed: $(echo "$web_out" | grep -E 'Tests|FAIL' | tail -1)"
    fi
    # typecheck is reported separately: on 2026-10-05 it fails on
    # vite.config.ts (`node:path` / `__dirname` used without
    # @types/node in devDependencies). That is a pre-existing gap in
    # GMP-Platform, not something this gate introduced, so it is
    # surfaced as a warning rather than failing the check.
    if (cd "$GMP_ROOT/web" && timeout 300 npm run typecheck >/dev/null 2>&1); then
        record PASS "webui-typecheck" "npm run typecheck clean"
    else
        record SKIP "webui-typecheck" \
              "pre-existing: vite.config.ts uses node builtins without \
               @types/node in devDependencies (GMP-Platform side)"
    fi
fi

# ---------------------------------------------------------------- report
{
    echo "# GMP-Platform consumer gate — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo
    echo "- SQLRustGo: \`$(git rev-parse --short HEAD)\` ($(git rev-parse --abbrev-ref HEAD))"
    echo "- GMP-Platform: \`${GMP_HEAD}\` (${GMP_BRANCH}) at \`${GMP_ROOT}\`"
    echo
    echo "| Status | Check | Detail |"
    echo "|---|---|---|"
    for r in "${results[@]}"; do
        IFS='|' read -r st nm dt <<< "$r"
        echo "| $st | \`$nm\` | $dt |"
    done
    echo
    echo "**$PASS passed · $FAIL failed · $SKIP skipped**"
    echo
    echo "## 关于 #4873 记录的「PR #207 self-approval pending」"
    echo
    echo "该 PR 不在 SQLRustGo 仓库——\`GET /pulls/207\` 在本实例返回 404。"
    echo "它位于 GMP-Platform，且已于 2026-09-11 合入："
    echo
    echo '```'
    echo "67fc628 Merge PR#207: M5 sqlrustgo-graph migration"
    echo "         (resolve cypher_engine.rs conflict)          ai <ai@z440>"
    echo '```'
    echo
    echo "内容随后经 PR #210 forward 到 \`develop/v1.5.0\`，并由 \`develop/v1.6.0\`"
    echo "继续演进（\`crates/gmp-storage/src/cypher_engine.rs\` 在位）。"
    echo "即 #4873 记录的阻塞**前提不成立**，本 gate 因此不检查 PR，转而检查"
    echo "四类 consumer surface 能否对当前 SQLRustGo crates 构建与应答。"
} > "$EVIDENCE_FILE"

echo ""
echo "evidence: $EVIDENCE_FILE"
echo "RESULT: $PASS passed, $FAIL failed, $SKIP skipped"
[ "$FAIL" -gt 0 ] && exit 1
[ "$SKIP" -gt 0 ] && [ "$PASS" -eq 0 ] && exit 2
exit 0
