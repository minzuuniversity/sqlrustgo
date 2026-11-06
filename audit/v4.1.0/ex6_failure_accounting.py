#!/usr/bin/env python3
"""EX-6 的反向变异探针：标定账必须把「没答上来」与「答错了」分开记。

2026-11-06 合并说明：两条线各自实现了同一个 EX-6 修复。`scripts/gate/expert_ladder.py`
取 mcode-macmini 那版（超集：空返回先重试一次，再分 unanswered / wrong，并给出
competence_ratio），本探针因此改成断言那套字段名；场景与判别方式不变。
原版探针断言的 `failure`/`unavailable`/`wrong_answer` 字段随旧实现一并让位。

问题（看板 EX-6）：`run_calibration` 原先把「超时/空返回」与「答错」压成同一个
`{"ok": false, "answer": ""}`。实测受害样本：run 20261104-seats-002 给
flash-next-252 记 3/4，失败项 answer 为空、那一次延迟 101.14 s，而同席位在
20261104-seats-001 是 4/4（15.0 s）—— 一次空调用被记成了能力不足。

臂：
    A 超时/空返回 ⇒ unanswered=4 项、wrong=[]、answered=0、score=0
    B 答上来了但内容不对 ⇒ wrong=4 项、unanswered=[]、answered=4
    C 混合（假答案只命中 1 题）⇒ score=1、wrong=3、competence_ratio=0.25
    D 空返回后重试拿到答案 ⇒ 必须算 answered（负载尖峰不冤枉席位）
    E **反向变异**：把「空返回」也算成答错 ⇒ A 臂必须判红
      改前漏什么：超时被记成答错，台账分不出「模型不行」和「这次没连上」
      改后多报什么：把 unavailable 一律塞进 wrong ⇒ A 臂的 unanswered 变空、wrong 变 4

退出码：0 = 全部成立；1 = 有不成立项；2 = 用法错
本探针放在 audit/v4.1.0/：不是门禁，不进 STAGE_CONFIG，分母 168 不动。
"""
from __future__ import annotations

import importlib.util
import shutil
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
LADDER = REPO / "scripts" / "gate" / "expert_ladder.py"

MUTATE_SRC = '''        item_answered = bool(text.strip()) and bool(rec.get("ok"))'''
MUTATE_TO = '''        item_answered = True  # 变异：空返回也当答上来了'''


class FakeSeat:
    name = "fake-seat"
    model = "fake"
    provider = "fake"
    base_url = "http://127.0.0.1:1"
    api_key_env = "FAKE_KEY"
    role = "audit"
    max_tokens = 64
    timeout_sec = 1


def load_ladder(path: Path):
    spec = importlib.util.spec_from_file_location("el_probe_mod", path)
    mod = importlib.util.module_from_spec(spec)
    # 3.14 的 @dataclass 处理要查 sys.modules 里的模块命名空间，
    # 不注册就 AttributeError: 'NoneType' object has no attribute '__dict__'。
    sys.modules[spec.name] = mod
    spec.loader.exec_module(mod)
    return mod


def run_with(path: Path, responses: list[dict]) -> tuple[dict, int]:
    """responses 按调用顺序回放（空返回会触发重试，故可给多条）。"""
    el = load_ladder(path)
    orig = el.ask
    state = {"n": 0}

    def fake_ask(seat, prompt, **kw):
        i = state["n"]
        state["n"] += 1
        r = dict(responses[i] if i < len(responses) else responses[-1])
        r.setdefault("ok", True)
        r.setdefault("content", "")
        r.setdefault("output_sha256", "sha256:" + "0" * 64)
        r.setdefault("requested_at", "2026-11-05T00:00:00Z")
        return r

    el.ask = fake_ask
    try:
        return el.run_calibration(FakeSeat()), state["n"]
    finally:
        el.ask = orig


def ids(xs):
    return sorted(xs) if isinstance(xs, list) else xs


def main() -> int:
    checks: list[tuple[str, bool, str]] = []
    n_items = None

    # A 超时 / 空返回（两次都空：重试一次仍空）
    r, calls = run_with(LADDER, [{"ok": False, "content": "", "error": "TimeoutError: timed out"}])
    n_items = r.get("total")
    ok_a = (ids(r.get("unanswered")) == ids([i["id"] for i in r.get("items", [])])
            and r.get("wrong") == [] and r.get("answered") == 0 and r.get("score") == 0)
    checks.append(("A-timeout-counted-unanswered", ok_a,
                   f"unanswered={r.get('unanswered')} wrong={r.get('wrong')} "
                   f"answered={r.get('answered')} score={r.get('score')} calls={calls}"))

    # B 答上来了但内容不对
    r, _ = run_with(LADDER, [{"ok": True, "content": "definitely-not-the-answer"}])
    ok_b = (r.get("unanswered") == [] and len(r.get("wrong") or []) == n_items
            and r.get("answered") == n_items and r.get("score") == 0)
    checks.append(("B-wronganswer-counted-wrong", ok_b,
                   f"unanswered={r.get('unanswered')} wrong={r.get('wrong')} "
                   f"answered={r.get('answered')}"))

    # C 假答案只命中其中一题
    r, _ = run_with(LADDER, [{"ok": True, "content": "391"}])
    got = r.get("score")
    ok_c = (got == 1 and len(r.get("wrong") or []) == n_items - 1
            and r.get("unanswered") == [] and r.get("answered") == n_items
            and abs((r.get("competence_ratio") or 0) - round(got / n_items, 3)) < 1e-6)
    checks.append(("C-mixed-competence-ratio", ok_c,
                   f"score={got} wrong={r.get('wrong')} answered={r.get('answered')} "
                   f"competence_ratio={r.get('competence_ratio')}（期望 {round(got / n_items, 3)}）"))

    # D 空返回后重试拿到正确答案 ⇒ 必须算 answered（负载尖峰不冤枉席位）
    first_ok = None
    for probe in ([{"ok": False, "content": "", "error": "EmptyResponse"},
                   {"ok": True, "content": ""}],):
        r, calls = run_with(LADDER, probe)
        first_ok = any(i.get("retried") for i in r.get("items", []))
    checks.append(("D-empty-response-retried", bool(first_ok),
                   f"items={[{i['id']: i.get('retried')} for i in r.get('items', [])]} "
                   f"answered={r.get('answered')}"))

    # E 反向变异：空返回也当答上来了
    src = LADDER.read_text(encoding="utf-8")
    if MUTATE_SRC not in src:
        checks.append(("E-mutation-anchor-found", False,
                       f"变异锚点没找到：{MUTATE_SRC!r} 不在源码里 ⇒ 判据可能已失效"))
    else:
        mutated = src.replace(MUTATE_SRC, MUTATE_TO, 1)
        tmp = Path(tempfile.mkdtemp(prefix="ex6mut-"))
        try:
            mut = tmp / "expert_ladder.py"
            mut.write_text(mutated, encoding="utf-8")
            r_mut, _ = run_with(mut, [{"ok": False, "content": "", "error": "TimeoutError: timed out"}])
            detected = (r_mut.get("unanswered") != r.get("unanswered")
                        or len(r_mut.get("wrong") or []) > 0)
            checks.append(("E-mutation-timeout-as-wrong-detected", detected,
                           f"变异后 unanswered={r_mut.get('unanswered')} wrong={r_mut.get('wrong')} "
                           f"（真实现下 unanswered 非空 ⇒ 该臂有判别力）"))
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    bad = 0
    for name, ok, detail in checks:
        print(f"{'OK ' if ok else 'BAD'} {name}: {detail}")
        bad += 0 if ok else 1
    print(f"合计 {len(checks) - bad}/{len(checks)} 条成立")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
