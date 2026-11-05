#!/usr/bin/env python3
"""EX-6 改动 1 的反向变异：标定账必须把「没答上来」与「答错了」分开记。

问题（看板 EX-6）：
    `scripts/gate/expert_ladder.py` 的 `run_calibration` 原来把两种失效压成同一个形状
    `{"ok": false, "answer": ""}`。台账读者因此分不出「这席位不可信」和
    「这席位当时正忙」—— 20261104-seats-002 的 3/4 就是**超时**被记成了「答错」。

改法：item 记录增加 `failure` 字段 ∈ {None, "unavailable", "wrong-answer"}，
    汇总增加 `unavailable` / `wrong_answer` 两个计数。

本探针用**假 answer** 驱动同一份 `run_calibration`：
    A 超时/不可用 ⇒ 必须 unavailable=4、wrong_answer=0
    B 答上来但内容不对 ⇒ 必须 wrong_answer=4、unavailable=0
    C 全对 ⇒ 两个计数都为 0、score=4
    D **反向变异**：把源码改回「超时也记成 wrong-answer」的旧写法，
      跑同一个超时场景 ⇒ A 臂必须失败（否则这次改动没判别力）

退出码：0 = 全部成立；1 = 有不成立项；2 = 用法错
本探针放在 audit/v4.1.0/：不是门禁，不进 STAGE_CONFIG，分母 168 不动。
"""
from __future__ import annotations

import json
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
LADDER = REPO / "scripts" / "gate" / "expert_ladder.py"
sys.path.insert(0, str(REPO / "scripts" / "gate"))
import expert_ladder as el  # noqa: E402


class FakeSeat:
    id = "fake-seat"
    family = "fake"
    lineage = "fake"
    model = "fake-model"


def run_with(fake_rec: dict) -> dict:
    """用假的 ask 跑同一份 run_calibration。"""
    orig = el.ask

    def fake_ask(seat, prompt, **kw):
        r = dict(fake_rec)
        r.setdefault("content", "")
        r.setdefault("ok", True)
        r.setdefault("prompt_sha256", "sha256:" + "0" * 64)
        r.setdefault("output_sha256", "sha256:" + "0" * 64)
        r.setdefault("requested_at", "2026-11-05T00:00:00Z")
        return r
    el.ask = fake_ask
    try:
        return el.run_calibration(FakeSeat())
    finally:
        el.ask = orig


def main() -> int:
    checks: list[tuple[str, bool, str]] = []

    # A 超时 / 不可用
    r = run_with({"ok": False, "content": "", "error": "TimeoutError: timed out"})
    ok_a = (r.get("unavailable") == 4 and r.get("wrong_answer") == 0
            and r.get("score") == 0
            and all(i.get("failure") == "unavailable" for i in r.get("items", [])))
    checks.append(("timeout-counted-as-unavailable", ok_a,
                   f"unavailable={r.get('unavailable')} wrong_answer={r.get('wrong_answer')} "
                   f"score={r.get('score')}"))

    # B 答上来但内容不对
    r = run_with({"ok": True, "content": "definitely-not-the-answer"})
    ok_b = (r.get("unavailable") == 0 and r.get("wrong_answer") == 4
            and r.get("score") == 0
            and all(i.get("failure") == "wrong-answer" for i in r.get("items", [])))
    checks.append(("wrong-answer-counted-as-wrong", ok_b,
                   f"unavailable={r.get('unavailable')} wrong_answer={r.get('wrong_answer')}"))

    # C 同一个假答案只满足第 1 题（其余 3 题内容不对）
    r = run_with({"ok": True, "content": "391"})
    ok_c = (r.get("unavailable") == 0 and r.get("wrong_answer") == 3
            and r.get("score") == 1
            and sum(1 for i in r.get("items", []) if i.get("failure") is None) == 1)
    checks.append(("all-correct-has-no-failures", ok_c,
                   f"score={r.get('score')} unavailable={r.get('unavailable')} "
                   f"wrong_answer={r.get('wrong_answer')}（期望 1 对 3 错、0 不可用）"))

    # D 反向变异：把源码改回「超时也记成 wrong-answer」的旧写法
    src = LADDER.read_text(encoding="utf-8")
    mutated = src.replace(
        '''        if not rec.get("ok"):
            ok = False
            failure = "unavailable"      # 没答上来（超时 / 传输故障 / 产出不可用）
            n_unavailable += 1
        elif not ok:
            failure = "wrong-answer"     # 答上来了，但内容不对
            n_wrong += 1
        else:
            failure = None''',
        '''        if not rec.get("ok"):
            ok = False
            failure = "wrong-answer"
            n_wrong += 1
        else:
            failure = None if ok else "wrong-answer"''')
    if mutated == src:
        checks.append(("mutation-timeout-as-wrong-is-detected", False,
                       "变异没生效：源码形状与预期不符（EX-6 改动可能被改过）"))
    else:
        tmp = Path(tempfile.mkdtemp(prefix="ex6mut-"))
        try:
            mut_ladder = tmp / "expert_ladder.py"
            mut_ladder.write_text(mutated, encoding="utf-8")
            # 把被测文件与它的依赖都放到临时目录，import 那里的副本
            for dep in ("expert_triage.py",):
                src_dep = REPO / "scripts" / "gate" / dep
                if src_dep.exists():
                    shutil.copy(src_dep, tmp / dep)
            probe = (
                "import json,sys;sys.path.insert(0,%r);import expert_ladder as el;"
                "el.ask=lambda seat,prompt,**kw:{'ok':False,'content':'','error':'TimeoutError',"
                "'prompt_sha256':'sha256:'+'0'*64,'output_sha256':'sha256:'+'0'*64,"
                "'requested_at':'2026-11-05T00:00:00Z'};"
                "r=el.run_calibration(type('S',(),{'id':'f','family':'f','lineage':'f','model':'m'})());"
                "print(json.dumps({'unavailable':r.get('unavailable'),"
                "'wrong_answer':r.get('wrong_answer')}))"
            ) % str(tmp)
            p = subprocess.run([sys.executable, "-c", probe], capture_output=True,
                               text=True, timeout=120)
            try:
                got = json.loads(p.stdout.strip().splitlines()[-1])
            except Exception:
                got = {"unavailable": "?", "wrong_answer": "?"}
            detected = got.get("unavailable") != 4 or got.get("wrong_answer") != 0
            checks.append(("mutation-timeout-as-wrong-is-detected", detected,
                           f"变异后 unavailable={got.get('unavailable')} "
                           f"wrong_answer={got.get('wrong_answer')}（应与真实现不同 ⇒ 判据有判别力）"))
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    bad = [(n, d) for n, ok, d in checks if not ok]
    print(json.dumps({"kind": "ex6_failure_accounting",
                      "checks": len(checks), "ok": len(checks) - len(bad),
                      "not_ok": len(bad),
                      "detail": [{"check": n, "detail": d} for n, d in bad],
                      "verdict": "FAILED" if bad else "VERIFIED"}, ensure_ascii=False))
    return 1 if bad else 0


if __name__ == "__main__":
    raise SystemExit(main())
