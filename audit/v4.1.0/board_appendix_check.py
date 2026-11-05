#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""board_appendix_check.py — 核对看板 ISSUE 附录与仓内交接正文是否同一份。

交接提示词是接手者唯一入口：看板 #5592 附录 A 与 ISSUE #5599 正文都必须是
`docs/knowledge-loop/handoffs/HANDOFF_..._ONBOARDING.md` 的原文。历史上这类
「正文已改、ISSUE 还挂着旧版」的漂移全靠人眼发现，所以把它做成机器判据。

用法：
  python3 audit/v4.1.0/board_appendix_check.py                 # 核对 #5592 + #5599
  python3 audit/v4.1.0/board_appendix_check.py --base-url URL  # 反向变异用（假 API）

退出码严格三态（与 kanban_render 同规）：
  0 = 两处都与仓内源文件逐字一致
  1 = 不一致（含附录缺失、字节/哈希漂移、ISSUE 正文与源文件分叉）
  2 = 没能核对（API 不可达 / 响应不合法）。**绝不把「没能核对」算成 0。**
"""
import hashlib
import json
import pathlib
import re
import subprocess
import sys
import urllib.error
import urllib.request

REPO = pathlib.Path(__file__).resolve().parents[2]
SRC = REPO / "docs/knowledge-loop/handoffs/HANDOFF_2026-11-05_4.1.0_OMP_ONBOARDING.md"
BOARD, HANDOFF = "5592", "5599"
BASE = "http://192.168.0.252:3000/api/v1"
HEADER = "## 附录 A：交接正文全文"


def token():
    p = subprocess.run(["git", "credential", "fill"],
                       input=b"url=http://192.168.0.252:3000\n\n", capture_output=True)
    for line in p.stdout.decode().splitlines():
        if line.startswith("password="):
            return line.split("=", 1)[1]
    raise RuntimeError("no credential")


def get_body(base, issue):
    req = urllib.request.Request(base + "/repos/openclaw/sqlrustgo/issues/" + issue,
                                 headers={"Authorization": "token " + token()})
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.loads(r.read().decode())["body"]


def _opt(args, name):
    return args[args.index(name) + 1] if name in args else None


def main():
    base = BASE
    args = sys.argv[1:]
    if "--base-url" in args:
        base = args[args.index("--base-url") + 1]
    # 离线夹具入口：反向变异 harness 用假正文/假源文件喂进来，不碰线上 ISSUE。
    src = pathlib.Path(_opt(args, "--src") or SRC)
    board_f, handoff_f = _opt(args, "--board-file"), _opt(args, "--handoff-file")
    if not src.is_file():
        print("INCOMPLETE: 源文件不在：" + str(src) + " —— 没能核对")
        return 2
    src_text = src.read_text(encoding="utf-8").strip()
    src_bytes = len(src.read_bytes())
    inj_bytes = len(src_text.encode("utf-8"))
    sha = hashlib.sha256(src_text.encode("utf-8")).hexdigest()
    findings = []

    try:
        board = pathlib.Path(board_f).read_text(encoding="utf-8") if board_f else get_body(base, BOARD)
        handoff = pathlib.Path(handoff_f).read_text(encoding="utf-8") if handoff_f else get_body(base, HANDOFF)
    except (urllib.error.URLError, urllib.error.HTTPError, OSError, KeyError, ValueError) as e:
        print("INCOMPLETE: 读不到 ISSUE 正文（%s）—— 没能核对，不算通过" % type(e).__name__)
        return 2

    if HEADER not in board:
        findings.append("看板 #%s 没有附录 A 段" % BOARD)
    else:
        sec = board.split(HEADER, 1)[1]
        m = re.search(r"文件 (\d+) 字节｜注入正文 (\d+) 字节（strip 后）｜正文 sha256 `([0-9a-f]{64})`", sec)
        if not m:
            findings.append("看板附录头缺「文件字节/注入字节/正文 sha256」三元组（旧格式或漏写）")
        else:
            got_file, got_inj, got_sha = int(m.group(1)), int(m.group(2)), m.group(3)
            if (got_file, got_inj) != (src_bytes, inj_bytes):
                findings.append("字节口径漂移：看板标 %d/%d，仓内实测 %d/%d"
                                % (got_file, got_inj, src_bytes, inj_bytes))
            if got_sha != sha:
                findings.append("哈希漂移：看板附录 sha256 %s…%s，仓内源文件 %s…%s"
                                % (got_sha[:12], got_sha[-6:], sha[:12], sha[-6:]))
        if src_text[:200] not in sec:
            findings.append("看板附录正文与仓内源文件不同（前 200 字不匹配）—— 正文已改而看板没重刷")

    if src_text[:200] not in handoff:
        findings.append("交接 ISSUE #%s 正文与仓内源文件不同（前 200 字不匹配）" % HANDOFF)

    out = {"kind": "board_appendix_check", "src": str(src.relative_to(REPO) if src.is_relative_to(REPO) else src),
           "src_bytes": src_bytes, "injected_bytes": inj_bytes, "src_sha256": sha,
           "board_len": len(board), "handoff_len": len(handoff),
           "findings": len(findings)}
    print(json.dumps(out, ensure_ascii=False))
    for f in findings:
        print("NOT-OK:", f)
    if findings:
        return 1
    print("OK: 看板附录与交接 ISSUE 均与仓内源文件逐字一致")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
