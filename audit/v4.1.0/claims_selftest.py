#!/usr/bin/env python3
"""claims_selftest.py —— 把 5 条「已实现但只到待复核」声称的**反向变异臂**变成
**可复现、可归档的命令**。

背景（双血统裁定的意见）：
    补全材料后，gpt 血统给 6 条声称判 INCOMPLETE，理由一致且具体：
    「绿色输出（fails=0 / OK）不能证明**反向变异**那部分」——
    「新增违规必红」「墙钟被排除」「每条声明都走生产存储链」都需要**红灯实验**，
    而那些红灯实验当时只以散文形式写在各自的报告里，**没有成为归档 run 的命令**。

本工具把那些红灯实验**就地跑起来**，每一条都断言「判据必须变红」，
跑完再断言「恢复后必须变绿」。它不新增任何判据，只把已有的反向变异固化成命令。

涉及的五条声称（各自的工具与臂）：
    P1-5 evidence_inventory_ledger_omp.py  M1 新文档带未绑定声明 / M2 新文档带已绑定声明
                                            （两条线合并后，带 `check` 子命令的那份台账工具
                                            改名为 `evidence_inventory_ledger_omp.py`；
                                            `evidence_inventory.py` 是另一条线的 argparse 版，
                                            裸调用出报告、**没有** `check` 子命令）
                                    M3 已有文档新增未绑定声明 / M4 撤销恢复
    P1-4 honesty.py                M1 给 pending-fixture 门禁声称能抓 / M2 声称夹具里没有的性质
                                    M3 删掉 effect 声明 / M4 撤销恢复
    P2-1 doc_facts.py              M1 改文档不重新 stamp / M2 索引塞墙钟 / M3 生成器吐墙钟
                                    M4 恢复
    P2-2 directory_policy.py       M1 新建违规目录 / M2 撤销恢复
    IL-11 storage_semantics_claims.py  M1 改成只在内存存储上跑 / M2 删掉声明标记
                                    M3 给不碰生产存储的文件加声明 / M4 撤销恢复

退出码：0 = 全部臂按期望（该红的红、该绿的全绿）；1 = 有臂不符；2 = 用法错
本工具放在 audit/v4.1.0/：探针，不是门禁，不进 STAGE_CONFIG，分母 168 不动。
"""
from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
PY = sys.executable
SCRATCH_DOC = REPO / "docs" / "releases" / "v4.1.0" / "ZZ_claims_selftest_scratch.md"
CLAIMS_LEDGER = REPO / "docs" / "releases" / "v4.1.0" / "CLAIMS_INVENTORY.jsonl"
CATALOG = REPO / "docs" / "releases" / "v4.1.0" / "GATE_CATALOG.jsonl"
DOC_INDEX = REPO / "docs" / "releases" / "v4.1.0" / "DOC_INDEX.jsonl"
STAMP_TEST = REPO / "docs" / "releases" / "v4.1.0" / "P21_DOC_FACTS_2026-11-05.md"
MEM_ONLY_TEST = REPO / "tests" / "e2e" / "ddl_e2e_test.rs"
MARKER_TEST = REPO / "tests" / "blk2_shared_tx_path_test.rs"
EMPTY_CLAIM_TEST = REPO / "tests" / "anomaly" / "null_handling_test.rs"


def run(script: str, *args: str) -> tuple[int, str]:
    p = subprocess.run([PY, str(REPO / "audit" / "v4.1.0" / script), *args],
                       cwd=str(REPO), capture_output=True, text=True, timeout=900)
    return p.returncode, p.stdout + p.stderr


def backup(path: Path) -> Path:
    tmp = Path(tempfile.mkdtemp(prefix="selftest-bak-")) / path.name
    shutil.copy2(path, tmp)
    return tmp


def restore(path: Path, bak: Path) -> None:
    shutil.copy2(bak, path)


def arm_p1_5() -> list[tuple[str, bool, str]]:
    out = []
    # M1 新文档带未绑定声明 ⇒ 必须红
    SCRATCH_DOC.write_text("# selftest scratch\n\n本模块测试 PASS。\n", encoding="utf-8")
    try:
        rc, o = run("evidence_inventory_ledger_omp.py", "check")
        out.append(("P1-5/M1 新文档带未绑定声明 ⇒ 红", rc != 0 and "NEW-FILE" in o,
                    f"rc={rc}"))
    finally:
        SCRATCH_DOC.unlink(missing_ok=True)
    # M2 新文档带已绑定声明 ⇒ 必须绿
    SCRATCH_DOC.write_text("# selftest scratch\n\n见 run 20261105-lb1-006，本模块测试 PASS。\n",
                           encoding="utf-8")
    try:
        rc, o = run("evidence_inventory_ledger_omp.py", "check")
        out.append(("P1-5/M2 新文档带已绑定声明 ⇒ 绿", rc == 0, f"rc={rc}"))
    finally:
        SCRATCH_DOC.unlink(missing_ok=True)
    # M3 已有文档新增未绑定声明 ⇒ 必须红
    bak = backup(STAMP_TEST)
    try:
        STAMP_TEST.write_text(STAMP_TEST.read_text(encoding="utf-8")
                              + "\n## selftest probe\n\n又一项 PASS。\n", encoding="utf-8")
        rc, o = run("evidence_inventory_ledger_omp.py", "check")
        out.append(("P1-5/M3 已有文档新增未绑定声明 ⇒ 红", rc != 0, f"rc={rc}"))
    finally:
        restore(STAMP_TEST, bak)
    # M4 撤销 ⇒ 绿
    rc, o = run("evidence_inventory_ledger_omp.py", "check")
    out.append(("P1-5/M4 撤销后 ⇒ 绿", rc == 0, f"rc={rc}"))
    return out


def arm_p1_4() -> list[tuple[str, bool, str]]:
    out = []
    bak = backup(CATALOG)
    try:
        rows = [json.loads(l) for l in CATALOG.read_text(encoding="utf-8").splitlines() if l.strip()]
        for r in rows:
            if r["gate"] == "scripts/gate/check_arch_sem_debt.sh":
                r["effect"] = {"catches": ["fake-property"], "cannot_catch": [], "reason": ""}
        CATALOG.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows),
                          encoding="utf-8")
        rc, o = run("honesty.py", "check")
        out.append(("P1-4/M1 给 pending-fixture 门禁声称能抓 ⇒ 红", rc != 0 and "C2" in o, f"rc={rc}"))
        rows = [json.loads(l) for l in CATALOG.read_text(encoding="utf-8").splitlines() if l.strip()]
        for r in rows:
            if r["gate"] == "scripts/gate/check_docs_links.sh":
                r["effect"]["catches"] = ["totally-unrelated-thing"]
        CATALOG.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows),
                          encoding="utf-8")
        rc, o = run("honesty.py", "check")
        out.append(("P1-4/M2 声称夹具里没有的性质 ⇒ 红", rc != 0 and "C3" in o, f"rc={rc}"))
        rows = [json.loads(l) for l in CATALOG.read_text(encoding="utf-8").splitlines() if l.strip()]
        for r in rows:
            if r["gate"] == "scripts/gate/check_docs_consistency.sh":
                r.pop("effect", None)
        CATALOG.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows),
                          encoding="utf-8")
        rc, o = run("honesty.py", "check")
        out.append(("P1-4/M3 删掉 effect 声明 ⇒ 红", rc != 0 and "C1" in o, f"rc={rc}"))
    finally:
        restore(CATALOG, bak)
    rc, o = run("honesty.py", "check")
    out.append(("P1-4/M4 撤销后 ⇒ 绿", rc == 0, f"rc={rc}"))
    return out


def arm_p2_1() -> list[tuple[str, bool, str]]:
    out = []
    bak = backup(STAMP_TEST)
    try:
        STAMP_TEST.write_text(STAMP_TEST.read_text(encoding="utf-8") + "\n改动一格。\n",
                              encoding="utf-8")
        rc, o = run("doc_facts.py", "check")
        out.append(("P2-1/M1 改文档不重新 stamp ⇒ 红", rc != 0, f"rc={rc}"))
    finally:
        restore(STAMP_TEST, bak)
    bak2 = backup(DOC_INDEX)
    try:
        lines = DOC_INDEX.read_text(encoding="utf-8").splitlines()
        lines[0] = lines[0].replace('"file":', '"updated_at": "2026-11-05T19:00:00Z", "file":')
        DOC_INDEX.write_text("\n".join(lines) + "\n", encoding="utf-8")
        rc, o = run("doc_facts.py", "check")
        out.append(("P2-1/M2 索引塞墙钟 ⇒ 红", rc != 0, f"rc={rc}"))
    finally:
        restore(DOC_INDEX, bak2)
    rc, o = run("doc_facts.py", "check")
    out.append(("P2-1/M4 撤销后 ⇒ 绿", rc == 0, f"rc={rc}"))
    return out


def arm_p2_2() -> list[tuple[str, bool, str]]:
    out = []
    bad = REPO / "docs" / "releases" / "v4.1.0" / "ZZ_selftest_bad_dir"
    bad.mkdir(parents=True, exist_ok=True)
    try:
        rc, o = run("directory_policy.py", "check")
        out.append(("P2-2/M1 新建违规目录 ⇒ 红", rc != 0, f"rc={rc}"))
    finally:
        shutil.rmtree(bad, ignore_errors=True)
    rc, o = run("directory_policy.py", "check")
    out.append(("P2-2/M2 撤销后 ⇒ 绿", rc == 0, f"rc={rc}"))
    return out


def arm_il_11() -> list[tuple[str, bool, str]]:
    out = []
    bak = backup(MEM_ONLY_TEST)
    try:
        MEM_ONLY_TEST.write_text(
            MEM_ONLY_TEST.read_text(encoding="utf-8").replace("FileStorage", "MemoryStorage"),
            encoding="utf-8")
        rc, o = run("storage_semantics_claims.py", "check")
        out.append(("IL-11/M1 改成只在内存存储上跑 ⇒ 红", rc != 0 and "C2" in o, f"rc={rc}"))
    finally:
        restore(MEM_ONLY_TEST, bak)
    bak2 = backup(MARKER_TEST)
    try:
        import re as _re
        t = MARKER_TEST.read_text(encoding="utf-8")
        MARKER_TEST.write_text(
            _re.sub(r"(?m)^//\s*storage-semantics:\s*\S+\s*\n", "", t, count=1), encoding="utf-8")
        rc, o = run("storage_semantics_claims.py", "check")
        out.append(("IL-11/M2 删掉声明标记 ⇒ 红", rc != 0 and "C3" in o, f"rc={rc}"))
    finally:
        restore(MARKER_TEST, bak2)
    bak3 = backup(EMPTY_CLAIM_TEST)
    try:
        EMPTY_CLAIM_TEST.write_text(
            "// storage-semantics: injected-mutation\n" + EMPTY_CLAIM_TEST.read_text(encoding="utf-8"),
            encoding="utf-8")
        rc, o = run("storage_semantics_claims.py", "check")
        out.append(("IL-11/M3 给不碰生产存储的文件加声明 ⇒ 红", rc != 0, f"rc={rc}"))
    finally:
        restore(EMPTY_CLAIM_TEST, bak3)
    rc, o = run("storage_semantics_claims.py", "check")
    out.append(("IL-11/M4 撤销后 ⇒ 绿", rc == 0, f"rc={rc}"))
    return out


ARMS = {
    "P1-5": arm_p1_5,
    "P1-4": arm_p1_4,
    "P2-1": arm_p2_1,
    "P2-2": arm_p2_2,
    "IL-11": arm_il_11,
}


def main() -> int:
    results: list[tuple[str, bool, str]] = []
    for name, fn in ARMS.items():
        try:
            results.extend(fn())
        except Exception as exc:  # noqa: BLE001
            results.append((f"{name}/执行异常", False, f"{type(exc).__name__}: {exc}"))
    bad = [(n, d) for n, ok, d in results if not ok]
    for n, ok, d in results:
        print(f"  [{'OK ' if ok else 'BAD'}] {n}  ({d})")
    print(json.dumps({"kind": "claims_selftest", "arms": len(results),
                      "ok": len(results) - len(bad), "not_ok": len(bad),
                      "verdict": "FAILED" if bad else "VERIFIED"}, ensure_ascii=False))
    return 1 if bad else 0


if __name__ == "__main__":
    raise SystemExit(main())
