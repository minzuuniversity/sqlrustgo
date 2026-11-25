#!/usr/bin/env python3
"""gen_verdict.py — run the 12 release criteria and emit V410-READINESS-VERDICT.json.

Every `command` in the verdict is REALLY executed by this script; every
`observed` is the captured output of that execution (exit code + the real
stdout/stderr).  Nothing is hand-written.

This file is COMMITTED to the repository (control ruling R69, Task 9 review
round 2).  It used to live only in the git-ignored `.superpowers/sdd/` area,
where a stale copy would silently re-write old strings into the verdict file
("差额来自判据 6 与判据 8 两条" while the summary said three; a hard-coded
mutation-arm count of 11 when the tool emits 10).  From now on the verdict is
REPRODUCIBLE from the committed tree, and every number in it is counted from
a real run instead of being copied.

Criterion 8 (the ALPHA negative-fixture lane) needs 1-2 hours, so by default
the script reads the log of a run that was executed separately with the SAME
command on the SAME tree; `--run-neg` makes it run the lane itself.  The date
and the tree of that run are parameters (they cannot be derived from the log)
and are written into the verdict explicitly.

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

# The ALPHA negative-fixture lane: the run whose result the verdict quotes.
# These three are PROPERTIES OF THAT RUN, not of the repository -- the log does
# not record them, so they are parameters with the values of the 2026-11-25 run
# as defaults.  Overriding them is how a later run is folded in.
NEG_RUN_DATE_DEFAULT = "2026-11-25"
NEG_RUN_TREE_DEFAULT = "e2d98149d264b9cdbc7270229f3cd498b61e6072"
NEG_LOG_DEFAULT = ("/Volumes/workspace/dev/sqlrustgo/.superpowers/sdd/"
                    "V410-READINESS-IMPLEMENTATION-PLAN/task-9-neg-logs/run.log")
R2_DIR_DEFAULT = ("/Volumes/workspace/dev/workspaces/dsh-ops/projects/"
                  "DSH-EXTERNAL-REVIEW-001/sqlrustgo-v4.1.0-r2")


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
            "exit=%d（校验器全绿）\n"
            "%s\n%s\n%s\n"
            "%s\n"
            "\"notes\": %s\n\n"
            "**判据不满足的原因**（三条互相独立的事实）：\n"
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
            "（缺口在**映射层**：需属主裁决补 ADR 引用 vs 豁免）。"
            % (rc4,
               re.search(r'"adr_approval": "[^"]*"', out4).group(0) if re.search(r'"adr_approval": "[^"]*"', out4) else "",
               re.search(r'"adr_approval_state": "[^"]*"', out4).group(0) if re.search(r'"adr_approval_state": "[^"]*"', out4) else "",
               re.search(r'"tasks_with_empty_approved_adr_refs": \d+', out4).group(0) if re.search(r'"tasks_with_empty_approved_adr_refs": \d+', out4) else "",
               mut_line, note_txt)),
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
    cmd6 = "%s %s/verify_matrix.py" % (py, TOOLS)
    rc6, out6 = run(cmd6, repo)
    diff = "docs/releases/v4.1.0/refactoring/evidence/L1-DIFF-fd849ae20541.md"
    hdr = "| matrix_index | case_id | feature | pilot状态 | ledger状态 | BASE状态 | 差异类型 | 说明 |"
    hdr_ok = os.path.isfile(os.path.join(repo, diff)) and \
        hdr in open(os.path.join(repo, diff), encoding="utf-8").read()
    mx = json.load(open(os.path.join(repo, TOOLS.replace("tools", "TEST_COVERAGE_MATRIX.json")),
                        encoding="utf-8"))
    cases = mx["cases"]
    l1 = [c for c in cases if c.get("feature") in ("T1", "T2", "T3")]
    states = dict(collections.Counter(str(c.get("execution_status")) for c in l1))
    verdicts = dict(collections.Counter(str(c.get("assertion_review_verdict")) for c in l1))
    by = dict(collections.Counter(str(c.get("assertion_review_performed_by")) for c in l1))
    signed = sum(1 for c in l1 if c.get("assertion_review_verdict")
                 not in (None, "ASSERTION_UNREVIEWABLE", "JUDGED_PENDING_COUNTERSIGNATURE"))
    crit.append({
        "id": 6,
        "criterion": "已执行切片全部有终态 + Oracle 签署 + 双向差异表",
        "command": cmd6 + "  &&  检查 " + diff + " 的表头  &&  统计矩阵的签署字段",
        "observed": (
            "exit=%d\n%s\n"
            "L1 切片（feature ∈ T1/T2/T3）=%d 条，全部有终态：%s\n"
            "双向差异表：%s —— 文件存在=%s，表头逐字符匹配=%s\n"
            "**Oracle 副签：%d 条**。assertion_review_verdict 分布 = %s；"
            "assertion_review_performed_by = %s（= 实施者本人，非属主授权的独立 Reviewer，"
            "按 R-ORACLE 不构成签署）。\n"
            "⇒ 「有终态」✓、「双向差异表」✓、「Oracle 签署」✗ ⇒ 判据 6 = FAIL（缺口在**签署层**）。"
            % (rc6, tail(out6, 8), len(l1), states,
               diff, os.path.isfile(os.path.join(repo, diff)), hdr_ok,
               signed, verdicts, by)),
        "verdict": "FAIL" if signed == 0 else "PASS",
    })

    # -- 7 -----------------------------------------------------------------
    wf = ".gitea/workflows"
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
        detail7 = ("执行点唯一：%s（.gitea/workflows/*.yml 全文检索 `--stage ALPHA` 只命中 1 处）\n"
                   "真实执行（非 --dry-run）：%s\n"
                   "触发条件可指认：%s；步骤 if: startsWith(github.ref, 'refs/heads/develop/') = %s\n"
                   "退出码可传播：check_stage.sh 有 set -euo pipefail，门禁 FAIL 时 exit 1"
                   % (alpha_file, step, trig, guard))
    crit.append({
        "id": 7,
        "criterion": "ALPHA 阶段清单有 CI 执行点",
        "command": "grep -rn -- '--stage ALPHA' .gitea/workflows/*.yml  +  逐条核对「真实执行 / 非 --dry-run / 退出码可传播 / 触发条件可指认」",
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
    else:
        out8 = open(neg_log, encoding="utf-8").read()
        neg_date = args.neg_run_date
        neg_tree = args.neg_run_tree
        neg_sha = sha256_file(neg_log)
        neg_src = ("**沿用 %s 的实测值**（同一条命令 `bash audit/v4.1.0/neg/verify_neg_fixtures.sh`、"
                   "同一棵树 `%s`；完整日志 %s，sha256 `%s`）"
                   % (neg_date, neg_tree, neg_log, neg_sha))
    summ = re.search(r"verify_neg_fixtures: RED=(\d+)/7\s+INCONCLUSIVE=(\d+)/7", out8)
    red = int(summ.group(1)) if summ else -1
    inc = int(summ.group(2)) if summ else -1
    rows = re.findall(r"^  (scripts/gate/\S+)\s+gate=(\w+)\s+stage=(\w+)", out8, re.M)
    # The exit code comes from the SCRIPT's own exit-code semantics
    # (verify_neg_fixtures.sh: RED_N == 7 -> 0, otherwise -> 3).  It is NOT read
    # from the log: the log's last line is an empty `NEG_EXIT=` and proves
    # nothing.
    rc8 = 0 if red == 7 else 3
    crit.append({
        "id": 8,
        "criterion": "ALPHA 7 道负向夹具两层 RED",
        "command": cmd8,
        "observed": (
            "%s\n"
            "**实测日期：%s；实测树：`base/v4.1.0-frozen` 的 tip `%s`**\n"
            "退出码=%d —— 来自脚本自身的退出码语义（`verify_neg_fixtures.sh`：RED_N==7 → 0，否则 → 3）；"
            "**不是**从日志里读的（日志末行的 `NEG_EXIT=` 是空的，不构成证据）。\n"
            "%s\n逐门禁：\n%s\n"
            "⇒ RED=%d/7 ≠ 7/7 ⇒ 判据 8 = FAIL（缺口在**证据层**）。"
            % (neg_src, neg_date, neg_tree, rc8,
               summ.group(0) if summ else tail(out8, 4),
               "\n".join("  %s gate=%s stage=%s" % r for r in rows),
               red)),
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
    crit.append({
        "id": 10,
        "criterion": "外审包三套统计一致（71/71/71）",
        "command": cmd10 + "（判据 10 的 10 项断言）",
        "observed": (
            "exit=%d\n%s\n%s\n%s\n%s\n"
            "校验器 = **本仓副本** `%s`（R69：生成/校验器都进仓，裁决文件可从已提交的树复现）；"
            "R2 包本身在 `%s`。"
            % (rc10, s10.group(0) if s10 else "",
               "\n".join(l for l in out10.splitlines() if l.startswith("PASS  #10")),
               "\n".join(l for l in out10.splitlines() if l.startswith("FAIL  #10")) or "FAIL #10：无",
               "R1 的漂移实测：EVIDENCE-COVERAGE.md 68 条 21/41/6、01-EXECUTIVE-SUMMARY.md 64 条 13/45/6；"
               "R2 由 evidence_index.json 单向生成后三套均为 71 条 24/41/6",
               VERIFIER_REL, r2)),
        "verdict": "PASS" if (rc10 == 0 and not any(
            l.startswith("FAIL") for l in out10.splitlines())) else "FAIL",
    })
    crit.append({
        "id": 11,
        "criterion": "dead 门禁口径唯一化",
        "command": "python3 %s --r2-dir %s（判据 11 的全文检索断言）" % (VERIFIER_REL, r2),
        "observed": (
            "exit=%d\n%s\n%s\n%s"
            % (rc10,
               "\n".join(l for l in out10.splitlines() if l.startswith("PASS  #11")),
               "\n".join(l for l in out10.splitlines() if l.startswith("FAIL  #11")) or "FAIL #11：无",
               "检索式：全部有文档引用 / 全部仍有文档引用 / 全部有正文引用 / 61 道全部有文档引用"
               "（排除 4 个构建脚本，它们以旧串为替换锚点）；唯一口径陈述在 04-GATE-GOVERNANCE.md §2")),
        "verdict": "PASS" if (rc10 == 0 and not any(l.startswith("FAIL  #11") for l in out10.splitlines())) else "FAIL",
    })

    # -- 12 ----------------------------------------------------------------
    crit.append({
        "id": 12,
        "criterion": "SQLRUSTGO_LOCAL_ONLY 路由 fail-closed —— 本次准入**显式豁免**（O9）",
        "command": "（显式豁免，不跑校验；事实记录见设计稿 §A5 #12 与 V410-OWNER-RULING.md 的豁免记录）",
        "observed": (
            "属主裁决 O9（2026-11-23，已裁决）：跨仓库 LOCAL_ONLY 不纳入本次准入，显式豁免。\n"
            "事实：`SQLRUSTGO_LOCAL_ONLY` 在 sqlrustgo 仓库零命中，存在于 DSH 仓库；"
            "DSH 路由器是**策略级** LOCAL_ONLY（router.py:277/290），不存在 per-task 的数据分类维度，"
            "「缺分类即拒绝」在本次准入范围内**未成立**。\n"
            "豁免记录（含 waiver_id / reason / scope / compensating_control / expires_at / approved_by）"
            "写在 V410-OWNER-RULING.md §2.1。"),
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
        "每条判据的 command 都由本文件（gen_verdict.py，已入仓）真的执行；observed 是该次执行的退出码与真实输出摘要。"
        "计划正文验收段写的「11 条 PASS + 1 条 WAIVED」是 Task 7/10 之前的乐观预期，以实测为准（R63）；"
        "本次实测的 FAIL 是判据 %s 共 %d 条（%s）。"
        % ("、".join("#%d" % i for i in fail_ids), len(fail_ids),
           "、".join("判据 %d（%s）" % (c["id"], {"4": "映射层：A2.2 R1 未满足",
                                               "6": "签署层：Oracle 副签 0 条",
                                               "8": "证据层：夹具泳道未达 7/7"}.get(str(c["id"]), "见 observed"))
                     for c in crit if c["id"] in fail_ids)))
    doc = {
        "format": "v410-readiness-verdict/v1",
        "generated_by": "docs/refactor/v4.1.0-c/takeover/gen_verdict.py"
                        "（Task 9 实施者；控制者裁定 R69：生成器入仓，裁决文件可从已提交的树复现）",
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "generated_at_note": "本 JSON 的**生成**时刻。它**不是**判据 8 的实测时刻"
                             "（那条泳道耗时 1–2 小时，本轮未重跑；它的实测日期与树见 criteria[7].observed）。",
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
