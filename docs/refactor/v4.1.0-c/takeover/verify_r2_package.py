#!/usr/bin/env python3
"""verify_r2_package.py — the machine check for A5 #10 and A5 #11 on the R2 package.

It answers, with measured values (never with expectations):

  #10  三套统计一致：EVIDENCE-INDEX.json / EVIDENCE-COVERAGE.md /
       01-EXECUTIVE-SUMMARY.md must state the same evidence count and the same
       grade distribution, and all three must be derivable from
       evidence_index.json (the single source of truth).
  #11  dead 口径唯一化：the false claim "61 道死门禁全部有文档引用" must not
       appear anywhere in the R2 tree, and the canonical statement must be
       present.

It also re-checks the package's internal integrity (MANIFEST self-exclusion,
SHA256SUMS, ZIP membership, freeze state, and that owner/approvals/ holds no
approval file -- an approval is the owner's act, never the implementer's).

Exit code 0 = all checks pass; 1 = at least one check failed.

Usage:  python3 verify_r2_package.py
"""
import collections
import hashlib
import io
import json
import os
import re
import sys
import zipfile

# This file has TWO homes (M-8 of the Task 9 review round 1):
#   * the authoritative copy in the DSH repo, next to the R2 package it checks:
#       …/DSH-EXTERNAL-REVIEW-001/sqlrustgo-v4.1.0-r2/verify_r2_package.py
#   * the reproducibility anchor in the sqlrustgo repo:
#       docs/refactor/v4.1.0-c/takeover/verify_r2_package.py
# The sqlrustgo copy does NOT live next to the package, so it needs to be told
# where the R2 package is.  R2_DIR (env) or --r2-dir wins; otherwise the
# default DSH location is used.
HERE = os.path.dirname(os.path.abspath(__file__))
DEFAULT_R2 = ("/Volumes/workspace/dev/workspaces/dsh-ops/projects/"
              "DSH-EXTERNAL-REVIEW-001/sqlrustgo-v4.1.0-r2")


def _resolve_r2_dir():
    if "--r2-dir" in sys.argv:
        i = sys.argv.index("--r2-dir")
        if i + 1 < len(sys.argv):
            return os.path.abspath(sys.argv[i + 1])
    env = os.environ.get("R2_DIR")
    if env:
        return os.path.abspath(env)
    if os.path.isdir(os.path.join(HERE, "package")):
        return HERE          # the DSH copy: the package is right here
    return DEFAULT_R2       # the sqlrustgo copy: point at the DSH package


R2_DIR = _resolve_r2_dir()
PKG = os.path.join(R2_DIR, "package")
ZIP = os.path.join(R2_DIR, "SQLRUSTGO-V410-REVIEW-PACK-R2-DRAFT.zip")

FAILS = []
CHECKS = []


def check(ok, label, detail=""):
    CHECKS.append((ok, label, detail))
    print("%s  %s%s" % ("PASS" if ok else "FAIL", label,
                         ("  -- " + detail) if detail else ""))
    if not ok:
        FAILS.append(label)
    return ok


def read(p):
    with io.open(p, encoding="utf-8") as f:
        return f.read()


def sha256_file(p):
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def main():
    # N-11: say what to do instead of raising a FileNotFoundError traceback.
    missing = [p for p in (os.path.join(R2_DIR, "package"),
                           os.path.join(R2_DIR, "package", "EVIDENCE-INDEX.json"),
                           os.path.join(R2_DIR, "evidence_index.json"))
               if not os.path.isfile(p) and not os.path.isdir(p)]
    if missing:
        print("FATAL: no R2 package under %s" % R2_DIR, file=sys.stderr)
        print("       This checker verifies an EXISTING R2 package; it does not build one.", file=sys.stderr)
        print("       Point it at the package with either:", file=sys.stderr)
        print("         --r2-dir /path/to/sqlrustgo-v4.1.0-r2", file=sys.stderr)
        print("         R2_DIR=/path/to/sqlrustgo-v4.1.0-r2  python3 %s" % os.path.basename(__file__), file=sys.stderr)
        print("       (the R2 package itself lives in the DSH workspace, or in the", file=sys.stderr)
        print("        distributed ZIP SQLRUSTGO-V410-REVIEW-PACK-R2-DRAFT.zip).", file=sys.stderr)
        print("       Missing: %s" % ", ".join(missing), file=sys.stderr)
        return 2
    print("== R2 package under check: %s" % R2_DIR)
    print("== A5 #10: three statistics sets")
    src = json.loads(read(os.path.join(R2_DIR, "evidence_index.json")))
    idx = json.loads(read(os.path.join(PKG, "EVIDENCE-INDEX.json")))

    # -- the SSOT
    ssot_counts = collections.Counter(e["evidence_grade"] for e in src["evidence"])
    ssot_total = len(src["evidence"])
    print("   evidence_index.json (SSOT): total=%d counts=%s"
          % (ssot_total, dict(ssot_counts)))

    # -- set 1: EVIDENCE-INDEX.json
    idx_counts = collections.Counter(e["evidence_grade"] for e in idx["evidence"])
    idx_total = len(idx["evidence"])
    check(idx_total == ssot_total,
          "#10 EVIDENCE-INDEX.json 条数 == SSOT",
          "index=%d ssot=%d" % (idx_total, ssot_total))
    check(dict(idx_counts) == dict(ssot_counts),
          "#10 EVIDENCE-INDEX.json 等级分布 == SSOT",
          "index=%s ssot=%s" % (dict(idx_counts), dict(ssot_counts)))
    check(idx.get("total") == ssot_total,
          "#10 EVIDENCE-INDEX.json 的 total 字段 == 实测条数",
          "field=%s measured=%d" % (idx.get("total"), ssot_total))

    # -- set 2: EVIDENCE-COVERAGE.md
    cov = read(os.path.join(PKG, "EVIDENCE-COVERAGE.md"))
    rows = re.findall(r"^\| (E-\d+) \| (.*?) \| ([A-Z_]+) \|", cov, re.M)
    cov_counts = collections.Counter(r[2] for r in rows)
    check(len(rows) == ssot_total,
          "#10 EVIDENCE-COVERAGE.md 表格行数 == SSOT",
          "rows=%d ssot=%d" % (len(rows), ssot_total))
    check(dict(cov_counts) == dict(ssot_counts),
          "#10 EVIDENCE-COVERAGE.md 表格等级分布 == SSOT",
          "table=%s ssot=%s" % (dict(cov_counts), dict(ssot_counts)))
    cov_ids = [r[0] for r in rows]
    check(cov_ids == sorted(cov_ids) and len(set(cov_ids)) == len(cov_ids),
          "#10 EVIDENCE-COVERAGE.md 行 id 无重复且有序")
    m = re.search(r"合计 \*\*(\d+)\*\* 条，等级分布 \*\*VERIFIED (\d+) / REPORTED (\d+) / NOT_RUN (\d+)\*\*", cov)
    check(m is not None and int(m.group(1)) == ssot_total
          and int(m.group(2)) == ssot_counts.get("VERIFIED", 0)
          and int(m.group(3)) == ssot_counts.get("REPORTED", 0)
          and int(m.group(4)) == ssot_counts.get("NOT_RUN", 0),
          "#10 EVIDENCE-COVERAGE.md §2 声明的统计 == 实测",
          m.group(0) if m else "声明行未找到")

    # -- set 3: 01-EXECUTIVE-SUMMARY.md
    ex = read(os.path.join(PKG, "01-EXECUTIVE-SUMMARY.md"))
    m = re.search(r"证据索引合计 (\d+) 条", ex)
    check(m is not None and int(m.group(1)) == ssot_total,
          "#10 01-EXECUTIVE-SUMMARY.md 声明的总数 == 实测",
          m.group(0) if m else "声明行未找到")
    for grade in ("VERIFIED", "REPORTED", "NOT_RUN"):
        mm = re.search(r"\| %s \| (\d+) \|" % grade, ex)
        check(mm is not None and int(mm.group(1)) == ssot_counts.get(grade, 0),
              "#10 01-EXECUTIVE-SUMMARY.md %s 计数 == 实测" % grade,
              mm.group(0) if mm else "未找到")

    print()
    print("== A5 #11: dead 口径唯一化")
    forbidden = ["全部有文档引用", "全部仍有文档引用", "全部有正文引用",
                 "61 道全部有文档引用", "61 道死门禁全部有文档引用"]
    hits = []
    for root, dirs, files in os.walk(R2_DIR):
        dirs[:] = [d for d in dirs if d not in ("__pycache__",)]
        for fn in files:
            if fn.endswith((".md", ".json", ".py", ".txt")):
                p = os.path.join(root, fn)
                rel = os.path.relpath(p, R2_DIR)
                if rel in ("gen_r2_package.py", "patch_r2_dirlevel.py",
                           "patch_r2_tools.py", "verify_r2_package.py"):
                    continue  # the generators carry the old string as an anchor
                txt = read(p)
                for f_ in forbidden:
                    if f_ in txt:
                        hits.append("%s :: %s" % (rel, f_))
    check(not hits, "#11 R2 全文无「61 道…全部有文档引用」",
          "; ".join(hits) if hits else "0 命中")
    gov = read(os.path.join(PKG, "04-GATE-GOVERNANCE.md"))
    check("61 + 22 = 83 道无执行点" in gov,
          "#11 唯一口径陈述在 04-GATE-GOVERNANCE.md §2（83 = 61 无引用 + 22 有引用）")

    print()
    print("== package integrity")
    man = json.loads(read(os.path.join(PKG, "MANIFEST.json")))
    se = man.get("self_exclusion")
    check(isinstance(se, dict) and "rule" in se and "excluded_from_manifest_files" in se,
          "MANIFEST.json 显式声明 MANIFEST/SHA256SUMS 的自排除")
    if isinstance(se, dict):
        check(se.get("file_count_in_manifest_files") == len(man.get("files", []))
              and se.get("file_count_in_zip") == len(man.get("files", [])) + 2,
              "MANIFEST.json 自排除计数与实际一致",
              "manifest=%s declared_manifest=%s declared_zip=%s"
              % (len(man.get("files", [])), se.get("file_count_in_manifest_files"),
                 se.get("file_count_in_zip")))
    check(man.get("source_commit") == "fd849ae20541901d5e59f45cd3ba2c51093f5d5e",
          "MANIFEST.json source_commit == BASE",
          str(man.get("source_commit")))
    check(man.get("review_level") == "R2" and man.get("review_id", "").endswith("-R2-20261125"),
          "MANIFEST.json 身份 == R2", str(man.get("review_id")))
    check(man.get("status") == "DRAFT" and man.get("approval_state") == "NOT_APPROVED",
          "MANIFEST.json 仍是 DRAFT / NOT_APPROVED")

    rows = [l.strip() for l in read(os.path.join(PKG, "SHA256SUMS")).splitlines() if l.strip()]
    bad = []
    for line in rows:
        sha, name = line.split("  ", 1)
        p = os.path.join(PKG, name)
        if not os.path.isfile(p) or sha256_file(p) != sha:
            bad.append(name)
    check(not bad, "SHA256SUMS 逐项复算通过（%d 项）" % len(rows),
          ",".join(bad) if bad else "")

    with zipfile.ZipFile(ZIP) as z:
        zfiles = sorted(z.namelist())
    expected = sorted([r["name"] for r in man.get("files", [])]
                      + ["MANIFEST.json", "SHA256SUMS"])
    check(zfiles == expected, "ZIP 文件清单 == MANIFEST.files + MANIFEST.json + SHA256SUMS",
          "zip=%d expected=%d" % (len(zfiles), len(expected)))

    frz = json.loads(read(os.path.join(PKG, "FREEZE.json")))
    check(frz.get("zip_sha256") == sha256_file(ZIP), "FREEZE.json 的 zip_sha256 == 实际 zip 哈希")
    check(frz.get("source_commit") == "fd849ae20541901d5e59f45cd3ba2c51093f5d5e",
          "FREEZE.json source_commit == BASE", str(frz.get("source_commit")))

    appr = os.path.join(R2_DIR, "owner", "approvals")
    files = []
    if os.path.isdir(appr):
        files = [f for f in os.listdir(appr) if f.endswith(".json")]
    check(not files, "owner/approvals/ 无审批文件（签署是属主的行为）",
          ",".join(files) if files else "空")

    print()
    print("%d check(s): %d PASS, %d FAIL"
          % (len(CHECKS), len(CHECKS) - len(FAILS), len(FAILS)))
    for f_ in FAILS:
        print("  FAILED: %s" % f_)
    return 1 if FAILS else 0


if __name__ == "__main__":
    sys.exit(main())
