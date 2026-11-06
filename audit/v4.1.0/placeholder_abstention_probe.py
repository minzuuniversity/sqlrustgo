#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# EX-7：席位「页面占位文案」必须判为弃权（expert_ladder.http_chat）。
#
#   python3 audit/v4.1.0/placeholder_abstention_probe.py [--ladder <path>]
#
# 背景（实测 run 20261105-est2-008 / log 003）：ChatGPT 桥返回
# ok=true、finish_reason=stop，但 content 是页面占位「无法加载此回复」。
# 席位层 fail-open ⇒ 该读数进入估算血统。本探针把「占位文案=弃权」变成可核对判据。
#
# 判据：占位文案 ⇒ ok=false 且 error 以 placeholder-content 开头；
#       长答案里只是「引用」该词 ⇒ ok=true（否则判据永远红）。
# 退出码：0 = 全部符合；1 = 有 case 不符；3 = 无法加载待测模块。
from __future__ import annotations

import argparse
import importlib.util
import json
import pathlib
import sys

DEFAULT_LADDER = pathlib.Path(__file__).resolve().parents[2] / "scripts" / "gate" / "expert_ladder.py"

# (名称, content, finish_reason, 期望 ok)
CASES: list[tuple[str, str, str, bool]] = [
    ("zh-placeholder-exact", "无法加载此回复", "stop", False),
    ("zh-placeholder-prefixed", "无法加载此回复，请稍后重试", "stop", False),
    ("en-placeholder-caps", "Couldn't Load This Response.", "stop", False),
    ("zh-generic-error-page", "出了点问题", "stop", False),
    ("control-long-answer-quoting-term",
     '{"verdict":"REAL","confidence":0.8,"evidence":"audit/v4.1.0/x"} 说明：上一轮曾出现「无法加载此回复」，'
     '本轮已重试并拿到完整读数，三档口径与并集区间均已按机器复算核对，复现命令见 run 归档。',
     "stop", True),
    ("control-normal-short", '{"verdict":"REAL","confidence":0.9,"evidence":"a/b/c"}', "stop", True),
]


class FakeResp:
    def __init__(self, payload: dict) -> None:
        self._raw = json.dumps(payload).encode("utf-8")

    def read(self) -> bytes:
        return self._raw

    def __enter__(self):
        return self

    def __exit__(self, *exc) -> bool:
        return False


def load(path: pathlib.Path):
    spec = importlib.util.spec_from_file_location("ladder_under_probe", path)
    mod = importlib.util.module_from_spec(spec)
    assert spec and spec.loader
    sys.modules[spec.name] = mod
    spec.loader.exec_module(mod)
    return mod


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--ladder", default=str(DEFAULT_LADDER))
    args = ap.parse_args()
    ladder_path = pathlib.Path(args.ladder)
    if not ladder_path.is_file():
        print(f"placeholder_probe: 找不到待测模块 {ladder_path}", file=sys.stderr)
        return 3
    try:
        L = load(ladder_path)
    except Exception as exc:  # noqa: BLE001
        print(f"placeholder_probe: 加载失败 {type(exc).__name__}: {exc}", file=sys.stderr)
        return 3

    seat = L.Seat(id="probe-seat", family="probe", endpoint="http://127.0.0.1:1/v1/chat/completions",
                  model="probe", tier=1, roles=["adjudicate"], timeout_s=5)
    failures: list[str] = []
    for name, content, finish, want_ok in CASES:
        payload = {"choices": [{"message": {"content": content}, "finish_reason": finish}],
                   "usage": {"completion_tokens": 9, "prompt_tokens": 9}}
        L.urllib.request.urlopen = lambda req, timeout=None, _p=payload: FakeResp(_p)
        rec = L.http_chat(seat, "q")
        err = rec.get("error") or ""
        if rec.get("ok") is not want_ok:
            failures.append(f"{name}: ok={rec.get('ok')} 期望 {want_ok} error={err[:80]}")
        elif not want_ok and not err.startswith("placeholder-content"):
            failures.append(f"{name}: ok=false 但 error 未标 placeholder-content: {err[:80]}")
        print(("  OK   " if name not in [f.split(':', 1)[0] for f in failures] else "  FAIL ") + name)

    print(f"cases={len(CASES)} failures={len(failures)} ladder={ladder_path.name}")
    for f in failures:
        print("  " + f, file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
