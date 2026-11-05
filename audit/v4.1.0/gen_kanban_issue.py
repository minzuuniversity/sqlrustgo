#!/usr/bin/env python3
"""Render the v4.1.0 ALPHA kanban ISSUE body from the kanban SSOT.

    python3 audit/v4.1.0/gen_kanban_issue.py "<度量口径行>"            # print to stdout
    python3 audit/v4.1.0/gen_kanban_issue.py --post 5592 "<度量口径行>"  # PATCH ISSUE 5592

Two honesty rules baked in (learned 2026-11-05):
  * archived-run verdicts are computed from each manifest's `results` section,
    never hardcoded; a run with non-zero commands must be reported as such.
  * "most recent run" uses `completed_at`, not run-id lexicographic order.
"""

# 从看板 SSOT(jsonl) 生成 ISSUE #5592 正文。可重复运行，每次取当期 HEAD/run/度量。
import json, subprocess, sys, pathlib, hashlib, urllib.request, urllib.error

BASE = "http://192.168.0.252:3000/api/v1"


def token() -> str:
    """PAT comes from the git credential helper at call time; it is never
    stored in the repository, printed, or passed on the command line."""
    p = subprocess.run(["git", "credential", "fill"],
                       input=b"url=http://192.168.0.252:3000\n\n", capture_output=True)
    for line in p.stdout.decode().splitlines():
        if line.startswith("password="):
            return line.split("=", 1)[1]
    raise SystemExit("no credential for http://192.168.0.252:3000")


def req(method, path, payload=None, page=0, pagesize=50):
    url = BASE + path
    if page:
        url += ("&" if "?" in url else "?") + f"page={page}&limit={pagesize}"
    data = json.dumps(payload).encode() if payload is not None else None
    headers = {"Authorization": "token " + token(), "Accept": "application/json"}
    if data is not None:
        headers["Content-Type"] = "application/json"
    r = urllib.request.Request(url, data=data, method=method, headers=headers)
    try:
        with urllib.request.urlopen(r, timeout=40) as resp:
            b = resp.read().decode()
    except urllib.error.HTTPError as e:
        return {"__http_error__": e.code, "body": e.read().decode()[:600]}
    try:
        return json.loads(b)
    except Exception:
        return b

REPO = pathlib.Path("/Volumes/workspace/dev/sqlrustgo/.worktrees/410")
KAN = REPO / "docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.jsonl"
RUNS = REPO / "audit/v4.1.0/runs"
ORDER = ["阻塞", "待复核", "未开始", "完成"]


def sh(*a):
    return subprocess.run(a, cwd=str(REPO), capture_output=True, text=True).stdout.strip()


def esc(s, n):
    s = str(s or "").replace("|", "\\|").replace("\n", " ")
    return s[:n] + ("…" if len(s) > n else "")


def appendix_section(appendix_path):
    """Inject the handoff text verbatim into the board body.

    fail-closed: a given path must exist and be long enough, otherwise the
    generator exits non-zero. A board published *without* the handoff appendix
    is a silent loss of the onboarding contract, which is exactly the kind of
    thing this repo treats as a red line.
    """
    ap = pathlib.Path(appendix_path)
    if not ap.is_absolute():
        ap = REPO / ap
    if not ap.is_file():
        raise SystemExit("APPENDIX-MISSING: " + str(ap))
    txt = ap.read_text(encoding="utf-8").strip()
    if len(txt) < 200:
        raise SystemExit("APPENDIX-TOO-SHORT: " + str(ap) + " 只有 " + str(len(txt)) + " 字符")
    digest = hashlib.sha256(txt.encode("utf-8")).hexdigest()
    out = ["## 附录 A：交接正文全文（接手必读，SSOT 在仓内）", ""]
    out.append(f"- 源文件：`{appendix_path}`｜注入 {len(txt.encode('utf-8'))} 字节｜sha256 `{digest}`")
    out.append("- 本节由 `gen_kanban_issue.py --appendix` 原样注入，不进看板 SSOT；改交接改源文件，别改这里。")
    out.append("")
    out.append(txt)
    return out


def main(issue_no="5592", metrics_line="（见最近归档 run 的 manifest results 段）", post_to=False,
     appendix_path=None):
    rows = [json.loads(l) for l in KAN.read_text(encoding="utf-8").splitlines() if l.strip()]
    counts = {}
    for r in rows:
        counts[r["status"]] = counts.get(r["status"], 0) + 1
    import yaml
    # 退出码在 manifest 的 results 段（不是 commands 段），按序号对齐。
    # 取「最近完成」的 run 用 completed_at，不能按 run id 的字典序：
    # 20261105-triage-001 排在 20261105-negfix-004 之后，字典序会挑错 run。
    mfs = {}
    for d in RUNS.iterdir():
        mf = d / "manifest.yaml"
        if d.is_dir() and mf.is_file():
            try:
                mfs[d.name] = yaml.safe_load(mf.read_text(encoding="utf-8"))
            except Exception:
                continue
    runs = sorted(mfs, key=lambda k: (str(mfs[k].get("completed_at") or ""), k))[-3:]
    run_lines = []
    for rid in runs:
        m = mfs[rid]
        codes = [r.get("exit_code") for r in m.get("results") or []]
        nz = [f"{r.get('id')}={r.get('exit_code')}" for r in (m.get("results") or [])
              if r.get("exit_code") != 0]
        if not codes:
            verdict = "无 results 段（不可复核，见 #5596）"
        else:
            verdict = (f"{len(codes)} 条命令全部 exit 0" if not nz
                       else f"{len(codes)} 条命令，非零 {len(nz)} 条（{', '.join(nz)}）—— 按原样保留，不销账")
        run_lines.append(f"- `{rid}` frozen `{str(m.get('frozen_commit',''))[:10]}` / "
                         f"completed {m.get('completed_at','-')} / {verdict}")
        run_lines.append(f"  evidence_hash `{m.get('evidence_hash','-')}`")
    branch = sh("git", "rev-parse", "--abbrev-ref", "HEAD")
    head = sh("git", "rev-parse", "HEAD")
    # STAGE.yaml 里 stage 的键是 current_stage（不是 stage），且注释里也出现该词，
    # 因此必须跳过注释行、取第一个非注释命中。
    stage = ""
    st = (REPO / "docs/releases/v4.1.0/STAGE.yaml").read_text(encoding="utf-8")
    for line in st.splitlines():
        t = line.strip()
        if t.startswith("#") or not t:
            continue
        if t.startswith("current_stage:"):
            stage = t.split(":", 1)[1].strip().strip('"')
            break
    if not stage:
        raise SystemExit("STAGE.yaml 里没读到 current_stage —— 看板头部的阶段字段不能留空")
    L = []
    L.append("# v4.1.0 ALPHA 工作看板（SSOT 视图，勿手改正文）")
    L.append("")
    L.append(f"- 看板数据源：`docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.jsonl`（{len(rows)} 行）｜本正文由 `kanban_render.py` 同源渲染")
    L.append(f"- 分支 `{branch}`｜HEAD `{head[:10]}`｜STAGE.yaml `stage: {stage}`｜PR #5593 → `develop/v4.1.0`")
    L.append(f"- 状态计数：" + " / ".join(f"{k} {counts.get(k,0)}" for k in ORDER))
    L.append("- 度量口径（自指，以最近 run 现值为准）：" + metrics_line)
    L.append("")
    L.append("## 最近归档 run")
    L += run_lines
    L.append("")
    L.append("> 口径声明：所有归档 run 都**不含** `ci_gate.sh`，因此**不构成门禁通过声明**；")
    L.append("> `待复核` = 证据已落盘但只有单血统/单机取证，需要第二血统或 CI 侧复核；`阻塞` = 需 owner/hermes 动作，Agent 不自行处置。")
    for st_ in ORDER:
        sub = [r for r in rows if r["status"] == st_]
        if not sub:
            continue
        L.append(f"## {st_}（{len(sub)}）")
        L.append("")
        L.append("| 任务 | 验收 | 证据 / 阻塞 |")
        L.append("|---|---|---|")
        for r in sub:
            L.append(f"| **{r['id']}** {esc(r['task'],150)} | {esc(r['acceptance'],130)} | {esc(r['evidence'],260)} |")
        L.append("")
    L.append("## 需要 owner / hermes 的 ISSUE（Agent 不得代办）")
    L.append("")
    L.append("| 事项 | ISSUE | 为什么只能人做 |")
    L.append("|---|---|---|")
    for a, b, c in [
        ("分支保护 / 门禁清单变更", "#5595", "AGENTS 2.1/2.2：门禁清单与分支保护只能 owner 在独立 PR 改"),
        ("168h SOAK 未跑", "ING-1", "需要连续 7 天真机窗口，属基础设施排期"),
        ("CI 执行面：runner-03 缺镜像", "#5597", "改 runner 主机属 owner/hermes 权限"),
        ("MX-1 多机矩阵基线", "#5594", "需要第二台执行主机与凭据"),
        ("7 条不可复核审计记录", "#5596", "原始证据不在仓内，无法由 Agent 补造"),
        ("deepseek 席位登录", "EX-5", "浏览器登录必须人做"),
        ("dup #5466 → #5389", "IL-3", "关他人 issue 需 owner 确认"),
    ]:
        L.append(f"| {a} | {b} | {c} |")
    L.append("")
    L.append("## 怎么核对这张表（不用信我）")
    L.append("")
    L.append("```bash")
    L.append("python3 scripts/gate/kanban_render.py check docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.md")
    L.append("python3 scripts/gate/kanban_render.py counts docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.md")
    L.append("python3 audit/v4.1.0/audit_run.py verify --run-id <下面的 run id>")
    L.append("python3 audit/v4.1.0/verify_ledger.py            # 台账自洽（默认当期 B 代）")
    L.append("python3 audit/v4.1.0/probe_ledger_freshness.py snapshot-count   # 台账是否过期")
    L.append("```")
    L.append("")
    L.append("## 阶段跃迁规矩（跃迁必须人批准，AGENTS 2.7）")
    L.append("")
    L.append("- 每个阶段（draft/alpha/beta/rc/ga）各有一份看板 ISSUE；阶段任务全部收口后，前一张 ISSUE 在逐条写明结论后关闭。")
    L.append("- `阻塞` 与 `待复核` 未清零之前，任何跃迁结论都必须写成「没能核对」，不许写成通过。")
    L.append("- 门禁分母固定 168，新判据一律挂靠既有门禁；新工具只放 `audit/v4.1.0/`。")
    if appendix_path:
        L += appendix_section(appendix_path)
    body = "\n".join(L) + "\n"
    if not post_to:
        return body
    r = req("PATCH", "/repos/openclaw/sqlrustgo/issues/" + issue_no, {"body": body})
    if not isinstance(r, dict) or r.get("__http_error__"):
        raise SystemExit("PATCH 失败: " + str(r)[:300])
    print("patched:", r.get("html_url"), "len", len(body))
    return body


def selftest() -> int:
    """Three assertions that keep this generator honest.

    Run `--selftest` from a clean checkout: rc 0 means the body is generated
    purely from the SSOT without touching the network. Mutation arms (see the
    run log) flip each assertion red by re-introducing the old defect in a
    scratch copy of this file, never in the repository copy.
    """
    import re

    calls = []

    def _boom(*a, **k):
        calls.append((a[0] if a else "?", a[1] if len(a) > 1 else "?"))
        return {"__http_error__": "selftest-network-blocked"}

    global req
    req = _boom

    body = main(metrics_line="SELFTEST")
    bad = []

    # A1: 默认模式（无 --post）绝不允许 PATCH 线上 ISSUE。
    if calls:
        bad.append("A1 selftest 里出现了网络调用")

    # A2: 不得把 f-string 片段当字面量吐进正文（曾泄漏 ' + str(len(rows)) + '）。
    for token in ("' + str(", "' + ", "{len(rows)}", "{stage}"):
        if token in body:
            bad.append("A2 正文残留模板片段: " + token)

    # A3: 头部阶段字段必须来自 STAGE.yaml 的 current_stage，不能为空。
    m = re.search(r"STAGE\.yaml `stage: ([^`]*)`", body)
    if not m or not re.fullmatch(r"[A-Z][A-Z0-9_]*", m.group(1)):
        bad.append("A3 头部 stage 字段为空或非法: " + (m.group(1) if m else "<missing>"))

    # A4: 行数必须等于看板 SSOT 的行数（口径自洽）。
    n = len([l for l in KAN.read_text(encoding="utf-8").splitlines() if l.strip()])
    if f"（{n} 行）" not in body:
        bad.append(f"A4 正文行数口径 != SSOT {n} 行")

    # A5: 指定了附录文件，正文必须带上附录原文。
    # A6: 附录文件缺失必须 fail-closed（非零退出），不许静默出「没有附录的看板」。
    import tempfile
    with tempfile.TemporaryDirectory() as td:
        ap = pathlib.Path(td) / "appendix.md"
        ap.write_text("# SENTINEL-APPENDIX\n" + ("x" * 400), encoding="utf-8")
        body_ap = main(metrics_line="SELFTEST", appendix_path=str(ap))
        if "SENTINEL-APPENDIX" not in body_ap:
            bad.append("A5 指定了附录，正文却没注入附录原文")
        try:
            main(metrics_line="SELFTEST", appendix_path=str(pathlib.Path(td) / "nope.md"))
            bad.append("A6 附录缺失时静默生成看板（应非零退出）")
        except SystemExit:
            pass

    for b in bad:
        print("NOT-OK:", b)
    print("selftest arms=6 failures=%d" % len(bad))
    return 1 if bad else 0


if __name__ == "__main__":
    args = sys.argv[1:]
    if args and args[0] == "--selftest":
        raise SystemExit(selftest())
    metrics, post, appendix = "（见最近归档 run 的 manifest results 段）", None, None
    i = 0
    while i < len(args):
        if args[i] == "--post":
            post = args[i + 1]; i += 2
        elif args[i] == "--appendix":
            appendix = args[i + 1]; i += 2
        else:
            metrics = args[i]; i += 1
    if post is None:
        sys.stdout.write(main(metrics_line=metrics, appendix_path=appendix))
    else:
        main(issue_no=post, metrics_line=metrics, post_to=True, appendix_path=appendix)
