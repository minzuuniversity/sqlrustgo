#!/usr/bin/env bash
#
# scripts/gate/check_workflow_expressions.sh
#
# Meta-gate: a workflow must not contain an expression template that the
# platform's expression parser cannot read — **not even inside a comment**.
#
# #5517 — `ci.yml` 自 2026-10-21 04:23 起，每个 run 都是 **0 个 job**：
# run 立刻 failure，而 runner 是 online 且空闲、workflow 在 workflow 列表里
# 也是 active —— 但 Gitea **从未为它物化过任何一个 job**
#（`rerun` 一个 ci.yml 的 run 直接 404；`rerun` 一个 gate.yml 的 run 会 201
# 并且 job 被物化）。
#
# Gitea 服务端日志给出了确切原因（98 次同一条）：
#
#   services/convert/convert.go:585:getActionWorkflowEntry() [E]
#   getActionWorkflowEntry: Failed to parse workflow: yaml: construct errors:
#   line 1: (Line: 537, Col: 14): Unexpected symbol: '<'. Located at position 7
#   within expression: steps.<id>.outputs.<name>
#
# ⇒ `ci.yml` 的 `postcheck` 里有一行**注释**，写的是「step output 要用
# `${{ steps.<id>.outputs.<name> }}` 这种形式读」。
#
# Gitea 扫描的是整个 `run:` 标量里的表达式占位符 —— **注释也在扫描范围内** ——
# 而占位符里的尖括号不是合法的表达式符号 ⇒ **整个 workflow 解析失败**
# ⇒ `DetectWorkflows()` 直接「ignore invalid workflow」⇒ 一个 run 都不创建。
#
# THE INVARIANTS
# --------------
#
#   C1  一行**注释**里不得出现 `${{` —— 注释是给人看的，Gitea 不该读到它；
#       真要用，就把它挪到不会被扫描的地方（或者干脆不写模板）。
#
#   C2  任何表达式的内容里不得出现 `<` 或 `>` —— 尖括号在 Gitea 表达式里
#       永远不是合法符号，它只可能出现在**文档占位符**里。
#
# Exit codes:
#   0  [PASS] — 没有会被平台解析器读崩的表达式
#   1  [FAIL] — 至少一条判据被违反
#   2  usage error / a required input is missing
#
# Usage:
#   bash scripts/gate/check_workflow_expressions.sh
#   bash scripts/gate/check_workflow_expressions.sh --json <out.json>
#   bash scripts/gate/check_workflow_expressions.sh --selftest

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
REPO_ROOT_OVERRIDE=""
WORKFLOW_DIR=""
JSON_OUT=""
SELFTEST=0

color_red()   { printf '\033[0;31m%s\033[0m' "$*"; }
color_green() { printf '\033[0;32m%s\033[0m' "$*"; }

step() { echo ""; echo "=== $* ==="; }
pass() { echo "  $(color_green PASS): $*"; }
fail() { echo "  $(color_red FAIL): $*"; }

while [[ $# -gt 0 ]]; do
    case "$1" in
        --json)      JSON_OUT="$2"; shift 2 ;;
        --json=*)    JSON_OUT="${1#--json=}"; shift ;;
        --selftest)  SELFTEST=1; shift ;;
        --repo-root) REPO_ROOT_OVERRIDE="$2"; shift 2 ;;
        --repo-root=*) REPO_ROOT_OVERRIDE="${1#--repo-root=}"; shift ;;
        -h|--help)
            echo "Usage: bash scripts/gate/check_workflow_expressions.sh [--json <out>] [--selftest] [--repo-root <dir>]"
            exit 0
            ;;
        *) echo "ERROR: unknown argument: $1" >&2; exit 2 ;;
    esac
done

if ! command -v python3 >/dev/null 2>&1; then
    echo "ERROR: python3 is required" >&2
    exit 2
fi

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/workflow-expressions.XXXXXX")"
trap 'rm -rf -- "${TMP_DIR}"' EXIT

# ---------------------------------------------------------------------------
# The analyzer
# ---------------------------------------------------------------------------
cat > "${TMP_DIR}/analyze.py" <<'PYEOF'
import json
import os
import re
import sys

EXPR_RE = re.compile(r"\$\{\{(.*?)\}\}", re.S)

# A plain dotted path: `runner.temp`, `github.ref`, `steps.version.outputs.version`.
# That is the ONLY shape we tolerate inside a comment — see C2.
PLAIN_PATH_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*(\.[A-Za-z_][A-Za-z0-9_]*)*$")


def analyze(path):
    """Return the violations of C1/C2 in one workflow file."""
    out = []
    with open(path, errors="replace") as fh:
        for lineno, line in enumerate(fh, start=1):
            for m in EXPR_RE.finditer(line):
                content = m.group(1).strip()
                stripped = line.strip()
                is_comment = stripped.startswith("#")
                # C1 — angle brackets are never valid in a Gitea expression;
                # they only ever appear in documentation placeholders, and one
                # of them makes the platform's parser fail on the WHOLE
                # workflow (#5517).
                if "<" in content or ">" in content:
                    out.append({
                        "file": os.path.basename(path),
                        "line": lineno,
                        "rule": "C1",
                        "content": content[:120],
                        "text": stripped[:160],
                    })
                # C2 — inside a comment, only a plain dotted path is allowed.
                # The platform scans the whole `run:` scalar, comments
                # included, so anything richer there is a silent landmine:
                # a reader cannot tell whether it parses.
                if is_comment and not PLAIN_PATH_RE.match(content):
                    out.append({
                        "file": os.path.basename(path),
                        "line": lineno,
                        "rule": "C2",
                        "content": content[:120],
                        "text": stripped[:160],
                    })
    return out


def main(argv):
    workflow_dir = argv[0]
    json_out = argv[1] if len(argv) > 1 else None
    violations = []
    scanned = []
    if os.path.isdir(workflow_dir):
        for fn in sorted(os.listdir(workflow_dir)):
            if fn.endswith((".yml", ".yaml")):
                p = os.path.join(workflow_dir, fn)
                scanned.append(fn)
                violations.extend(analyze(p))
    report = {
        "scanned": scanned,
        "violations": violations,
        "failure_count": len(violations),
        "status": "PASS" if not violations else "FAIL",
    }
    if json_out:
        with open(json_out, "w") as fh:
            json.dump(report, fh, indent=2, ensure_ascii=False)
            fh.write("\n")
    else:
        json.dump(report, sys.stdout, indent=2, ensure_ascii=False)
        sys.stdout.write("\n")
    return 0 if not violations else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
PYEOF

# ---------------------------------------------------------------------------
# --selftest: the gate must be able to fail
# ---------------------------------------------------------------------------
if [[ "${SELFTEST}" -eq 1 ]]; then
    step "Self-test: synthetic fixtures"

    SELFTEST_FAILURES=0

    expect_verdict() {
        local dir="$1" expected="$2" label="$3"
        local out="${TMP_DIR}/verdict.json"
        if python3 "${TMP_DIR}/analyze.py" "${dir}" "${out}" >/dev/null 2>&1; then
            local got="PASS"
        else
            local got="FAIL"
        fi
        if [[ "${got}" == "${expected}" ]]; then
            pass "${label}: expected ${expected}, got ${got}"
            return 0
        fi
        fail "${label}: expected ${expected}, got ${got}"
        return 1
    }

    # 1. clean workflow → PASS
    mkdir -p "${TMP_DIR}/clean"
    cat > "${TMP_DIR}/clean/ok.yml" <<'EOF'
name: ok
on: [push]
jobs:
  build:
    runs-on: [hp-z6g4]
    steps:
      - name: Checkout
        uses: actions/checkout@v4
      - name: Run
        run: |
          echo "version=${{ steps.version.outputs.version }}"
EOF
    expect_verdict "${TMP_DIR}/clean" PASS "clean workflow" || SELFTEST_FAILURES=$((SELFTEST_FAILURES + 1))

    # 2. the #5517 shape: a template with angle brackets inside a comment → FAIL
    mkdir -p "${TMP_DIR}/c1"
    cat > "${TMP_DIR}/c1/bad.yml" <<'EOF'
name: bad
on: [push]
jobs:
  build:
    runs-on: [hp-z6g4]
    steps:
      - name: Run
        run: |
          # a step output has to be read as `${{ steps.<id>.outputs.<name> }}`.
          echo hello
EOF
    expect_verdict "${TMP_DIR}/c1" FAIL "C1 template inside a comment" || SELFTEST_FAILURES=$((SELFTEST_FAILURES + 1))

    # 3. angle brackets outside a comment → FAIL (C1)
    mkdir -p "${TMP_DIR}/c2"
    cat > "${TMP_DIR}/c2/bad.yml" <<'EOF'
name: bad
on: [push]
jobs:
  build:
    runs-on: [hp-z6g4]
    steps:
      - name: Run
        run: |
          echo "${{ steps.<id>.outputs.<name> }}"
EOF
    expect_verdict "${TMP_DIR}/c2" FAIL "C1 angle brackets in an expression" || SELFTEST_FAILURES=$((SELFTEST_FAILURES + 1))

    # 4. a *valid* expression inside a comment → PASS (no false positive)
    mkdir -p "${TMP_DIR}/okcomment"
    cat > "${TMP_DIR}/okcomment/ok.yml" <<'EOF'
name: ok
on: [push]
jobs:
  build:
    runs-on: [hp-z6g4]
    steps:
      - name: Run
        run: |
          # `${{ runner.temp }}` is evaluated to the literal string `/tmp`
          echo "${{ runner.temp }}"
EOF
    expect_verdict "${TMP_DIR}/okcomment" PASS "valid template in a comment" || SELFTEST_FAILURES=$((SELFTEST_FAILURES + 1))

    echo ""
    if [[ "${SELFTEST_FAILURES}" -eq 0 ]]; then
        pass "workflow expression gate self-test: 4/4 fixtures behaved as expected"
        exit 0
    fi
    fail "workflow expression gate self-test: ${SELFTEST_FAILURES} fixture(s) misbehaved"
    exit 1
fi

# ---------------------------------------------------------------------------
# Real run
# ---------------------------------------------------------------------------
if [[ -n "${REPO_ROOT_OVERRIDE}" ]]; then
    REPO_ROOT="$(cd "${REPO_ROOT_OVERRIDE}" && pwd)"
fi
WORKFLOW_DIR="${REPO_ROOT}/.gitea/workflows"

step "Scanning ${WORKFLOW_DIR}"

if [[ ! -d "${WORKFLOW_DIR}" ]]; then
    echo "ERROR: workflow directory not found: ${WORKFLOW_DIR}" >&2
    exit 2
fi

RESULT_JSON="${TMP_DIR}/report.json"
python3 "${TMP_DIR}/analyze.py" "${WORKFLOW_DIR}" "${RESULT_JSON}"
ANALYZER_EXIT=$?
if [[ "${ANALYZER_EXIT}" -ge 2 ]]; then
    echo "ERROR: analyzer failed (exit ${ANALYZER_EXIT})" >&2
    exit 2
fi

SCANNED="$(python3 -c "import json,sys; print(len(json.load(open(sys.argv[1]))['scanned']))" "${RESULT_JSON}")"
VIOLATIONS="$(python3 - "${RESULT_JSON}" <<'PYEOF'
import json
import sys
with open(sys.argv[1]) as fh:
    data = json.load(fh)
for v in data.get("violations", []):
    print("%s:%s [%s] %s" % (v["file"], v["line"], v["rule"], v["text"]))
PYEOF
)"

echo "  workflow files scanned: ${SCANNED}"

if [[ -n "${VIOLATIONS}" ]]; then
    fail "the workflow contains an expression the platform's parser cannot read:"
    while IFS= read -r v; do
        [[ -z "${v}" ]] && continue
        echo "    ${v}"
    done <<< "${VIOLATIONS}"
    echo "    (fix: do not write expression templates in comments; Gitea scans the"
    echo "     whole run: scalar — comments included — and an unparsable template"
    echo "     makes it ignore the workflow entirely: #5517)"
    echo "  workflow expression gate: FAIL"
    exit 1
fi

pass "no unparsable expression template in the workflows"
echo "  workflow expression gate: PASS"
exit 0
