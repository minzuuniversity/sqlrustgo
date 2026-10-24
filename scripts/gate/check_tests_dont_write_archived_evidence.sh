#!/usr/bin/env bash
# ============================================================================
# check_tests_dont_write_archived_evidence.sh
#
# #5577 / #5389 —— 测试**不得**写入仓库里已归档的证据。
#
# 问题（#5577）：跑一次 `cargo test --workspace` 会把
# `docs/releases/v4.1.0/evidence/a04-5226/wire/*.txt`（21 个）整体替换成
# 一行 `ENGINE_UNAVAILABLE: container a04-pg-ref not running`。
# 一次普通的全仓测试，就能把 #5226 的验收材料抹掉。
#
# 本门禁有两层判据：
#
#   L1（静态，精确）—— 测试源码里的**写调用**，其实参直接含 `docs/` 路径
#        字面量 => FAIL，并点名文件与行号。这一层抓「直接写」。
#
#   L2（行为，承重）—— 工作树里 `docs/` 下的**已跟踪文件**被改动 => FAIL。
#        这一层抓「一切间接写」：经由常量、经由函数、经由 env 拼出来的路径，
#        只要最后落到 `docs/` 下就会被抓到。
#        L2 需要「测试已经跑过」，所以它检查的是**当前工作树的状态**。
#
# 为什么 L2 才是承重的那一层：L1 只认字面量。
# 修之前 A04 就是经由 `evidence_root()` 拼出路径的 —— L1 看不见它。
#
# 用法：
#   bash scripts/gate/check_tests_dont_write_archived_evidence.sh
#       # 默认：L1 + L2
#   bash scripts/gate/check_tests_dont_write_archived_evidence.sh --static-only
#       # 只跑 L1（在没跑过测试的干净树上用）
#   bash scripts/gate/check_tests_dont_write_archived_evidence.sh --selftest
#       # 自测：把一条违规注入进去，门禁必须变红
#
# 退出码：0 = PASS；1 = FAIL；2 = 用法错误。
# ============================================================================
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

STATIC_ONLY=0
SELFTEST=0
for arg in "$@"; do
  case "$arg" in
    --static-only) STATIC_ONLY=1 ;;
    --selftest)    SELFTEST=1 ;;
    -h|--help)
      sed -n '2,45p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "unknown argument: $arg" >&2
      exit 2
      ;;
  esac
done

fail() { echo "FAIL: $*"; exit 1; }

# ---------------------------------------------------------------------------
# L1 —— 静态：写调用的实参里直接含 `docs/` 路径字面量
# ---------------------------------------------------------------------------
#
# 扫描范围：测试源码。`tests/` 是根包的集成测试，`crates/*/tests/` 是各 crate 的。
# 判据用**同一条**规则表达，不按文件名开例外。
#
# 这里用 python 而不是 grep：判据是「写调用的实参」，
# grep 无法区分 `fs::write(p)` 与 `fs::read(p)`。
#
# 输出的**明细行与计数行都是纯 ASCII**：这一段输出会被 `sed`/`grep` 再处理，
# 而 macOS 自带的 BSD sed 在管道里遇到非 ASCII 字节时会说
# `RE error: illegal byte sequence`（实测踩过），会把整条判据的输出吃掉。
SCAN_PY='
import re, sys, pathlib

WRITE_CALL = re.compile(
    r"(?:fs::write|std::fs::write|fs::File::create|File::create|fs::create_dir_all"
    r"|std::fs::create_dir_all|OpenOptions::new)\s*\(\s*([^;]{0,300})"
)
DOCS = re.compile(r"[\"\x27][^\"\x27]*\bdocs/[^\"\x27]*[\"\x27]")

roots = [pathlib.Path("tests")]
crates = pathlib.Path("crates")
if crates.is_dir():
    for c in sorted(crates.iterdir()):
        t = c / "tests"
        if t.is_dir():
            roots.append(t)

hits = []
for root in roots:
    for p in sorted(root.rglob("*.rs")):
        try:
            lines = p.read_text(encoding="utf-8", errors="replace").split("\n")
        except OSError:
            continue
        for i, line in enumerate(lines, 1):
            code = "" if line.lstrip().startswith("//") else line.split("//")[0]
            if not code.strip():
                continue
            for m in WRITE_CALL.finditer(code):
                arg = m.group(1)
                if DOCS.search(arg):
                    hits.append((str(p), i, m.group(0).split("(")[0]))

for path, line, fn in hits:
    # ASCII only - see the note above.
    print("  %s:%d: write call %s() targets a docs/ path directly" % (path, line, fn))
print("L1_HITS=%d" % len(hits))
sys.exit(0)
'

scan_static() {
  python3 -c "$SCAN_PY"
}

# 从 scan_static 的输出里取计数行（输入是纯 ASCII，见上面的说明）。
l1_hits_of() {
  printf '%s\n' "$1" | LC_ALL=C sed -n 's/^L1_HITS=//p'
}

# ---------------------------------------------------------------------------
# L2 —— 行为：工作树里 docs/ 下的已跟踪文件被改动
# ---------------------------------------------------------------------------
scan_worktree() {
  # `git status --porcelain -- docs/` 只报**已跟踪**文件的变化
  # （未跟踪的新文件用 `??`，这里不关心：新文件不是「覆写已归档证据」）。
  LC_ALL=C git status --porcelain -- docs/ 2>/dev/null | grep -v '^??' || true
}

if [ "$SELFTEST" -eq 1 ]; then
  echo "=== L1 selftest: inject one violation ==="
  TMP_PROBE="tests/gate_selftest_probe_5577.rs"
  # trap 保证即使脚本中途失败，探针也不会留在仓库里。
  trap 'rm -f "$TMP_PROBE"' EXIT
  cat > "$TMP_PROBE" <<'EOF'
// #5577 selftest probe - a deliberate violation; the gate must catch it.
fn main() {
    let _ = std::fs::write("docs/releases/v4.1.0/evidence/selftest.txt", "x");
}
EOF
  OUT="$(scan_static)"
  rm -f "$TMP_PROBE"
  trap - EXIT
  HITS="$(l1_hits_of "$OUT")"
  printf '%s\n' "$OUT" | LC_ALL=C grep -v '^L1_HITS=' || true
  if [ "${HITS:-0}" -lt 1 ]; then
    echo "FAIL: the injected violation was NOT caught by L1 (the gate has no discriminating power)"
    exit 1
  fi
  echo "PASS: L1 caught the injected violation (HITS=$HITS)"

  echo "=== L1 selftest: the tree itself must be clean ==="
  OUT2="$(scan_static)"
  HITS2="$(l1_hits_of "$OUT2")"
  if [ "${HITS2:-0}" -ne 0 ]; then
    printf '%s\n' "$OUT2" | LC_ALL=C grep -v '^L1_HITS=' || true
    echo "FAIL: the current tree still has ${HITS2} L1 violation(s)"
    exit 1
  fi
  echo "PASS: the current tree is L1-clean (HITS=0)"
  exit 0
fi

echo "=== L1 - static: write calls targeting a docs/ path directly ==="
L1_OUT="$(scan_static)"
L1_HITS="$(l1_hits_of "$L1_OUT")"
printf '%s\n' "$L1_OUT" | LC_ALL=C grep -v '^L1_HITS=' || true
if [ "${L1_HITS:-1}" -ne 0 ]; then
  fail "L1: ${L1_HITS} write call(s) target a docs/ path directly"
fi
echo "PASS: L1 - no write call targets a docs/ path directly"

if [ "$STATIC_ONLY" -eq 1 ]; then
  echo "SKIP: L2 (--static-only)"
  exit 0
fi

echo "=== L2 - behavioural: tracked files under docs/ were modified ==="
DIRTY="$(scan_worktree)"
if [ -n "$DIRTY" ]; then
  echo "$DIRTY"
  fail "L2: tests modified committed evidence under docs/ (files listed above). Archived evidence is a PRODUCT, not a log - regenerating it must be declared explicitly."
fi
echo "PASS: L2 - no tracked file under docs/ was modified"

echo "RESULT: PASS"
exit 0
