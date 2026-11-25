#!/usr/bin/env python3
"""test_gen_neg_ledger.py —— 台账生成器 `gen_neg_ledger.py` 的自测（M-6）。

背景：最终整支评审的 M-6 指出，这个工具**只有一个模式，而且是破坏性的**：
它就地覆写已提交的台账 `docs/releases/v4.1.0/NEG_FIXTURES_v4.1.0.jsonl`，
证据 run id 直接取自 `argv[1]` 且**无任何校验**。评审期间误用 `--help`，
12 行的 `evidence_run` 被改写成 `"--help"`，而工具仍打印出一份看起来完全正常的
摘要（`wrote 57 rows … kinds={…}`）—— 一次手滑就能改写一份产物，工具不吭声。

本自测覆盖三条要求，且**绝不修改已提交的台账**（只用临时目录；台账的字节与
mtime 在每条臂前后都比对）：

    A. `--check` 在台账一致时退出 0，且**不写任何文件**；
    B. 非法 run id 被拒（非 0 退出 + 人话），且**不写任何文件**；
    C. `--out` 写到临时路径时，**已提交的台账一字未动**。

外加两条单元级的断言（直接打 `validate_run_id`）：默认的两个 run id 必须被
接受；`--help` 与「形态合法但目录不存在」的 run id 必须被拒。

退出码：0 = 全部臂符合预期；1 = 有臂不符；2 = 用法错
本文件放在 audit/v4.1.0/：探针，不是门禁，不进 STAGE_CONFIG，分母 168 不动。
"""
from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
GEN = REPO / "audit" / "v4.1.0" / "gen_neg_ledger.py"
LEDGER = REPO / "docs" / "releases" / "v4.1.0" / "NEG_FIXTURES_v4.1.0.jsonl"
PY = sys.executable

sys.path.insert(0, str(GEN.parent))
import gen_neg_ledger  # noqa: E402  (the module only READS the gate catalogue)


def run_gen(*args: str) -> tuple[int, str]:
    p = subprocess.run([PY, str(GEN), *args], cwd=str(REPO),
                       capture_output=True, text=True, timeout=300)
    return p.returncode, p.stdout + p.stderr


def git_status() -> str:
    p = subprocess.run(["git", "status", "--porcelain"], cwd=str(REPO),
                       capture_output=True, text=True, timeout=300)
    return p.stdout


def fingerprint(path: Path) -> tuple[str, int]:
    """(sha256, mtime_ns) -- mtime catches a rewrite with identical bytes."""
    st = path.stat()
    return hashlib.sha256(path.read_bytes()).hexdigest(), st.st_mtime_ns


def arm_check_non_destructive() -> list[tuple[str, bool, str]]:
    out: list[tuple[str, bool, str]] = []
    before, before_status = fingerprint(LEDGER), git_status()
    rc, text = run_gen("--check")
    after, after_status = fingerprint(LEDGER), git_status()
    out.append(("A1 --check 退出码为 0", rc == 0, "rc=%d" % rc))
    out.append(("A1b --check 打印「逐字节一致」", "逐字节一致" in text,
                "stdout 有「逐字节一致」=%s" % ("逐字节一致" in text)))
    out.append(("A2 --check 未改台账字节", after[0] == before[0],
                "sha256 before=%s after=%s" % (before[0][:12], after[0][:12])))
    out.append(("A2b --check 未改台账 mtime", after[1] == before[1],
                "mtime_ns before=%d after=%d" % (before[1], after[1])))
    out.append(("A2c --check 未在仓里落下任何文件", after_status == before_status,
                "git status 前后一致=%s" % (after_status == before_status)))
    return out


def arm_bad_run_id_rejected() -> list[tuple[str, bool, str]]:
    out: list[tuple[str, bool, str]] = []
    # Two shapes of "not a legal run id": a mistyped flag, and a well-formed id
    # whose run directory does not exist.
    for label, bad in (("B1 --help 被当作 run id 拒掉", "--help"),
                       ("B3 不存在的 run 目录被拒掉", "20261125-no-such-run")):
        before, before_status = fingerprint(LEDGER), git_status()
        rc, text = run_gen(bad)
        after, after_status = fingerprint(LEDGER), git_status()
        out.append((label + "（退出码非 0）", rc != 0, "rc=%d" % rc))
        out.append((label + "（打印人话）", "拒绝执行" in text,
                    "stdout 有「拒绝执行」=%s" % ("拒绝执行" in text)))
        out.append((label + "（未改台账字节）", after[0] == before[0],
                    "sha256 before=%s after=%s" % (before[0][:12], after[0][:12])))
        out.append((label + "（未改台账 mtime）", after[1] == before[1],
                    "mtime_ns before=%d after=%d" % (before[1], after[1])))
        out.append((label + "（未在仓里落下任何文件）", after_status == before_status,
                    "git status 前后一致=%s" % (after_status == before_status)))
    return out


def arm_out_does_not_touch_committed() -> list[tuple[str, bool, str]]:
    out: list[tuple[str, bool, str]] = []
    before, before_status = fingerprint(LEDGER), git_status()
    with tempfile.TemporaryDirectory(prefix="gen-neg-ledger-selftest-") as tmp:
        target = Path(tmp) / "ledger.jsonl"
        rc, text = run_gen("--out", str(target))
        out.append(("C1 --out 退出码为 0", rc == 0, "rc=%d" % rc))
        out.append(("C1b --out 写出了文件", target.exists(),
                    "exists=%s" % target.exists()))
        same = target.exists() and target.read_bytes() == LEDGER.read_bytes()
        out.append(("C1c --out 的内容与已提交的台账逐字节相同", same,
                    "same=%s" % same))
        out.append(("C1d --out 明说已提交的台账未被改动", "未被改动" in text,
                    "stdout 有「未被改动」=%s" % ("未被改动" in text)))
    after, after_status = fingerprint(LEDGER), git_status()
    out.append(("C2 --out 未改已提交台账的字节", after[0] == before[0],
                "sha256 before=%s after=%s" % (before[0][:12], after[0][:12])))
    out.append(("C2b --out 未改已提交台账的 mtime", after[1] == before[1],
                "mtime_ns before=%d after=%d" % (before[1], after[1])))
    out.append(("C2c --out 未在仓里落下任何文件", after_status == before_status,
                "git status 前后一致=%s" % (after_status == before_status)))
    return out


def arm_validate_run_id_unit() -> list[tuple[str, bool, str]]:
    out: list[tuple[str, bool, str]] = []
    for good in (gen_neg_ledger.DEFAULT_RUN, gen_neg_ledger.DEFAULT_RUN8):
        problem = gen_neg_ledger.validate_run_id(good, "run id")
        out.append(("D1 默认 run id %s 被接受" % good, problem is None,
                    "problem=%r" % problem))
    for bad in ("--help", "-h", "20261125-no-such-run", "", "NEGFIX", "20261105-NEGFIX-003"):
        problem = gen_neg_ledger.validate_run_id(bad, "run id")
        out.append(("D2 非法 run id %r 被拒" % bad, problem is not None,
                    "problem=%r" % (problem or "")[:80]))
    return out


ARMS = (
    ("A --check 非破坏", arm_check_non_destructive),
    ("B 非法 run id 被拒且不写文件", arm_bad_run_id_rejected),
    ("C --out 不动已提交的台账", arm_out_does_not_touch_committed),
    ("D run id 校验的单元断言", arm_validate_run_id_unit),
)


def main() -> int:
    results: list[tuple[str, bool, str]] = []
    for name, fn in ARMS:
        try:
            results.extend(fn())
        except Exception as exc:  # noqa: BLE001
            results.append(("%s/执行异常" % name, False, "%s: %s" % (type(exc).__name__, exc)))
    bad = [(n, d) for n, ok, d in results if not ok]
    for n, ok, d in results:
        print("  [%s] %s  (%s)" % ("OK " if ok else "BAD", n, d))
    print(json.dumps({"kind": "test_gen_neg_ledger", "arms": len(results),
                      "ok": len(results) - len(bad), "not_ok": len(bad),
                      "verdict": "FAILED" if bad else "VERIFIED"}, ensure_ascii=False))
    return 1 if bad else 0


if __name__ == "__main__":
    raise SystemExit(main())
