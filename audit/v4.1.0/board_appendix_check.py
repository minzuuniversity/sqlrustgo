#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""board_appendix_check.py — 核对看板 ISSUE 附录与仓内交接正文是否同一份。

交接提示词是接手者唯一入口：看板 #5592 附录 A 与 ISSUE #5599 正文都必须是
`docs/knowledge-loop/handoffs/HANDOFF_..._ONBOARDING.md` 的原文。历史上这类
「正文已改、ISSUE 还挂着旧版」的漂移全靠人眼发现，所以把它做成机器判据。

判据口径（2026-11-05 收紧，见 docs/releases/v4.1.0/HO2_APPENDIX_RULES_2026-11-05.md）：
  看板附录 A 的**正文**与仓内源文件**逐字相等**（不是只比前 200 字）；
  交接 ISSUE #5599 必须**整段包含**仓内源文件（它按设计额外带「发布元数据」段）；
  附录头的「文件字节 / 注入字节 / 正文 sha256」三元组必须与仓内实测一致。
只比前 200 字时，在正文**末尾**动手（追加一句、删掉最后一节）不会被发现，
而交接文档恰恰会不断长胖，末尾改动是最常见的漂移。

用法：
  python3 audit/v4.1.0/board_appendix_check.py                 # 核对 #5592 + #5599
  python3 audit/v4.1.0/board_appendix_check.py --base-url URL  # 反向变异用（假 API）
  python3 audit/v4.1.0/board_appendix_check.py --src S --board-file B --handoff-file H  # 离线夹具

NL = chr(10) 被用来代替写死的换行转义：本仓的判据脚本经常经 heredoc 生成，
反斜杠转义在那条链路上容易被吞掉，用 chr(10) 就没有这个歧义。
"""
import hashlib
import json
import pathlib
import re
import subprocess
import sys
import urllib.error
import urllib.request

NL = chr(10)
REPO = pathlib.Path(__file__).resolve().parents[2]
SRC = REPO / "docs/knowledge-loop/handoffs/HANDOFF_2026-11-05_4.1.0_OMP_ONBOARDING.md"
BOARD, HANDOFF = "5592", "5599"
BASE = "http://192.168.0.252:3000/api/v1"
HEADER = "## 附录 A：交接正文全文"
HEADER_RE = re.compile("文件 ([0-9]+) 字节｜注入正文 ([0-9]+) 字节（strip 后）｜正文 sha256 `([0-9a-f]{64})`")
BULLET_MARK = "- 本节由 `gen_kanban_issue.py --appendix` 原样注入"


def token():
    p = subprocess.run(["git", "credential", "fill"],
                       input=b"url=http://192.168.0.252:3000" + NL.encode() + NL.encode(),
                       capture_output=True)
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


def appendix_body(sec):
    """从附录段里取出**注入的正文**（丢掉标题行尾巴与说明 bullet，含正文自身的空行）。"""
    lines = sec.split(NL)
    i = 0
    while i < len(lines) and not lines[i].startswith(BULLET_MARK):
        i += 1
    if i + 1 >= len(lines):
        return ""
    return NL.join(lines[i + 2:]).strip()


def first_diff(a, b):
    n = min(len(a), len(b))
    for k in range(n):
        if a[k] != b[k]:
            return k
    return n


def _diff_msg(what, a, b):
    return what + " 与仓内源文件不逐字相同（第 %d 字起不同；此处 %d 字，源 %d 字）" % (first_diff(a, b), len(a), len(b))


def _incomplete(reason):
    print("INCOMPLETE: " + reason + " —— 没能核对，不算通过")
    return 2


def main():
    args = sys.argv[1:]
    base = _opt(args, "--base-url") or BASE
    src = pathlib.Path(_opt(args, "--src") or SRC)
    board_f, handoff_f = _opt(args, "--board-file"), _opt(args, "--handoff-file")
    if not src.is_file():
        return _incomplete("源文件不在：" + str(src))
    src_text = src.read_text(encoding="utf-8").strip()
    src_bytes = len(src.read_bytes())
    inj_bytes = len(src_text.encode("utf-8"))
    sha = hashlib.sha256(src_text.encode("utf-8")).hexdigest()
    findings = []

    try:
        board = pathlib.Path(board_f).read_text(encoding="utf-8") if board_f else get_body(base, BOARD)
        handoff = pathlib.Path(handoff_f).read_text(encoding="utf-8") if handoff_f else get_body(base, HANDOFF)
    except (urllib.error.URLError, urllib.error.HTTPError, OSError, KeyError, ValueError) as e:
        return _incomplete("读不到 ISSUE 正文（" + type(e).__name__ + "）")

    if HEADER not in board:
        findings.append("看板 #" + BOARD + " 没有附录 A 段")
    else:
        sec = board.split(HEADER, 1)[1].split(NL, 1)[1]
        m = HEADER_RE.search(sec)
        if not m:
            findings.append("看板附录头缺「文件字节/注入字节/正文 sha256」三元组（旧格式或漏写）")
        else:
            got_file, got_inj, got_sha = int(m.group(1)), int(m.group(2)), m.group(3)
            if (got_file, got_inj) != (src_bytes, inj_bytes):
                findings.append("字节口径漂移：看板标 %d/%d，仓内实测 %d/%d" % (got_file, got_inj, src_bytes, inj_bytes))
            if got_sha != sha:
                findings.append("哈希漂移：看板附录标 %s…%s，仓内源文件 %s…%s" % (got_sha[:12], got_sha[-6:], sha[:12], sha[-6:]))
        body = appendix_body(sec)
        if body != src_text:
            findings.append(_diff_msg("看板附录正文", body, src_text))

    if src_text not in handoff:
        findings.append(_diff_msg("交接 ISSUE 正文（应整段包含源文件）", handoff, src_text))

    out = {"kind": "board_appendix_check",
           "src": str(src.relative_to(REPO) if src.is_relative_to(REPO) else src),
           "src_bytes": src_bytes, "injected_bytes": inj_bytes, "src_sha256": sha,
           "board_len": len(board), "handoff_len": len(handoff),
           "appendix_body_len": len(appendix_body(board.split(HEADER, 1)[1].split(NL, 1)[1])) if HEADER in board else -1,
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
