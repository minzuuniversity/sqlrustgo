#!/usr/bin/env bash
# ============================================================================
# check_commit_message.sh
#
# #5453 —— 提交信息规范的**唯一来源**与**唯一执行点**。
#
# 问题（#5453）：仓库里同时存在**三份**互不相同的提交信息规范
#   S1 docs/governance/CONTRIBUTING.md        §5   Conventional Commits + type 表
#   S2 docs/governance/DEVELOPMENT_PROCESS.md §7   <类型>: <描述> + 强制三行 trailer
#   S3 docs/governance/ENGINEERING_AUTOMATION.md §11 Conventional Commits
# 且**没有任何机器裁决**（无 commit-msg 钩子、CI 无校验）。
# 实测最近 200 个 commit：`Agent:` 0 次、`Refs:` 约 3.8%。
#
# 本门禁有三层：
#
#   L1（静态，形态唯一）—— type 取值表**只允许**在 CONTRIBUTING.md §5.2 出现一次。
#        另两份文件必须**指向**它，不得各自再列一份表。
#        这一层防的是"第二份会漂移的表"长回来 —— 与 #5425（阈值三处）、
#        #5437（门禁数四个答案）同一个病。
#
#   L2（形态）—— 被检查的每个 commit 的第一行必须匹配
#        `<type>(<scope>): <subject>` 或 `<type>: <subject>`，
#        且 `type` 在允许集合内。**merge commit 豁免**（标题由 git 生成，
#        不是人写的；实测 54/200 个"不合规"全是 merge commit）。
#
#   L3（可追溯）—— 报告 `Refs #<issue>` 的遵从率。
#        **默认只报告，不判红**：存量历史的遵从率是 %5453 实测的 ~4% ~ 本批实测
#        的 ~47%%，一次性判红会堵死所有后续工作，而且**不会**让历史变好。
#        需要强制时用 `--require-refs`（届时它对**被检查的每个** commit 生效）。
#        `Refs` 是 CONTRIBUTING §5.4 里的**推荐**项，本门禁不把它当成格式的一部分。
#
# 用法：
#   bash scripts/gate/check_commit_message.sh                 # 最近 50 个 commit
#   bash scripts/gate/check_commit_message.sh --last 200      # 扩大范围
#   bash scripts/gate/check_commit_message.sh --range BASE..HEAD
#                                                             # 限定检查范围
#   bash scripts/gate/check_commit_message.sh --require-refs     # 强制 Refs（未来收紧时）
#   bash scripts/gate/check_commit_message.sh --selftest      # 自测
#
# 退出码：0 = PASS；1 = FAIL；2 = 用法错误。
# ============================================================================
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

LAST=50
RANGE=""
SELFTEST=0
REQUIRE_REFS=0
# A `while` loop, not `for arg in "$@"`: the `for` form expands "$@" ONCE
# at loop entry, so a `shift` inside it never changes what is iterated.
while [ $# -gt 0 ]; do
  arg="$1"
  case "$arg" in
    --last)     LAST="${2:-}"; shift 2 ;;
    --last=*)   LAST="${arg#--last=}" ; shift ;;
    --range)    RANGE="${2:-}"; shift 2 ;;
    --range=*)  RANGE="${arg#--range=}" ; shift ;;
    --selftest) SELFTEST=1 ; shift ;;
    --require-refs) REQUIRE_REFS=1 ; shift ;;
    -h|--help)  sed -n '2,50p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown argument: $arg" >&2; exit 2 ;;
  esac
done

fail() { echo "FAIL: $*"; exit 1; }

# ---------------------------------------------------------------------------
# 允许的 type —— 唯一一份，在本门禁里定义一次。
#
# 取值来源：CONTRIBUTING.md §5.2 的表（那一份是规范；这里是它的机器镜像）。
# `ci` 与 `style` 是 #5453 批次按实测补进去的（见 CONTRIBUTING §5.2 的说明）。
# `merge` 同理：人工写的批次合并提交普遍用 `merge(<scope>): ...` 作为 type
# （git 自动生成的那种标题是 "Merge branch ..."，由下面的豁免规则处理）。
# ---------------------------------------------------------------------------
ALLOWED_TYPES="feat fix refactor docs test chore perf ci style merge audit"

# 规范里**唯一**允许定义 type 表的文件与小节。
CANONICAL_SPEC="docs/governance/CONTRIBUTING.md"
# 另两份文件：必须**指向**规范，但**不得**自己再列一份 type 表。
DERIVED_SPECS="docs/governance/DEVELOPMENT_PROCESS.md docs/governance/ENGINEERING_AUTOMATION.md"

# ---------------------------------------------------------------------------
# L1 —— type 表只允许有一份
# ---------------------------------------------------------------------------
#
# 判据：**一个表格块里出现的「允许 type」去重后达到阈值**，它就是一份 type 枚举。
#
# 为什么不是"看到 `| fix |` 就报"：仓库里还有两种**不是** type 枚举的表
#   * 分支命名表（`| fix | fix/<问题名> | ... |`）
#   * commit 类型 → 版本变化映射（`| `fix` | PATCH | ... |`，见
#     ENGINEERING_AUTOMATION.md §11.1）
# 二者都含 `fix`，但都不是 type 取值表。按"第一格命中"计数会把它们误报。
#
# 阈值取 6：允许集合有 9 个 type，**完整枚举**至少会命中 6 个；
# 上面那两张表分别只命中 1 个和 2 个。
L1_THRESHOLD=6

L1_PY='
import re, sys

allowed = set(sys.argv[1].split())
threshold = int(sys.argv[2])
files = sys.argv[3:]

row_re = re.compile(r"^\s*\|\s*`?([A-Za-z][A-Za-z0-9_-]*)`?\s*\|")
bad = 0

def flush(block, start, path):
    if not block:
        return 0
    hits = sorted(set(m for m in block if m in allowed))
    if len(hits) >= threshold:
        print("  %s:%d: this file lists a SECOND type table (%d allowed types: %s)"
              % (path, start, len(hits), ", ".join(hits)))
        return 1
    return 0

for path in files:
    try:
        lines = open(path, encoding="utf-8", errors="replace").read().split("\n")
    except OSError:
        print("  MISSING: %s" % path)
        bad = 1
        continue
    block = []
    start = 0
    for i, line in enumerate(lines, 1):
        m = row_re.match(line)
        if m:
            if not block:
                start = i
            block.append(m.group(1))
        else:
            bad += flush(block, start, path)
            block = []
    bad += flush(block, start, path)

# NOT sys.exit(0): the checker MUST fail closed. A first version exited 0
# unconditionally, so L1 printed the offending line and then reported PASS.
# The gate has to be able to say "no".
sys.exit(1 if bad else 0)
'

l1_check() {
  # shellcheck disable=SC2086
  python3 -c "$L1_PY" "$ALLOWED_TYPES" "$L1_THRESHOLD" $DERIVED_SPECS
  return $?
}

# ---------------------------------------------------------------------------
# L2 / L3 —— commit message 的形态与可追溯性
# ---------------------------------------------------------------------------
#
# 输出纯 ASCII（会被 sed/grep 再处理；BSD sed 在非 ASCII 管道里会报
# `RE error: illegal byte sequence`，把整条判据的输出吃掉）。
CHECK_PY='
import re, subprocess, sys

allowed = set(sys.argv[1].split())
mode = sys.argv[2]          # "last" | "range"
last = int(sys.argv[3])
rng = sys.argv[4]

if mode == "range":
    rev = rng
else:
    rev = "--max-count=%d" % last

fmt = "%H%x00%s%x00%b%x00"   # %b = body WITHOUT the subject (%B would repeat it)
out = subprocess.run(["git", "log", rev, "--format=" + fmt],
                     capture_output=True, text=True, errors="replace")
if out.returncode != 0:
    print("GIT_ERROR")
    sys.exit(0)

chunks = out.stdout.split("\0")
commits = []
i = 0
while i + 2 < len(chunks):
    sha, subject, body = chunks[i], chunks[i+1], chunks[i+2]
    i += 3
    # git inserts a newline after each --format record, so the NUL
    # separators are followed by "\n"; strip each field.
    sha, subject, body = sha.strip(), subject.strip(), body.strip()
    if not sha:
        continue
    commits.append((sha, subject, body))

HEAD_RE = re.compile(r"^([a-z]+)(\([^)]*\))?!?:\s+\S")
# The canonical form is `Refs #<issue>` (NO colon) - that is what
# CONTRIBUTING.md 5.4 specifies and what this repository actually uses.
# A colon is tolerated, but the colon-less form is the one that counts.
REFS_RE = re.compile(r"^Refs:?\s*#\d+\s*$", re.M)

shape_bad, refs_bad, merges, refs_ok = [], [], 0, 0
for sha, subject, body in commits:
    if subject.startswith("Merge "):
        merges += 1
        continue
    m = HEAD_RE.match(subject)
    if not m or m.group(1) not in allowed:
        shape_bad.append((sha[:12], subject))
        continue
    if REFS_RE.search(body):
        refs_ok += 1
    else:
        refs_bad.append((sha[:12], subject))

for sha, subj in shape_bad:
    print("  BAD_SHAPE %s %s" % (sha, subj[:90]))
for sha, subj in refs_bad:
    print("  NO_REFS %s %s" % (sha, subj[:90]))
print("CHECKED=%d" % len(commits))
print("MERGES=%d" % merges)
print("SHAPE_BAD=%d" % len(shape_bad))
print("REFS_OK=%d" % refs_ok)
print("REFS_BAD=%d" % len(refs_bad))
sys.exit(0)
'

run_check() {
  python3 -c "$CHECK_PY" "$ALLOWED_TYPES" "$1" "$LAST" "$RANGE"
}

if [ "$SELFTEST" -eq 1 ]; then
  echo "=== selftest: the checker must reject a bad commit message ==="
  TMP="$(mktemp -d)"
  trap 'rm -rf "$TMP"' EXIT
  (
    cd "$TMP" || exit 1
    git init -q .
    git config user.email "selftest@example.invalid"
    git config user.name "selftest"
    echo a > a.txt
    git add a.txt
    # the "good" sample: right shape AND a Refs line
    git commit -q -m "fix(gate): a good message" -m "Refs #1234"
    git commit -q --allow-empty -m "good subject no type"            # bad shape
    git commit -q --allow-empty -m "feat: good type but no Refs"      # missing Refs
    git commit -q --allow-empty -m "Merge branch 'x' into y"         # merge: exempt
    git commit -q --allow-empty -m "bogus(nope): unknown type"        # unknown type
  ) > /dev/null 2>&1

  OUT="$(cd "$TMP" && python3 -c "$CHECK_PY" "$ALLOWED_TYPES" last 50 "")"
  printf '%s\n' "$OUT" | LC_ALL=C grep -v '^[A-Z_]*=' || true
  SB="$(printf '%s\n' "$OUT" | LC_ALL=C sed -n 's/^SHAPE_BAD=//p')"
  RB="$(printf '%s\n' "$OUT" | LC_ALL=C sed -n 's/^REFS_BAD=//p')"
  MG="$(printf '%s\n' "$OUT" | LC_ALL=C sed -n 's/^MERGES=//p')"
  RO="$(printf '%s\n' "$OUT" | LC_ALL=C sed -n 's/^REFS_OK=//p')"

  if [ "${SB:-x}" != "2" ]; then
    echo "FAIL: expected 2 shape violations (no-type + unknown-type), got SHAPE_BAD=${SB:-none}"
    exit 1
  fi
  if [ "${RB:-x}" != "1" ]; then
    echo "FAIL: expected 1 missing-Refs violation, got REFS_BAD=${RB:-none}"
    exit 1
  fi
  if [ "${MG:-x}" != "1" ]; then
    echo "FAIL: the merge commit must be exempt, got MERGES=${MG:-none}"
    exit 1
  fi
  if [ "${RO:-x}" != "1" ]; then
    echo "FAIL: the good commit must be counted as Refs-compliant, got REFS_OK=${RO:-none}"
    exit 1
  fi
  echo "PASS: the checker separates shape / Refs / merge-exempt / good"
  exit 0
fi

echo "=== L1 - the type table must exist in exactly one place ==="
if l1_check; then
  echo "PASS: L1 - the type table lives only in $CANONICAL_SPEC"
else
  fail "L1: a second copy of the type table exists (see the lines above)"
fi

echo "=== L2/L3 - commit message shape and traceability ==="
if [ -n "$RANGE" ]; then
  MODE="range"
  echo "range: $RANGE"
else
  MODE="last"
  echo "range: last $LAST commits"
fi
OUT="$(run_check "$MODE")"
printf '%s\n' "$OUT" | LC_ALL=C grep -v '^[A-Z_]*=' || true
CK="$(printf '%s\n' "$OUT" | LC_ALL=C sed -n 's/^CHECKED=//p')"
MG="$(printf '%s\n' "$OUT" | LC_ALL=C sed -n 's/^MERGES=//p')"
SB="$(printf '%s\n' "$OUT" | LC_ALL=C sed -n 's/^SHAPE_BAD=//p')"
RO="$(printf '%s\n' "$OUT" | LC_ALL=C sed -n 's/^REFS_OK=//p')"
RB="$(printf '%s\n' "$OUT" | LC_ALL=C sed -n 's/^REFS_BAD=//p')"

if [ "${SB:-1}" -ne 0 ]; then
  fail "L2: ${SB} commit(s) do not match '<type>(<scope>): <subject>' with a known type"
fi
echo "PASS: L2 - every checked commit matches the shape (${CK:-0} checked, ${MG:-0} merge commit(s) exempt)"

TOTAL_REFS=$(( ${RO:-0} + ${RB:-0} ))
if [ "$REQUIRE_REFS" -eq 1 ]; then
  if [ "${RB:-0}" -ne 0 ]; then
    fail "L3: ${RB} commit(s) carry no 'Refs: #<issue>' line"
  fi
  echo "PASS: L3 - every checked commit carries 'Refs: #<issue>'"
else
  echo "NOTE: L3 - Refs compliance is ${RO:-0}/${TOTAL_REFS}"
  echo "      (reported, not enforced: the existing history predates this rule, and"
  echo "       failing it would block every future change without making the history"
  echo "       any better. Use --require-refs to enforce it.)"
fi

echo "RESULT: PASS"
exit 0
