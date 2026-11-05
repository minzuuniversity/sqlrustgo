#!/usr/bin/env bash
# 反向变异复现器（AGENTS §2.5）：证明 verify_ledger.py 的判据是"活的"，不是恒绿。
# 六臂：CTRL 必须绿；A/B1/B2/B3/B4 五类故障注入必须红。任一臂反着来 ⇒ 本脚本 exit 1。
# 本脚本不进门禁分母（audit/ 下的 run 级复现器，不是 check_*，不在 STAGE_CONFIG 阶段清单）。
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
V="$HERE/verify_ledger.py"
LMD="$REPO/docs/releases/v4.1.0/ISSUE_LEDGER_2026-11-05B.md"
LJL="$REPO/docs/releases/v4.1.0/ISSUE_LEDGER_2026-11-05B.jsonl"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

N_OK=0; N_BAD=0
run_expect() {  # $1=arm $2=expect(green|red) rest=verify_ledger args
  local arm="$1" want="$2"; shift 2
  local out rc got ok
  out="$(python3 "$V" "$@" 2>&1)"; rc=$?
  got=green; [[ $rc -ne 0 ]] && got=red
  ok=0; [[ "$got" == "$want" ]] && ok=1
  if [[ $ok -eq 1 ]]; then N_OK=$((N_OK+1)); else
    N_BAD=$((N_BAD+1))
    echo "ARM-MISMATCH $arm want=$want got=$got rc=$rc"
    printf '%s\n' "$out" | tail -3
  fi
  echo "ARM $arm want=$want got=$got rc=$rc ok=$ok"
}

run_expect CTRL green --label CTRL

# A：family 门限放宽 2 -> 1 ⇒ 等级标签不再自洽（证明等级受规则约束，不是自报）
run_expect A-MUT red --label A-MUT --family-min 1

# B1：改一条输入指纹的一个十六进制位 ⇒ L3a 抓指纹漂移
python3 - "$LMD" "$WORK/b1.md" <<'PY'
import re, sys, pathlib
src, dst = sys.argv[1], sys.argv[2]
s = pathlib.Path(src).read_text(encoding="utf-8")
m = re.search(r"- `[A-Za-z0-9_.-]+\.(?:jsonl|json)` = `sha256:([0-9a-f])([0-9a-f]{63})`", s)
assert m, "没找到输入指纹行"
first = "0" if m.group(1) != "0" else "1"
n = s[:m.start(1)] + first + s[m.start(1) + 1:]
assert n != s
pathlib.Path(dst).write_text(n, encoding="utf-8")
PY
run_expect B1-MUT red --label B1-MUT --ledger-md "$WORK/b1.md"

# B2：把一行 advisory-only 抬成 cross-checked（指纹未动，只有等级虚报）⇒ L2 必须红
python3 - "$LJL" "$WORK/b2.jsonl" <<'PY'
import json, sys, pathlib
src, dst = sys.argv[1], sys.argv[2]
lines = [l for l in pathlib.Path(src).read_text(encoding="utf-8").splitlines() if l.strip()]
hit = False
for i, l in enumerate(lines):
    r = json.loads(l)
    if r.get("evidence_grade") == "advisory-only" and len(r.get("triage_families") or []) < 2:
        r["evidence_grade"] = "cross-checked"
        lines[i] = json.dumps(r, ensure_ascii=False)
        hit = True
        break
assert hit, "找不到可注入的单血统行"
pathlib.Path(dst).write_text("\n".join(lines) + "\n", encoding="utf-8")
PY
run_expect B2-MUT red --label B2-MUT --ledger-jsonl "$WORK/b2.jsonl"

# B3：改 evidence_hash 的一个十六进制位 ⇒ L3b 重算必须抓（指纹与总哈希脱钩）
python3 - "$LMD" "$WORK/b3.md" <<'PY'
import re, sys, pathlib
src, dst = sys.argv[1], sys.argv[2]
s = pathlib.Path(src).read_text(encoding="utf-8")
m = re.search(r"evidence_hash \| `?(sha256:[0-9a-f])([0-9a-f]{63})`?", s)
assert m, "没找到 evidence_hash 行"
first = "0" if m.group(1) != "0" else "1"
n = s[:m.start(1)] + first + s[m.start(1) + 1:]
assert n != s
pathlib.Path(dst).write_text(n, encoding="utf-8")
PY
run_expect B3-MUT red --label B3-MUT --ledger-md "$WORK/b3.md"

# B4：从 MD 删掉一条输入指纹（少报一个输入）⇒ 必须红
python3 - "$LMD" "$WORK/b4.md" <<'PY'
import re, sys, pathlib
src, dst = sys.argv[1], sys.argv[2]
s = pathlib.Path(src).read_text(encoding="utf-8")
lines = s.splitlines(keepends=True)
for i, l in enumerate(lines):
    if re.match(r"- `[A-Za-z0-9_.-]+\.(jsonl|json)` = `sha256:[0-9a-f]{64}`\s*$", l):
        del lines[i]
        break
else:
    raise SystemExit("没找到可删的指纹行")
pathlib.Path(dst).write_text("".join(lines), encoding="utf-8")
PY
run_expect B4-MUT red --label B4-MUT --ledger-md "$WORK/b4.md"

echo "verify_ledger_mutation: ok=$N_OK bad=$N_BAD"
if [[ $N_BAD -ne 0 ]]; then
  echo "verify_ledger_mutation: $N_BAD 个臂未按预期 ⇒ 判据可疑（不可据此声称校验有效）"
  exit 1
fi
echo "verify_ledger_mutation: OK (CTRL green; A/B1/B2/B3/B4 red)"
exit 0
