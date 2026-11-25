#!/usr/bin/env python3
"""gen_verdict.py — run the 12 release criteria and emit V410-READINESS-VERDICT.json.

PROVENANCE — WHICH TEXT IS CAPTURED AND WHICH IS AUTHORED
==========================================================
This file used to claim "Nothing is hand-written".  That was false: criteria
4, 7, 8, 10, 11 and 12 embedded prose that no command ever printed.  The
three kinds of text are now kept apart, and every reader can tell them apart:

* `command`  — the shell command this script really runs.
* `observed` — the CAPTURED result of that command: its exit code plus a
                verbatim excerpt of its real stdout/stderr.  Where the script
                itself computes a boolean or a count from files (criteria 6
                and 7), the computed value carries the marker
                【本脚本计算】 so it cannot be mistaken for stdout.
* `note`     — an AUTHORED statement: prose written by a human/agent, NOT
                printed by any command.  Every authored paragraph starts with
                【作者陈述，非本次命令输出】 and names the file that actually
                carries the fact.

Concretely, the following are AUTHORED and now live in `note`, not in
`observed`: criterion 4's FAIL basis (the reason `verify_handoff.py --check`
exiting 0 does not satisfy the criterion), criterion 8's tree identity (the
lane log does not record it — see below), criterion 10's R1 drift counts
(`verify_r2_package.py` never looks at the R1 directory), and criterion 12's
facts (the criterion is waived; no command is run).

Criterion 8 (the ALPHA negative-fixture lane) needs 1-2 hours, so by default
the script reads the log of a run that was executed separately with the SAME
command on the SAME tree; `--run-neg` makes it run the lane itself.  The date
and the tree of that run are parameters (they cannot be derived from the log)
and are written into the verdict explicitly.  The tree is therefore a
RECONSTRUCTION, not an observation -- see NEG_RUN_TREE_DEFAULT below.

This file is COMMITTED to the repository (control ruling R69, Task 9 review
round 2).  It used to live only in the git-ignored `.superpowers/sdd/` area,
where a stale copy would silently re-write old strings into the verdict file
("差额来自判据 6 与判据 8 两条" while the summary said three; a hard-coded
mutation-arm count of 11 when the tool emits 10).  From now on the verdict is
REPRODUCIBLE from the committed tree (in the tree that generated it), and
every number in it is counted from a real run instead of being copied.

Usage (from the repository root):

  # 1. the working tree must be clean -- criterion 3's verifier refuses a dirty
  #    tree, and this script writes the verdict file at the very END
  git status --porcelain            # must be empty

  # 2. regenerate (the verdict file is written last, so every command above ran
  #    on a clean tree)
  python3 docs/refactor/v4.1.0-c/takeover/gen_verdict.py

  # 3. review the diff, then commit

Options: --repo --out --python --neg-log --neg-run-date --neg-run-tree --run-neg
         --r2-dir --allow-dirty
"""
import argparse
import collections
import hashlib
import json
import os
import re
import subprocess
import sys
import time

# ---------------------------------------------------------------------------
# Facts that are PROPERTIES of the repository, not of a run.  They are read
# from the repository itself, never hard-coded from a previous run.
# ---------------------------------------------------------------------------
HERE = os.path.dirname(os.path.abspath(__file__))
REPO_DEFAULT = os.path.abspath(os.path.join(HERE, "..", "..", "..", ".."))
BASE = "fd849ae20541901d5e59f45cd3ba2c51093f5d5e"
TOOLS = "docs/releases/v4.1.0/refactoring/tools"
VERDICT_REL = "docs/refactor/v4.1.0-c/takeover/V410-READINESS-VERDICT.json"
VERIFIER_REL = "docs/refactor/v4.1.0-c/takeover/verify_r2_package.py"
OWNER_RULING_REL = "docs/refactor/v4.1.0-c/takeover/V410-OWNER-RULING.md"

# The ALPHA negative-fixture lane: the run whose result the verdict quotes.
# These three are PROPERTIES OF THAT RUN, not of the repository -- the log does
# not record them, so they are parameters with the values of the 2026-11-25 run
# as defaults.  Overriding them is how a later run is folded in.
#
# NEG_RUN_TREE_DEFAULT IS A RECONSTRUCTION, NOT AN OBSERVATION.
# The lane log records only "scratch tree = <path>" and the fixture verdicts;
# it contains NO commit SHA (grep for a 40-hex string in run.log finds
# nothing).  The tree below was reconstructed from two independent facts:
#   (1) `git reflog` -- HEAD was 09ba360f6c from 2026-11-25 09:16:07 until
#       2026-11-25 12:08:52, when e2d98149d2 was committed;
#   (2) the lane log's mtime is 2026-11-25 12:07:40, i.e. BEFORE
#       e2d98149d2 existed as a commit.
# The lane uses the repository itself as its scratch tree and refuses to start
# on a dirty tree, so the whole lane ran with HEAD == 09ba360f6c.
# The previously shipped value (e2d98149d2) was the commit that was created
# AFTER the lane had already finished -- it was a copy of "the tip at the
# time the verdict was written", not the tree the lane ran on.
NEG_RUN_DATE_DEFAULT = "2026-11-25"
NEG_RUN_TREE_DEFAULT = "09ba360f6cfbf0be2aa4cf7d2064c4b8b50f7958"
NEG_LOG_DEFAULT = ("/Volumes/workspace/dev/sqlrustgo/.superpowers/sdd/"
                    "V410-READINESS-IMPLEMENTATION-PLAN/task-9-neg-logs/run.log")
R2_DIR_DEFAULT = ("/Volumes/workspace/dev/workspaces/dsh-ops/projects/"
                  "DSH-EXTERNAL-REVIEW-001/sqlrustgo-v4.1.0-r2")

# The paths the negative-fixture lane actually reads.  A change to any of
# them between the lane's tree and HEAD would make the RED=5/7 measurement
# non-transferable; a change outside them cannot.  The generator computes
# this set itself (see `lane_relevant_changes`) instead of asserting it.
LANE_RELEVANT_PREFIXES = (
    "crates/", "src/", "tests/", "tools/", "scripts/", "audit/", ".gitea/",
    "docs/governance/",
    "docs/releases/v4.1.0/GATE_CATALOG.jsonl",
    "docs/releases/v4.1.0/NEG_FIXTURES_v4.1.0.jsonl",
)

# The owner-approved reviewer list lives in V410-OWNER-RULING.md, between the
# two markers below.  Rows whose FIRST cell is a plain number are the list;
# the header row, the separator row and any prose row are ignored.  An empty
# list (no numbered row) means "the owner has not authorised any reviewer
# yet" and makes criterion 6 fail -- see criterion 6 below.
REVIEWER_LIST_BEGIN = "<!-- OWNER_APPROVED_REVIEWERS:BEGIN -->"
REVIEWER_LIST_END = "<!-- OWNER_APPROVED_REVIEWERS:END -->"


def run(cmd, cwd, timeout=1800, env=None):
    """Run a command; return (rc, combined output)."""
    e = dict(os.environ)
    if env:
        e.update(env)
    p = subprocess.run(cmd, cwd=cwd, shell=isinstance(cmd, str),
                       capture_output=True, text=True, timeout=timeout, env=e)
    return p.returncode, (p.stdout or "") + (p.stderr or "")


def tail(text, n=14):
    lines = [l for l in text.splitlines() if l.strip()]
    return "\n".join(lines[-n:])


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def owner_approved_reviewers(repo):
    """Read the owner-approved reviewer list from V410-OWNER-RULING.md.

    Returns (names, error).  `names` is None when the list cannot be located
    at all (error explains why); an empty list means "the owner has not
    authorised anyone yet".  The two are different facts and the verdict
    must not conflate them.
    """
    path = os.path.join(repo, OWNER_RULING_REL)
    if not os.path.isfile(path):
        return None, "%s 不存在" % OWNER_RULING_REL
    txt = open(path, encoding="utf-8").read()
    if REVIEWER_LIST_BEGIN not in txt or REVIEWER_LIST_END not in txt:
        return None, ("%s 里没有 %s … %s 标记块"
                      % (OWNER_RULING_REL, REVIEWER_LIST_BEGIN, REVIEWER_LIST_END))
    block = txt.split(REVIEWER_LIST_BEGIN, 1)[1].split(REVIEWER_LIST_END, 1)[0]
    names = []
    for line in block.splitlines():
        line = line.strip()
        if not line.startswith("|"):
            continue
        cells = [c.strip() for c in line.strip("|").split("|")]
        if len(cells) < 2:
            continue
        if not re.fullmatch(r"\d+", cells[0]):
            continue                      # header, separator, or a prose row
        name = cells[1].strip().strip("`").strip()
        if name:
            names.append(name)
    return names, None


def lane_relevant_changes(repo, a, b):
    """Paths changed between tree `a` and tree `b` that the neg lane reads."""
    rc, out = run("git diff --name-only %s %s" % (a, b), repo)
    if rc != 0:
        return None
    hits = []
    for p in out.split():
        for pref in LANE_RELEVANT_PREFIXES:
            if p.startswith(pref):
                hits.append(p)
                break
    return sorted(set(hits))


def all_changed_paths(repo, a, b):
    rc, out = run("git diff --name-status %s %s" % (a, b), repo)
    if rc != 0:
        return []
    rows = []
    for line in out.splitlines():
        parts = line.split("\t")
        if len(parts) >= 2:
            rows.append((parts[0], parts[-1]))
    return rows


def pick_python(explicit):
    """Choose the interpreter; jsonschema is needed by verify_handoff.py."""
    cands = []
    if explicit:
        cands.append(explicit)
    if os.environ.get("V410_PYTHON"):
        cands.append(os.environ["V410_PYTHON"])
    cands.append("/tmp/v410-task9-venv/bin/python")
    cands.append("python3")
    for c in cands:
        try:
            p = subprocess.run([c, "-c", "import jsonschema"], capture_output=True)
            if p.returncode == 0:
                return c
        except OSError:
            continue
    sys.stderr.write(
        "FATAL: no interpreter with jsonschema found (tried: %s).\n"
        "       verify_handoff.py --check needs jsonschema==4.23.0; create a venv:\n"
        "         python3 -m venv /tmp/v410-task9-venv\n"
        "         /tmp/v410-task9-venv/bin/pip install 'jsonschema==4.23.0'\n"
        "       or point at an interpreter with --python / $V410_PYTHON.\n" % ", ".join(cands))
    raise SystemExit(4)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--repo", default=REPO_DEFAULT)
    ap.add_argument("--out", default=None)
    ap.add_argument("--python", default=None)
    ap.add_argument("--neg-log", default=NEG_LOG_DEFAULT)
    ap.add_argument("--neg-run-date", default=NEG_RUN_DATE_DEFAULT)
    ap.add_argument("--neg-run-tree", default=NEG_RUN_TREE_DEFAULT)
    ap.add_argument("--r2-dir", default=R2_DIR_DEFAULT)
    ap.add_argument("--run-neg", action="store_true")
    ap.add_argument("--allow-dirty", action="store_true")
    args = ap.parse_args()

    repo = os.path.abspath(args.repo)
    out = os.path.abspath(args.out) if args.out else os.path.join(repo, VERDICT_REL)
    r2 = os.path.abspath(args.r2_dir)

    # -- preflight: criterion 3's verifier refuses a dirty tree --------------
    dirty = run("git status --porcelain", repo)[1].strip()
    if dirty and not args.allow_dirty:
        sys.stderr.write(
            "FATAL: the working tree is not clean:\n%s\n"
            "       criterion 3 (verify_sha_scope.py) refuses a dirty tree, so the\n"
            "       verdict would be a statement about uncommitted bytes.\n"
            "       Commit first, or pass --allow-dirty if you know what you are doing.\n"
            % dirty)
        return 4

    py = pick_python(args.python)
    crit = []

    # -- 1 / 2 -------------------------------------------------------------
    cmd1 = "%s %s/verify_baseline.py --strict" % (py, TOOLS)
    rc1, out1 = run(cmd1, repo)
    rc1b, out1b = run("git ls-remote origin refs/heads/base/v4.1.0-frozen", repo)
    remote = out1b.strip().split()[0] if out1b.strip() else "<none>"
    rc1c, _ = run("git merge-base --is-ancestor %s %s" % (BASE, remote), repo)
    anc = (rc1c == 0)
    rc1d, out1d = run("git ls-remote origin refs/heads/develop/v4.1.0", repo)
    dev_live = out1d.strip().split()[0] if out1d.strip() else "<none>"
    rc1e, _ = run("git merge-base --is-ancestor %s %s" % (dev_live, BASE), repo)
    dev_anc = (rc1e == 0)
    dev_frozen = os.path.join(repo, "docs/refactor/v4.1.0-c/takeover/BASELINE_FREEZE.json")
    dev_tip_frozen = BASE
    if os.path.isfile(dev_frozen):
        try:
            dev_tip_frozen = json.load(open(dev_frozen, encoding="utf-8")).get(
                "parent", BASE)          # `parent` IS the frozen develop tip
        except Exception:
            pass
    crit.append({
        "id": 1,
        "criterion": "BASE（内容基线）唯一、develop tip 是其祖先、含 BASE 的分支已推送",
        "command": "\n".join([
            cmd1,
            "git ls-remote origin refs/heads/base/v4.1.0-frozen",
            "git merge-base --is-ancestor %s %s" % (BASE, remote),
            "git ls-remote origin refs/heads/develop/v4.1.0",
            "git merge-base --is-ancestor %s %s" % (dev_live, BASE),
        ]),
        "observed": (
            "exit=%d\n%s\n"
            "git ls-remote origin refs/heads/base/v4.1.0-frozen -> %s (exit=%d)\n"
            "git merge-base --is-ancestor %s %s -> exit=%d (%s)\n"
            "git ls-remote origin refs/heads/develop/v4.1.0 -> %s (exit=%d)\n"
            "git merge-base --is-ancestor %s %s -> exit=%d (%s)\n"
            "本地分支 tip = %s；远端 base/v4.1.0-frozen tip 是 BASE 的后代 = %s"
            % (rc1, tail(out1, 8), remote, rc1b, BASE, remote, rc1c,
               "是" if anc else "否",
               dev_live, rc1d, dev_live, BASE, rc1e,
               "是" if dev_anc else "否",
               run("git rev-parse HEAD", repo)[1].strip(),
               "是" if anc else "否")),
        "verdict": "PASS" if (rc1 == 0 and rc1b == 0 and anc) else "FAIL",
        "caveats": [
            "判据按**计划正文 Step 1 的命令**判：verify_baseline.py --strict（4/4）+ "
            "`git ls-remote origin refs/heads/base/v4.1.0-frozen` 的 tip 须是 BASE 的后代（成立）。",
            "**设计稿 §A5 #1 的另一条可核方式**（`git merge-base --is-ancestor origin/develop/v4.1.0 BASE`）"
            "在本次实测中**不成立**：远端 develop/v4.1.0 的 tip 已是 %s —— 它是 BASE 的**后代**而非祖先。"
            "原因是 develop/v4.1.0 被合并了 `base/v4.1.0-frozen`（PR #5928，merge commit ea285c5fda）"
            "—— 冻结分支已被并入 develop（= O11 事实上已被行动回答）。这不是 BASE 的缺陷。"
            "判据 1 的三项实质条件仍全部成立：BASE 唯一 ✓；**冻结时点的 develop tip** %s "
            "是 BASE 的祖先 ✓（BASELINE_FREEZE.json 记录，verify_baseline 的 B1 用的就是它）；"
            "含 BASE 的分支已推送 ✓（远端 `base/v4.1.0-frozen` 的 tip 现为 %s）。"
            % (dev_live, dev_tip_frozen[:12], remote),
        ],
    })
    m = re.search(r"\[B3\] PASS\s+(.*)", out1)
    crit.append({
        "id": 2,
        "criterion": "45da6968ae 的 123 个材料文件全在 BASE 树内",
        "command": "%s %s/verify_baseline.py --strict（断言 B3）" % (py, TOOLS),
        "observed": (
            "exit=%d；%s\n"
            "（同一次运行覆盖 B1/B2/B4，见判据 1）" % (rc1, m.group(0).strip() if m else tail(out1, 4))),
        "verdict": "PASS" if (rc1 == 0 and m) else "FAIL",
    })

    # -- 3 -----------------------------------------------------------------
    cmd3 = "%s %s/verify_sha_scope.py" % (py, TOOLS)
    rc3, out3 = run(cmd3, repo)
    crit.append({
        "id": 3,
        "criterion": "前向/历史 SHA 分界（前向绑定唯一等于 BASE；历史绑定逐项未变）",
        "command": cmd3,
        "observed": "exit=%d\n%s" % (rc3, tail(out3, 9)),
        "verdict": "PASS" if rc3 == 0 else "FAIL",
    })

    # -- 4 / 5 -------------------------------------------------------------
    # Both criteria run the SAME command, so their evidence is the SAME run.
    cmd4 = "%s %s/verify_handoff.py --check" % (py, TOOLS)
    rc4, out4 = run(cmd4, repo)
    pend = re.search(r'"tasks_pending_adr_approval": \[(.*?)\]', out4, re.S)
    pending = re.findall(r'"(C01-\d+)"', pend.group(1)) if pend else []
    mut = re.search(r'"mutation_rejections": \[(.*?)\]', out4, re.S)
    mut_list = re.findall(r'"([a-z_]+)"', mut.group(1)) if mut else []
    n_mut = len(mut_list)          # COUNTED from the real output, never copied
    note = re.search(r'"notes": \[(.*?)\]', out4, re.S)
    note_txt = re.sub(r"\s+", " ", note.group(1)).strip() if note else ""
    mut_line = "\"mutation_rejections\"（**%d 条**变异臂全部被拒）：%s" % (n_mut, ", ".join(mut_list))
    crit.append({
        "id": 4,
        "criterion": "ADR 机器字段非空、档位正确、映射满足 A2.2 五条规则",
        "command": cmd4,
        "observed": (
            "exit=%d（校验器退出码）\n"
            "%s\n%s\n%s\n"
            "%s\n"
            "\"notes\": %s"
            % (rc4,
               re.search(r'"adr_approval": "[^"]*"', out4).group(0) if re.search(r'"adr_approval": "[^"]*"', out4) else "",
               re.search(r'"adr_approval_state": "[^"]*"', out4).group(0) if re.search(r'"adr_approval_state": "[^"]*"', out4) else "",
               re.search(r'"tasks_with_empty_approved_adr_refs": \d+', out4).group(0) if re.search(r'"tasks_with_empty_approved_adr_refs": \d+', out4) else "",
               mut_line, note_txt)),
        "note": (
            "【作者陈述，非本次命令输出】**判据不满足的原因**（三条互相独立的事实）：\n"
            "(1) 计划正文 Step 4 的口径：「O1（`ADAPTER-MAPPING-PENDING.md` 中的 task 列表，"
            "**若非空则 A5 #4 不能判 PASS**）」——该清单**非空**："
            "`docs/refactor/v4.1.0-c/takeover/ADAPTER-MAPPING-PENDING.md` 有 "
            "`| C01-15 | … |` 一行，并写明其 `approved_adr_refs` 保持 `[]`。\n"
            "(2) 判据自身的措辞「**映射满足 A2.2 五条规则**」：A2.2 **R1** = 「每个 task 的 "
            "`approved_adr_refs` 至少 1 项」——C01-15 违反 R1（该清单明写「R1 与 R2 冲突时以 R2 为准："
            "该包保持 `[]`」，即**明知违反 R1 而保留**）。\n"
            "(3) 校验器**不担保 R1**：`verify_handoff.py:383-384` 在 `refs` 为空时**直接 return**；"
            "且它自己的输出是反面证据 —— `\"adr_approval_state\": \"PENDING\"` 与 note "
            "「the owner's ADR decision is PENDING, not verified」。\n"
            "⇒ `--check` 退出 0 只说明「可核方式里的那几条断言成立」，**不等于**判据 4 成立。"
            "按 R63「不得为了让总数好看而改判、拆分或重定义判据」，判据 4 = **FAIL**"
            "（缺口在**映射层**：需属主裁决补 ADR 引用 vs 豁免）。\n"
            "**事实来源**：`docs/refactor/v4.1.0-c/takeover/ADAPTER-MAPPING-PENDING.md`、"
            "`docs/releases/v4.1.0/refactoring/tools/verify_handoff.py:383-384`、"
            "`docs/refactor/v4.1.0-c/takeover/V410-REFACTOR-READINESS-IMPLEMENTATION-PLAN.md`"
            "（Task 4 Step 4 的口径）、设计稿 A2.2 R1。"),
        "verdict": "FAIL" if pending else ("PASS" if rc4 == 0 else "FAIL"),
    })
    mani = re.search(r'"handoff_manifest_sha256": "([0-9a-f]+)"', out4)
    bound = re.search(r'"manifest_bound_files": (\d+)', out4)
    crit.append({
        "id": 5,
        "criterion": "manifest 零漂移",
        "command": cmd4 + "（含 manifest 全文件哈希复算；与判据 4 同一条命令、同一次运行）",
        "observed": (
            "exit=%d\n%s\n%s\n%s\n"
            "manifest 绑定文件数 = %s；manifest 自身 sha256 = %s\n"
            "%s\n"
            "（判据 5 与判据 4 跑的是**同一条** `verify_handoff.py --check`，证据同源。）"
            % (rc4,
               re.search(r'"hara_hashed_files": \d+', out4).group(0) if re.search(r'"hara_hashed_files": \d+', out4) else "",
               re.search(r'"local_links": \d+', out4).group(0) if re.search(r'"local_links": \d+', out4) else "",
               re.search(r'"task_dag": "[^"]*"', out4).group(0) if re.search(r'"task_dag": "[^"]*"', out4) else "",
               bound.group(1) if bound else "?",
               mani.group(1) if mani else "?",
               mut_line)),
        "verdict": "PASS" if rc4 == 0 else "FAIL",
    })

    # -- 6 -----------------------------------------------------------------
    # Criterion 6 needs FOUR independent conditions to hold at the same time.
    # The old logic only counted a non-empty verdict field and never looked at
    # WHO signed -- so a single field value could flip the criterion to PASS.
    # R-ORACLE ("the signer must not be the implementer of that scenario") and
    # C7 ("the Oracle is signed by the Owner / an independent Reviewer; the
    # implementer must not self-sign") are now enforced by the generator.
    cmd6 = "%s %s/verify_matrix.py" % (py, TOOLS)
    rc6, out6 = run(cmd6, repo)
    diff = "docs/releases/v4.1.0/refactoring/evidence/L1-DIFF-fd849ae20541.md"
    hdr = "| matrix_index | case_id | feature | pilot状态 | ledger状态 | BASE状态 | 差异类型 | 说明 |"
    diff_file = os.path.join(repo, diff)
    hdr_ok = os.path.isfile(diff_file) and \
        hdr in open(diff_file, encoding="utf-8").read()
    mx = json.load(open(os.path.join(repo, TOOLS.replace("tools", "TEST_COVERAGE_MATRIX.json")),
                        encoding="utf-8"))
    cases = mx["cases"]
    l1 = [c for c in cases if c.get("feature") in ("T1", "T2", "T3")]
    states = dict(collections.Counter(str(c.get("execution_status")) for c in l1))
    verdicts = dict(collections.Counter(str(c.get("assertion_review_verdict")) for c in l1))
    by = dict(collections.Counter(str(c.get("assertion_review_performed_by")) for c in l1))

    # (i) every L1 case carries a terminal execution_status (the matrix's own
    #     status vocabulary -- a status outside it is NOT a terminal state).
    vocab = mx.get("status_vocabulary_mapping", {})
    terminal = set(vocab.keys()) | {"PASS", "KNOWN_FAIL", "FAIL", "NOT_RUN",
                                    "INFRA_ERROR", "UNSUPPORTED", "DIALECT_DIFFERENCE"}
    no_status = [c.get("case_id") for c in l1
                 if str(c.get("execution_status")) not in terminal]
    all_terminal = not no_status

    # (ii) the owner-approved reviewer list.
    approved, list_err = owner_approved_reviewers(repo)

    # (iii) the cases the judgement PASSED: `assertion_review_verdict` present
    #      and not "ASSERTION_UNREVIEWABLE" (i.e. the judgement concluded the
    #      assertion is independently decidable).  These are the cases that
    #      MUST carry a qualified Oracle signature.
    judged_pass = [c for c in l1
                   if c.get("assertion_review_verdict") not in (None, "ASSERTION_UNREVIEWABLE")]
    qualified, unqualified = [], []
    for c in judged_pass:
        signer = str(c.get("assertion_reviewed_by") or "").strip()
        performer = str(c.get("assertion_review_performed_by") or "").strip()
        why = []
        if approved is None:
            why.append("没有属主授权的 Reviewer 名单（%s）" % list_err)
        if not signer:
            why.append("缺 assertion_reviewed_by")
        if signer and performer and signer == performer:
            why.append("签署人 == 判定人（R-ORACLE）")
        if signer and approved is not None and signer not in approved:
            why.append("签署人不在属主授权名单内")
        (qualified if not why else unqualified).append((c.get("case_id"), why))
    signed_now = sum(1 for c in l1
                     if str(c.get("assertion_reviewed_by") or "").strip())

    cond6 = {
        "c6_verify_matrix_exit_0": (rc6 == 0),
        "c6_diff_table_exists_and_header_matches": bool(hdr_ok),
        "c6_all_l1_have_terminal_status": all_terminal,
        "c6_owner_approved_reviewer_list_present": (approved is not None),
        "c6_at_least_one_case_judged_pass": (len(judged_pass) > 0),
        "c6_every_judged_pass_case_qualified_signer": (len(unqualified) == 0),
    }
    c6_pass = all(cond6.values())

    if approved is None:
        list_state = "**没有属主授权的 Reviewer 名单**（%s）" % list_err
    elif not approved:
        list_state = "**没有属主授权的 Reviewer 名单**（名单为空：属主尚未指认任何独立 Reviewer）"
    else:
        list_state = "属主授权的 Reviewer 名单 = %s" % ", ".join(approved)

    unqual_txt = ("；".join("%s：%s" % (cid, "、".join(why)) for cid, why in unqualified)
                  if unqualified else "无")
    crit.append({
        "id": 6,
        "criterion": "已执行切片全部有终态 + Oracle 签署（签署人合格）+ 双向差异表",
        "command": cmd6 + "  &&  检查 " + diff + " 的表头  &&  读 " + OWNER_RULING_REL
                   + " 的属主授权 Reviewer 名单  &&  统计矩阵的签署字段",
        "observed": (
            "exit=%d\n%s\n"
            "【本脚本计算】L1 切片（feature ∈ T1/T2/T3）=%d 条，execution_status 分布 = %s；"
            "全部有终态 = %s%s\n"
            "【本脚本计算】双向差异表：%s —— 文件存在=%s，表头逐字符匹配=%s\n"
            "【本脚本计算】assertion_review_verdict 分布 = %s；"
            "assertion_review_performed_by = %s；"
            "已有 assertion_reviewed_by 的用例数 = %d\n"
            "【本脚本计算】判定为「可独立判定」（即被判 PASS、待副签）的用例 = %d 条；"
            "其中签署人合格的 = %d 条，不合格的 = %d 条\n"
            "【本脚本计算】%s\n"
            "【本脚本计算】判据 6 的四个条件：\n"
            "  (1) verify_matrix.py 退出 0 = %s\n"
            "  (2) 双向差异表存在且表头匹配 = %s\n"
            "  (3) L1 全部用例有终态 = %s\n"
            "  (4) 每条被判 PASS 的用例有合格签署人（有 assertion_reviewed_by、≠ 判定人、"
            "在属主授权名单内）= %s\n"
            "签署人不合格的用例：%s\n"
            "⇒ 判据 6 = %s"
            % (rc6, tail(out6, 8), len(l1), states, all_terminal,
               ("；无终态的用例：%s" % ", ".join(str(x) for x in no_status)) if no_status else "",
               diff, os.path.isfile(diff_file), hdr_ok,
               verdicts, by, signed_now,
               len(judged_pass), len(qualified), len(unqualified),
               list_state,
               cond6["c6_verify_matrix_exit_0"],
               cond6["c6_diff_table_exists_and_header_matches"],
               cond6["c6_all_l1_have_terminal_status"],
               cond6["c6_every_judged_pass_case_qualified_signer"],
               unqual_txt,
               "PASS" if c6_pass else "FAIL")),
        "note": (
            "【作者陈述，非本次命令输出】判据 6 的判定规则（本波按整支评审 I-2 重写）：\n"
            "PASS 需**同时**满足四条：(1) `verify_matrix.py` 退出 0；(2) 双向差异表存在且表头逐字符匹配；"
            "(3) L1 全部用例有终态；(4) **每条被判 PASS（`assertion_review_verdict` 存在且不是 "
            "`ASSERTION_UNREVIEWABLE`）的用例都有 `assertion_reviewed_by`，且该值 ≠ "
            "`assertion_review_performed_by`，且落在属主授权的 Reviewer 名单里**。\n"
            "名单从 `%s` 的 `%s … %s` 标记块读取（第一列是序号的行才算数）。\n"
            "**名单为空 ⇒ 一律判 FAIL**；已判 PASS 但签署人不合格 ⇒ **判 FAIL**。\n"
            "旧逻辑只数「`assertion_review_verdict` 非空」这一个字段、**从不看是谁签的**，"
            "因此属主一旦指认 Reviewer，仅凭一个字段值就能把 #6 翻成 PASS —— 这正是 Task 7 评审"
            "留给 Task 9 的义务（台账 :174）。\n"
            "**事实来源**：`docs/refactor/v4.1.0-c/takeover/V410-READINESS-LEDGER.md`（R-ORACLE、"
            "R40–R42、Task 7 评审第 1 轮）、`%s` 的 §3.1 属主授权 Reviewer 名单、"
            "`docs/refactor/v4.1.0-c/takeover/ORACLE-REVIEW-LOG-T1.md` §0。"
            % (OWNER_RULING_REL, REVIEWER_LIST_BEGIN, REVIEWER_LIST_END, OWNER_RULING_REL)),
        "verdict": "PASS" if c6_pass else "FAIL",
    })

    # -- 7 -----------------------------------------------------------------
    wf = ".gitea/workflows"
    cmd7 = "grep -rn -- '--stage ALPHA' .gitea/workflows/*.yml"
    rc7, out7 = run(cmd7, repo)
    hits = []
    for fn in sorted(os.listdir(os.path.join(repo, wf))):
        if not fn.endswith(".yml"):
            continue
        txt = open(os.path.join(repo, wf, fn), encoding="utf-8").read()
        if "--stage ALPHA" in txt:
            hits.append(fn)
    alpha_file = hits[0] if len(hits) == 1 else None
    alpha_ok = False
    detail7 = ""
    if alpha_file:
        txt = open(os.path.join(repo, wf, alpha_file), encoding="utf-8").read()
        cmd_lines = [l for l in txt.splitlines() if "--stage ALPHA" in l and "run:" in l]
        step = bool(cmd_lines) and all("--dry-run" not in l for l in cmd_lines)
        guard = "startsWith(github.ref, 'refs/heads/develop/')" in txt
        trig = "workflow_dispatch" in txt
        alpha_ok = bool(step and guard and trig)
        detail7 = (
            "exit=%d\n"
            "【命令 stdout】\n%s\n"
            "【本脚本计算】执行点唯一：%s（命中 %d 个 workflow 文件）\n"
            "【本脚本计算】真实执行（非 --dry-run）：%s\n"
            "【本脚本计算】触发条件可指认（workflow_dispatch）：%s；"
            "步骤 if: startsWith(github.ref, 'refs/heads/develop/') = %s\n"
            "【作者陈述，非本次命令输出】退出码可传播：check_stage.sh 有 set -euo pipefail，"
            "门禁 FAIL 时 exit 1（事实来源：`scripts/gate/check_stage.sh`）"
            % (rc7, out7.strip() or "（无命中）",
               alpha_file, len(hits), step, trig, guard))
    else:
        detail7 = "exit=%d\n%s\n【本脚本计算】`--stage ALPHA` 命中 %d 个 workflow 文件 ⇒ 执行点不唯一" % (
            rc7, out7.strip() or "（无命中）", len(hits))
    crit.append({
        "id": 7,
        "criterion": "ALPHA 阶段清单有 CI 执行点",
        "command": cmd7 + "  +  逐条核对「真实执行 / 非 --dry-run / 退出码可传播 / 触发条件可指认」",
        "observed": detail7,
        "verdict": "PASS" if alpha_ok else "FAIL",
    })

    # -- 8 -----------------------------------------------------------------
    cmd8 = "bash audit/v4.1.0/neg/verify_neg_fixtures.sh"
    neg_log = args.neg_log
    if args.run_neg or not os.path.isfile(neg_log):
        rc8, out8 = run(cmd8, repo, timeout=4 * 3600)
        open(neg_log, "w", encoding="utf-8").write(out8)
        neg_src = "本次运行（脚本内实跑）"
        neg_date = time.strftime("%Y-%m-%d")
        neg_tree = run("git rev-parse HEAD", repo)[1].strip()
        neg_sha = sha256_file(neg_log)
        neg_tree_origin = "本次运行：HEAD 即实测树（观测值）。"
    else:
        out8 = open(neg_log, encoding="utf-8").read()
        neg_date = args.neg_run_date
        neg_tree = args.neg_run_tree
        neg_sha = sha256_file(neg_log)
        neg_tree_origin = ("**推定/重构值**（不是从日志里读出来的）："
                           "日志本身**不含**树身份 —— `run.log` 里没有任何 40 位十六进制串，"
                           "只有 `scratch tree = <路径>` 与逐夹具判决。")
    summ = re.search(r"verify_neg_fixtures: RED=(\d+)/7\s+INCONCLUSIVE=(\d+)/7", out8)
    red = int(summ.group(1)) if summ else -1
    inc = int(summ.group(2)) if summ else -1
    rows = re.findall(r"^  (scripts/gate/\S+)\s+gate=(\w+)\s+stage=(\w+)", out8, re.M)
    # The exit code comes from the SCRIPT's own exit-code semantics
    # (verify_neg_fixtures.sh: RED_N == 7 -> 0, otherwise -> 3).  It is NOT read
    # from the log: the log's last line is an empty `NEG_EXIT=` and proves
    # nothing.
    rc8 = 0 if red == 7 else 3
    # Is the measurement transferable from the lane's tree to HEAD?  COMPUTED,
    # not asserted: the generator diffs the two trees and classifies every
    # changed path against the set of paths the lane actually reads.
    changed_rows = all_changed_paths(repo, neg_tree, "HEAD")
    lane_hits = lane_relevant_changes(repo, neg_tree, "HEAD")
    if lane_hits is None:
        lane_txt = "【本脚本计算】`git diff --name-only %s HEAD` 执行失败 ⇒ 不可判定" % neg_tree
        lane_ok = None
    elif lane_hits:
        lane_txt = ("【本脚本计算】两棵树之间有 **%d** 个泳道相关路径改动：%s "
                    "⇒ RED=%d/7 的测量**不可**直接迁移到 HEAD" % (len(lane_hits), ", ".join(lane_hits), red))
        lane_ok = False
    else:
        lane_txt = ("【本脚本计算】`git diff --name-only %s HEAD` 共 %d 个改动路径，"
                    "其中**泳道相关路径（产品代码 / 门禁脚本 / 负向夹具 / 门禁编目 / 夹具台账 / "
                    "阶段配置 / workflow）= 0** ⇒ RED=%d/7 的测量**可**迁移到 HEAD，"
                    "本次 FAIL 判定不受影响。改动路径全表：%s"
                    % (neg_tree, len(changed_rows), red,
                       "；".join("%s %s" % (st, p) for st, p in changed_rows) or "（空）"))
        lane_ok = True
    crit.append({
        "id": 8,
        "criterion": "ALPHA 7 道负向夹具两层 RED",
        "command": cmd8,
        "observed": (
            "【沿用 %s 的实测值】完整日志 %s，sha256 `%s`\n"
            "退出码=%d —— 来自脚本自身的退出码语义（`verify_neg_fixtures.sh`：RED_N==7 → 0，否则 → 3）；"
            "**不是**从日志里读的（日志末行的 `NEG_EXIT=` 是空的，不构成证据）。\n"
            "%s\n逐门禁：\n%s\n"
            "%s\n"
            "⇒ RED=%d/7 ≠ 7/7 ⇒ 判据 8 = FAIL（缺口在**证据层**）。"
            % (neg_date, neg_log, neg_sha, rc8,
               summ.group(0) if summ else tail(out8, 4),
               "\n".join("  %s gate=%s stage=%s" % r for r in rows),
               lane_txt,
               red)),
        "note": (
            "【作者陈述，非本次命令输出】**实测日期：%s；实测树：`%s`**\n"
            "**该树身份是推定/重构，不是观测值。** %s\n"
            "重构依据（两个互相独立的事实，均可复算）：\n"
            "(1) `git reflog --date=iso`：HEAD 从 2026-11-25 09:16:07（09ba360f6c 的提交时刻）"
            "到 2026-11-25 12:08:52（e2d98149d2 的提交时刻）一直是 `09ba360f6c`；\n"
            "(2) 泳道日志的 mtime 是 2026-11-25 12:07:40 —— **早于** e2d98149d2 作为提交存在的那一刻。"
            "泳道以仓库自身作 scratch 树，且 `verify_neg_fixtures.sh` 在脏树上**拒绝启动**，"
            "所以全程 HEAD = `09ba360f6c`。\n"
            "**先前写的 `e2d98149d2…` 是错的**：那是泳道跑完之后才被创建的那个提交，"
            "它被当成了「写裁决文件那一刻的 tip」，而不是泳道实际跑过的那棵树 —— "
            "这正是 C5（证据必须绑定完整 commit SHA，跨 SHA 证据不可继承）的缺陷类，"
            "且出现在**属主要签的那份**文件里。\n"
            "**结论未受影响**：%s\n"
            "**事实来源**：`git reflog`、泳道日志的 mtime、"
            "`audit/v4.1.0/neg/verify_neg_fixtures.sh`（脏树拒绝逻辑）、"
            "`docs/refactor/v4.1.0-c/takeover/V410-READINESS-STATE.md` §4 的 R63。"
            % (neg_date, neg_tree, neg_tree_origin, lane_txt)),
        "verdict": "PASS" if red == 7 else "FAIL",
    })

    # -- 9 -----------------------------------------------------------------
    cmd9 = "python3 scripts/gate/gate_registry.py check"
    rc9, out9 = run(cmd9, repo)
    crit.append({
        "id": 9,
        "criterion": "门禁编目内部自洽",
        "command": cmd9,
        "observed": "exit=%d\n%s" % (rc9, tail(out9, 4)),
        "verdict": "PASS" if rc9 == 0 else "FAIL",
    })

    # -- 10 / 11 -----------------------------------------------------------
    # The verifier that lives in THIS repository (R69 / review round 2, M-8):
    # the in-repo copy is the reproducibility anchor; the DSH tree holds the
    # R2 package itself.
    cmd10 = "python3 %s --r2-dir %s" % (VERIFIER_REL, r2)
    rc10, out10 = run(cmd10, repo)
    s10 = re.search(r"evidence_index\.json \(SSOT\): total=(\d+) counts=(\{[^}]*\})", out10)
    obs10 = (
        "exit=%d\n%s\n%s\n%s\n"
        "校验器 = **本仓副本** `%s`（R69：生成/校验器都进仓，裁决文件可从已提交的树复现）；"
        "R2 包本身在 `%s`。"
        % (rc10, s10.group(0) if s10 else "",
           "\n".join(l for l in out10.splitlines() if l.startswith("PASS  #10")),
           "\n".join(l for l in out10.splitlines() if l.startswith("FAIL  #10")) or "FAIL #10：无",
           VERIFIER_REL, r2))
    crit.append({
        "id": 10,
        "criterion": "外审包三套统计一致（71/71/71）",
        "command": cmd10 + "（判据 10 的 10 项断言）",
        "observed": obs10,
        "note": (
            "【作者陈述，非本次命令输出】**R1 的漂移实测**：EVIDENCE-COVERAGE.md 68 条 21/41/6、"
            "01-EXECUTIVE-SUMMARY.md 64 条 13/45/6；R2 由 evidence_index.json 单向生成后三套均为 "
            "71 条 24/41/6。\n"
            "**这条命令产不出上面那三个数** —— `verify_r2_package.py --r2-dir …` **只读 R2 目录**，"
            "根本不读 R1 目录，所以 R1 的漂移数是**别处测的**。\n"
            "**事实来源**：`docs/refactor/v4.1.0-c/takeover/R2-RELEASE-PACKAGE.md` §3.2"
            "（三套统计的实测值与差额来源表）与 §3.1（O8 授权的 5 类修正）。"),
        "verdict": "PASS" if (rc10 == 0 and not any(
            l.startswith("FAIL") for l in out10.splitlines())) else "FAIL",
    })
    obs11 = (
        "exit=%d\n%s\n%s"
        % (rc10,
           "\n".join(l for l in out10.splitlines() if l.startswith("PASS  #11")),
           "\n".join(l for l in out10.splitlines() if l.startswith("FAIL  #11")) or "FAIL #11：无"))
    crit.append({
        "id": 11,
        "criterion": "dead 门禁口径唯一化",
        "command": "python3 %s --r2-dir %s（判据 11 的全文检索断言）" % (VERIFIER_REL, r2),
        "observed": obs11,
        "note": (
            "【作者陈述，非本次命令输出】检索式：全部有文档引用 / 全部仍有文档引用 / 全部有正文引用 / "
            "61 道全部有文档引用（排除 4 个构建脚本，它们以旧串为替换锚点）；"
            "唯一口径陈述在 04-GATE-GOVERNANCE.md §2。\n"
            "**事实来源**：`docs/refactor/v4.1.0-c/takeover/R2-RELEASE-PACKAGE.md` §3.1（R1-F-003/R1-F-004）"
            "与 §6；R2 包内的 `04-GATE-GOVERNANCE.md` §2。"),
        "verdict": "PASS" if (rc10 == 0 and not any(l.startswith("FAIL  #11") for l in out10.splitlines())) else "FAIL",
    })

    # -- 12 ----------------------------------------------------------------
    crit.append({
        "id": 12,
        "criterion": "SQLRUSTGO_LOCAL_ONLY 路由 fail-closed —— 本次准入**显式豁免**（O9）",
        "command": "（显式豁免，**本次不跑任何校验命令**；事实记录见设计稿 §A5 #12 与 V410-OWNER-RULING.md §2.1）",
        "observed": (
            "本次**未运行任何命令**（显式豁免）：判据 12 没有可核方式，它的 `observed` 里没有命令输出。\n"
            "判定 = WAIVED。"),
        "note": (
            "【作者陈述，非本次命令输出】**豁免依据与事实**（全部来自既有文件，命令产不出）：\n"
            "(1) 设计稿 §A5 #12 与 §6.1「已裁决」表的 O9 行称「不纳入，显式豁免」；"
            "**但该设计稿头部写的是 `design_status: PROPOSED — 待属主批准后转为实施计划`、"
            "`prepared_by: DSH-B 主导`**，整个 takeover 目录里**没有**任何独立于执行方的、"
            "属主签署的 O9 记录 ⇒ **这份豁免目前只有一份自称 PROPOSED 的设计稿背书**。\n"
            "(2) 事实：`SQLRUSTGO_LOCAL_ONLY` 在 sqlrustgo 仓库零命中，存在于 DSH 仓库；"
            "DSH 路由器是**策略级** LOCAL_ONLY（router.py:277/290），不存在 per-task 的数据分类维度，"
            "「缺分类即拒绝」在本次准入范围内**未成立**。\n"
            "(3) 豁免记录（含 waiver_id / reason / scope / compensating_control / expires_at / approved_by）"
            "写在 `docs/refactor/v4.1.0-c/takeover/V410-OWNER-RULING.md` §2.1；"
            "**`approved_by` 字段已改指向该文件 §4 旁边新增的、留空的 O9 确认位**"
            "（不指向那份自称 PROPOSED 的设计稿）。\n"
            "**事实来源**：`docs/refactor/v4.1.0-c/takeover/V410-REFACTOR-READINESS-DESIGN.md`"
            "（头部 + §6.1 O9 行）、`docs/refactor/v4.1.0-c/takeover/V410-OWNER-RULING.md` §2.1 与 §4。"),
        "verdict": "WAIVED",
    })

    # -- the summary -------------------------------------------------------
    counts = {}
    for c in crit:
        counts[c["verdict"]] = counts.get(c["verdict"], 0) + 1
    fail_ids = [c["id"] for c in crit if c["verdict"] == "FAIL"]
    # The note is DERIVED from the criteria, never hard-coded: the whole point
    # of R69 is that a stale sentence in here cannot contradict the summary.
    note = (
        "每条判据的 `command` 都由本文件（gen_verdict.py，已入仓）真的执行；"
        "**`observed` 只放该次执行的退出码与真实输出摘要**（本脚本自己算出来的值标注【本脚本计算】）；"
        "**手写的散文一律在 `note` 里，并以【作者陈述，非本次命令输出】开头并指向它的事实来源文件** —— "
        "判据 4 的 FAIL 依据、判据 8 的树身份、判据 10/11 的口径说明、判据 12 的事实都属于这一类。"
        "计划正文验收段写的「11 条 PASS + 1 条 WAIVED」是 Task 7/10 之前的乐观预期，以实测为准（R63）；"
        "本次实测的 FAIL 是判据 %s 共 %d 条（%s）。"
        % ("、".join("#%d" % i for i in fail_ids), len(fail_ids),
           "、".join("判据 %d（%s）" % (c["id"], {"4": "映射层：A2.2 R1 未满足",
                                               "6": "签署层：Oracle 副签 0 条 / 签署人不合格",
                                               "8": "证据层：夹具泳道未达 7/7"}.get(str(c["id"]), "见 observed"))
                     for c in crit if c["id"] in fail_ids)))
    doc = {
        "format": "v410-readiness-verdict/v1",
        "generated_by": "docs/refactor/v4.1.0-c/takeover/gen_verdict.py"
                        "（Task 9 实施者；控制者裁定 R69：生成器入仓，裁决文件可从已提交的树复现）",
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "generated_at_note": "本 JSON 的**生成**时刻。它**不是**判据 8 的实测时刻"
                             "（那条泳道耗时 1–2 小时，本轮未重跑；它的实测日期与树见 criteria[7].note —— "
                             "那个树身份是**推定/重构**的，不是从日志里观测到的）。",
        "repo": repo,
        "base_sha": BASE,
        "develop_tip_frozen_at_baseline": dev_tip_frozen,
        "develop_tip_note": "**冻结时点**的 develop tip（= BASE 的 parent，记于 BASELINE_FREEZE.json）。"
                            "live develop tip 会前移，实测值见 criteria[0].observed。",
        "note": note,
        "summary": {
            "total": len(crit),
            "pass": counts.get("PASS", 0),
            "fail": counts.get("FAIL", 0),
            "waived": counts.get("WAIVED", 0),
            "fail_criteria": fail_ids,
        },
        "criteria": crit,
    }
    with open(out, "w", encoding="utf-8") as f:
        json.dump(doc, f, ensure_ascii=False, indent=2)
        f.write("\n")
    print(json.dumps(doc["summary"], ensure_ascii=False, indent=2))
    print("written: %s" % out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
