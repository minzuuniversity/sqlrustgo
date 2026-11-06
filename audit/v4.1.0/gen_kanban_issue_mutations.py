#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Reverse-mutation runner for the kanban ISSUE generator (AGENTS 2.5).
#
#   python3 audit/v4.1.0/gen_kanban_issue_mutations.py
#
# Each arm re-introduces one of the six defects found in the ad-hoc /tmp
# generator, but only inside a throwaway copy under a temp dir; the repository
# copy is never touched. An arm counts as load-bearing only when the generator
# selftest flips red. If every arm stayed green, the selftest would be vacuous.
#
# Exit codes: 0 = every arm flipped red; 1 = at least one arm stayed green
# (the selftest is not load-bearing); 3 = a mutation could not be applied
# (the source drifted, so the evidence is stale).
import pathlib
import shutil
import subprocess
import sys
import tempfile

SRC = pathlib.Path(__file__).resolve().parent / "gen_kanban_issue.py"

ARMS = [
    ("m1-always-post",
     "    if not post_to:\n        return body\n",
     "    if False:\n        return body\n",
     "A1/network: 默认模式必须不 PATCH"),
    ("m2-template-leak",
     "\uff08{len(rows)} \u884c\uff09",
     "\uff08' + str(len(rows)) + ' \u884c\uff09",
     "A2: \u6b63\u6587\u4e0d\u5f97\u6b8b\u7559\u6a21\u677f\u7247\u6bb5"),
    ("m3-stage-key-wrong",
     'if t.startswith("current_stage:"):',
     'if t.startswith("stage:"):',
     "A3/stage-guard: \u5934\u90e8 stage \u5b57\u6bb5\u4e0d\u5f97\u4e3a\u7a7a"),
    ("m4-rowcount-drift",
     "\uff08{len(rows)} \u884c\uff09",
     "\uff08{len(rows) + 1} \u884c\uff09",
     "A4: \u884c\u6570\u53e3\u5f84\u5fc5\u987b\u7b49\u4e8e SSOT"),
    # m5: 把 --appendix 的 fail-closed 守卫改成静默跳过（交接正文会「消失」而看板照发），
    #     只有当自检真的因此变红，这条守卫才算承重。
    ("m5-appendix-silent-skip",
     "        L += appendix_section(appendix_path)\n",
     "        pass\n",
     "A5/A6: 附录缺失必须 fail-closed，不许静默出无附录看板"),
]


def main() -> int:
    base = SRC.read_text(encoding="utf-8")
    tmp = tempfile.mkdtemp(prefix="gen-kanban-mut-")
    failed_open, drifted = [], []
    try:
        for name, old, new, expect in ARMS:
            if base.count(old) != 1:
                drifted.append(f"{name}: 源文本命中 {base.count(old)} 次，变异无法施加")
                print(f"DRIFT {name}: anchor count={base.count(old)} != 1")
                continue
            f = pathlib.Path(tmp) / (name + ".py")
            f.write_text(base.replace(old, new), encoding="utf-8")
            p = subprocess.run([sys.executable, str(f), "--selftest"],
                               capture_output=True, text=True, timeout=120)
            red = p.returncode != 0
            first = next((l for l in (p.stdout + "\n" + p.stderr).splitlines() if l.strip()), "")
            print(f"{'RED ' if red else 'GREEN'} {name}: rc={p.returncode} expect={expect} first_sighting={first[:120]}")
            if not red:
                failed_open.append(name)
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    print(f"arms={len(ARMS)} flipped_red={len(ARMS) - len(failed_open) - len(drifted)} "
          f"stayed_green={len(failed_open)} drift={len(drifted)}")
    if drifted:
        return 3
    return 1 if failed_open else 0


if __name__ == "__main__":
    raise SystemExit(main())
