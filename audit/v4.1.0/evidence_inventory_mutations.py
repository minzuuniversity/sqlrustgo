#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""evidence_inventory 的反向变异夹具（AGENTS 2.5）。

  /usr/bin/python3 audit/v4.1.0/evidence_inventory_mutations.py

每条判据两个臂：
  有问题侧（must 非零）：造出该判据要挡的缺陷；
  撤条件侧（must 零）：把同一缺陷的条件去掉，其余一切不变。
两侧跑同一条命令（verify），只有夹具不同 ⇒ 若「撤条件侧」仍红，说明这道判据永远红（假阳性/夹具坏了）；
若「有问题侧」绿，说明判据永远绿（假阴性）。二者都判为臂不合格。

臂合格 = 三个条件同时成立（第三条是 2026-11-06 新加的）：
  (a) 有问题侧 rc != 0；(b) 撤条件侧 rc == 0；
  (c) 有问题侧的 finding 里必须出现**该臂声称要挡的那条判据名**。
`expect` 允许写成列表：臂声称挡几条判据，就必须几条都出现（m10 声称挡两条，就两条都要命中）。
只要求 (a) 时，夹具坏掉会让「别的 finding」把臂顶成红，臂看起来判别成立，实际它挡的是另一件事。
2026-11-06 实测就是这么暴露的：十臂撤条件全红，首见 finding 恒为 nonzero_run，
根因是夹具用 `python3`（homebrew 3.15，无 yaml）调被测脚本，`import yaml` 被咽掉，
所有 run 判成 invalid ⇒ 分级名一律 nonzero_run；而 m1 的 bad_ops 是空的（声称要加一条
未绑定声称却没加），m3 的假 frozen 写成 40 个 0，被 YAML 当数字吞了 ⇒ 走的是 invalid 而不是
stale_binding。三条都是夹具错，不是判据错；(c) 之后这类「挡错东西的臂」不可能再判 DISC。

夹具是临时 git 仓（跑完删），不碰真仓文档与真归档 run。
退出码：0 = 所有臂合格；1 = 有臂不合格；2 = 夹具没能跑起来（解释器/依赖），不算通过。
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
PY = sys.executable  # 被测脚本必须与本夹具同一解释器：换解释器 = 换依赖 = 换结论
RUN = "20261106-fixture-001"
DOC = "docs/CLAIMS.md"
STRONG = "本 run 的读数见 run:20261106-fixture-001：门禁结果 PASS。\n"
# 一条不含 run: 标记的声称（与 gov_probe.CLAIM_RE 同源：PASS|全绿|✅）
UNBOUND = "本次整改后全量门禁 PASS。\n"


def sh(args, cwd):
    p = subprocess.run(args, cwd=str(cwd), capture_output=True)
    return p.returncode, p.stdout.decode(errors="replace") + p.stderr.decode(errors="replace")


def inv(root, mode, extra=(), ledger="LEDGER.jsonl"):
    args = [PY, str(INV), mode, "--repo-root", str(root),
            "--docs-dir", "docs", "--runs-dir", "runs", "--ledger", ledger]
    return sh(args + list(extra), root)


def manifest(frozen, codes=(0, 0), broken=False):
    if broken:
        # YAML 语法坏 ⇒ 「读不动」，只能判 INCOMPLETE，不得换算成某种分级
        return "run_id: %s\nfrozen_commit: [unclosed\n  - %s\n" % (RUN, frozen)
    res = "\n".join("  - {id: c%d, exit_code: %d}" % (i, c) for i, c in enumerate(codes))
    cmds = "\n".join("  - {id: c%d, name: n%d, cmd: python3 evidence_inventory.py report}" % (i, i)
                     for i in range(len(codes)))
    return ("run_id: %s\nfrozen_commit: %s\nsource_agent: fixture\ncommands:\n%s\nresults:\n%s\n"
            % (RUN, frozen, cmds, res))


def build(tmp, frozen_mode="head", codes=(0, 0), doc_extra="", broken_manifest=False):
    """基线：一份文档（一条强绑定声称 + 可选未绑定声称）+ 归档 run + 由该 run 生成的台账 + 基线提交。

    台账在基线之后生成 ⇒ 基线里的声称都不算「新增」；臂新增的声称才算。
    frozen_mode: head=真祖先；dangling=真 sha 但不是 HEAD 祖先（stale_binding）；
                 zero=写成 40 个 0（YAML 会当数字 ⇒ 结构不对 invalid_manifest）。
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
    if frozen_mode == "head":
        frozen = head
    elif frozen_mode == "zero":
        frozen = "0" * 40
    else:  # dangling：造一个真 sha，再 reset 掉 ⇒ 对象存在但不是 HEAD 祖先
        sh(["git", "commit", "-q", "--allow-empty", "-m", "side"], root)
        _, sha = sh(["git", "rev-parse", "HEAD"], root)
        sh(["git", "reset", "-q", "--hard", "HEAD~1"], root)
        frozen = sha.strip()
    (root / "runs" / RUN / "manifest.yaml").write_text(manifest(frozen, codes, broken_manifest),
                                                       encoding="utf-8")
    rc, out = inv(root, "report")
    if rc != 0:
        raise SystemExit("夹具 report 失败 rc=%d：%s" % (rc, out[:400]))
    # 归档 log 必须含汇总行 + 逐文件行（台账的逐文件基线要能在 log 里逐条对上）
    (root / "runs" / RUN / "logs" / "001-report.stdout").write_text(out, encoding="utf-8")
    rc, out = inv(root, "emit-ledger",
                  ["--out", "LEDGER.jsonl", "--provenance-run", RUN,
                   "--provenance-log", "logs/001-report.stdout", "--debt-expiry", "2099-12-31"])
    if rc != 0:
        raise SystemExit("夹具 emit-ledger 失败 rc=%d：%s" % (rc, out[:400]))
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

# (name, bad_kwargs, bad_ops, good_kwargs, good_ops, today, expect, note)
# expect: 有问题侧必须出现的 finding 片段——臂必须挡它声称的那件事，不能靠别的 finding 顶红。
ARMS = [
    ("m1-unbound-new-claim",
     {}, [add_line("\n" + UNBOUND)], {}, [], "",
     "unbound-new-claim:", "台账之后新增的 PASS 声称没有 run: 标记 ⇒ 必须红（撤掉该行 ⇒ 绿）"),
    ("m2-fake-run-marker",
     {}, [add_line("\n门禁 run:20991231-nope-001 结果 PASS。\n")], {}, [], "",
     "分级 run_missing", "引用不存在的 run（格式像、无归档）⇒ 必须红"),
    ("m3-stale-run-frozen-not-ancestor",
     {"frozen_mode": "dangling"}, [], {"frozen_mode": "head"}, [], "",
     "分级 stale_binding", "run 的 frozen_commit 不是 HEAD 祖先 ⇒ 只能算 stale_binding，不得当绑定"),
    ("m4-run-has-nonzero-exit",
     {"codes": (0, 1)}, [], {"codes": (0, 0)}, [], "",
     "分级 nonzero_run", "被引用的 run 有非零退出码 ⇒ 声称不得借用它当「通过」"),
    ("m5-ledger-totals-tampered",
     {}, [patch_ledger(lambda rows: [dict(r, totals={k: 0 for k in r["totals"]}) if PROVENANCE(r) else r
                                     for r in rows])], {}, [], "",
     "ledger-totals-not-in-log", "手改台账总数（归档 log 里找不到同一行）⇒ 必须红"),
    ("m6-ledger-provenance-run-missing",
     {}, [patch_ledger(lambda rows: [dict(r, provenance_run="20991231-ghost-001") if PROVENANCE(r) else r
                                     for r in rows])], {}, [], "",
     "ledger-provenance-run-missing", "台账指名的生成 run 没有归档 ⇒ 必须红"),
    ("m7-wash-raise-file-baseline",
     {}, [patch_ledger(lambda rows: [dict(r, unbound=r["unbound"] + 5, debt=r["debt"] + 5)
                                     if FILE_ROW(r) else r for r in rows])],
     {}, [], "",
     "ledger-file-rows-not-in-log", "只抬台账里的逐文件债务：台账行与归档 log 不再逐字相同 ⇒ 必须红"),
    ("m8-missing-ledger-must-not-be-green",
     {}, [lambda root: (root / "LEDGER.jsonl").write_text("", encoding="utf-8")], {}, [], "",
     "INCOMPLETE", "台账读不到 ⇒ 必须 INCOMPLETE(2)，不许当通过(0)"),
    ("m9-debt-expiry-lapsed",
     {"doc_extra": "\n历史债务声称 PASS（无 run 引用）。\n"},
     [patch_ledger(lambda rows: [dict(r, debt_expiry_date="2020-01-01") if PROVENANCE(r) else r
                                 for r in rows])],
     {"doc_extra": "\n历史债务声称 PASS（无 run 引用）。\n"}, [], "2026-11-06",
     "stale_debt:", "债务到期日已过仍未清零 ⇒ stale_debt 必须红（到期日放回 2099 ⇒ 绿）"),
    ("m10-m7-variant-claim-plus-raised-baseline",
     {"doc_extra": ""},
     [patch_ledger(lambda rows: [dict(r, unbound=r["unbound"] + 5, debt=r["debt"] + 5)
                                 if FILE_ROW(r) else r for r in rows]),
      add_line("\n" + UNBOUND)],
     {}, [], "",
     ["ledger-file-rows-not-in-log", "unbound-new-claim:"],
     "同一次改动里既加未绑定声称又抬逐文件基线 ⇒ 两条判据都必须红"),
    ("m11-invalid-manifest-frozen-not-hex",
     {"frozen_mode": "zero"}, [], {"frozen_mode": "head"}, [], "",
     "invalid_manifest:", "frozen_commit 写成 40 个 0（YAML 当数字）⇒ 结构不对，必须红且不得冒充 nonzero_run"),
    ("m12-unread-manifest-incomplete",
     {"broken_manifest": True}, [], {"broken_manifest": False}, [], "",
     "INCOMPLETE-coverage", "归档 manifest 语法坏（读不动）⇒ 只能 INCOMPLETE(2)，不得换算成某个分级"),
]


def main():
    if not shutil.which(PY):
        print("INCOMPLETE: 夹具找不到解释器 " + PY + " —— 没能核对，不算通过")
        return 2
    rc, out = sh([PY, "-c", "import yaml"], REPO)
    if rc != 0:
        print("INCOMPLETE: 解释器 %s 没有 yaml（%s）—— 分级器读不动归档，夹具结论无意义"
              % (PY, out.strip()[:120]))
        return 2
    arms = ok = 0
    bads = []
    for name, bk, bops, gk, gops, today, expect, note in ARMS:
        arms += 1
        rc_b, rc_g, out_b, out_g = run_arm(bk, bops, gk, gops, today)
        red_when_bad = rc_b != 0
        green_when_cleared = rc_g == 0
        needs = (expect,) if isinstance(expect, str) else tuple(expect)
        hits_expected = all(e in out_b for e in needs)
        if red_when_bad and green_when_cleared and hits_expected:
            ok += 1
            tag = "DISC"
        else:
            tag = "NOT-DISC"
            bads.append(name)
        first = next((l for l in out_b.splitlines()
                      if l.startswith("NOT-OK") or l.startswith("INCOMPLETE")), out_b.strip()[:120])
        print("%s %s: 有问题 rc=%d(应非零) 撤条件 rc=%d(应零) 期望判据 %r 命中=%s 挡的是=%s"
              % (tag, name, rc_b, rc_g, expect, hits_expected, note))
        print("     首见: %s" % first[:190])
        if not hits_expected:
            miss = [e for e in needs if e not in out_b]
            print("     有问题侧缺期望判据 " + repr(miss) + " ⇒ 臂挡的是别的事（或判据缺失）："
                  + out_b.strip()[:300])
        if not green_when_cleared:
            print("     撤条件侧仍非零 ⇒ 夹具或判据有问题：" + out_g.strip()[:300])
    print("arms=%d discriminating=%d not_discriminating=%d drift=%d"
          % (arms, ok, len(bads), 0 if not bads else 1))
    return 0 if ok == arms else 1


if __name__ == "__main__":
    raise SystemExit(main())
