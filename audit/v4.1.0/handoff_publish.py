#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""handoff_publish.py —— 把交接正文（仓内 SSOT）发布到 ISSUE #5599。

为什么要有它：`gen_kanban_issue.py --appendix` 只负责把正文注入看板 #5592；
#5599 的正文此前每次都是**临时手敲** PATCH，一旦「改了源文件忘了刷 #5599」
就只能靠 `board_appendix_check.py` 事后抓（它判据是「整段包含」，能抓但不能发）。
发布路径本身可复跑，才符合本仓「把『做过了』变成可追溯记录」。

口径：
  * 正文 = 仓内源文件（strip 后）逐字 + 自动生成的「发布元数据」段（字节数/sha256/分支/HEAD 全部现读，
    不写死任何数字；写死数字正是 §8 记过的「夹具写死 = 静默脱靶」）。
  * fail-closed：源文件不存在 / 过短 / 读到空 HEAD ⇒ 非零退出，不发。
  * 不加 `--post` 只打印，绝不 PATCH（沿用 gen_kanban_issue.py 的 A1 规矩）。
  * 发完必须再跑 `board_appendix_check.py`（0 才算三处一致）。这一步不在本脚本里做，
    因为「发布器自己说自己发对了」不算证据（§10 第 13 条）。

用法：
  python3 audit/v4.1.0/handoff_publish.py                 # 打印到 stdout（不发）
  python3 audit/v4.1.0/handoff_publish.py --post 5599     # PATCH ISSUE 5599
退出码：0 成功 / 1 失败（含 PATCH 失败）/ 2 INCOMPLETE（没能核对，不算通过）。
"""
from __future__ import annotations

import hashlib
import json
import pathlib
import subprocess
import sys
import urllib.error
import urllib.request

NL = chr(10)
REPO = pathlib.Path(__file__).resolve().parents[2]
SRC = REPO / "docs/knowledge-loop/handoffs/HANDOFF_2026-11-05_4.1.0_OMP_ONBOARDING.md"
BASE = "http://192.168.0.252:3000/api/v1"
MIN_BODY_BYTES = 200


def token() -> str:
    p = subprocess.run(["git", "credential", "fill"],
                       input=b"url=http://192.168.0.252:3000" + NL.encode() + NL.encode(),
                       capture_output=True)
    for line in p.stdout.decode().splitlines():
        if line.startswith("password="):
            return line.split("=", 1)[1]
    raise RuntimeError("no credential for http://192.168.0.252:3000")


def sh(*args: str) -> str:
    return subprocess.run(args, cwd=str(REPO), capture_output=True, text=True).stdout.strip()


def build_body() -> str:
    if not SRC.is_file():
        raise SystemExit("SRC-MISSING: " + str(SRC))
    raw = SRC.read_bytes()
    txt = raw.decode("utf-8").strip()
    if len(txt.encode("utf-8")) < MIN_BODY_BYTES:
        raise SystemExit(f"SRC-TOO-SHORT: {len(txt.encode('utf-8'))} 字节")
    branch = sh("git", "rev-parse", "--abbrev-ref", "HEAD")
    head = sh("git", "rev-parse", "HEAD")
    if not branch or not head or branch == "HEAD":
        # 分离 HEAD 上发布 = 坐标写不准，宁可不发
        raise SystemExit("COORD-INCOMPLETE: 读不到分支/HEAD（branch=%r head=%r）" % (branch, head[:10]))
    meta = [
        "",
        "## 发布元数据（机器生成，勿手改）",
        f"- 源文件（SSOT）：`docs/knowledge-loop/handoffs/{SRC.name}`",
        f"- 文件 {len(raw)} 字节｜正文（strip 后）{len(txt.encode('utf-8'))} 字节"
        f"｜sha256 `{hashlib.sha256(txt.encode('utf-8')).hexdigest()}`",
        f"- 分支 `{branch}` HEAD `{head[:10]}`（已推 gitea252）",
        "- 同一份内容注入在看板 #5592 附录 A；一致性由 "
        "`python3 audit/v4.1.0/board_appendix_check.py` 机器核对（0 一致 / 1 漂移 / 2 没能核对）",
    ]
    return txt + NL + NL.join(meta) + NL


def main(argv: list[str]) -> int:
    post = None
    for i, a in enumerate(argv):
        if a == "--post":
            if i + 1 >= len(argv):
                print("用法: --post <issue_no>", file=sys.stderr)
                return 2
            post = argv[i + 1]
    if post is None:
        sys.stdout.write(build_body())
        return 0
    body = build_body()
    payload = json.dumps({"body": body}).encode()
    req = urllib.request.Request(
        BASE + "/repos/openclaw/sqlrustgo/issues/" + post,
        data=payload, method="PATCH",
        headers={"Authorization": "token " + token(), "Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=40) as resp:
            resp.read()
    except urllib.error.HTTPError as e:
        print("PATCH-FAIL: http=%s %s" % (e.code, e.read().decode()[:300]), file=sys.stderr)
        return 1
    except (urllib.error.URLError, TimeoutError) as e:
        # 发不出去是「没能核对」，不是「已发布」
        print("INCOMPLETE: 发不出去 %r" % (e,), file=sys.stderr)
        return 2
    print(json.dumps({"kind": "handoff_publish", "issue": post, "chars": len(body),
                      "src": str(SRC.relative_to(REPO))}, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
