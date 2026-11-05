#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""evidence_inventory.py —— 把文档里的「声称」按归档 run 分级（P1-5，探针，不是门禁）。

为什么要它：`docs/releases/v4.1.0/*.md` 里 1144 处 `PASS/全绿/✅`（gov_probe.metrics
claims_hits）此前**没有任何一处**指回归档证据（claims_ref=0）。这正是「报告好看、一用就崩」
的直接机制（看板 Q3）。本工具不新增门禁、不改门禁清单（分母仍 168），只把「声称」变成
可分级、可单调收敛、可反向变异的台账。

设计取舍经两系专家复核（run 里归档了原文）：
  * 上下文（同块内出现 run id）只能算**线索**，不算绑定 ⇒ 单列 `ambiguous`，不得计入绑定数；
    真绑定必须在声明**同一行/同一表格行**上有显式 `run:<id>` 标记（strong）。
  * 历史文档不逐份补标记（成本爆炸）⇒ 用**单调台账**记账：现场债务 ≤ 台账债务；
    台账之后**新增/改写**的声称必须 strong，否则红。
  * 台账只能由归档 run 生成：台账声明的总数必须能在该 run 的归档 log 里逐字找到，
    否则「手改台账洗白」判红。
  * 债务必须有到期日（台账字段 `debt_expiry_date`）：过期后仍带债务的文档记 `stale_debt` 判红。

分级（每个声称 occurrence 一条）：
  strong_bound      同一行有 `run:<id>`，且该 run 归档存在、results 全 exit_code=0
  nonzero_run       同上但 run 里有非零退出码（声称不得借用它当「通过」）
  run_missing       同一行引用了不存在的 run id（假绑定，判红级）
  stale_binding     同一行有显式标记，但 run 的 frozen_commit 不是 HEAD 的祖先（证据只「曾经为真」）
  invalid_manifest  引用到的 run 归档存在，但 manifest 结构不对（缺 run_id/frozen_commit/results、
                    frozen_commit 不是十六进制串）—— 这是**确凿的**缺陷，判红
  unread_manifest   归档 manifest 存在但**读不动**（不是结构错，是读不了：如解释器缺 yaml、YAML 语法坏）
                    ⇒ 这一档**既不算绑定也不算债务**，只能把整体判成 INCOMPLETE(2)
  ambiguous         同块（连续非空行）内出现 run id，但本行没有显式标记
  unbound           连线索都没有
绑定有效性（挂在 strong/nonzero 上，不改变分级名）：
  stale_binding     run 的 frozen_commit 不是当前 HEAD 的祖先（证据不覆盖现状）
  irrelevant_run    声称所在行/块点名了某个门禁或工具，但该 run 的 commands 里没有它
  invalid_manifest  run 的 manifest 缺 run_id/frozen_commit/results 字段

用法：
  python3 audit/v4.1.0/evidence_inventory.py report
  python3 audit/v4.1.0/evidence_inventory.py emit-ledger --out <path> --provenance-run <run_id> \
         --provenance-log <run 内 log 相对路径> [--debt-expiry 2026-12-31]
  python3 audit/v4.1.0/evidence_inventory.py verify [--ledger <path>] [--diff-base <ref>]
退出码：0 无问题 / 1 有 finding / 2 INCOMPLETE（没能核对，不算通过）。
  解释器读不动任何归档 manifest（典型：homebrew python3 没有 yaml）⇒ **一律 2**，
  且直接指出该用 /usr/bin/python3。上一版在这里出过事：`import yaml` 失败被咽掉，
  所有 run 都判成 invalid ⇒ 分级名变成 nonzero_run，十臂反向变异全红，
  看起来像「判据不成立」，实际是解释器缺包。读不动必须显式暴露，不许换算成判定。
反向变异：python3 audit/v4.1.0/evidence_inventory_mutations.py
"""
from __future__ import annotations

import argparse
import datetime
import importlib.util
import json
import pathlib
import re
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[1]
DEFAULT_DOCS = "docs/releases/v4.1.0"
DEFAULT_RUNS = "audit/v4.1.0/runs"
DEFAULT_LEDGER = "docs/releases/v4.1.0/CLAIM_BINDINGS_LEDGER.jsonl"
RUNID_RE = re.compile(r"20\d{6}-[a-z0-9]+-\d{3}")
HEXSHA_RE = re.compile(r"[0-9a-f]{7,40}")
# yaml 只在这里探一次：读不动归档就必须显式 INCOMPLETE，不能把「读不了」翻译成某种分级。
try:
    import yaml  # noqa: F401
    YAML_OK, YAML_WHY = True, ""
except Exception as _e:  # noqa: BLE001
    YAML_OK, YAML_WHY = False, type(_e).__name__
STRONG_RE = re.compile(r"run[:：]\s*(20\d{6}-[a-z0-9]+-\d{3})")
TOKEN_RE = re.compile(r"(check_[A-Za-z0-9_.\-]+|[A-Za-z0-9_]+\.py)")
KEYS = ("strong_bound", "nonzero_run", "run_missing", "stale_binding",
        "invalid_manifest", "unread_manifest", "ambiguous", "unbound")
# 债务口径只有一处定义：台账、report、单调比较、到期判定共用，避免四份写法各自漂。
DEBT_KEYS = ("run_missing", "stale_binding", "invalid_manifest", "ambiguous", "unbound")
# 确凿的假绑定/借证：当场判红（unread_manifest 不在这里，它只能判 INCOMPLETE）
UNSAFE_GRADES = ("run_missing", "stale_binding", "nonzero_run", "invalid_manifest")


def claim_re():
    """声称判据引用 gov_probe 的实现（SSOT），加载失败 ⇒ INCOMPLETE，不许自己另写一条正则。"""
    spec = importlib.util.spec_from_file_location("gov_probe", REPO / "scripts" / "gate" / "gov_probe.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod.CLAIM_RE


def sh(args, cwd):
    p = subprocess.run(args, cwd=str(cwd), capture_output=True)
    return p.returncode, p.stdout.decode(errors="replace"), p.stderr.decode(errors="replace")


class Runs:
    """归档 run 的只读视图：存在性 / 退出码 / frozen_commit / commands。"""

    def __init__(self, runs_dir: pathlib.Path, repo: pathlib.Path):
        self.dir, self.repo = runs_dir, repo
        self._cache, self._anc = {}, {}

    def load(self, rid):
        if rid in self._cache:
            return self._cache[rid]
        man = self.dir / rid / "manifest.yaml"
        rec = {"exists": man.is_file(), "valid": False, "unread": "", "why": "",
               "clean": False, "frozen": "", "cmds": []}
        if rec["exists"]:
            if not YAML_OK:
                rec["unread"] = "import yaml: " + YAML_WHY
                self._cache[rid] = rec
                return rec
            try:
                d = yaml.safe_load(man.read_text(encoding="utf-8")) or {}
            except Exception as e:  # noqa: BLE001 — 读不动就是读不动，不猜、也不换算成判定
                rec["unread"] = type(e).__name__
                self._cache[rid] = rec
                return rec
            res = d.get("results") or []
            frozen = d.get("frozen_commit")
            if d.get("run_id") != rid:
                rec["why"] = "run_id 与目录名不一致"
            elif not isinstance(frozen, str) or not HEXSHA_RE.fullmatch(frozen):
                # YAML 1.1 会把 0000...0 这类串当数字解析（前导零被吞），必须显式挡住
                rec["why"] = "frozen_commit 不是十六进制串（实际值 %r）" % (frozen,)
            elif not isinstance(res, list) or not res:
                rec["why"] = "results 缺失或为空"
            else:
                rec["valid"] = True
                rec["clean"] = all(int(r.get("exit_code", 1)) == 0 for r in res)
                rec["frozen"] = frozen
                rec["cmds"] = [(str(c.get("id", "")) + " " + str(c.get("name", "")) + " " + str(c.get("cmd", "")))
                               for c in (d.get("commands") or [])]
        self._cache[rid] = rec
        return rec

    def ancestor(self, sha):
        if not sha:
            return False
        if sha in self._anc:
            return self._anc[sha]
        rc, _, _ = sh(["git", "merge-base", "--is-ancestor", sha, "HEAD"], self.repo)
        self._anc[sha] = (rc == 0)
        return self._anc[sha]


def grade(docs_dir: pathlib.Path, runs: Runs, repo: pathlib.Path):
    per_file, notes, unsafe = {}, {}, []
    for doc in sorted(docs_dir.glob("*.md")):
        rel = str(doc.relative_to(repo))
        lines = doc.read_text(encoding="utf-8", errors="replace").splitlines()
        rows = []
        for idx, line in enumerate(lines):
            hits = len(claim_re_re.findall(line))
            if not hits:
                continue
            marks = STRONG_RE.findall(line)
            block = _block(lines, idx)
            clues = [r for r in RUNID_RE.findall(block) if r not in marks]
            grades, flags = [], []
            for n in range(hits):
                rid = marks[n] if n < len(marks) else (None if not marks else marks[-1])
                if rid:
                    rec = runs.load(rid)
                    if not rec["exists"]:
                        g = "run_missing"
                    elif rec["unread"]:
                        flags.append("unread_manifest:" + rid + ":" + rec["unread"])
                        g = "unread_manifest"
                    elif not rec["valid"]:
                        flags.append("invalid_manifest:" + rid + ":" + rec["why"])
                        g = "invalid_manifest"
                    elif rec["clean"]:
                        # 有效性的最后一道：证据必须覆盖当前树，否则它只是「曾经为真」，不能算绑定
                        g = "strong_bound" if runs.ancestor(rec["frozen"]) else "stale_binding"
                        if g == "stale_binding":
                            flags.append("stale_binding:" + rid)
                    else:
                        g = "nonzero_run"
                    if rec["exists"] and rec["valid"]:
                        toks = TOKEN_RE.findall(line) + TOKEN_RE.findall(block)
                        if toks and not any(t in c for c in rec["cmds"] for t in set(toks)):
                            flags.append("irrelevant_run:" + rid)
                elif clues and n == 0:
                    g = "ambiguous"
                else:
                    g = "unbound"
                grades.append(g)
            if any(g in UNSAFE_GRADES for g in grades):
                unsafe.append((rel, idx + 1, "/".join(sorted(set(grades))), line.strip()[:100]))
            rows.append((idx + 1, grades, flags))
        cnt = {k: 0 for k in KEYS}
        for _, grades, flags in rows:
            for g in grades:
                cnt[g] += 1
            if flags:
                notes[rel] = sorted(set(flags))
        if sum(cnt.values()):
            per_file[rel] = cnt
    return per_file, notes, unsafe


def _block(lines, idx):
    lo = idx
    while lo > 0 and lines[lo - 1].strip():
        lo -= 1
    hi = idx
    while hi + 1 < len(lines) and lines[hi + 1].strip():
        hi += 1
    return "\n".join(lines[lo:hi + 1])


def summary(per_file):
    tot = {k: 0 for k in KEYS}
    for cnt in per_file.values():
        for k, v in cnt.items():
            tot[k] += v
    tot["files"] = len(per_file)
    tot["total"] = sum(tot[k] for k in KEYS)
    tot["debt"] = sum(tot[k] for k in DEBT_KEYS)
    tot["claims_ref"] = tot["strong_bound"]
    return tot


def file_row(f, cnt):
    """台账与 report 共用的行形状（台账的每一行必须能在归档 log 里逐字找到）。"""
    row = {"kind": "file", "file": f}
    row.update(cnt)
    row["debt"] = sum(cnt.get(k, 0) for k in DEBT_KEYS)
    return row


def emit_ledger(out_path, per_file, args, repo):
    rc, head, _ = sh(["git", "rev-parse", "--short", "HEAD"], repo)
    man = repo / DEFAULT_RUNS / args.provenance_run / "manifest.yaml"
    frozen = ""
    if man.is_file():
        for ln in man.read_text(encoding="utf-8").splitlines():
            if ln.startswith("frozen_commit:"):
                frozen = ln.split(":", 1)[1].strip()
    head_row = {"kind": "provenance", "schema": 1,
                "generated_at": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
                "generated_by_agent": args.agent, "provenance_run": args.provenance_run,
                "provenance_log": args.provenance_log, "ledger_head": head, "frozen_commit": frozen,
                "debt_expiry_date": args.debt_expiry, "totals": summary(per_file)}
    lines = [json.dumps(head_row, ensure_ascii=False)]
    for f in sorted(per_file):
        lines.append(json.dumps(file_row(f, per_file[f]), ensure_ascii=False))
    pathlib.Path(out_path).write_text("\n".join(lines) + "\n", encoding="utf-8")
    return head_row


def read_ledger(path):
    if not pathlib.Path(path).is_file():
        return None, []
    head, files = None, {}
    for ln in pathlib.Path(path).read_text(encoding="utf-8").splitlines():
        if not ln.strip():
            continue
        d = json.loads(ln)
        if d.get("kind") == "provenance":
            head = d
        elif d.get("kind") == "file":
            files[d["file"]] = d
    return head, files


def cmd_verify(args):
    repo = pathlib.Path(args.repo_root).resolve()
    docs = repo / args.docs_dir
    runs = Runs(repo / args.runs_dir, repo)
    global claim_re_re
    try:
        claim_re_re = claim_re()
    except Exception as e:  # noqa: BLE001
        print("INCOMPLETE: 声称判据（gov_probe.CLAIM_RE）加载失败 " + type(e).__name__ + " —— 没能核对，不算通过")
        return 2
    if not docs.is_dir():
        print("INCOMPLETE: 文档目录不在 " + str(docs) + " —— 没能核对")
        return 2
    per_file, notes, unsafe = grade(docs, runs, repo)
    tot = summary(per_file)
    findings = []

    head, base = read_ledger(args.ledger)
    if head is None:
        print("INCOMPLETE: 台账不在（" + args.ledger + "）—— 没能核对，不算通过")
        return 2
    # 台账必须由归档 run 生成，且其总数必须在该 run 的归档 log 里逐字存在
    man = repo / args.runs_dir / str(head.get("provenance_run")) / "manifest.yaml"
    if not man.is_file():
        findings.append("ledger-provenance-run-missing: 台账声称由 "
                        + str(head.get("provenance_run")) + " 生成，但该归档 run 不存在")
    else:
        log = repo / args.runs_dir / str(head["provenance_run"]) / str(head.get("provenance_log"))
        if not log.is_file():
            findings.append("ledger-provenance-log-missing: 台账指名的 log 不在 " + str(log))
        else:
            logged = log_records(log)
            if logged["summary"] is None:
                findings.append("ledger-provenance-log-no-summary: 指名的 log 里没有 evidence_inventory 汇总行"
                                " ⇒ 台账不是这台机器从该 run 的读数生成的")
            else:
                diff = [k for k, v in (head.get("totals") or {}).items()
                        if k not in ("files", "total", "debt", "claims_ref") and logged["summary"].get(k) != v]
                if diff:
                    findings.append("ledger-totals-not-in-log: 台账总数与归档 log 不符 "
                                    + ",".join("%s 台账 %s / log %s" % (k, (head["totals"])[k], logged["summary"][k])
                                               for k in diff))
                bad_files = [f for f, row in sorted(base.items())
                             if logged["files"].get(f) is None
                             or any(logged["files"][f].get(k) != int(row.get(k, 0)) for k in KEYS)]
                if bad_files:
                    findings.append("ledger-file-rows-not-in-log: %d 份文档的台账行与归档 log 不符（第一个 %s）"
                                    % (len(bad_files), bad_files[0]))
    # 错误绑定（假 run / 证据不覆盖当前树 / run 里有非零退出码）当场判红，不等「是不是新增」
    for f, ln, gs, text in unsafe:
        findings.append("unsafe-binding:%s:%d 分级 %s：%s" % (f, ln, gs, text))
    # 单调：现场债务不得高于台账债务（逐文件 + 总量）
    for f, row in sorted(base.items()):
        now = per_file.get(f, {})
        nd = sum(now.get(k, 0) for k in DEBT_KEYS)
        if nd > int(row.get("debt", 0)):
            findings.append("debt-grew:%s 台账 %d → 现场 %d（未绑定债务只准减不准增）" % (f, int(row.get("debt", 0)), nd))
    if tot["debt"] > sum(int(r.get("debt", 0)) for r in base.values()):
        findings.append("total-debt-grew: 台账 %d → 现场 %d"
                        % (sum(int(r.get("debt", 0)) for r in base.values()), tot["debt"]))
    # 台账之后新增/改写的声称必须 strong（用 git diff 现场判定，不靠「是不是新文档」的说法）
    diff_base = args.diff_base or str(head.get("frozen_commit") or "")
    new_claims = diff_claims(repo, docs, diff_base) if diff_base else None
    if new_claims is None:
        findings.append("INCOMPLETE-diff: 无法用 git diff 界定新增声称（diff-base=" + diff_base + "）⇒ 新增未绑定声明这条判据没能核对")
    else:
        for f, ln, text in new_claims:
            if not STRONG_RE.search(text):
                findings.append("unbound-new-claim:%s:%d %s" % (f, ln, text.strip()[:80]))
            elif not runs.load(STRONG_RE.search(text).group(1)).get("clean"):
                findings.append("bound-but-unclean:%s:%d 指名的 run 不存在或有非零退出码" % (f, ln))
    # 债务到期：过期仍带债务 ⇒ stale_debt（让历史负债失效，而不是永远合法）
    today = args.today or datetime.date.today().isoformat()
    expiry = str(head.get("debt_expiry_date") or "")
    if expiry and today > expiry:
        for f, row in sorted(per_file.items()):
            if any(row.get(k, 0) for k in DEBT_KEYS):
                findings.append("stale_debt:%s 债务到期日 %s 已过（%s）仍未清零" % (f, expiry, today))

    unread = sorted({fl for fs in notes.values() for fl in fs if fl.startswith("unread_manifest:")})
    out = {"kind": "evidence_inventory", **tot, "mode": "verify", "ledger": args.ledger,
           "diff_base": diff_base, "today": today, "binding_validity": notes}
    print(json.dumps(out, ensure_ascii=False))
    for f in findings:
        print("NOT-OK:", f)
    if unread:
        # 覆盖不全：这些 run 读不动，绑定有效性**没核对**，既不算红也不算绿（AGENTS 2.3）
        print("INCOMPLETE-coverage: %d 个归档 manifest 读不动 ⇒ 绑定有效性没能核对" % len(unread))
        for u in unread[:12]:
            print("  ", u)
        print("findings=%d unread=%d" % (len(findings), len(unread)))
        return 2
    if findings:
        print("findings=%d" % len(findings))
        return 1
    print("findings=0")
    return 0


def log_records(log_path):
    """读归档 log 里的 evidence_inventory 汇总行与逐文件行（语义比对，不靠逐字串）。

    台账必须由某次归档 run 的读数生成：台账里的每个分级计数都必须能在该 run 的 log 里
    找到同值。否则「手改台账」就能把债务基线抬上去洗白。
    """
    rec = {"summary": None, "files": {}}
    for ln in log_path.read_text(encoding="utf-8", errors="replace").splitlines():
        ln = ln.strip()
        if not ln.startswith("{"):
            continue
        try:
            d = json.loads(ln)
        except ValueError:
            continue
        if d.get("kind") == "evidence_inventory":
            rec["summary"] = d
        elif d.get("kind") == "file" and d.get("file"):
            rec["files"][d["file"]] = d
    return rec


def diff_claims(repo, docs, base):
    rc, out, err = sh(["git", "diff", "--unified=0", base + "..HEAD", "--", str(docs.relative_to(repo))], repo)
    if rc != 0:
        return None
    rows, cur = [], None
    for ln in out.splitlines():
        if ln.startswith("+++ b/"):
            cur = ln[6:]
        elif ln.startswith("+") and not ln.startswith("+++"):
            text = ln[1:]
            if claim_re_re.search(text):
                rows.append((cur, 0, text))
    return rows


def main():
    if not YAML_OK:
        # 读不动归档时「跑完给个数」是最坏的失败方式：数字看着齐全，分级其实全是猜的。
        print("INCOMPLETE: 当前解释器读不动归档 manifest（import yaml ⇒ %s）—— 没能核对，不算通过；"
              "改用 /usr/bin/python3 重跑" % YAML_WHY)
        return 2
    ap = argparse.ArgumentParser()
    ap.add_argument("mode", choices=["report", "emit-ledger", "verify"])
    ap.add_argument("--repo-root", default=str(REPO))
    ap.add_argument("--docs-dir", default=DEFAULT_DOCS)
    ap.add_argument("--runs-dir", default=DEFAULT_RUNS)
    ap.add_argument("--ledger", default=DEFAULT_LEDGER)
    ap.add_argument("--diff-base", default="")
    ap.add_argument("--today", default="")
    ap.add_argument("--out", default="")
    ap.add_argument("--provenance-run", default="")
    ap.add_argument("--provenance-log", default="")
    ap.add_argument("--debt-expiry", default="")
    ap.add_argument("--agent", default="mcode-macmini")
    args = ap.parse_args()

    global claim_re_re
    try:
        claim_re_re = claim_re()
    except Exception as e:  # noqa: BLE001
        print("INCOMPLETE: 声称判据加载失败 " + type(e).__name__)
        return 2
    repo = pathlib.Path(args.repo_root).resolve()
    docs = repo / args.docs_dir
    if not docs.is_dir():
        print("INCOMPLETE: 文档目录不在 " + str(docs))
        return 2
    runs = Runs(repo / args.runs_dir, repo)
    per_file, notes, unsafe = grade(docs, runs, repo)
    tot = summary(per_file)

    if args.mode == "report":
        print(json.dumps({"kind": "evidence_inventory", **tot, "mode": "report",
                          "binding_validity": notes}, ensure_ascii=False))
        for f in sorted(per_file):
            print(json.dumps(file_row(f, per_file[f]), ensure_ascii=False))
        return 0
    if args.mode == "emit-ledger":
        if not args.out or not args.provenance_run or not args.provenance_log:
            print("INCOMPLETE: emit-ledger 需要 --out/--provenance-run/--provenance-log")
            return 2
        head = emit_ledger(args.out, per_file, args, repo)
        print(json.dumps({"kind": "ledger_written", "path": args.out, "rows": len(per_file),
                          "provenance_run": head["provenance_run"]}, ensure_ascii=False))
        return 0
    return cmd_verify(args)


if __name__ == "__main__":
    raise SystemExit(main())
