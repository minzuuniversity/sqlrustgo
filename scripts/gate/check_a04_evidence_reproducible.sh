#!/usr/bin/env bash
# ==========================================================================
# GATE-DISPOSITION-SELF-MARK —— 本文件**不会在权威轮里被执行**，登记结论就地写在这里
#
# 分类：死门禁（dead）—— 既无执行点也无正文引用 ⇒ 永远不会运行
# 处置：待接入执行点（needs-execution-point）
# 责任任务：P0-4
# 登记说明：判据可在仓内自足执行，但不在任何阶段清单/CI 执行点 ⇒ P0-2 定义执行点后必须挂上，否则撤声称
# 效果声明：尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。
#
# 为什么保留文件而不是删掉：删文件会改变门禁总数（当前 168，固定不可动），
# 而「不执行」是**登记在册的已知状态**，不是遗漏。删除会让「这一项为什么
# 没有判定」变成不可追溯。因此：**保留文件 + 就地自标 + 登记在册**。
#
# 权威登记：docs/releases/v4.1.0/GATE_CATALOG.jsonl（同一 gate 键）
# ==========================================================================
# ============================================================================
# check_a04_evidence_reproducible.sh
#
# #5466 —— 「A04 差分矩阵的**已入库证据**能否从 clean checkout 复现？」
#
# #5466 的结论是「不能」，而且给出了三方对比（入库 wire / 入库 matrix.json /
# 当前引擎实测）。本门禁把这个判定变成**可执行**的：
#
#   1. 在**当前工作树**上跑一次 A04 差分矩阵。
#      #5577 / #5389 修完之后，A04 默认写 `target/a04-5226/`，
#      **不再改写仓库里的归档证据**（批次 43）。
#   2. 把这次实测与 `docs/releases/v4.1.0/evidence/a04-5226/wire/` 里
#      **已入库**的证据**逐字节**比对。
#   3. 三种结论，分别对应三种真相：
#        - 一致        ⇒ 该 case 可从 clean checkout 复现
#        - 不一致      ⇒ **FAIL**：入库证据与当前引擎已经漂移
#        - 本次没产出  ⇒ **NOT RUN**（不是 PASS）：该引擎在当前环境不可用
#          （例如没有 PostgreSQL 参考容器），**无法验证** ⇒ 必须披露
#
# 为什么「NOT RUN」必须与「一致」分开：#5466 的取证之所以出错，
# 正是因为把「没验证」当成了「验证过」。
#
# 用法：
#   bash scripts/gate/check_a04_evidence_reproducible.sh
#   bash scripts/gate/check_a04_evidence_reproducible.sh --selftest
#
# 退出码：0 = PASS（可能含 NOT RUN 披露）；1 = FAIL；2 = 用法错误。
# ============================================================================
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

SELFTEST=0
for arg in "$@"; do
  case "$arg" in
    --selftest) SELFTEST=1 ;;
    -h|--help)
      sed -n '2,40p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *) echo "unknown argument: $arg" >&2; exit 2 ;;
  esac
done

EVIDENCE_DIR="docs/releases/v4.1.0/evidence/a04-5226"
ARCHIVED="$EVIDENCE_DIR/wire"
FRESH="target/a04-5226/wire"

# ---------------------------------------------------------------------------
# 比对器（纯 ASCII 输出：这一段输出会被 sed/grep 再处理，
# BSD sed 在含非 ASCII 的管道里会报 `RE error: illegal byte sequence`）。
# ---------------------------------------------------------------------------
COMPARE_PY='
import pathlib, sys

archived = pathlib.Path(sys.argv[1])
fresh = pathlib.Path(sys.argv[2])

if not archived.is_dir():
    print("ARCHIVED_MISSING=1")
    sys.exit(0)
if not fresh.is_dir():
    print("FRESH_MISSING=1")
    sys.exit(0)

A = {p.name: p for p in archived.glob("*.txt")}
B = {p.name: p for p in fresh.glob("*.txt")}
common = sorted(set(A) & set(B))
same, diff = [], []
for n in common:
    a = A[n].read_text(errors="replace").strip()
    b = B[n].read_text(errors="replace").strip()
    (same if a == b else diff).append(n)
only_a = sorted(set(A) - set(B))
only_b = sorted(set(B) - set(A))

for n in diff:
    print("  MISMATCH %s" % n)
for n in only_b:
    print("  NEW_NOT_ARCHIVED %s" % n)
print("SAME=%d" % len(same))
print("DIFF=%d" % len(diff))
print("NOT_PRODUCED=%d" % len(only_a))
print("NEW=%d" % len(only_b))
sys.exit(0)
'

compare() {
  python3 -c "$COMPARE_PY" "$1" "$2"
}

if [ "$SELFTEST" -eq 1 ]; then
  echo "=== selftest: the comparer must detect a drift ==="
  TMP="$(mktemp -d)"
  trap 'rm -rf "$TMP"' EXIT
  mkdir -p "$TMP/a" "$TMP/b"
  printf 'same\n' > "$TMP/a/case_same.txt"
  printf 'same\n' > "$TMP/b/case_same.txt"
  printf 'archived\n' > "$TMP/a/case_drift.txt"
  printf 'fresh\n'   > "$TMP/b/case_drift.txt"
  printf 'archived-only\n' > "$TMP/a/case_only_archived.txt"
  printf 'fresh-only\n'   > "$TMP/b/case_only_fresh.txt"

  OUT="$(compare "$TMP/a" "$TMP/b")"
  printf '%s\n' "$OUT" | LC_ALL=C grep -v '^[A-Z_]*=' || true
  D="$(printf '%s\n' "$OUT" | LC_ALL=C sed -n 's/^DIFF=//p')"
  NP="$(printf '%s\n' "$OUT" | LC_ALL=C sed -n 's/^NOT_PRODUCED=//p')"
  N="$(printf '%s\n' "$OUT" | LC_ALL=C sed -n 's/^NEW=//p')"
  S="$(printf '%s\n' "$OUT" | LC_ALL=C sed -n 's/^SAME=//p')"

  if [ "${D:-x}" != "1" ]; then
    echo "FAIL: the comparer did not detect the drifted file (DIFF=${D:-none})"
    exit 1
  fi
  if [ "${NP:-x}" != "1" ]; then
    echo "FAIL: the comparer did not report the not-produced file (NOT_PRODUCED=${NP:-none})"
    exit 1
  fi
  if [ "${N:-x}" != "1" ]; then
    echo "FAIL: the comparer did not report the new file (NEW=${N:-none})"
    exit 1
  fi
  if [ "${S:-x}" != "1" ]; then
    echo "FAIL: the comparer did not count the identical file (SAME=${S:-none})"
    exit 1
  fi
  echo "PASS: the comparer separates same / drift / not-produced / new"
  exit 0
fi

if [ ! -d "$ARCHIVED" ]; then
  echo "FAIL: archived evidence directory is missing: $ARCHIVED"
  exit 1
fi

echo "=== run the A04 differential matrix (output goes to target/a04-5226/) ==="
# IL-10: 门禁拿 `target/a04-5226/` 跟归档证据比，**必须先把它清空**。
#
# 目录里可能留着**上一次**运行的文件：某个引擎这次不可用（比如 PATH 上取到了
# 2026-02-21 的陈旧全局 sqlrustgo，它只打印 REPL 横幅）⇒ 生成器判它 NOT RUN
# ⇒ **不写文件**；但上一次运行留下的旧文件还在目录里 ⇒ 门禁照样拿它去比，
# 把「这次没产出」误判成「产出了且一致」。
#
# ⇒ 比对域必须只含**本次**产出。残留文件参与比对，会同时制造假绿与假红。
rm -rf "$FRESH"
if ! cargo test --test a04_mysql_diff_matrix > /tmp/a04_repro_run.log 2>&1; then
  echo "FAIL: the A04 differential matrix did not pass; the comparison cannot be made"
  tail -n 30 /tmp/a04_repro_run.log || true
  exit 1
fi
tail -n 3 /tmp/a04_repro_run.log || true

echo "=== compare archived evidence against this run ==="
OUT="$(compare "$ARCHIVED" "$FRESH")"
printf '%s\n' "$OUT" | LC_ALL=C grep -v '^[A-Z_]*=' || true
S="$(printf '%s\n' "$OUT" | LC_ALL=C sed -n 's/^SAME=//p')"
D="$(printf '%s\n' "$OUT" | LC_ALL=C sed -n 's/^DIFF=//p')"
NP="$(printf '%s\n' "$OUT" | LC_ALL=C sed -n 's/^NOT_PRODUCED=//p')"
N="$(printf '%s\n' "$OUT" | LC_ALL=C sed -n 's/^NEW=//p')"

if [ "${D:-1}" -ne 0 ]; then
  echo "FAIL: ${D} archived evidence file(s) do NOT match what the current engine produces."
  echo "      The archived evidence has drifted away from the engine - it is no longer"
  echo "      reproducible from a clean checkout."
  exit 1
fi

if [ "${N:-0}" -ne 0 ]; then
  echo "NOTE: ${N} file(s) produced by this run are not archived (new cases, not drift)."
fi

echo "PASS: ${S:-0}/${S:-0} comparable archived evidence file(s) match this run byte-for-byte."

if [ "${NP:-0}" -ne 0 ]; then
  # NOT RUN is not PASS. Say so, loudly, and name the files.
  echo ""
  echo "STATUS: NOT RUN for ${NP} archived file(s) - the engine that produces them is"
  echo "        unavailable in this environment (typically the PostgreSQL reference"
  echo "        container). Those files were NOT verified by this run:"
  python3 - "$ARCHIVED" "$FRESH" << 'PYEOF'
import pathlib, sys
A = {p.name for p in pathlib.Path(sys.argv[1]).glob("*.txt")}
B = {p.name for p in pathlib.Path(sys.argv[2]).glob("*.txt")}
for n in sorted(A - B):
    print("  NOT VERIFIED: %s" % n)
PYEOF
  echo ""
  echo "RESULT: PASS (with NOT RUN disclosure) - the verified subset is reproducible."
  exit 0
fi

echo "RESULT: PASS"
exit 0
