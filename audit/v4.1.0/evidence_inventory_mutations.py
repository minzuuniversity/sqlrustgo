#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""evidence_inventory 的反向变异夹具（AGENTS 2.5）。

  python3 audit/v4.1.0/evidence_inventory_mutations.py

每条判据两个臂：
  有问题侧（must 非零）：造出该判据要挡的缺陷；
  撤条件侧（must 零）：把同一缺陷的条件去掉，其余一切不变。
两侧跑同一条命令（verify），只有夹具不同 ⇒ 若「撤条件侧」仍红，说明这道判据永远红（假阳性/夹具坏了）；
若「有问题侧」绿，说明判据永远绿（假阴性）。二者都判为臂不合格。
夹具是临时 git 仓（跑完删），不碰真仓文档与真归档 run。
退出码：0 = 所有臂合格；1 = 有臂不合格。
"""
from __future__ import annotations

import json
import pathlib
import shutil
import subprocess
import sys
import tempfile

REPO = pathlib.Path(__file__).resolve().parents[2]
INV = REPO / "audit" / "v4.1.0" / "evidence_inventory.py"
RUN = "20261106-fixture-001"
DOC = "docs/CLAIMS.md"
STRONG = "本 run 的读数见 run:20261106-fixture-001：门禁结果 PASS。\n"


def sh(args, cwd):
    p = subprocess.run(args, cwd=str(cwd), capture_output=True)
    return p.returncode, p.stdout.decode(errors="replace") + p.stderr.decode(errors="replace")


def inv(root, mode, extra=(), ledger="LEDGER.jsonl"):
    args = ["python3", str(INV), mode, "--repo-root", str(root),
            "--docs-dir", "docs", "--runs-dir", "runs", "--ledger", ledger]
    return sh(args + list(extra), root)


def manifest(frozen, codes=(0, 0)):
    res = "\n".join("  - {id: c%d, exit_code: %d}" % (i, c) for i, c in enumerate(codes))
    cmds = "\n".join("  - {id: c%d, name: n%d, cmd: python3 evidence_inventory.py report}" % (i, i)
                     for i in range(len(codes)))
    return ("run_id: %s\nfrozen_commit: %s\nsource_agent: fixture\ncommands:\n%s\nresults:\n%s\n"
            % (RUN, frozen, cmds, res))


def build(tmp, frozen_ancestor=True, codes=(0, 0), doc_extra=""):
    """基线：一份文档（一条强绑定声称 + 可选未绑定声称）+ 归档 run + 由该 run 生成的台账 + 基线提交。

    台账在基线之后生成 ⇒ 基线里的声称都不算「新增」；臂新增的声称才算。
    """
    root = pathlib.Path(tmp)
    (root / "docs").mkdir(parents=True, exist_ok=True)
    (root / "runs" / RUN / "logs").mkdir(parents=True, exist_ok=True)
    (root / ".gitignore").write_text("LEDGER.jsonl\n", encoding="utf-8")
    (root / DOC).write_text("# 声称样本\n\n" + STRONG + doc_extra, encoding="utf-8")
    sh(["git", "init", "-q"], root)
    sh(["git", "config", "user.name", "fixture"], root)
    sh(["git", "config", "user.email", "fixture@example.invalid"], root)
    sh(["git", "add", "-A"], root)
    sh(["git", "commit", "-qm", "base"], root)
    _, head = sh(["git", "rev-parse", "HEAD"], root)
    head = head.strip()
    frozen = head if frozen_ancestor else "0" * 40
    (root / "runs" / RUN / "manifest.yaml").write_text(manifest(frozen, codes), encoding="utf-8")
    rc, out = inv(root, "report")
    if rc != 0:
        raise SystemExit("夹具 report 失败：" + out[:400])
    # 归档 log 必须含汇总行 + 逐文件行（台账的逐文件基线要能在 log 里逐条对上）
    (root / "runs" / RUN / "logs" / "001-report.stdout").write_text(out, encoding="utf-8")
    rc, out = inv(root, "emit-ledger",
                  ["--out", "LEDGER.jsonl", "--provenance-run", RUN,
                   "--provenance-log", "logs/001-report.stdout", "--debt-expiry", "2099-12-31"])
    if rc != 0:
        raise SystemExit("夹具 emit-ledger 失败：" + out[:400])
    return root, head


def patch_ledger(fn):
    def f(root):
        p = root / "LEDGER.jsonl"
        rows = [json.loads(l) for l in p.read_text(encoding="utf-8").splitlines() if l.strip()]
        p.write_text("\n".join(json.dumps(r, ensure_ascii=False) for r in fn(rows)) + "\n", encoding="utf-8")
    return f


def add_line(text):
    def f(root):
        p = root / DOC
        p.write_text(p.read_text(encoding="utf-8") + text, encoding="utf-8")
    return f


def touch_bad_only(root, *fs):
    for f in fs:
        f(root)
    sh(["git", "add", "-A"], root)
    sh(["git", "commit", "-qm", "arm"], root)


def run_arm(bad_kwargs, bad_ops, good_kwargs, good_ops, today):
    bad = tempfile.mkdtemp(prefix="evidinv-bad-")
    good = tempfile.mkdtemp(prefix="evidinv-good-")
    try:
        rb, base_b = build(bad, **bad_kwargs)
        rg, base_g = build(good, **good_kwargs)
        touch_bad_only(rb, *bad_ops)
        touch_bad_only(rg, *good_ops)
        extra_b = ["--diff-base", base_b] + (["--today", today] if today else [])
        extra_g = ["--diff-base", base_g] + (["--today", today] if today else [])
        rc_b, out_b = inv(rb, "verify", extra_b)
        rc_g, out_g = inv(rg, "verify", extra_g)
        return rc_b, rc_g, out_b, out_g
    finally:
        shutil.rmtree(bad, ignore_errors=True)
        shutil.rmtree(good, ignore_errors=True)


PROVENANCE = lambda r: r.get("kind") == "provenance"  # noqa: E731
FILE_ROW = lambda r: r.get("kind") == "file"          # noqa: E731

ARMS = [
    ("m1-unbound-new-claim",
     {}, [], {}, [], "",
     "台账之后新增的 PASS 声称没有 run: 标记 ⇒ 必须红（撤掉该行 ⇒ 绿）"),
    ("m2-fake-run-marker",
     {}, [add_line("\n门禁 run:20991231-nope-001 结果 PASS。\n")], {}, [], "",
     "引用不存在的 run（格式像、无归档）⇒ 必须红"),
    ("m3-stale-run-frozen-not-ancestor",
     {"frozen_ancestor": False}, [], {"frozen_ancestor": True}, [], "",
     "run 的 frozen_commit 不是 HEAD 祖先 ⇒ 只能算 stale_binding，不得当绑定"),
    ("m4-run-has-nonzero-exit",
     {"codes": (0, 1)}, [], {"codes": (0, 0)}, [], "",
     "被引用的 run 有非零退出码 ⇒ 声称不得借用它当「通过」"),
    ("m5-ledger-totals-tampered",
     {}, [patch_ledger(lambda rows: [dict(r, totals={k: 0 for k in r["totals"]}) if PROVENANCE(r) else r
                                     for r in rows])], {}, [], "",
     "手改台账总数（归档 log 里找不到同一行）⇒ 必须红"),
    ("m6-ledger-provenance-run-missing",
     {}, [patch_ledger(lambda rows: [dict(r, provenance_run="20991231-ghost-001") if PROVENANCE(r) else r
                                     for r in rows])], {}, [], "",
     "台账指名的生成 run 没有归档 ⇒ 必须红"),
    ("m7-wash-raise-file-baseline-and-add-claim",
     {}, [patch_ledger(lambda rows: [dict(r, unbound=r["unbound"] + 5, debt=r["debt"] + 5)
                                     if FILE_ROW(r) else r for r in rows])],
     {}, [], "",
     "只抬台账里的逐文件债务：台账行与归档 log 不再逐字相同 ⇒ 必须红"),
    ("m8-missing-ledger-must-not-be-green",
     {}, [lambda root: (root / "LEDGER.jsonl").write_text("", encoding="utf-8")], {}, [], "",
     "台账读不到 ⇒ 必须 INCOMPLETE(2)，不许当通过(0)"),
    ("m9-debt-expiry-lapsed",
     {"doc_extra": "\n历史债务声称 PASS（无 run 引用）。\n"},
     [patch_ledger(lambda rows: [dict(r, debt_expiry_date="2020-01-01") if PROVENANCE(r) else r
                                 for r in rows])],
     {"doc_extra": "\n历史债务声称 PASS（无 run 引用）。\n"}, [], "2026-11-06",
     "债务到期日已过仍未清零 ⇒ stale_debt 必须红（到期日放回 2099 ⇒ 绿）"),
    ("m10-m7-variant-claim-plus-raised-baseline",
     {"doc_extra": ""},
     [patch_ledger(lambda rows: [dict(r, unbound=r["unbound"] + 5, debt=r["debt"] + 5)
                                 if FILE_ROW(r) else r for r in rows]),
      add_line("\n新增未绑定 PASS 声称（同一次改动里抬高了逐文件基线）。\n")],
     {}, [], "",
     "同一次改动里既加未绑定声称又抬逐文件基线 ⇒ 台账行与 log 不符 + 新增未绑定 两条都必须红"),
]


def main():
    arms = ok = 0
    bads = []
    for name, bk, bops, gk, gops, today, note in ARMS:
        arms += 1
        rc_b, rc_g, out_b, out_g = run_arm(bk, bops, gk, gops, today)
        red_when_bad, green_when_cleared = (rc_b != 0), (rc_g == 0)
        if red_when_bad and green_when_cleared:
            ok += 1
            tag = "DISC"
        else:
            tag = "NOT-DISC"
            bads.append(name)
        first = next((l for l in out_b.splitlines()
                      if l.startswith("NOT-OK") or l.startswith("INCOMPLETE")), out_b.strip()[:120])
        print("%s %s: 有问题 rc=%d(应非零) 撤条件 rc=%d(应零) 挡的是=%s" % (tag, name, rc_b, rc_g, note))
        print("     首见: %s" % first[:170])
        if not green_when_cleared:
            print("     撤条件侧仍非零 ⇒ 夹具或判据有问题：" + out_g.strip()[:300])
    print("arms=%d discriminating=%d not_discriminating=%d drift=%d"
          % (arms, ok, len(bads), 0 if not bads else 1))
    return 0 if ok == arms else 1


if __name__ == "__main__":
    raise SystemExit(main())
