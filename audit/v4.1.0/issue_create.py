#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""issue_create.py —— 按仓内 JSON 描述在 252 Gitea 建 ISSUE（默认只打印，不写）。

为什么要它：建 ISSUE 此前是临时手敲 urllib POST，标题/标签/正文没有落仓的副本，
下次没人能复现「当初这条 ISSUE 是怎么写出来的」。把正文放仓里（audit/v4.1.0/issues/<ID>.json），
ISSUE 就成了可追溯记录的一部分。

纪律（沿用 gen_kanban_issue.py / handoff_publish.py 的 A1 规矩）：
  * 不加 --post 只打印，绝不 POST。
  * fail-closed：正文过短 / 标题不带 [4.1.0] / 缺 cost+priority 标签 ⇒ 退出码非 0，不建。
  * 查重：同仓库 open ISSUE 标题里出现同一 kanban ID（如 BIN-1）⇒ 拒绝建（防止同一缺陷两条）。
  * PAT 只在调用瞬间从 git credential helper 取，不落盘、不打印、不进 argv。

用法：
  /usr/bin/python3 audit/v4.1.0/issue_create.py --from audit/v4.1.0/issues/BIN-1.json
  /usr/bin/python3 audit/v4.1.0/issue_create.py --from audit/v4.1.0/issues/BIN-1.json --post
退出码：0 成功 / 1 判据不过（含查重命中）/ 2 INCOMPLETE（没能核对，不算通过）。
"""
from __future__ import annotations

import argparse
import json
import pathlib
import re
import subprocess
import sys
import urllib.error
import urllib.request

REPO = pathlib.Path(__file__).resolve().parents[2]
BASE = "http://192.168.0.252:3000"
API = BASE + "/api/v1/repos/openclaw/sqlrustgo"
MIN_BODY = 300


def token() -> str:
    p = subprocess.run(["git", "credential", "fill"],
                       input=b"url=" + BASE.encode() + b"\n\n", capture_output=True)
    for line in p.stdout.decode().splitlines():
        if line.startswith("password="):
            return line.split("=", 1)[1]
    raise SystemExit("no credential for " + BASE)


def req(method: str, path: str, payload=None, params: str = "") -> dict:
    url = API + path + params
    data = json.dumps(payload).encode() if payload is not None else None
    headers = {"Authorization": "token " + token(), "Accept": "application/json"}
    if data is not None:
        headers["Content-Type"] = "application/json"
    r = urllib.request.Request(url, data=data, method=method, headers=headers)
    try:
        with urllib.request.urlopen(r, timeout=40) as resp:
            txt = resp.read().decode("utf-8", "replace")
            return json.loads(txt) if txt.strip() else {}
    except urllib.error.HTTPError as e:
        raise SystemExit("HTTP %s %s: %s" % (e.code, e.reason, e.read().decode("utf-8", "replace")[:300]))


def label_map() -> dict:
    """标签名 → id。Gitea 的 POST /issues 只收**标签 id 数组**，
    直接塞名字会得到 422（实测：`cannot unmarshal JSON string into Go int64 within "/labels/0"`）。"""
    out, page = {}, 1
    while True:
        # Gitea 把 limit 截到 30：用 limit=100 只会拿到第一页（实测 109 个标签里只回 50 个，
        # 于是 `priority/p1` 被判成「不存在」⇒ 假 FAIL）。按 30 一页翻到底。
        rows = req("GET", "/labels", params="?page=%d&limit=30" % page)
        if not rows:
            break
        for r in rows:
            out[r["name"]] = r["id"]
        if len(rows) < 30:
            break
        page += 1
        if page > 20:
            break
    return out


def open_title_hits(kanban_id: str) -> list:
    """同仓库 open ISSUE 里标题含该 kanban ID 的（查重，fail-closed）。"""
    # 同样按 30 一页翻（Gitea 截断 limit）；open 数在几百量级，翻到空页为止。
    # 不用 `q=` 关键字过滤：Gitea 的 q 是全文检索，命中面不可控（漏一页就等于查重失效），
    # 这里宁可老实翻完 open 列表（实测 101 条 open ⇒ 4 页）。
    hits, page = [], 1
    while page <= 20:
        rows = req("GET", "/issues", params="?state=open&type=issues&page=%d&limit=30" % page)
        if not rows:
            break
        for r in rows:
            text = (r.get("title") or "")
            if kanban_id in text or ("[%s]" % kanban_id) in text:
                hits.append((r["number"], r["title"][:120]))
        if len(rows) < 30:
            break
        page += 1
    return hits


def main(argv) -> int:
    ap = argparse.ArgumentParser(prog="issue_create.py")
    ap.add_argument("--from", dest="src", required=True)
    ap.add_argument("--post", action="store_true", help="真的 POST（不加只打印）")
    a = ap.parse_args(argv)
    src = pathlib.Path(a.src)
    if not src.is_file():
        print("INCOMPLETE: 描述文件不存在 %s" % src, file=sys.stderr)
        return 2
    spec = json.loads(src.read_text(encoding="utf-8"))
    # 正文优先取 body_file（markdown 单独存，便于 review/diff），内联 body 作为兜底。
    if spec.get("body_file"):
        bf = src.parent / spec["body_file"]
        if not bf.is_file():
            print("INCOMPLETE: body_file 不存在 %s" % bf, file=sys.stderr)
            return 2
        spec["body"] = bf.read_text(encoding="utf-8").strip()
    for k in ("id", "title", "body", "labels"):
        if k not in spec:
            print("FAIL: 描述缺字段 %s" % k, file=sys.stderr)
            return 1
    problems = []
    if len(spec["body"].encode("utf-8")) < MIN_BODY:
        problems.append("正文 %d 字节 < %d（证据不足，不得建 ISSUE）"
                        % (len(spec["body"].encode("utf-8")), MIN_BODY))
    if "[4.1.0]" not in spec["title"]:
        problems.append("标题必须带 [4.1.0] 版本口径")
    if not any(l.startswith("cost/") for l in spec["labels"]):
        problems.append("缺 cost/* 标签（LB-1 口径）")
    if not any(l.startswith("priority/") for l in spec["labels"]):
        problems.append("缺 priority/* 标签（LB-1 口径）")
    known = label_map()
    unknown = [l for l in spec["labels"] if l not in known]
    if unknown:
        problems.append("标签不存在于仓库：%s" % unknown)
    hits = open_title_hits(spec["id"])
    if hits:
        problems.append("查重命中（同 ID 已有 open ISSUE）：%s" % hits)
    if problems:
        for p in problems:
            print("FAIL: " + p, file=sys.stderr)
        return 1
    payload = {"title": spec["title"], "body": spec["body"],
               "labels": [known[l] for l in spec["labels"]],
               "assignees": spec.get("assignees", [])}
    if not a.post:
        print("DRY-RUN（未 POST）。title=%s" % spec["title"])
        print("labels=%s body_bytes=%d" % (spec["labels"], len(spec["body"].encode("utf-8"))))
        print("--- body 前 400 字 ---")
        print(spec["body"][:400])
        print("--- 加 --post 才会创建 ---")
        return 0
    got = req("POST", "/issues", payload)
    print(json.dumps({"kind": "issue_created", "number": got.get("number"),
                      "url": got.get("html_url"), "title": got.get("title"),
                      "labels": [l["name"] for l in (got.get("labels") or [])]},
                     ensure_ascii=False))
    return 0 if got.get("number") else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
