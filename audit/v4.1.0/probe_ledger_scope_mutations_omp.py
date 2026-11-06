#!/usr/bin/env python3.14
"""台账口径改动的反向变异探针（IL-4 / IL-2 批，2026-11-06）。

改了什么
--------
`verify_ledger.py` 的 L1 旧判据是「台账行数 == 有分诊行的 issue 数」—— 它把
**台账范围**与**分诊归档范围**当成一回事。范围一漂移（issue 在分诊之后被关闭，
或当期新开还没分诊）它就报「行数不对」，而且分不清「台账漏了一行」与「台账按
当期集收窄」。现在：

* L1 改成**逐行**判据：每行必须有分诊记录 + domain/automatable；
* 「台账该有的行集」交给 `probe_ledger_freshness.py snapshot-count`（它能拉现场
  open 集，参照集 = 已分诊 ∩ 当期 open，并点名 `untriaged_open`）。

所以必须证明一件事：**删掉一行，仍然会被抓住** —— 只是抓它的判据从「数行数」
换成了「能看见现场 open 集的探针」。

两臂
----
CTRL  原样台账 ⇒ verify_ledger 绿、snapshot-count consistent。
MUT   从台账副本里删掉一行（一个**仍然开放且已分诊**的 issue）
      ⇒ verify_ledger 仍绿（它不再数行数，这是改动的预期），
        **snapshot-count 必须变红并点名被删的那条** ⇒ 缺行没被放过。

工具不是门禁、不进 STAGE_CONFIG ⇒ 分母 168 不动。
"""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
LEDGER = REPO / "docs/releases/v4.1.0/ISSUE_LEDGER_2026-11-05B.jsonl"
LEDGER_MD = REPO / "docs/releases/v4.1.0/ISSUE_LEDGER_2026-11-05B.md"
VERIFY = REPO / "audit/v4.1.0/verify_ledger.py"
PROBE = REPO / "audit/v4.1.0/probe_ledger_freshness.py"


def run(cmd: list[str]) -> tuple[int, str]:
    p = subprocess.run(cmd, cwd=REPO, capture_output=True, text=True)
    return p.returncode, p.stdout + p.stderr


def first_json(out: str) -> dict:
    for line in out.splitlines():
        line = line.strip()
        if line.startswith("{"):
            try:
                return json.loads(line)
            except json.JSONDecodeError:
                continue
    return {}


def main() -> int:
    if not LEDGER.is_file():
        print(f"FAIL: 找不到台账 {LEDGER}", file=sys.stderr)
        return 2

    ok = True

    # ---- CTRL ----
    rc_v, out_v = run([sys.executable, str(VERIFY)])
    rc_p, out_p = run([sys.executable, str(PROBE), "snapshot-count"])
    ctrl_ok = rc_v == 0 and rc_p == 0
    print(f"  [CTRL ] verify_ledger rc={rc_v} / snapshot-count rc={rc_p} ⇒ "
          f"{'OK' if ctrl_ok else 'BAD'}（原样台账必须两个判据都绿）")
    ok = ok and ctrl_ok

    # ---- MUT：删一行 ----
    rows = [json.loads(l) for l in LEDGER.read_text(encoding="utf-8").splitlines() if l.strip()]
    if not rows:
        print("FAIL: 台账为空", file=sys.stderr)
        return 2
    victim = rows[0]
    tmp = Path(tempfile.mkdtemp(prefix="ledger-scope-mut-"))
    try:
        mut_jsonl = tmp / "ledger.jsonl"
        mut_md = tmp / "ledger.md"
        mut_jsonl.write_text(
            "".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows[1:]), encoding="utf-8"
        )
        shutil.copy(LEDGER_MD, mut_md)

        rc_mv, out_mv = run([sys.executable, str(VERIFY), "--ledger-jsonl", str(mut_jsonl),
                              "--ledger-md", str(mut_md)])
        rc_mp, out_mp = run([sys.executable, str(PROBE), "snapshot-count",
                              "--ledger-jsonl", str(mut_jsonl), "--ledger-md", str(mut_md)])
        body = first_json(out_mp)
        named = str(victim["number"]) in json.dumps(body, ensure_ascii=False)
        # 期望：verify_ledger 不再因「行数」红（它只判逐行站不站得住），
        # 而 snapshot-count 必须红且点名被删的那条。
        mut_ok = rc_mp != 0 and named and body.get("result") == "DRIFT"
        print(f"  [MUT  ] 删掉 #{victim['number']} ⇒ verify_ledger rc={rc_mv}（预期 0："
              f"它不再数行数）/ snapshot-count rc={rc_mp}、result={body.get('result')}、"
              f"点名该条={named} ⇒ {'OK' if mut_ok else 'BAD'}（缺行必须被抓住并点名）")
        ok = ok and mut_ok
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    print(json.dumps({"kind": "ledger_scope_mutations", "arms": 2,
                      "ok": 2 if ok else 0, "not_ok": 0 if ok else 2,
                      "verdict": "VERIFIED" if ok else "FAILED"}, ensure_ascii=False))
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
