#!/usr/bin/env python3.14
"""`directory_policy.py` 改动的反向变异：证明「滤掉 .gitignore 残留」只杀假红，不放过真违规。

改动内容（同日同一次提交）：判定口径从「扫文件系统」改成「扫文件系统但排除
git 忽略路径」，并在 check 的 JSON 里打印 `git_ignored_suppressed`。

四臂各自的期望：

  M1 改前口径（`raw_violations`，不过滤）在今天的 develop 上判出**大量**违规，
     而改后口径 `check` rc=0 ⇒ 差异全部是本地残留（本机实测 54 项）。
     ⇒ 改前漏什么：**不漏报**，改前是**误报**（同提交换机器结论翻转）。
  M2 新建**未被忽略**的违规目录 ⇒ `check` 必须 rc=1，且报错项点名该目录
     ⇒ 改后没有变松。
  M3 删掉它 ⇒ rc=0（判据可撤销，不是永久红）。
  M4 同样的违规目录放进**被 .gitignore 忽略**的子树 ⇒ `check` 仍 rc=0，
     但 `git_ignored_suppressed` 必须**增加**，且该项必须出现在 suppressed 列表里
     ⇒ 证明过滤走的是 .gitignore，不是「凡是未跟踪就不判」。

本工具放在 audit/v4.1.0/：探针，不是门禁，不进 STAGE_CONFIG，分母 168 不动。
"""
from __future__ import annotations

import importlib.util
import json
import shutil
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
TOOL = REPO / "audit" / "v4.1.0" / "directory_policy.py"
BAD_NAME = "ZZ_Mutation_Bad_Name"

spec = importlib.util.spec_from_file_location("directory_policy", TOOL)
dp = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(dp)


def check() -> tuple[int, dict, list[str]]:
    p = subprocess.run([sys.executable, str(TOOL), "check"],
                       cwd=REPO, capture_output=True, text=True)
    first = p.stdout.splitlines()[0] if p.stdout.strip() else "{}"
    body = json.loads(first)
    fails = [l[6:] for l in p.stdout.splitlines() if l.startswith("FAIL: ")]
    return p.returncode, body, fails


def arm_m1() -> tuple[str, bool, str]:
    raw = dp.raw_violations()
    live = dp.live_violations()
    n_raw = sum(len(v) for v in raw.values())
    rc, body, _ = check()
    n_live = body["live_violations"]
    ok = n_raw > n_live and rc == 0 and body["git_ignored_suppressed"] == n_raw - n_live
    return ("M1 改前口径误报（不过滤 n_raw > 判定口径 n_live），改后 rc=0",
            ok, f"raw={n_raw} live={n_live} rc={rc} 抑制={body['git_ignored_suppressed']}")


def arm_m2_and_m3() -> list[tuple[str, bool, str]]:
    out = []
    bad = dp.DOCS / "releases" / "v4.1.0" / BAD_NAME
    bad.mkdir(parents=True, exist_ok=True)
    try:
        rc, body, fails = check()
        named = any(BAD_NAME in f for f in fails)
        out.append(("M2 新建未忽略的违规目录 ⇒ rc=1 且点名该目录",
                    rc == 1 and named, f"rc={rc} new={body['new_violations']} 点名={named}"))
    finally:
        shutil.rmtree(bad, ignore_errors=True)
    rc, body, fails = check()
    out.append(("M3 撤销后 ⇒ rc=0（判据可撤销，不是永久红）",
                rc == 0 and not any(BAD_NAME in f for f in fails),
                f"rc={rc} new={body['new_violations']}"))
    return out


def arm_m4() -> tuple[str, bool, str]:
    """同样的违规塞进**被忽略**的子树：不判红，但必须记账。"""
    _, base, _ = check()
    before = base["git_ignored_suppressed"]
    raw_before = sum(len(v) for v in dp.raw_violations().values())
    bad = dp.DOCS / "教学计划" / BAD_NAME          # .gitignore:398 忽略整个 docs/教学计划/
    bad.mkdir(parents=True, exist_ok=True)
    try:
        rc, body, fails = check()
        raw_after = sum(len(v) for v in dp.raw_violations().values())
        sup = body["git_ignored_suppressed"]
        listed = any(BAD_NAME in i for v in dp.suppressed_violations().values() for i in v)
        ok = (rc == 0 and raw_after == raw_before + 1 and sup == before + 1 and listed
              and not any(BAD_NAME in f for f in fails))
        return ("M4 违规塞进 .gitignore 忽略的子树 ⇒ 不判红但计数增加、且列进 suppressed",
                ok, f"rc={rc} raw {raw_before}→{raw_after} 抑制 {before}→{sup} 记账={listed}")
    finally:
        shutil.rmtree(bad, ignore_errors=True)


def main() -> int:
    arms: list[tuple[str, bool, str]] = [arm_m1(), *arm_m2_and_m3(), arm_m4()]
    bad = 0
    for label, ok, detail in arms:
        print(f"  [{'OK ' if ok else 'BAD'}] {label}  ({detail})")
        bad += 0 if ok else 1
    print(json.dumps({"kind": "dir_policy_mutations", "arms": len(arms),
                      "ok": len(arms) - bad, "not_ok": bad,
                      "verdict": "VERIFIED" if bad == 0 else "FAILED"},
                     ensure_ascii=False))
    return 0 if bad == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
