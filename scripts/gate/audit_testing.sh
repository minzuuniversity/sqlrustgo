#!/usr/bin/env bash

set -euo pipefail

VERSION="${1:-}"
STAGE="${2:-unknown}"
OUT_DIR="${3:-}"

if [ -z "$OUT_DIR" ]; then
  echo "Usage: $0 <version> <stage> <out_dir>"
  exit 2
fi

mkdir -p "$OUT_DIR"
OUT_FILE="$OUT_DIR/TEST_AUDIT.md"

PASS_COUNT=0
WARN_COUNT=0
FAIL_COUNT=0

PASS_ITEMS=()
WARN_ITEMS=()
FAIL_ITEMS=()
CAPA_ITEMS=()

add_pass() { PASS_ITEMS+=("$1"); PASS_COUNT=$((PASS_COUNT + 1)); }
add_warn() { WARN_ITEMS+=("$1"); WARN_COUNT=$((WARN_COUNT + 1)); }
add_fail() { FAIL_ITEMS+=("$1"); FAIL_COUNT=$((FAIL_COUNT + 1)); }
add_capa() { CAPA_ITEMS+=("$1"); }

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$REPO_ROOT"

# Ensure cargo is on PATH (CI runners may not have it in default PATH).
if ! command -v cargo >/dev/null 2>&1; then
    if [ -x "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
fi

# #3844: this audit depends on ripgrep. When `rg` is NOT installed, the
# doc-target consistency check produced an EMPTY comparison set, the
# `if [ -s "$DOC_TESTS_TMP" ]` guard skipped the comparison entirely, and
# the audit then reported
#
#     PASS 测试计划中的 --test target 与 Cargo.toml 一致
#
# a PASS for a comparison that never happened. The same missing binary
# also turned the "does the test plan mention TPC-H / Sysbench / backup"
# checks into misleading "not specified" warnings.
#
# Missing ripgrep is now an explicit, loud audit failure (it is an
# environment blocker, and this audit's whole job is to describe the real
# state of the test plan), and the affected checks are reported as
# NOT-RUN rather than as PASS or as a content verdict.
RG_AVAILABLE=1
if ! command -v rg >/dev/null 2>&1; then
    RG_AVAILABLE=0
    add_fail "缺少 ripgrep (rg)：无法执行测试计划一致性校验（NOT-RUN，不得判定为 PASS）"
    add_capa "P0|在执行环境安装 ripgrep（Alpine: apk add --no-cache ripgrep；macOS: brew install ripgrep）。"
fi

# rg_check <pattern> <file> — 0 = found, 1 = not found, 2 = NOT-RUN (no rg)
rg_check() {
    if [ "$RG_AVAILABLE" -ne 1 ]; then
        return 2
    fi
    if rg -n "$1" "$2" >/dev/null 2>&1; then
        return 0
    fi
    return 1
}

if [ -z "$VERSION" ]; then
  if [ -f "CURRENT_VERSION.md" ]; then
    VERSION="$(head -n 1 CURRENT_VERSION.md | sed 's#.*/##' | tr -d '[:space:]')"
  else
    VERSION="unknown"
  fi
fi

TEST_PLAN="docs/releases/$VERSION/TEST_PLAN.md"
if [ -f "$TEST_PLAN" ]; then
  add_pass "测试计划存在：$TEST_PLAN"
else
  add_fail "测试计划缺失：$TEST_PLAN"
  add_capa "P0|补齐版本测试计划并绑定门禁阈值。"
fi

ROOT_TESTS_TMP="$(mktemp)"
DOC_TESTS_TMP="$(mktemp)"
INVALID_TESTS_TMP="$(mktemp)"
UNDOC_TESTS_TMP="$(mktemp)"

cleanup() {
  rm -f "$ROOT_TESTS_TMP" "$DOC_TESTS_TMP" "$INVALID_TESTS_TMP" "$UNDOC_TESTS_TMP"
}
trap cleanup EXIT

awk '
  /^\[\[test\]\]/{in_test=1; next}
  in_test && /^name = /{
    gsub(/name = "|"/, "", $0);
    print $0;
    in_test=0
  }
' Cargo.toml | sort -u > "$ROOT_TESTS_TMP"

if [ "$RG_AVAILABLE" -eq 1 ] && [ -f "$TEST_PLAN" ]; then
  # V312-31: the `\|\| true` here is NOT masking a cargo test invocation;
  # `cargo test` is the LITERAL SEARCH PATTERN passed to `rg` (it appears
  # inside the double-quoted regex), not an executed command. The `\|\| true`
  # tolerates rg | awk | sort exiting non-zero (e.g., rg finds zero matches
  # in TEST_PLAN, awk writes nothing, sort exits 0 — but we guard defensively
  # for pipeline SIGPIPE). P16 step 2.5 detector's refined regex excludes
  # `cargo test` occurrences inside double-quoted rg/grep search patterns.
  rg -o --no-filename "cargo test --test [a-zA-Z0-9_\\-]+" "$TEST_PLAN" | awk '{print $4}' | sort -u > "$DOC_TESTS_TMP" || true
fi

# #3844: the comparison only *happened* when both sides are non-empty.
# An empty DOC set means "the plan declares no `cargo test --test` target",
# which is NOT the same as "the plan is consistent with Cargo.toml" — and
# neither is it "rg is missing". Three distinct states, three distinct
# verdicts. Before this fix all three collapsed into a PASS.
DOC_COMPARISON_RAN=0
if [ "$RG_AVAILABLE" -eq 1 ] && [ -s "$DOC_TESTS_TMP" ]; then
    DOC_COMPARISON_RAN=1
  comm -23 "$DOC_TESTS_TMP" "$ROOT_TESTS_TMP" > "$INVALID_TESTS_TMP" || true
  comm -13 "$DOC_TESTS_TMP" "$ROOT_TESTS_TMP" > "$UNDOC_TESTS_TMP" || true
fi

INVALID_DOC_TEST_COUNT="$(wc -l < "$INVALID_TESTS_TMP" | tr -d ' ')"
UNDOC_ROOT_TEST_COUNT="$(wc -l < "$UNDOC_TESTS_TMP" | tr -d ' ')"
ROOT_TEST_COUNT="$(wc -l < "$ROOT_TESTS_TMP" | tr -d ' ')"

if [ "$DOC_COMPARISON_RAN" -ne 1 ]; then
  if [ "$RG_AVAILABLE" -ne 1 ]; then
    add_warn "RG_MISSING: 测试计划 --test target 一致性校验 NOT-RUN（不得判定为一致）"
  else
    add_warn "测试计划未登记任何 'cargo test --test' 目标：一致性校验 NOT-RUN（不得判定为一致）"
    add_capa "P1|在 TEST_PLAN 中登记 cargo test --test 目标，或明确声明该版本不声明 root test target。"
  fi
elif [ "$INVALID_DOC_TEST_COUNT" -eq 0 ]; then
  add_pass "测试计划中的 --test target 与 Cargo.toml 一致"
else
  add_warn "检测到无效测试 target：$INVALID_DOC_TEST_COUNT"
  add_capa "P1|修正 TEST_PLAN 中无效的 cargo test --test 目标。"
fi

if [ "$DOC_COMPARISON_RAN" -ne 1 ]; then
  add_warn "root test target 文档化覆盖校验 NOT-RUN（比较集为空，不得判定为可接受）"
elif [ "$UNDOC_ROOT_TEST_COUNT" -le 5 ]; then
  add_pass "未文档化 root test target 数量可接受：$UNDOC_ROOT_TEST_COUNT"
else
  add_warn "未文档化 root test target 偏多：$UNDOC_ROOT_TEST_COUNT"
  add_capa "P2|在 TEST_PLAN 增加 root test target 覆盖清单。"
fi

if [ -f "crates/sql-corpus/tests/corpus_test.rs" ]; then
  add_pass "SQL Corpus 测试入口存在"
else
  add_fail "SQL Corpus 测试入口缺失：crates/sql-corpus/tests/corpus_test.rs"
  add_capa "P0|恢复 SQL Corpus 测试入口并加入门禁。"
fi

# #3844: with rg missing these three checks used to fall into the `else`
# branch and report "测试计划未明确 ..." — i.e. a content verdict about a
# document that was never read. NOT-RUN is now distinguished from "absent".
if rg_check "TPC-H|TPC-H|tpch" "$TEST_PLAN"; then rc=0; else rc=$?; fi
if [ "$rc" -eq 2 ]; then
  add_warn "RG_MISSING: TPC-H 维度校验 NOT-RUN（不得判定为缺失）"
elif [ "$rc" -eq 0 ]; then
  add_pass "测试计划包含 TPC-H 维度"
else
  add_warn "测试计划未明确 TPC-H 测试策略"
  add_capa "P1|补充 TPC-H 阶段策略（SF1 必跑，SF10 夜间）。"
fi

if rg_check "Sysbench|sysbench" "$TEST_PLAN"; then rc=0; else rc=$?; fi
if [ "$rc" -eq 2 ]; then
  add_warn "RG_MISSING: Sysbench 维度校验 NOT-RUN（不得判定为缺失）"
elif [ "$rc" -eq 0 ]; then
  add_pass "测试计划包含 Sysbench 维度"
else
  add_warn "测试计划未明确 Sysbench 测试"
  add_capa "P1|补充 Sysbench 三场景与阈值。"
fi

if rg_check "备份|恢复|崩溃|crash|backup" "$TEST_PLAN"; then rc=0; else rc=$?; fi
if [ "$rc" -eq 2 ]; then
  add_warn "RG_MISSING: 恢复类测试维度校验 NOT-RUN（不得判定为缺失）"
elif [ "$rc" -eq 0 ]; then
  add_pass "测试计划包含恢复类测试维度"
else
  add_warn "测试计划未明确备份/恢复/崩溃测试"
  add_capa "P1|补充备份恢复与崩溃注入测试要求。"
fi

if [ -f "tarpaulin.toml" ]; then
  add_pass "覆盖率配置存在：tarpaulin.toml"
else
  add_warn "覆盖率配置缺失：tarpaulin.toml"
  add_capa "P2|补充覆盖率工具配置并固化阈值。"
fi

RESULT="PASS"
if [ "$FAIL_COUNT" -gt 0 ]; then
  RESULT="FAIL"
elif [ "$WARN_COUNT" -gt 0 ]; then
  RESULT="WARN"
fi

{
  echo "# TEST_AUDIT"
  echo
  echo "- Version: \`$VERSION\`"
  echo "- Stage: \`$STAGE\`"
  echo "- Date: $(date '+%Y-%m-%d %H:%M:%S %z')"
  echo "- RESULT: $RESULT"
  echo
  echo "## Metrics"
  echo
  echo "- ROOT_TEST_COUNT: $ROOT_TEST_COUNT"
  echo "- INVALID_DOC_TEST_TARGET_COUNT: $INVALID_DOC_TEST_COUNT"
  echo "- UNDOCUMENTED_ROOT_TEST_COUNT: $UNDOC_ROOT_TEST_COUNT"
  echo
  echo "## Invalid Doc Targets"
  if [ "$INVALID_DOC_TEST_COUNT" -eq 0 ]; then
    echo "- None"
  else
    sed 's/^/- /' "$INVALID_TESTS_TMP"
  fi
  echo
  echo "## Undocumented Root Tests"
  if [ "$UNDOC_ROOT_TEST_COUNT" -eq 0 ]; then
    echo "- None"
  else
    sed 's/^/- /' "$UNDOC_TESTS_TMP"
  fi
  echo
  echo "## PASS Items"
  for item in "${PASS_ITEMS[@]}"; do echo "- $item"; done
  echo
  echo "## WARN Items"
  if [ "${#WARN_ITEMS[@]}" -eq 0 ]; then
    echo "- None"
  else
    for item in "${WARN_ITEMS[@]}"; do echo "- $item"; done
  fi
  echo
  echo "## FAIL Items"
  if [ "${#FAIL_ITEMS[@]}" -eq 0 ]; then
    echo "- None"
  else
    for item in "${FAIL_ITEMS[@]}"; do echo "- $item"; done
  fi
  echo
  echo "## CAPA Recommendations"
  if [ "${#CAPA_ITEMS[@]}" -eq 0 ]; then
    echo "- CAPA: P3|测试策略稳定，继续执行分层测试。"
  else
    for item in "${CAPA_ITEMS[@]}"; do
      echo "- CAPA: $item"
    done
  fi
} > "$OUT_FILE"

echo "Generated $OUT_FILE"
