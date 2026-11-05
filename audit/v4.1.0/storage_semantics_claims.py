#!/usr/bin/env python3
"""P1-4 之后的一步 / IL-11：哪些测试「声称验证存储语义」，以及那份声称是否成立。

背景（看板 IL-11）：
    「哪些测试声称验证存储语义」必须有一份**可机器读的清单**；
    清单里的每个测试都必须被生产存储门禁覆盖；
    清单与实际声明不符 ⇒ 红；
    反向变异：把某个测试改成**只在内存存储上跑** ⇒ 必须红。

为什么需要它（这是 IL-11 的真实风险面）：
    `crates/storage` 之外，仓里有大量测试**既碰生产存储又碰 MemoryStorage**
    （实测 28 份，见 report）。一个测试若声称自己验证的是**生产存储**语义，
    却在关键路径上用内存存储跑 ⇒ 它的断言从来没有在**生产配置**上验证过 ——
    这正是 `scripts/gate/check_correctness_tests_production_storage.sh`
    头部描述的那族问题（「测试是绿的，但它测的不是生产配置」）。

本工具的口径（刻意保守）：

  * **声明来自测试文件自己**，格式是一行标记：

        // storage-semantics: <slug>

    放在文件头部。slug 由文件名推导，等于「这是哪一个测试的声称」。
  * **清单** = `docs/releases/v4.1.0/STORAGE_SEMANTICS_TESTS.jsonl`，
    由标记生成；清单与标记不一致 ⇒ 红（有人改了标记却没重生成清单）。
  * **判据**：
      C1 声明了存储语义的测试，必须**真的构造生产存储**
         （FileStorage / MvccStorage / WalStorage / FileBackedWalManager /
         ParallelWalStorage 之一）。不构造 ⇒ 声称是空的 ⇒ 红。
         **这条就是验收口径的反向变异**：把测试改成只在内存存储上跑 ⇒ 立刻红。
      C2 声明了存储语义、却**只**构造内存存储的测试 ⇒ 红（更明确的形态）。
      C3 清单与树里的标记必须一致 ⇒ 红（有人漏生成/漏更新清单）。
  * 工具另外**报告**（不判红）碰生产存储但**尚未声明**的测试 ——
    它们是清单的候选，不是错。判红它们等于替作者断言「你验证了存储语义」，
    那是不诚实的。

本工具放在 audit/v4.1.0/：探针，不是门禁，不进 STAGE_CONFIG，分母 168 不动。
用法：
    storage_semantics_claims.py emit  [清单]
    storage_semantics_claims.py check [清单]
    storage_semantics_claims.py report
退出码：0 = 全部成立；1 = 有不成立项；2 = 用法错
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
LEDGER = REPO / "docs" / "releases" / "v4.1.0" / "STORAGE_SEMANTICS_TESTS.jsonl"

MARKER_RE = re.compile(r"(?m)^//\s*storage-semantics:\s*(\S+)\s*$")

# 生产存储面 —— AGENTS §1-5：`BufferPool`/`Page`/`BPlusTree` 是 Test-only，
# 生产实现在 crates/storage/src/file_storage.rs。
PROD_SURFACES = ("FileStorage", "MvccStorage", "WalStorage",
                 "FileBackedWalManager", "ParallelWalStorage")
# 内存存储面：出现它本身**不**是错（拿它当对照组是合法的），
# 但「只」构造它、却声称验证生产存储语义 ⇒ 是错。
MEM_SURFACES = ("MemoryStorage", "InMemoryStorage")


def test_files() -> list[Path]:
    roots = [REPO / "tests"] + sorted((REPO / "crates").glob("*/tests"))
    out: list[Path] = []
    for root in roots:
        if not root.is_dir():
            continue
        for f in sorted(root.rglob("*.rs")):
            if f.is_file():
                out.append(f)
    return out


def slug_of(path: Path) -> str:
    stem = path.stem
    s = re.sub(r"[^0-9a-zA-Z]+", "-", stem).strip("-").lower()
    return s


def surfaces_in(text: str) -> tuple[list[str], list[str]]:
    prod = [s for s in PROD_SURFACES if s in text]
    mem = [s for s in MEM_SURFACES if s in text]
    return prod, mem


def scan() -> dict[str, dict]:
    """每个测试文件 → 声明/构造面。"""
    out: dict[str, dict] = {}
    for f in test_files():
        text = f.read_text(encoding="utf-8", errors="replace")
        m = MARKER_RE.search(text)
        prod, mem = surfaces_in(text)
        out[str(f.relative_to(REPO))] = {
            "file": str(f.relative_to(REPO)),
            "declared": m.group(1) if m else None,
            "prod_surfaces": prod,
            "mem_surfaces": mem,
        }
    return out


def cmd_emit(argv: list[str]) -> int:
    """给「声明了但没有标记行」的文件补标记行，并把清单写出来。"""
    out = Path(argv[0]) if argv else LEDGER
    rows = scan()
    declared = {k: v for k, v in rows.items() if v["declared"]}
    lines = []
    for k in sorted(declared):
        v = declared[k]
        lines.append(json.dumps({
            "file": k,
            "slug": v["declared"],
            "prod_surfaces": v["prod_surfaces"],
            "mem_surfaces": v["mem_surfaces"],
            "marker": f"// storage-semantics: {v['declared']}",
        }, ensure_ascii=False, sort_keys=True))
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("".join(l + "\n" for l in lines), encoding="utf-8")
    touch = [v for v in rows.values() if v["prod_surfaces"] and not v["declared"]]
    print(json.dumps({"kind": "storage_semantics_emit", "ledger": str(out),
                      "test_files": len(rows), "declared": len(declared),
                      "candidates_not_declared": len(touch)}, ensure_ascii=False))
    return 0


def cmd_check(argv: list[str]) -> int:
    out = Path(argv[0]) if argv else LEDGER
    rows = scan()
    declared = {k: v for k, v in rows.items() if v["declared"]}
    fails: list[str] = []

    for k, v in sorted(declared.items()):
        if not v["prod_surfaces"]:
            # C1：声称验证存储语义，却一个生产存储面都没构造
            if v["mem_surfaces"]:
                # C2：只构造了内存存储 —— 正是「把测试改成只在内存存储上跑」的形态
                fails.append(f"C2 {k} 声称验证存储语义，却只构造内存存储 "
                             f"（{v['mem_surfaces']}）—— 它的断言验证的不是生产配置")
            else:
                fails.append(f"C1 {k} 声称验证存储语义，却没构造任何生产存储面 —— 声称是空的")

    # C3：清单与树里的标记一致
    ledger = {}
    if out.is_file():
        for line in out.read_text(encoding="utf-8").splitlines():
            if line.strip():
                r = json.loads(line)
                ledger[r["file"]] = r
    for k, v in sorted(declared.items()):
        if k not in ledger:
            fails.append(f"C3 {k} 有 storage-semantics 声明，但清单 {out.name} 里没有它 "
                         "—— 清单与实际声明不符")
        elif ledger[k].get("slug") != v["declared"]:
            fails.append(f"C3 {k} 清单记的 slug={ledger[k].get('slug')!r}，"
                         f"树里是 {v['declared']!r} —— 清单与实际声明不符")
    for k in ledger:
        if k not in declared:
            fails.append(f"C3 {k} 在清单里，但树里已经没有 storage-semantics 声明 "
                         "—— 清单与实际声明不符")

    print(json.dumps({"kind": "storage_semantics_check", "ledger": str(out),
                      "test_files": len(rows), "declared": len(declared),
                      "ledger_rows": len(ledger), "fails": len(fails)},
                     ensure_ascii=False))
    for x in fails[:40]:
        print("FAIL:", x)
    if fails:
        print(f"storage_semantics: {len(fails)} FAIL")
        return 1
    print("storage_semantics: OK")
    return 0


def cmd_report(argv: list[str]) -> int:
    rows = scan()
    declared = {k: v for k, v in rows.items() if v["declared"]}
    prod_touch = {k: v for k, v in rows.items() if v["prod_surfaces"]}
    mixed = {k: v for k, v in declared.items() if v["mem_surfaces"]}
    print(json.dumps({"kind": "storage_semantics_report",
                      "test_files": len(rows),
                      "touching_prod_storage": len(prod_touch),
                      "declared": len(declared),
                      "declared_and_also_memory": len(mixed),
                      "candidates_not_declared": len(prod_touch) - len(declared)},
                     ensure_ascii=False))
    print("--- 已声明（清单）---")
    for k in sorted(declared):
        v = declared[k]
        print(f"  {k}\n      slug={v['declared']} prod={v['prod_surfaces']} mem={v['mem_surfaces']}")
    print("--- 碰生产存储但未声明（候选，不判红）---")
    for k in sorted(set(prod_touch) - set(declared)):
        print("  ", k)
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
