#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# LB-1 标签口径核对器（不是门禁，不进 STAGE_CONFIG，不改门禁分母）。
#
#   python3 audit/v4.1.0/issue_labels.py check [--input live|<fixture.jsonl>] [--only priority,cost]
#   python3 audit/v4.1.0/issue_labels.py plan
#   python3 audit/v4.1.0/issue_labels.py apply
#
# 三态纪律：核对器只报「缺哪一维」，绝不把缺标签说成已登记。
# 台账 docs/releases/v4.1.0/ISSUE_LABEL_LEDGER_v4.1.0.jsonl 是判定的唯一依据：
# 没有台账理由的缺维 ⇒ UNLEDGERED ⇒ 退出码 1（fail-closed，不允许当场猜标签）。
import json
import pathlib
import subprocess
import sys
import urllib.error
import urllib.request

REPO = pathlib.Path(__file__).resolve().parents[2]
LEDGER = REPO / "docs/releases/v4.1.0/ISSUE_LABEL_LEDGER_v4.1.0.jsonl"
DIMS = ("priority", "cost", "cap")
API = "http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo"


def token():
    # PAT 只在调用瞬间从 git credential helper 取，不落盘、不打印、不进 argv。
    p = subprocess.run(["git", "credential", "fill"],
                       input=b"url=http://192.168.0.252:3000\n\n", capture_output=True)
    for line in p.stdout.decode().splitlines():
        if line.startswith("password="):
            return line.split("=", 1)[1]
    raise SystemExit("no credential for http://192.168.0.252:3000")


def req(method, path, payload=None, page=0, pagesize=30):
    url = API + path
    if page:
        url += ("&" if "?" in url else "?") + f"page={page}&limit={pagesize}"
    data = json.dumps(payload).encode() if payload is not None else None
    headers = {"Authorization": "token " + token(), "Accept": "application/json"}
    if data is not None:
        headers["Content-Type"] = "application/json"
    r = urllib.request.Request(url, data=data, method=method, headers=headers)
    try:
        with urllib.request.urlopen(r, timeout=40) as resp:
            body = resp.read().decode()
    except urllib.error.HTTPError as e:
        return {"__http_error__": e.code, "body": e.read().decode()[:300]}
    try:
        return json.loads(body)
    except Exception:
        return body


def read_ledger(path=None):
    rows = [json.loads(l) for l in (path or LEDGER).read_text(encoding="utf-8").splitlines() if l.strip()]
    bad = []
    for r in rows:
        if not str(r.get("rationale", "")).strip():
            bad.append(f"#{r.get('number')} 台账缺理由")
        for dim, val in (r.get("assign") or {}).items():
            if dim not in DIMS:
                bad.append(f"#{r.get('number')} 台账维度非法: {dim}")
            elif not str(val).startswith(dim + "/"):
                bad.append(f"#{r.get('number')} 标签 {val} 不属于 {dim}/ 命名空间")
    return rows, bad


def read_issues(source):
    if source == "live":
        out, page = [], 1
        while True:
            d = req("GET", "/issues?state=open&type=issues", page=page)
            if not isinstance(d, list) or not d:
                break
            for it in d:
                out.append({"number": it["number"], "title": it["title"],
                            "labels": sorted(lab["name"] for lab in it.get("labels", []))})
            if len(d) < 30:
                break
            page += 1
        return out
    return [json.loads(l) for l in pathlib.Path(source).read_text(encoding="utf-8").splitlines() if l.strip()]


def has(labels, dim):
    return any(str(x).startswith(dim + "/") for x in labels)


def audit(issues, ledger_by_num, ledger_bad, dims):
    findings = []
    for it in issues:
        for dim in dims:
            if has(it["labels"], dim):
                continue
            row = ledger_by_num.get(it["number"])
            if row is None:
                findings.append(f"UNLEDGERED {dim} #{it['number']}")
            else:
                findings.append(f"MISSING {dim} #{it['number']}")
    for b in ledger_bad:
        findings.append("LEDGER-BAD " + b)
    return findings


def dim_of(f, d):
    return f.split()[1] == d


def cmd_check(argv):
    source, dims = "live", list(DIMS)
    ledger_path = None
    if "--ledger" in argv:
        ledger_path = pathlib.Path(argv[argv.index("--ledger") + 1])
    if "--input" in argv:
        source = argv[argv.index("--input") + 1]
    if "--only" in argv:
        dims = [d.strip() for d in argv[argv.index("--only") + 1].split(",") if d.strip()]
        for d in dims:
            if d not in DIMS:
                print("BAD-DIM", d)
                return 2
    rows, bad = read_ledger(ledger_path)
    issues = read_issues(source)
    findings = audit(issues, {r["number"]: r for r in rows}, bad, dims)
    per_dim = {d: sum(1 for f in findings if dim_of(f, d)) for d in dims}
    print(json.dumps({"kind": "label_gap_check", "source": source, "open_total": len(issues),
                      "dims": dims, "per_dim": per_dim,
                      "unledgered": sum(1 for f in findings if f.startswith("UNLEDGERED")),
                      "ledger_rows": len(rows), "findings": len(findings)}, ensure_ascii=False))
    for f in findings[:40]:
        print("  " + f)
    if len(findings) > 40:
        print(f"  ...(共 {len(findings)} 条，只列前 40)")
    return 1 if findings else 0


def cmd_plan(argv):
    lp = pathlib.Path(argv[argv.index("--ledger") + 1]) if "--ledger" in argv else None
    rows, bad = read_ledger(lp)
    if bad:
        for b in bad:
            print("LEDGER-BAD", b)
        return 2
    issues = {i["number"]: i for i in read_issues("live")}
    ops = []
    for r in rows:
        it = issues.get(r["number"])
        if it is None:
            print(f"SKIP #{r['number']} 不在 open 列表")
            continue
        for dim, val in (r.get("assign") or {}).items():
            if not has(it["labels"], dim):
                ops.append((r["number"], val))
    for n, l in ops:
        print(f"#{n} += {l}")
    print("ops:", len(ops))
    return 0


def cmd_apply(argv):
    lp = pathlib.Path(argv[argv.index("--ledger") + 1]) if "--ledger" in argv else None
    rows, bad = read_ledger(lp)
    if bad:
        for b in bad:
            print("LEDGER-BAD", b)
        return 2
    issues = {i["number"]: i for i in read_issues("live")}
    added, failed = 0, []
    for r in rows:
        it = issues.get(r["number"])
        if it is None:
            continue
        wanted = [v for d, v in (r.get("assign") or {}).items() if not has(it["labels"], d)]
        if not wanted:
            continue
        res = req("POST", f"/issues/{r['number']}/labels", {"labels": wanted})
        # Gitea 的 POST /issues/{n}/labels 直接返回「该 issue 当前的标签列表」，
        # 不是 {"labels": [...]}。曾经按后者解析 => 把写入成功误报成 17 条失败。
        if isinstance(res, list):
            got = sorted(x["name"] for x in res)
        elif isinstance(res, dict) and not res.get("__http_error__"):
            got = sorted(x["name"] for x in res.get("labels", []))
        else:
            got = []
        if not all(w in got for w in wanted):
            failed.append(f"#{r['number']} wanted={wanted} got={got} resp={str(res)[:160]}")
        else:
            added += len(wanted)
            print(f"#{r['number']} added {wanted}")
    print("added:", added, "failed:", len(failed))
    for f in failed:
        print("FAIL", f)
    return 1 if failed else 0


def cmd_verify(argv):
    """只加不删的机器核对：把「整改前快照」与 live 逐条 diff。

      python3 audit/v4.1.0/issue_labels.py verify <before-snapshot.jsonl>

    rc=0 需要同时满足：没有任何标签被删除，且新增数 == 台账对这份快照算出的 ops 数。
    新增数少于 ops 说明有写入没落地；多于 ops 说明有人往同一批 issue 上加了台账外的标签。
    """
    if not argv:
        print("usage: issue_labels.py verify <before-snapshot.jsonl> [--ledger f]")
        return 2
    lp = pathlib.Path(argv[argv.index("--ledger") + 1]) if "--ledger" in argv else None
    pos = [a for a in argv if not a.startswith("--") and a != str(lp) if lp is None or a != str(lp)]
    before_issues = read_issues(pos[0] if pos else argv[0])
    before = {i["number"]: i["labels"] for i in before_issues}
    rows, bad = read_ledger(lp)
    if bad:
        for b in bad:
            print("LEDGER-BAD", b)
        return 2
    expected = 0
    for r in rows:
        labels = before.get(r["number"], [])
        expected += sum(1 for d, v in (r.get("assign") or {}).items() if not has(labels, d))
    lost, added = [], 0
    for it in read_issues("live"):
        old_labels = before.get(it["number"])
        if old_labels is None:
            continue
        for x in old_labels:
            if x not in it["labels"]:
                lost.append(f"#{it['number']} {x}")
        added += sum(1 for x in it["labels"] if x not in old_labels)
    print(json.dumps({"kind": "label_apply_verify", "snapshot": argv[0],
                      "issues_checked": len(before), "lost_labels": len(lost),
                      "added_labels": added, "expected_ops_from_ledger": expected},
                     ensure_ascii=False))
    for l in lost[:20]:
        print("LOST", l)
    if lost:
        return 1
    if added != expected:
        print(f"ADDED-MISMATCH added={added} expected={expected}")
        return 1
    return 0


if __name__ == "__main__":
    argv = sys.argv[1:]
    sub = argv[0] if argv else "check"
    rest = argv[1:]
    if sub == "check":
        raise SystemExit(cmd_check(rest))
    if sub == "plan":
        raise SystemExit(cmd_plan(rest))
    if sub == "apply":
        raise SystemExit(cmd_apply(rest))
    if sub == "verify":
        raise SystemExit(cmd_verify(rest))
    print("usage: issue_labels.py {check [--input live|<fixture>] [--only dims] [--ledger f] | plan | apply | verify <snapshot.jsonl> [--ledger f]}")
    raise SystemExit(2)
