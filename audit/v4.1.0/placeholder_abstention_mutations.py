#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# EX-7 判据的反向变异（AGENTS 2.5：每条红灯必须能证伪）。
#
#   python3 audit/v4.1.0/placeholder_abstention_mutations.py
#
# 每条臂只在临时目录的 expert_ladder 副本里注入一个缺陷，仓库文件不动。
# 期望：注入后 placeholder_abstention_probe 必须变红；基线副本必须保持绿
# （基线变红 ⇒ 判据本身已过敏/过期）。
#
# 退出码：0 = 全部反臂变红且基线保持绿；1 = 有反臂仍绿（判据不承重）；
#         3 = 变异锚点命中次数 != 1（源已漂，证据过期）。
from __future__ import annotations

import pathlib
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[1]
LADDER = REPO / "scripts" / "gate" / "expert_ladder.py"
PROBE = HERE / "placeholder_abstention_probe.py"

# (name, [(锚点, 替换), ...]) —— 每个锚点必须在源里恰好出现一次。
# m3 是「整个短回复纪律」两道护栏一起撤：实测单独撤 64 字上限或单独撤邻近性
# 都仍然绿（两道护栏互相兜底），所以最小失效面是同时撤两道。这一点本身就是
# 反向变异测出来的结论，写在这里而不是猜一条假臂。
ARMS = [
    ("m1-detector-unhooked",
     [("ok=bool(content) and not truncated_empty and placeholder is None,",
       "ok=bool(content) and not truncated_empty,")]),
    ("m2-marker-list-emptied",
     [('PLACEHOLDER_MARKERS = (\n    "无法加载此回复",',
       'PLACEHOLDER_MARKERS = (\n    "NEVER-APPEARS-MARKER",')]),
    ("m3-short-reply-discipline-removed",
     [("    if not text or len(text) > PLACEHOLDER_MAX_CHARS:",
       "    if not text:"),
      ("or (m in low and len(low) <= len(m) + 24)",
       "or (m in low)")]),
]


def main() -> int:
    src = LADDER.read_text(encoding="utf-8")
    for name, edits in ARMS:
        for anchor, _ in edits:
            n = src.count(anchor)
            if n != 1:
                print(f"drift: 锚点 {name} 命中 {n} 次（期望 1）⇒ 源已漂，证据过期", file=sys.stderr)
                return 3

    def run_probe(ladder: pathlib.Path) -> int:
        p = subprocess.run([sys.executable, str(PROBE), "--ladder", str(ladder)],
                           capture_output=True, text=True, timeout=120)
        return p.returncode

    with tempfile.TemporaryDirectory(prefix="srg-ex7-mut-") as td:
        tmp = pathlib.Path(td)
        base = tmp / "expert_ladder_base.py"
        base.write_text(src, encoding="utf-8")
        baseline = run_probe(base)
        if baseline != 0:
            print("DRIFT: 基线副本未保持绿 ⇒ 探针/判据本身已坏，变异结论无效", file=sys.stderr)
            return 1

        flipped = 0
        stayed = 0
        drift = 0
        for name, edits in ARMS:
            mutated = tmp / f"expert_ladder_{name}.py"
            text = src
            for anchor, repl in edits:
                text = text.replace(anchor, repl, 1)
            mutated.write_text(text, encoding="utf-8")
            code = run_probe(mutated)
            if code == 0:
                stayed += 1
                print(f"  NOT-RED {name}: 注入缺陷后探针仍绿 ⇒ 判据不承重", file=sys.stderr)
            else:
                flipped += 1
                print(f"  red  {name}: exit={code}")

        print(f"arms={len(ARMS)} flipped_red={flipped} stayed_green={stayed} drift={drift} baseline_green=1")
        return 0 if stayed == 0 and drift == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
