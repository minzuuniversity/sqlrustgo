#!/usr/bin/env python3.14
"""`directory_policy.py` 改动的反向变异：证明「滤掉 .gitignore 残留」只杀假红，不放过真违规。

改动内容（同日同一次提交）：判定口径从「扫文件系统」改成「扫文件系统但排除
git 忽略路径」，并在 check 的 JSON 里打印 `git_ignored_suppressed`。

五臂各自的期望（每一臂都**自带夹具**，不依赖本机恰好残留了什么）：

  M1 改前口径（`raw_violations`，不过滤忽略项）把「被 .gitignore 忽略的残留」
     判成违规，而改后口径 `check` rc=0 ⇒ 差异是本地残留，不是真违规。
     夹具：一个放在**被忽略子树**里的违规目录（探针自己造 ⇒ raw>live 恒成立）。
  M2 新建**未被忽略**的违规目录 ⇒ `check` 必须 rc=1，且报错项点名该目录
     ⇒ 改后没有变松。
  M3 删掉它 ⇒ rc=0（判据可撤销，不是永久红）。
  M4 同样的违规目录放进**被 .gitignore 忽略**的子树 ⇒ `check` 仍 rc=0，
     但 `git_ignored_suppressed` 必须**增加**，且该项必须出现在 suppressed 列表里
     ⇒ 证明过滤走的是 .gitignore，不是「凡是未跟踪就不判」。
  M5 撤掉夹具 ⇒ raw 计数回到夹具前 ⇒ 探针自身不污染工作树。

**这一版为什么重写（2026-11-06，CI 批归档 run 20261106-ci-001 现场打红）**：
旧版 M1 依赖「本机恰好有被忽略的残留」（raw>live 才成立），M4 用的是被
`directory_policy` 模块缓存的忽略目录列表（`_IGNORED_DIRS` 是模块级缓存，
夹具建好之后不失效 ⇒ suppressed 记账读到旧快照 ⇒ 恒判「没记账」）。
旧版在看板批的 worktree 里 4/4 VERIFIED 纯属**环境侥幸**：那台机器的
worktree 里恰好有被忽略的残留目录。换一台机器/换一个 worktree ⇒ M1/M4 转红。
空洞的绿比红更坏，所以这里让每一臂自己造夹具、自己重载模块。

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
# .gitignore 里 `docs/教学计划/` 整棵被忽略（.gitignore:291）。
IGNORED_SUBTREE = "教学计划"


def load_policy():
    """重新载入策略模块。

    `directory_policy.ignored_dirs()` 把 git 眼里的忽略目录缓存在模块级
    `_IGNORED_DIRS` 里；夹具增删之后那份快照就过时了（这正是旧版 M4
    恒判「没记账」的根因）。每次测量前重载 ⇒ 快照必然新鲜。
    """
    spec = importlib.util.spec_from_file_location("directory_policy", TOOL)
    mod = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(mod)
    return mod


def check() -> tuple[int, dict, list[str]]:
    p = subprocess.run([sys.executable, str(TOOL), "check"],
                       cwd=REPO, capture_output=True, text=True)
    first = p.stdout.splitlines()[0] if p.stdout.strip() else "{}"
    body = json.loads(first)
    fails = [l[6:] for l in p.stdout.splitlines() if l.startswith("FAIL: ")]
    return p.returncode, body, fails


def counts(mod) -> tuple[int, int, int]:
    raw = sum(len(v) for v in mod.raw_violations().values())
    live = sum(len(v) for v in mod.live_violations().values())
    sup = sum(len(v) for v in mod.suppressed_violations().values())
    return raw, live, sup


def make_fixture(mod) -> tuple[Path, bool]:
    """在**被忽略**的子树里造一个违规目录。返回 (路径, 是否由我们创建父目录)。"""
    fixture = mod.DOCS / IGNORED_SUBTREE / BAD_NAME
    created_parent = not fixture.parent.exists()
    fixture.mkdir(parents=True, exist_ok=True)
    return fixture, created_parent


def drop_fixture(fixture: Path, created_parent: bool) -> None:
    shutil.rmtree(fixture, ignore_errors=True)
    if created_parent:
        shutil.rmtree(fixture.parent, ignore_errors=True)


def arm_m1() -> tuple[str, bool, str]:
    """改前口径误报 / 改后口径不误报。夹具自带 ⇒ 不依赖本机残留。"""
    dp0 = load_policy()
    raw0, live0, _ = counts(dp0)
    fixture, created_parent = make_fixture(dp0)
    try:
        dp = load_policy()
        n_raw, n_live, _ = counts(dp)
        rc, body, fails = check()
        sup = body["git_ignored_suppressed"]
        ok = (n_raw > n_live and n_raw > raw0 and rc == 0
              and sup == n_raw - n_live
              and not any(BAD_NAME in f for f in fails))
        return ("M1 改前口径误报（夹具自带：被忽略子树里的违规目录 ⇒ raw>live），改后 rc=0",
                ok, f"raw {raw0}→{n_raw} live={n_live} rc={rc} 抑制={sup}")
    finally:
        drop_fixture(fixture, created_parent)


def arm_m2_and_m3() -> list[tuple[str, bool, str]]:
    out = []
    dp = load_policy()
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
    dp0 = load_policy()
    raw_before, _, sup_before = counts(dp0)
    fixture, created_parent = make_fixture(dp0)
    try:
        dp = load_policy()
        raw_after, live_after, sup_items = counts(dp)
        rc, body, fails = check()
        sup = body["git_ignored_suppressed"]
        listed = any(BAD_NAME in i for v in dp.suppressed_violations().values() for i in v)
        delta = raw_after - raw_before
        ok = (rc == 0 and delta > 0 and sup == sup_before + delta and listed
              and not any(BAD_NAME in f for f in fails))
        return ("M4 违规塞进 .gitignore 忽略的子树 ⇒ 不判红但计数增加、且列进 suppressed",
                ok, f"rc={rc} raw {raw_before}→{raw_after} 抑制 {sup_before}→{sup} "
                     f"记账={listed} live={live_after}")
    finally:
        drop_fixture(fixture, created_parent)


def arm_m5() -> tuple[str, bool, str]:
    """夹具撤掉后计数必须回到原值 ⇒ 探针不污染工作树。"""
    dp0 = load_policy()
    raw0, _, _ = counts(dp0)
    fixture, created_parent = make_fixture(dp0)
    drop_fixture(fixture, created_parent)
    dp1 = load_policy()
    raw1, _, _ = counts(dp1)
    rc, body, _ = check()
    return ("M5 撤掉夹具 ⇒ raw 计数回到原值（探针不污染工作树）",
            raw1 == raw0 and rc == 0, f"raw {raw0}→{raw1} rc={rc}")


def main() -> int:
    arms: list[tuple[str, bool, str]] = [arm_m1(), *arm_m2_and_m3(), arm_m4(), arm_m5()]
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
