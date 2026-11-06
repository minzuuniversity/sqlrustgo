#!/usr/bin/env python3
"""P2-2 目录策略机器化：把 DIRECTORY_POLICY.md 的规则变成**可执行的**判据。

背景（看板 P2-2）：`docs/governance/DIRECTORY_POLICY.md` 早就有规则，
但**没有执行点** —— 规则只存在于散文里，谁违反谁都没人拦。
本工具把其中可机器判定的部分变成判据，并且：

    **新增违规 ⇒ 红；存量违规记账但不判红。**

为什么只卡增量：仓里存量违规不少（一级目录缺 README、索引未登记、
中文目录名、vX.Y 目录……）。一次性清算要动几十个目录，
是 Owner 的决定；本工具的职责是**止血** —— 从此刻起不再新增。

规则与依据（每条都带出处，不自创）：

  R1 dir-name-charset    子目录名必须小写、用连字符、不得含中文
                         依据 DIRECTORY_POLICY.md §2.2
  R2 no-new-vxy-dir      不得新建 vX.Y 格式目录
                         依据 DIRECTORY_POLICY.md §2.1
  R3 dir-has-readme      docs/ 下每个一级目录必须含 README.md
                         依据 DIRECTORY_POLICY.md §3.1
  R4 index-registers-dir docs/README.md 必须登记每个一级目录
                         依据 DIRECTORY_POLICY.md §3.2
  R5 top-level-md-cap    顶层 .md 数量不得超过基线
                         依据 看板 P2-2「顶层 45 个 .md」
  R6 bench-dir-single    bench/benchmarks/benches 至多存在一个
                         依据 看板 P2-2「bench/benches/benchmarks 归位」

用法：
    directory_policy.py emit  [基线]     写出当前基线（存量违规）
    directory_policy.py check [基线]     新增违规 ⇒ rc=1
    directory_policy.py report           打印汇总（不改任何东西）

本工具放在 audit/v4.1.0/：它是探针，**不是门禁**，不进 STAGE_CONFIG，
门禁分母 168 不动。基线落 docs/releases/v4.1.0/DIRECTORY_POLICY_BASELINE.jsonl
（仓内禁 *.json/*.log，故用 .jsonl）。
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
BASELINE = REPO / "docs" / "releases" / "v4.1.0" / "DIRECTORY_POLICY_BASELINE.jsonl"
DOCS = REPO / "docs"
DOCS_INDEX = DOCS / "README.md"

# DIRECTORY_POLICY.md §2.1：「禁止新建 `vX.Y` 格式目录（历史迁移后只保留索引说明）」
VXY_RE = re.compile(r"^v\d+\.\d+$")
# 目录名必须：小写字母、数字、连字符、下划线（且不含中文）
NON_ASCII_RE = re.compile(r"[^\x00-\x7f]")


def rule_dir_name_charset() -> list[str]:
    """R1：docs/ 下所有子目录 + 仓库顶层目录，名字必须小写连字符且无中文。"""
    bad = []
    for p in sorted(DOCS.rglob("*")):
        if not p.is_dir():
            continue
        name = p.name
        if name in (".git", "node_modules", "target"):
            continue
        rel = p.relative_to(REPO)
        if NON_ASCII_RE.search(name) or (name != name.lower() and name != name):
            bad.append(f"docs/{rel.relative_to('docs')}")
            continue
        if name != name.lower():
            bad.append(f"docs/{rel.relative_to('docs')}")
            continue
        if "_" in name:
            bad.append(f"docs/{rel.relative_to('docs')}")
    for p in sorted(REPO.iterdir()):
        if not p.is_dir() or p.name.startswith("."):
            continue
        if p.name in ("target", "node_modules"):
            continue
        if NON_ASCII_RE.search(p.name) or p.name != p.name.lower():
            bad.append(p.name + "/")
    return sorted(set(bad))


def rule_no_new_vxy_dir() -> list[str]:
    """R2：vX.Y 格式目录不得存在。"""
    bad = []
    for p in sorted(DOCS.rglob("*")):
        if p.is_dir() and VXY_RE.match(p.name):
            bad.append(str(p.relative_to(REPO)))
    return sorted(bad)


def rule_dir_has_readme() -> list[str]:
    """R3：docs/ 下每个一级目录必须有 README.md。"""
    bad = []
    for p in sorted(DOCS.iterdir()):
        if p.is_dir() and not (p / "README.md").is_file():
            bad.append(f"docs/{p.name}/")
    return sorted(bad)


def rule_index_registers_dir() -> list[str]:
    """R4：docs/README.md 必须**链接**到每个一级目录。"""
    if not DOCS_INDEX.is_file():
        return ["docs/README.md (缺失)"]
    text = DOCS_INDEX.read_text(encoding="utf-8", errors="replace")
    bad = []
    for p in sorted(DOCS.iterdir()):
        if not p.is_dir():
            continue
        # 必须是 markdown 链接指向该目录，而不是顺口提一句名字
        if not re.search(r"\]\(" + re.escape(p.name) + r"/", text):
            bad.append(f"docs/{p.name}/")
    return sorted(bad)


def rule_top_level_md_cap() -> list[str]:
    """R5：顶层 .md 数量不得超过基线（看板 P2-2 记的是 45）。"""
    n = len(list(REPO.glob("*.md")))
    return [] if n <= 45 else [f"顶层 .md {n} 个 > 45"]


def rule_bench_dir_single() -> list[str]:
    """R6：bench/benchmarks/benches 至多存在一个。"""
    present = [d for d in ("bench", "benchmarks", "benches") if (REPO / d).is_dir()]
    return [] if len(present) <= 1 else [f"并存: {'/'.join(present)}"]


RULES = [
    ("dir-name-charset", rule_dir_name_charset, "DIRECTORY_POLICY.md §2.2"),
    ("no-new-vxy-dir", rule_no_new_vxy_dir, "DIRECTORY_POLICY.md §2.1"),
    ("dir-has-readme", rule_dir_has_readme, "DIRECTORY_POLICY.md §3.1"),
    ("index-registers-dir", rule_index_registers_dir, "DIRECTORY_POLICY.md §3.2"),
    ("top-level-md-cap", rule_top_level_md_cap, "看板 P2-2"),
    ("bench-dir-single", rule_bench_dir_single, "看板 P2-2"),
]


def live_violations() -> dict[str, list[str]]:
    return {rid: fn() for rid, fn, _ in RULES}


def cmd_emit(argv: list[str]) -> int:
    out = Path(argv[0]) if argv else BASELINE
    v = live_violations()
    lines = []
    for rid, _, src in RULES:
        for item in v[rid]:
            lines.append(json.dumps({"rule": rid, "item": item, "source": src},
                                    ensure_ascii=False, sort_keys=True))
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("".join(l + "\n" for l in lines), encoding="utf-8")
    print(json.dumps({"kind": "directory_policy_emit", "path": str(out),
                      "rules": len(RULES),
                      "violations": len(lines),
                      "by_rule": {rid: len(v[rid]) for rid, _, _ in RULES}},
                     ensure_ascii=False))
    return 0


def cmd_check(argv: list[str]) -> int:
    path = Path(argv[0]) if argv else BASELINE
    if not path.is_file():
        print(f"FAIL: 没有基线 {path} —— 先 emit，否则「新增」无从判定", file=sys.stderr)
        return 1
    known = set()
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            r = json.loads(line)
            known.add((r["rule"], r["item"]))
    live = live_violations()
    new = []
    for rid, _, src in RULES:
        for item in live[rid]:
            if (rid, item) not in known:
                new.append(f"{rid}: {item}（依据 {src}）")
    print(json.dumps({"kind": "directory_policy_check", "baseline": str(path),
                      "baseline_violations": len(known),
                      "live_violations": sum(len(x) for x in live.values()),
                      "new_violations": len(new)}, ensure_ascii=False))
    for n in new[:40]:
        print("FAIL:", n)
    if new:
        print(f"directory_policy: {len(new)} FAIL")
        return 1
    print("directory_policy: OK")
    return 0


def cmd_report(argv: list[str]) -> int:
    v = live_violations()
    print(json.dumps({"kind": "directory_policy_report",
                      "rules": len(RULES),
                      "violations": sum(len(x) for x in v.values()),
                      "by_rule": {rid: len(v[rid]) for rid, _, _ in RULES}},
                     ensure_ascii=False))
    for rid, _, src in RULES:
        if v[rid]:
            print(f"--- {rid}（{src}）{len(v[rid])} 条")
            for item in v[rid][:8]:
                print("    ", item)
            if len(v[rid]) > 8:
                print(f"     …还有 {len(v[rid]) - 8} 条")
    return 0


def main(argv: list[str] | None = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    if not argv or argv[0] in ("-h", "--help"):
        print(__doc__)
        return 0 if argv else 2
    fn = {"emit": cmd_emit, "check": cmd_check, "report": cmd_report}[argv[0]]
    return fn(argv[1:])


if __name__ == "__main__":
    raise SystemExit(main())
