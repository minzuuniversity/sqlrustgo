#!/usr/bin/env python3
"""gov_probe —— 治理度量探针（测量工具，不是门禁，不进任何门禁清单）。

用途：把"文档里声称的治理数字"变成"一条命令可重放的输出"。
运行结果由 audit/v4.1.0/audit_run.py 作为 run 的 stdout 日志固化，
因此本脚本自身不写任何证据文件（避免另立证据布局）。

子命令（均为 stdout JSON Lines，一行一条记录）：
  metrics   一行：仓级治理度量（门禁数、可证伪数、孤儿数、声明数……）
  honesty   每道 check_* 一行：能否失败 / 是否吞错 / 是否有负向夹具
  issues    每个 open issue 一行：优先级、标签、里程碑、认领状态
  ci        Actions 汇总一行：终态分布、真实成功率、时长分位（反馈周期与吞吐上界）

issues 子命令需要 Gitea 凭据，凭据经 `git credential fill` 获取，
不得出现在参数、输出或日志里（AGENTS 凭据纪律）。

复现：
  python3 scripts/gate/gov_probe.py metrics
  python3 scripts/gate/gov_probe.py honesty
  python3 scripts/gate/gov_probe.py issues
  python3 scripts/gate/gov_probe.py ci

判据夹具（离线，含反向变异）：python3 scripts/gate/test_gov_probe.py
"""
from __future__ import annotations

import collections
import datetime
import glob
import json
import os
import re
import subprocess
import sys
import urllib.error
import urllib.request
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
GITEA = os.environ.get("SRG_GITEA", "http://192.168.0.252:3000")
REPO_SLUG = os.environ.get("SRG_REPO_SLUG", "openclaw/sqlrustgo")

# 静态判据：一个"能抓问题"的门禁必须同时具备 (a) 非零退出路径 (b) 负向夹具。
# 这两条判据本身是可证伪的：scripts/gate/test_gov_probe.py 会构造反例。
NEG_FIXTURE_RE = re.compile(
    r"selftest|self_test|negative|反例|反向变异|canary|负向", re.I)
CAN_FAIL_RE = re.compile(
    r"\bexit\s+([1-9]|\$\{?(?:FAIL|RC|rc|ERR|bad|violations|fail_count)\b)")
SYS_EXIT_RE = re.compile(r"sys\.exit\(\s*[1-9]")
RETURN_ONE_RE = re.compile(r"return\s+1\b")
SWALLOW_TOKENS = ("|| true", "|| :", "||  true")


def _src_without_comments(path: str) -> str:
    body = Path(path).read_text(encoding="utf-8", errors="replace")
    return "\n".join(l for l in body.splitlines() if not l.strip().startswith("#"))


def can_fail_paths(src: str) -> bool:
    """任一"能让门禁变红"的退出路径。

    `return 1` 对 bash 函数同样成立（bash 的 check 函数就是这么失败上抛的），
    所以这些判据不按扩展名分流；按扩展名分流会把 .sh 里的 `return 1`
    误判成"结构上永不红"，从而虚高 nofail 计数。
    """
    return bool(CAN_FAIL_RE.search(src) or SYS_EXIT_RE.search(src)
                or RETURN_ONE_RE.search(src))


def check_scripts() -> list[str]:
    return sorted(
        glob.glob("scripts/gate/check_*.sh") + glob.glob("scripts/gate/check_*.py")
    )


def cmd_honesty(_argv: list[str]) -> int:
    for f in check_scripts():
        src = _src_without_comments(f)
        can_fail = can_fail_paths(src)
        swallow = sum(src.count(t) for t in SWALLOW_TOKENS)
        yield_row({
            "kind": "honesty",
            "gate": os.path.basename(f),
            "can_fail": can_fail,
            "swallow_count": swallow,
            "set_e": bool(re.search(r"set\s+-[a-z]*e", src)),
            "neg_fixture": bool(NEG_FIXTURE_RE.search(src)),
        })
    return 0


def cmd_metrics(_argv: list[str]) -> int:
    import yaml

    sys.path.insert(0, str(REPO_ROOT / "scripts" / "gate"))
    import gate_executability as ge

    cfg = yaml.safe_load((REPO_ROOT / "docs/governance/STAGE_CONFIG.yaml").read_text())
    stages = cfg.get("stages") or {}
    crit = {s: (len(v.get("exit_criteria") or []),
                len(v.get("criteria_gates") or {}),
                len(v.get("manual_only_criteria") or {}))
            for s, v in stages.items()}

    # 注意：ge.ALL_REQUIRED_SCRIPT_PATHS 是 build_matrix() 里懒填充的全局，
    # 不调用 build_matrix() 它就是空集，会让"孤儿门禁"虚高。这里先构建，
    # 并把 listed_n 打进输出，使这个陷阱一旦被漏掉就能从数字上看出来。
    ge.build_matrix()
    listed = set(ge.ALL_REQUIRED_SCRIPT_PATHS)
    roots, steps = ge.ci_roots()
    reach = set(roots) | set(ge.reachable_from_roots(roots, steps, {}, None))
    allg = sorted("scripts/gate/" + os.path.basename(f) for f in check_scripts())
    orphan = [g for g in allg if g not in listed and g not in reach]

    refs = set()
    for pat in ("scripts/**/*.sh", "scripts/**/*.py", "gate/*.sh"):
        for p in glob.glob(pat, recursive=True):
            body = Path(p).read_text(encoding="utf-8", errors="replace")
            refs |= {"scripts/gate/" + n for n in ge.GATE_PATH_RE.findall(body)}
    gh_refs = set()
    for p in glob.glob(".github/workflows/*.yml") + glob.glob(".github/workflows/*.yaml"):
        gh_refs |= {"scripts/gate/" + n
                    for n in ge.GATE_PATH_RE.findall(Path(p).read_text(encoding="utf-8", errors="replace"))}

    dead = sorted(g for g in orphan if g not in refs)
    phantom = sorted((gh_refs - set(roots)) & set(allg))

    rows = []
    for f in allg:
        src = _src_without_comments(f)
        can_fail = can_fail_paths(src)
        rows.append((can_fail, sum(src.count(t) for t in SWALLOW_TOKENS),
                     bool(NEG_FIXTURE_RE.search(src))))

    docs410 = glob.glob("docs/releases/v4.1.0/*.md")
    claim_re = re.compile(r"PASS|全绿|✅")

    yield_row({
        "kind": "metrics",
        "check_n": len(allg),
        "listed_n": len(listed),
        "stage_crit": sum(v[0] for v in crit.values()),
        "stage_map": sum(v[1] for v in crit.values()),
        "stage_manual": sum(v[2] for v in crit.values()),
        "gate_defs": len(cfg.get("gate_definitions") or {}),
        "ci_roots": len(roots),
        "ci_reach": len(reach),
        "orphan": len(orphan),
        "dead": len(dead),
        "dead_list": [d.replace("scripts/gate/", "") for d in dead],
        "phantom": len(phantom),
        "phantom_list": [p.replace("scripts/gate/", "") for p in phantom],
        "can_fail_n": sum(1 for r in rows if r[0]),
        "nofail_n": sum(1 for r in rows if not r[0]),
        "swallow_n": sum(1 for r in rows if r[1]),
        "neg_fixture_n": sum(1 for r in rows if r[2]),
        "both_bad_n": sum(1 for r in rows if not r[0] and not r[2]),
        "claims_hits": sum(len(claim_re.findall(
            Path(p).read_text(encoding="utf-8", errors="replace"))) for p in docs410),
        "claims_files": len(glob.glob("claims/**/*", recursive=True)),
        "docs410": len(docs410),
        "audit_runs": len([d for d in glob.glob("audit/v4.1.0/*") if os.path.isdir(d)]),
        "audit_manifests": len(glob.glob("audit/**/manifest.yaml", recursive=True)),
        "audit_gate_scripts": len(glob.glob("scripts/gate/check_audit*.sh")),
        "audit_manifests_runs": len(glob.glob("audit/v4.1.0/runs/*/manifest.yaml")),
        "gh_wf": len(glob.glob(".github/workflows/*.yml")),
        "gt_wf": len(glob.glob(".gitea/workflows/*.yml")),
        "lessons": len(glob.glob("docs/governance/lessons/*.md")),
        "govdocs": len(glob.glob("docs/governance/*")),
        "topmd": len(glob.glob("*.md")),
        "stages": {s: dict(zip(("crit", "map", "manual"), crit[s]),
                            gates=len(v.get("required_gates") or []))
                   for s, v in stages.items()},
    })
    return 0


def _git_token() -> str:
    p = subprocess.run(["git", "credential", "fill"],
                       input=f"url={GITEA}\n\n".encode(), capture_output=True)
    for line in p.stdout.decode().splitlines():
        if line.startswith("password="):
            return line.split("=", 1)[1]
    raise SystemExit("gov_probe: no Gitea credential from `git credential fill`")


def issue_row(it: dict) -> dict:
    """单条 issue → 台账行（纯函数，可离线断言）。

    判据修正（2026-11-05 实测）：本仓优先级标签的真实形态是 `priority/p0`，
    不是裸 `P0`。旧写法 `[l for l in labels if l in ("P0","P1","P2")]`
    恒返回空列表 ⇒ 普查把 47 条 P0 全记成"无优先级"。
    test_gov_probe.py 用真实标签形态钉住这条口径。
    """
    labels = sorted(l["name"] if isinstance(l, dict) else str(l)
                    for l in (it.get("labels") or []))
    return {
        "kind": "issue", "number": it["number"], "title": it["title"],
        "priority": [l for l in labels if l.startswith("priority/")] or ["(none)"],
        "cost": [l for l in labels if l.startswith("cost/")] or ["(none)"],
        "cap": [l for l in labels if l.startswith("cap/")],
        "labels": labels, "milestone": bool(it.get("milestone")),
        "assignees": [a["login"] for a in (it.get("assignees") or [])],
        "pull_request": bool(it.get("pull_request")),
        "comments": it.get("comments", 0),
    }


def cmd_issues(_argv: list[str]) -> int:
    tok = _git_token()
    page, total = 1, 0
    pri: collections.Counter = collections.Counter()
    cost: collections.Counter = collections.Counter()
    cap: collections.Counter = collections.Counter()
    while True:
        url = (f"{GITEA}/api/v1/repos/{REPO_SLUG}/issues"
               f"?state=open&type=issues&page={page}&limit=50")
        req = urllib.request.Request(url, headers={
            "Authorization": f"token {tok}", "Accept": "application/json"})
        try:
            with urllib.request.urlopen(req, timeout=40) as resp:
                batch = json.loads(resp.read().decode())
        except urllib.error.HTTPError as e:
            # 不打印响应体以外的凭据相关信息
            print(json.dumps({"kind": "issues_error", "http": e.code,
                              "page": page}, ensure_ascii=False))
            return 2
        if not batch:
            break
        for it in batch:
            total += 1
            row = issue_row(it)
            pri[row["priority"][0]] += 1
            cost[row["cost"][0]] += 1
            cap[row["cap"][0] if row["cap"] else "(none)"] += 1
            yield_row(row)
        if len(batch) < 50:
            break
        page += 1
    yield_row({"kind": "issue_summary", "open_total": total, "pages": page,
               "priority": dict(sorted(pri.items())),
               "cost": dict(sorted(cost.items())),
               "cap": dict(sorted(cap.items()))})
    return 0


def _iso(s: str) -> datetime.datetime:
    return datetime.datetime.fromisoformat(s.replace("Z", "+00:00"))


def summarize_runs(runs: list[dict]) -> dict:
    """Actions run 列表 → 一行度量（纯函数，可离线断言）。

    口径（2026-11-05 实测）：Gitea 终态的 `status` 是 `completed`，
    真实结果在 `conclusion`。用 `status=="success"` 计数 ⇒ 成功恒 0。
    时长取 `started_at`→`completed_at`，>3h 视为挂死样本剔除（避免污染分位）。
    """
    concl: dict[str, int] = {}
    durs: list[float] = []
    for w in runs:
        key = w.get("conclusion") or w.get("status") or "unknown"
        concl[key] = concl.get(key, 0) + 1
        a, b = w.get("started_at"), w.get("completed_at")
        if a and b:
            d = (_iso(b) - _iso(a)).total_seconds()
            if 0 < d < 10800:
                durs.append(d)
    durs.sort()
    n = len(runs)
    succ = concl.get("success", 0)
    return {
        "kind": "ci_summary", "runs": n, "conclusions": dict(sorted(concl.items())),
        "success_rate_pct": round(100.0 * succ / n, 2) if n else None,
        "dur_n": len(durs),
        "dur_median_s": round(durs[len(durs) // 2]) if durs else None,
        "dur_p90_s": round(durs[int(len(durs) * 0.9)]) if durs else None,
        "dur_max_s": round(durs[-1]) if durs else None,
    }


def cmd_ci(argv: list[str]) -> int:
    """只读拉取 Actions 历史并汇总；不写任何文件（证据由 audit run 的 stdout 固化）。"""
    max_pages = 200
    for i, a in enumerate(argv):
        if a == "--max-pages" and i + 1 < len(argv):
            max_pages = int(argv[i + 1])
    tok = _git_token()
    runs: list[dict] = []
    total_count = None
    for page in range(1, max_pages + 1):
        url = (f"{GITEA}/api/v1/repos/{REPO_SLUG}/actions/runs"
               f"?page={page}&limit=50")
        req = urllib.request.Request(url, headers={
            "Authorization": f"token {tok}", "Accept": "application/json"})
        try:
            with urllib.request.urlopen(req, timeout=40) as resp:
                blob = json.loads(resp.read().decode())
        except urllib.error.HTTPError as e:
            yield_row({"kind": "ci_error", "http": e.code, "page": page})
            return 2
        batch = blob.get("workflow_runs") or []
        total_count = blob.get("total_count")
        if not batch:
            break
        runs.extend(batch)
        if total_count and len(runs) >= total_count:
            break
    yield_row(summarize_runs(runs))
    yield_row({"kind": "ci_window", "api_total": total_count,
               "paged": len(runs),
               "truncated": bool(total_count and len(runs) < total_count),
               "first_created_at": min((w.get("created_at") or "" for w in runs), default=None),
               "last_created_at": max((w.get("created_at") or "" for w in runs), default=None)})
    # jobs 分母单独给一行：文档里出现过 "3911" 这类数字，必须能指回真实口径
    try:
        req = urllib.request.Request(
            f"{GITEA}/api/v1/repos/{REPO_SLUG}/actions/jobs?page=1&limit=1",
            headers={"Authorization": f"token {tok}", "Accept": "application/json"})
        with urllib.request.urlopen(req, timeout=40) as resp:
            jobs_total = json.loads(resp.read().decode()).get("total_count")
    except urllib.error.HTTPError as e:
        jobs_total = None
        yield_row({"kind": "ci_jobs_error", "http": e.code})
    yield_row({"kind": "ci_jobs", "jobs_total": jobs_total})
    return 0


def yield_row(obj: dict) -> None:
    print(json.dumps(obj, ensure_ascii=False), flush=True)


def cmd_auditmanifest(argv: list[str]) -> int:
    """对每个已提交的 audit run 调用仓自己的证据校验器，如实记录退出码。

    这是测量，不是断言：本命令恒返回 0，但每行都写 verify 的真实 exit_code。
    （既有门禁 scripts/gate/check_audit_manifest.sh 会因此变红；本命令的目的
    是把"红在哪几个 run、缺什么"变成可重放的数字，供整改对账。）
    """
    skip = set()
    for i, a in enumerate(argv):
        if a == "--skip" and i + 1 < len(argv):
            skip.add(argv[i + 1])

    audit_dir = REPO_ROOT / "audit" / "v4.1.0"
    tool = audit_dir / "audit_run.py"
    runs: list[str] = []
    for mf in sorted(glob.glob(str(audit_dir / "runs" / "*" / "manifest.yaml"))
                     + glob.glob(str(audit_dir / "*" / "manifest.yaml"))):
        rid = Path(mf).parent.name
        if rid not in skip:
            runs.append(rid)

    missing_re = re.compile(r"missing (log|meta): (.*)")
    for rid in runs:
        proc = subprocess.run(
            ["python3", str(tool), "verify", "--run-id", rid],
            cwd=REPO_ROOT, capture_output=True, text=True, timeout=300,
        )
        blob = (proc.stdout + proc.stderr)
        m = missing_re.search(blob)
        yield_row({
            "kind": "audit_verify", "run_id": rid, "exit_code": proc.returncode,
            "reason": (m.group(0).replace(str(REPO_ROOT) + "/", "") if m
                       else ("ok" if proc.returncode == 0 else blob.strip().splitlines()[-1][:160])),
        })
    return 0


SUBS = {"metrics": cmd_metrics, "honesty": cmd_honesty,
        "issues": cmd_issues, "auditmanifest": cmd_auditmanifest,
        "ci": cmd_ci}


def main(argv: list[str] | None = None) -> int:
    os.chdir(REPO_ROOT)
    argv = list(sys.argv[1:] if argv is None else argv)
    if not argv or argv[0] not in SUBS:
        print(f"usage: gov_probe.py {{{'|'.join(SUBS)}}}", file=sys.stderr)
        return 2
    return SUBS[argv[0]](argv[1:])


if __name__ == "__main__":
    raise SystemExit(main())
