#!/usr/bin/env python3
r"""ISSUE_CLOSING_VERIFICATION Step 1 的 Gitea 原生机器判据（#5797 选项 B 的执行点）。

放在 audit/ 而不是 scripts/gate/：run 级探针，不进门禁分母（不命名 check_*，
不进 STAGE_CONFIG，`gov_probe.metrics.check_n` 恒为 168）。先例：
`audit/v4.1.0/probe_ledger_freshness.py`、`audit/v4.1.0/honesty.py`。

WHY THIS FILE EXISTS
--------------------

`docs/governance/ISSUE_CLOSING_VERIFICATION.md` §四 Step 1 的判据写的是：

    gh issue view <issue_number> --json closedByPullRequestsReferences

在本仓这条判据**结构性为空**（实测 400/400 为空），且 `gh` CLI 被
`AGENTS.md` §3.2 明令禁用 ⇒ 按字面执行，任何 issue 都永远不可关单；
按「例外」执行，则每个关单人自己发明一套等价证据 ⇒ 口径不统一。

本文件把「PR 是否引用了这条 issue」做成**一条可跑、可回归验证的判据**，
口径即 SSOT，不需要每个关单人临时发明。

THE CRITERION
-------------

**硬性条件（3 条，必须由同一个 PR 同时满足）**

  R1 merged    PR state=closed ∧ merged=true ∧ merged_at 非空
  R2 ancestor  merge_commit_sha 是目标分支的祖先（git merge-base --is-ancestor）
  R3 declared  PR 正文有**独立一行**的引用声明：
               `^Refs:?\s*#<N>\s*$`（多行模式）

R3 的文法**不是本文件自创的**：它从
`scripts/gate/check_commit_message.sh` 里 `REFS_RE = re.compile(r"^Refs:?\s*#\d+\s*$", re.M)`
**读出来**（本仓对该文法的唯一实现，见 `docs/releases/v4.1.0/evidence/
BATCH45_COMMIT_MESSAGE_SSOT_5453_2026-10-25.md`：判据曾要求带冒号而规范
不带，冒号已被容忍并写明「无冒号才是规范形式」）。本仓 PR 正文的实测分布：
「独立一行」形态 316 次 / 任意位置形态 330 次 ⇒ **只有 14 处是内联**，
而这 14 处绝大多数是**在正文里引用这条约定本身**（散文），不是声明：

    PR #5536: 「按 CONTRIBUTING.md:151 教的是 `Closes #123`」   ← 引用条文
    PR #5654: 「**独立一行** `Refs #<issue>`（正则 `^Refs:?\s*#\d+\s*$`）」 ← 讲口径
    PR #5796: 「（用 `Refs #5793`，不用 `Closes`）」                 ← 引用条文

⇒ 「独立一行」这一条形态要求，正好把散文挡在外面，同时不误伤真正的声明
（PR #5776 的正文里就是独立一行 `Refs #5774`，虽然标题没带 issue 号）。

**为什么 R2 才是承重项、base 不是**（堆叠 PR 实测）

PR #5796 是堆叠 PR（base = `fix/drift-check-server-authority-20261116`）。
那个 base 分支**确实已并入 develop**（develop tip 就是它的合并提交
7fc7b48f8f），但 PR #5796 是在 base 分支并入 develop **之后**才合进
base 分支的 ⇒ 它的合并提交**不在** develop 的历史里 ⇒ R2=no ⇒ 不成立。
（内容落地由 PR #5798 单独完成。）⇒ 判据看「合并提交是不是目标分支的祖先」，
不看「base 是不是目标分支」。

**佐证（记录，不构成硬性条件）**

  C1 cross_ref  Gitea 自己的交叉引用索引（issue timeline 的 `pull_ref` 条目）
                含该 PR。
  C2 identity    PR 的标题或 head 分支名点名了该 issue 编号。
  C3 fix_claim   PR 标题带 fix 类动词。

**C1 不能承重**（实测两条反证）：

  (1) C1 **太宽**：本仓 open issue 的 timeline 交叉引用里混着
      `chore(labels): #5711 #5712 标签台账两行`、`docs: #5735 立案登记`
      这类**只改标签/只改文档**的 PR —— 它们在 timeline 里同样是被引用的
      PR，但显然不是「修复了这条 issue 的 PR」。
  (2) C1 **对历史 issue 不完整**：实测 4 例「正文有合规 Refs 引用但
      timeline 无 pull_ref 条目」，全部是早已关闭的老 issue
      （#95 / #123 / #3900 / #4708）—— 例如 PR #4934 正文写
      `Refs: #3900`、标题与分支名都点名 #3900，但 #3900 的 timeline
      里没有它。

⇒ 承重的是 R1+R2+R3；C1 只作佐证。这纠正了 #5797 里候选口径把
「issue timeline 有 pull_ref 事件」写成硬性合取项的那一处 ——
那样写会对上述 4 例产生假阴性。

**标记**

  AUTO_CLOSE_PATH  正文只有独立一行的 `Closes #<N>`（Gitea 自动关单路径）。
                    `CONTRIBUTING.md` §5.4 不许 Agent 用它（合并即关单，
                    绕过本文件 5 步复核）⇒ 标出但**不判为可关单依据**。

**近失（near-miss）**

  引用了本 issue、但形态不是「独立一行」规范形的 PR 会单独列出。
  它们不是合规证据，但列出来是为了让「判据判红」时能看清**为什么**红，
  而不是只给一个布尔值。

**判据的边界（必须写明，否则会被误用）**

本判据只证明「**存在一个已合并、已进入目标分支、正文合规声明解决本 issue
的 PR**」。它**不**证明那个 PR 真的修好了这个 issue —— 那是 §四 Step 2~5
（复跑、贴证据、复核）要人做的事。例：#5688 的判据成立（PR #5696/#5697
已合并并声明 `Refs #5688`），但占位基线守卫本身是 PR #5800（未合并）。

用法
----

    python3 audit/v4.1.0/issue_closing_refs.py --issue 5498
    python3 audit/v4.1.0/issue_closing_refs.py --issue 5498 --issue 5711 --json
    python3 audit/v4.1.0/issue_closing_refs.py mutation-selftest

退出码：0 全部「可关单」/ 1 任一「禁止」/ 2 输入不可用（INCOMPLETE）
"""
from __future__ import annotations

import argparse
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
CRED_SH = ROOT / "scripts" / "sync" / "gitea_credential_common.sh"
GATE_COMMIT_MSG = ROOT / "scripts" / "gate" / "check_commit_message.sh"

GITEA = os.environ.get("SRG_GITEA", "http://192.168.0.252:3000").rstrip("/")
REPO_SLUG = os.environ.get("SRG_REPO_SLUG", "openclaw/sqlrustgo")
DEFAULT_BRANCH = os.environ.get("SRG_BRANCH", "develop/v4.1.0")

# 本仓「引用声明」文法的**唯一**实现住在门禁脚本里；本工具从那里读，
# 不另立一份（AGENTS「一个规则只能有一份实现」）。读不到 ⇒ 直接失败。
CANONICAL_REFS_PATTERN = r"^Refs:?\s*#\d+\s*$"
# Closes 只用来标记「自动关单路径」，与 Refs 分开判
CLOSE_LINE_RE = re.compile(r"^Closes:?\s*#\d+\s*$", re.M)
# 宽松形态（任意位置的关键词 + 编号）—— 只用于报告近失，不作证据
LOOSE_RE = re.compile(
    r"(?i)(?<![A-Za-z0-9])(?:refs?|references|fixes|resolves|closes)\s*:?\s*"
    r"(?:\*\*|\*)?\s*#\s*(\d+)")
FIX_TITLE_RE = re.compile(r"(?i)\b(fix(?:e[sd])?|resolve[sd]?|close[sd]?)\b")


class Incomplete(Exception):
    """输入不可用（凭据/API/本地对象/文法来源缺失）—— 不等于通过。"""


def load_canonical_refs_re() -> re.Pattern:
    """从 check_commit_message.sh 取本仓唯一的 Refs 文法。

    取不到就失败（不静默退回自带正则）—— 文法漂移必须是可见的，
    而不是让两处实现各走各的。
    """
    try:
        src = GATE_COMMIT_MSG.read_text(encoding="utf-8")
    except OSError as exc:
        raise Incomplete(f"读不到 {GATE_COMMIT_MSG}: {exc}") from exc
    m = re.search(r"REFS_RE\s*=\s*re\.compile\(\s*r?([\"'])(.*?)\1",
                  src)
    if not m:
        raise Incomplete(
            f"{GATE_COMMIT_MSG} 里找不到 REFS_RE 定义 —— 引用文法来源漂移")
    if m.group(2) != CANONICAL_REFS_PATTERN:
        raise Incomplete(
            f"门禁里的 Refs 文法已变（现在是 {m.group(2)!r}，"
            f"本工具按 {CANONICAL_REFS_PATTERN!r} 实现）⇒ 需同步后重跑")
    return re.compile(m.group(2), re.M)


# ── 凭据与 API ────────────────────────────────────────────────────────────
# 复用仓内**唯一**的凭据实现（`gitea_credential_common.sh`）：凭据只出现在
# 一个 0600 的 curl 配置文件里，命令行上只有它的路径（AGENTS #5520 / C2）。

class Gitea:
    def __init__(self, host: str) -> None:
        self.base = f"{GITEA}/api/v1/repos/{REPO_SLUG}"
        self._cfg = None
        self._tmpdir = tempfile.mkdtemp(prefix="issue-closing-refs-")
        script = (
            f'source "{CRED_SH}"; '
            f'gitea_credential_resolve "{host}" || exit 2; '
            f'gitea_credential_curl_config_file'
        )
        p = subprocess.run(["bash", "-c", script], capture_output=True, text=True)
        cfg = (p.stdout or "").strip().splitlines()
        if p.returncode != 0 or not cfg:
            raise Incomplete(f"凭据取不到（rc={p.returncode}）：{host}")
        self._cfg = cfg[-1].strip()

    def close(self) -> None:
        if self._cfg:
            try:
                os.unlink(self._cfg)
            except OSError:
                pass
        shutil.rmtree(self._tmpdir, ignore_errors=True)

    def get(self, path: str):
        cmd = ["curl", "-s", "-m", "60", "-K", self._cfg, f"{self.base}/{path}"]
        p = subprocess.run(cmd, capture_output=True, text=True)
        if p.returncode != 0:
            raise Incomplete(f"HTTP 取不到 {path}（curl rc={p.returncode}）")
        try:
            return json.loads(p.stdout)
        except json.JSONDecodeError as exc:
            raise Incomplete(f"{path} 返回非 JSON：{exc}") from exc


# ── 证据收集 ──────────────────────────────────────────────────────────────

def timeline_pull_refs(api: Gitea, issue: int) -> list[int]:
    """Gitea 自己的交叉引用索引：该 issue 的 timeline 里被引用的 PR 编号。"""
    data = api.get(f"issues/{issue}/timeline?limit=50")
    out: list[int] = []
    if not isinstance(data, list):
        return out
    for entry in data:
        if not isinstance(entry, dict) or entry.get("type") != "pull_ref":
            continue
        num = (entry.get("ref_issue") or {}).get("number")
        if isinstance(num, int):
            out.append(num)
    return sorted(set(out))


def git_ancestor(sha: str, branch: str) -> str:
    """merge commit 是否是 branch 的祖先 → 'yes' / 'no' / 'unknown'。"""
    if not sha:
        return "unknown"
    exists = subprocess.run(["git", "cat-file", "-e", f"{sha}^{{commit}}"],
                             cwd=ROOT, capture_output=True)
    if exists.returncode != 0:
        return "unknown"
    r = subprocess.run(["git", "merge-base", "--is-ancestor", sha, branch],
                       cwd=ROOT, capture_output=True)
    return "yes" if r.returncode == 0 else "no"


def _declared_refs(body: str, refs_re: re.Pattern, issue: int) -> list[str]:
    """正文里**独立一行**声明引用了本 issue 的编号。

    只接受规范形态（`Refs #N` / `Refs: #N` 独占一行）。
    """
    out = []
    for m in refs_re.finditer(body or ""):
        num = re.search(r"#\s*(\d+)", m.group(0))
        if num and int(num.group(1)) == issue:
            out.append(num.group(1))
    return out


def collect(api: Gitea, issue: int, branch: str,
            refs_re: re.Pattern) -> dict:
    """把一条 issue 的关单证据收齐（纯取证，不下结论）。"""
    refs = timeline_pull_refs(api, issue)
    prs: list[dict] = []
    for pr in refs:
        p = api.get(f"pulls/{pr}")
        if not isinstance(p, dict) or "number" not in p:
            continue
        body = p.get("body") or ""
        title = p.get("title") or ""
        head_ref = (p.get("head") or {}).get("ref") or ""
        base_ref = (p.get("base") or {}).get("ref") or ""
        merged = bool(p.get("merged"))
        merged_at = p.get("merged_at") or ""
        sha = p.get("merge_commit_sha") or ""
        close_hits = []
        for m in CLOSE_LINE_RE.finditer(body):
            num = re.search(r"#\s*(\d+)", m.group(0))
            if num and int(num.group(1)) == issue:
                close_hits.append(num.group(1))
        loose = [x for x in LOOSE_RE.findall(body) if int(x) == issue]
        prs.append({
            "pr": p["number"],
            "title": title,
            "head_ref": head_ref,
            "base_ref": base_ref,
            "merged": merged,
            "merged_at": merged_at,
            "merge_commit": sha,
            "declared_refs": _declared_refs(body, refs_re, issue),
            "closes_refs": close_hits,
            "loose_refs": loose,
            "fix_title_claim": bool(FIX_TITLE_RE.search(title)),
            "ancestor_of_branch": git_ancestor(sha, branch) if merged else "no",
            "url": f"{GITEA}/{REPO_SLUG}/pulls/{p['number']}",
        })
    return {"issue": issue, "branch": branch, "prs": prs,
            "cross_ref_index": refs}


# ── 判定 ──────────────────────────────────────────────────────────────────

def _identity(pr: dict, issue: int) -> bool:
    """C2：PR 的标题或 head 分支名**点名**了这条 issue（佐证，不是硬性条件）。"""
    if f"#{issue}" in (pr.get("title") or ""):
        return True
    return bool(re.search(rf"(?<!\d){issue}(?!\d)", pr.get("head_ref") or ""))


def judge(ev: dict) -> dict:
    """R1..R3 硬性条件；C1..C3 佐证；返回 verdict 与逐条理由。"""
    issue = ev["issue"]
    qualifying, near_miss, auto_close = [], [], []
    for pr in ev["prs"]:
        declared = [str(x) for x in (pr.get("declared_refs") or [])]
        closes = [str(x) for x in (pr.get("closes_refs") or [])]
        checks = {
            "R1_merged": bool(pr.get("merged")),
            "R1_merged_at": bool(pr.get("merged_at")),
            # R2 必须确认引用的是**本** issue，不只是「正文里有个引用」
            "R3_declared": str(issue) in declared,
            "R2_ancestor": pr.get("ancestor_of_branch") == "yes",
        }
        pr["checks"] = checks
        pr["identity"] = _identity(pr, issue)
        if all(checks.values()):
            qualifying.append(pr)
        # near_miss / auto_close 是并列的两类，不是互斥分支：
        # 一个用 Closes 引用的 PR 同时满足「提到本 issue」与「正文无合规 Refs」，
        # 早先用 elif 会让 auto_close 分支被 near_miss 抢走（实测翻红）。
        if not checks["R3_declared"] and (pr.get("loose_refs") or closes):
            near_miss.append(pr)
            if closes:
                auto_close.append(pr)

    if qualifying:
        names = ", ".join(f"#{p['pr']}" for p in qualifying)
        verdict = "PASS"
        reason = (f"{names} 同时满足 R1-R3（已合并、合并提交是 "
                  f"{ev['branch']} 的祖先、正文有独立一行的 `Refs "
                  f"#{issue}` 声明）")
    elif auto_close:
        verdict = "AUTO_CLOSE_PATH"
        reason = (f"仅有 PR #{auto_close[0]['pr']} 用独立一行的 `Closes` "
                  f"引用本 issue —— 自动关单路径，CONTRIBUTING §5.4 不许 "
                  f"Agent 使用，不作为可关单依据")
    else:
        verdict = "FAIL"
        if not ev["prs"]:
            reason = ("Gitea 交叉引用索引里没有任何引用本 issue 的 PR"
                      "（既无正文引用，也无 timeline pull_ref）")
        elif near_miss:
            reason = (f"{len(near_miss)} 个 PR 提到本 issue，但正文没有"
                      f"**独立一行**的 `Refs #{issue}` 声明（R3 不成立）")
        else:
            reason = "候选 PR 未同时满足 R1-R3"

    return {
        "issue": ev["issue"],
        "verdict": verdict,
        "reason": reason,
        "qualifying": [p["pr"] for p in qualifying],
        "near_miss": [{"pr": p["pr"], "title": p["title"]} for p in near_miss],
        "auto_close": [p["pr"] for p in auto_close],
        "cross_ref_index": ev["cross_ref_index"],
        "detail": ev["prs"],
    }


def render(res: dict) -> str:
    lines = []
    for r in res:
        mark = {"PASS": "✅ 可关单", "FAIL": "⛔ 禁止",
                "AUTO_CLOSE_PATH": "⚠️ 自动关单路径（不作依据）"}[r["verdict"]]
        lines.append(f"#{r['issue']}  {mark}")
        lines.append(f"    理由：{r['reason']}")
        if r["cross_ref_index"]:
            lines.append(f"    佐证 C1（Gitea 交叉引用索引）：PR {r['cross_ref_index']}")
        for p in r["detail"]:
            c = p.get("checks", {})
            bad = [k for k, v in c.items() if not v]
            lines.append(
                f"      PR #{p['pr']} merged={p['merged']} "
                f"ancestor={p['ancestor_of_branch']} "
                f"R3={bool(p['declared_refs'])} "
                f"base={p['base_ref']}"
                + (f"  未满足={bad}" if bad else "")
                + f"  | {p['title'][:52]}")
        for n in r["near_miss"]:
            lines.append(f"      近失：PR #{n['pr']} {n['title'][:58]}")
    return "\n".join(lines)


# ── 反向变异自证（AGENTS 2.5）────────────────────────────────────────────
# 判据本身也要被证明会红：把一份合规证据逐条破坏，判定必须相应变红，
# 且**失效条件集合与期望完全一致**（精确翻红，不是「红了一下」）。

def _fixture() -> dict:
    return {
        "issue": 900001,
        "branch": "develop/v4.1.0",
        "cross_ref_index": [4242],
        "prs": [{
            "pr": 4242, "title": "fix(engine): 修掉 #900001 的空指针",
            "head_ref": "fix/900001-null", "base_ref": "develop/v4.1.0",
            "merged": True, "merged_at": "2026-11-16T00:00:00Z",
            "merge_commit": "0" * 40,
            "declared_refs": ["900001"], "closes_refs": [], "loose_refs": [],
            "fix_title_claim": True,
            "ancestor_of_branch": "yes",
            "url": "http://example/pulls/4242",
        }],
    }


# (臂名, 期望失效的条件集合, 变异动作)
MUTATIONS = [
    ("M1_merged_false", {"R1_merged"},
     lambda e: e["prs"][0].update(merged=False)),
    ("M2_merged_at_empty", {"R1_merged_at"},
     lambda e: e["prs"][0].update(merged_at="")),
    ("M3_ancestor_no", {"R2_ancestor"},
     lambda e: e["prs"][0].update(ancestor_of_branch="no")),
    ("M4_ancestor_unknown", {"R2_ancestor"},
     lambda e: e["prs"][0].update(ancestor_of_branch="unknown")),
    ("M5_no_declared_ref", {"R3_declared"},
     lambda e: e["prs"][0].update(declared_refs=[], loose_refs=[])),
    ("M6_declared_ref_wrong_issue", {"R3_declared"},
     lambda e: e["prs"][0].update(declared_refs=["123"])),
    ("M7_loose_ref_only", {"R3_declared"},
     lambda e: e["prs"][0].update(declared_refs=[], loose_refs=["900001"])),
]


def mutation_selftest() -> int:
    ok = True
    print("== 反向变异自证（每臂必须精确翻红，且失效条件集合与期望完全一致）==")
    try:
        refs_re = load_canonical_refs_re()
        print(f"  ✓ 引用文法来源一致：{GATE_COMMIT_MSG.name} → {refs_re.pattern}")
    except Incomplete as exc:
        print(f"  ✗ 引用文法来源读不到：{exc}")
        return 2

    base = judge(_fixture())
    if base["verdict"] != "PASS":
        print(f"  ✗ 原样夹具判红（应为 PASS）：{base['verdict']} {base['reason']}")
        ok = False
    else:
        print("  ✓ 原样夹具 PASS")
    for name, expected, mutate in MUTATIONS:
        ev = _fixture()
        mutate(ev)
        res = judge(ev)
        red = res["verdict"] != "PASS"
        failed = {k for k, v in res["detail"][0].get("checks", {}).items() if not v}
        precise = failed == expected
        if not (red and precise):
            ok = False
        print(f"  {'✓' if (red and precise) else '✗'} {name:28s} ⇒ "
              f"{res['verdict']:16s} 失效={sorted(failed)} "
              f"(期望 {sorted(expected)})")

    # 文法臂：正文里的引用形态必须被如实区分
    # （形态, 是否算合规 Refs 声明, 是否算自动关单路径）
    forms = [
        ("own_line_refs", "Refs #900001", True, False),
        ("own_line_refs_colon", "Refs: #900001", True, False),
        ("inline_mention", "本 PR 修的是 #900001 那个问题", False, False),
        ("quoted_convention", "按规范写 `Refs #900001`", False, False),
        ("closes_own_line", "Closes #900001", False, True),
        ("wrong_issue", "Refs #123456", False, False),
    ]
    for name, line, expect_decl, expect_close in forms:
        ev = _fixture()
        got = bool(_declared_refs(line, refs_re, ev["issue"]))
        close = bool(CLOSE_LINE_RE.findall(line))
        good = got == expect_decl and close == expect_close
        if not good:
            ok = False
        print(f"  {'✓' if good else '✗'} 文法 {name:22s} {line[:34]!r:38s} "
              f"⇒ 声明={got}(期望{expect_decl}) 自动关单={close}"
              f"(期望{expect_close})")

    # C1 非承重的回归：C1 缺失/存在都不得改变判定
    ev = _fixture()
    ev["cross_ref_index"] = []
    r_nocref = judge(ev)
    r_ref = judge(_fixture())
    same = r_nocref["verdict"] == r_ref["verdict"] == "PASS"
    print(f"  {'✓' if same else '✗'} C1 非承重回归：删掉交叉引用索引后判定仍为 "
          f"{r_nocref['verdict']}（与保留时 {r_ref['verdict']} 一致）")
    if not same:
        ok = False
    # Closes 只能是标记，不判 PASS
    ev = _fixture()
    ev["prs"][0].update(declared_refs=[], closes_refs=["900001"],
                        loose_refs=["900001"])
    r_close = judge(ev)
    good = r_close["verdict"] == "AUTO_CLOSE_PATH"
    print(f"  {'✓' if good else '✗'} Closes 只标记不判 PASS：{r_close['verdict']}")
    if not good:
        ok = False
    print("== 结论：%s ==" % ("全部臂符合预期" if ok else "存在不符合预期的臂"))
    return 0 if ok else 1


# ── CLI ───────────────────────────────────────────────────────────────────

def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(
        description="ISSUE_CLOSING_VERIFICATION Step 1 的 Gitea 原生机器判据")
    ap.add_argument("--issue", type=int, action="append", default=[],
                    help="issue 编号（可重复）")
    ap.add_argument("--branch", default=DEFAULT_BRANCH, help="目标分支")
    ap.add_argument("--json", action="store_true", help="机器可读输出")
    ap.add_argument("mode", nargs="?", default="check",
                    choices=["check", "mutation-selftest"])
    args = ap.parse_args(argv)

    if args.mode == "mutation-selftest":
        return mutation_selftest()

    if not args.issue:
        ap.print_usage()
        print("至少给一个 --issue", file=sys.stderr)
        return 2

    try:
        refs_re = load_canonical_refs_re()
    except Incomplete as exc:
        print(f"INCOMPLETE：{exc}", file=sys.stderr)
        return 2

    host = GITEA.split("//", 1)[-1]
    try:
        api = Gitea(host)
    except Incomplete as exc:
        print(f"INCOMPLETE：{exc}", file=sys.stderr)
        return 2
    try:
        results = []
        for n in args.issue:
            results.append(judge(collect(api, n, args.branch, refs_re)))
    except Incomplete as exc:
        print(f"INCOMPLETE：{exc}", file=sys.stderr)
        return 2
    finally:
        api.close()

    if args.json:
        print(json.dumps(results, ensure_ascii=False, indent=2))
    else:
        print(render(results))
        print()
        for r in results:
            print(f"#{r['issue']}：{r['verdict']}")

    return 0 if all(r["verdict"] == "PASS" for r in results) else 1


if __name__ == "__main__":
    sys.exit(main())
