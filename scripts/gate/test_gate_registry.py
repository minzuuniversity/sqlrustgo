#!/usr/bin/env python3
"""gate_registry 的判据夹具（P0-1 验收）。

夹具不往仓里写东西：把登记表复制到 /tmp 做变异，用 --catalog 指过去。
每条变异都必须 rc!=0；未变异的基线必须 rc=0。
"""
from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
CAT = REPO / "docs" / "releases" / "v4.1.0" / "GATE_CATALOG.jsonl"
TOOL = REPO / "scripts" / "gate" / "gate_registry.py"

fails: list[str] = []


def run(catalog: Path) -> tuple[int, str]:
    p = subprocess.run([sys.executable, str(TOOL), "check", str(catalog)],
                       capture_output=True, text=True, cwd=REPO)
    return p.returncode, p.stdout + p.stderr


def mutate(name: str, fn) -> None:
    d = Path(tempfile.mkdtemp(prefix="srg-gr-"))
    c = d / "GATE_CATALOG.jsonl"
    rows = [json.loads(l) for l in CAT.read_text(encoding="utf-8").splitlines() if l.strip()]
    changed = fn(rows)
    if not changed:
        fails.append(f"{name}: 夹具没能改到东西（判据可能空转）")
        shutil.rmtree(d, ignore_errors=True)
        return
    c.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in rows),
                 encoding="utf-8")
    rc, out = run(c)
    if rc == 0:
        fails.append(f"{name}: 变异后仍 rc=0 ⇒ 判据抓不住\n{out[:400]}")
    shutil.rmtree(d, ignore_errors=True)


def main() -> int:
    if not CAT.is_file():
        print("MISSING catalog，先跑 gate_registry.py emit")
        return 2
    rc, out = run(CAT)
    if rc != 0:
        print(f"基线必须 rc=0，实得 {rc}\n{out[:800]}")
        return 1
    print("base: rc=0", out.strip().splitlines()[0])

    dead = [r for r in map(json.loads, CAT.read_text(encoding="utf-8").splitlines())
            if r["class"] == "dead"]

    # M1 删一行登记 ⇒ 必须红（这是 P0-1 验收里点名的那条）
    def m1(rows):
        tgt = rows[0]["gate"]
        rows[:] = [r for r in rows if r["gate"] != tgt]
        print("M1 删除登记:", tgt)
        return True
    mutate("M1-删一行", m1)

    # M2 改 class（把死门禁谎报成阶段内）⇒ 必须红
    def m2(rows):
        rows[[i for i, r in enumerate(rows) if r["class"] == "dead"][0]]["class"] = "stage_required"
        print("M2 谎报 class 为 stage_required")
        return True
    mutate("M2-class谎报", m2)

    # M3 抹掉死门禁的 reason ⇒ 必须红
    def m3(rows):
        rows[[i for i, r in enumerate(rows) if r["class"] == "dead"][0]]["reason"] = ""
        print("M3 抹 reason")
        return True
    mutate("M3-缺reason", m3)

    # M4 翻 can_fail（把跑不红的门禁谎报成能红）⇒ 必须红
    def m4(rows):
        i = next(i for i, r in enumerate(rows) if not r["can_fail"])
        rows[i]["can_fail"] = True
        print("M4 谎报 can_fail:", rows[i]["gate"])
        return True
    mutate("M4-can_fail谎报", m4)

    # M5 抹 stages（登记表与 STAGE_CONFIG 脱钩）⇒ 必须红
    def m5(rows):
        i = next(i for i, r in enumerate(rows) if r["stages"])
        rows[i]["stages"] = []
        print("M5 抹 stages:", rows[i]["gate"])
        return True
    mutate("M5-抹stages", m5)

    # 反向：删掉一条门禁文件本身也必须被抓住（C1 的另一侧），这里用"多登记一行"模拟
    def m6(rows):
        rows.append(dict(rows[0], gate="scripts/gate/check_not_a_real_gate.sh"))
        print("M6 多一行无文件登记")
        return True
    mutate("M6-幽灵登记", m6)

    # M7 抹掉处置(不可运行的门禁不再交代) ⇒ 必须红
    def m7(rows):
        i = next(i for i, r in enumerate(rows) if r.get("disposition"))
        rows[i].pop("disposition")
        print("M7 抹 disposition:", rows[i]["gate"])
        return True
    mutate("M7-缺处置", m7)

    # M8 把 evidence 换成门禁文件里不存在的句子(编造理由) ⇒ 必须红
    def m8(rows):
        i = next(i for i, r in enumerate(rows) if r.get("evidence"))
        rows[i]["evidence"] = "本门禁从未被任何流程调用因此可以安全删除"
        print("M8 伪造 evidence:", rows[i]["gate"])
        return True
    mutate("M8-编造理由", m8)

    # M9 处置与执行点自相矛盾(归档候选却挂在阶段里) ⇒ 必须红
    def m9(rows):
        i = next(i for i, r in enumerate(rows) if r["class"] == "stage_required")
        rows[i]["disposition"] = "archive-candidate"
        rows[i]["note"] = "矛盾处置用于夹具"
        rows[i]["evidence"] = "夹具"
        print("M9 已挂执行点却标归档:", rows[i]["gate"])
        return True
    mutate("M9-处置矛盾", m9)

    print(f"dead_rows={len(dead)} catalog_rows={len(CAT.read_text(encoding='utf-8').splitlines())}")
    if fails:
        for f in fails:
            print("FAIL:", f)
        return 1
    print("test_gate_registry: OK (9 mutations all caught)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
