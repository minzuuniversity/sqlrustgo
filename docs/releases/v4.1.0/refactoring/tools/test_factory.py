#!/usr/bin/env python3
"""SQLRustGo Test Factory · 确定性执行器（Local-First Pilot）

设计原则：
- 只依赖 cargo/rust，不依赖任何 LLM；
- 结果分类严格（PASS/FAIL/KNOWN_FAIL/UNSUPPORTED/DIALECT_DIFFERENCE/NOT_RUN/INFRA_ERROR）；
- 每个测试运行记录 base_sha / 环境 / 退出码 / 原始日志 / artifact digest；
- KNOWN_FAIL != PASS；NOT_RUN != PASS。

用法：
  python3 test_factory.py run --suite T1 --out evidence/
  python3 test_factory.py list --manifest corpus.json
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

RESULT_CLASSES = ("PASS", "FAIL", "KNOWN_FAIL", "UNSUPPORTED",
                  "DIALECT_DIFFERENCE", "NOT_RUN", "INFRA_ERROR")

# cargo test 输出解析（行格式：test <name> ... ok/FAILED/ignored）
TEST_LINE = re.compile(
    r"^test\s+(?P<name>[\w:]+)(?:\s+-\s+should panic)?\s+\.\.\.\s+(?P<status>ok|FAILED|ignored)",
    re.M)
SUMMARY = re.compile(r"test result: (?P<status>\w+)\. (?P<passed>\d+) passed; "
                     r"(?P<failed>\d+) failed; (?P<ignored>\d+) ignored")


def sha(repo: Path) -> str:
    r = subprocess.run(["git", "rev-parse", "HEAD"], cwd=repo,
                       capture_output=True, text=True)
    return r.stdout.strip()


def file_digest(path: Path) -> str:
    if not path.exists():
        return ""
    return hashlib.sha256(path.read_bytes()).hexdigest()[:16]


def classify(status: str, known_fails: set[str], name: str) -> str:
    """把 cargo 的三态映射到七态，KNOWN_FAIL 优先。"""
    if status == "ok":
        return "KNOWN_FAIL" if name in known_fails else "PASS"
    if status == "ignored":
        return "UNSUPPORTED"
    return "FAIL"


def run_suite(repo: Path, package: str, filters: list[str], *,
              target_dir: str | None, timeout: int = 1800,
              known_fails: set[str] | None = None,
              test_target: str | None = None) -> dict:
    """执行一组 cargo test 过滤，返回结构化结果。

    test_target=None ⇒ --lib（单测）；指定测试名 ⇒ --test <name>（集成测试）。
    """
    known_fails = known_fails or set()
    cmd = ["cargo", "test", "-p", package]
    if test_target:
        cmd += ["--test", test_target]
    else:
        cmd += ["--lib"]
    cmd += ["--"]
    cmd.extend(filters)
    env = dict(os.environ)
    if target_dir:
        env["CARGO_TARGET_DIR"] = target_dir

    started = time.time()
    proc = subprocess.run(cmd, cwd=repo, capture_output=True, text=True,
                          timeout=timeout, env=env)
    duration = round(time.time() - started, 2)
    raw = proc.stdout + proc.stderr

    tests = []
    for m in TEST_LINE.finditer(raw):
        name, st = m.group("name"), m.group("status")
        tests.append({"name": name, "class": classify(st, known_fails, name),
                      "raw_status": st})
    summ = SUMMARY.search(raw)
    summary = {}
    if summ:
        summary = {"passed": int(summ.group("passed")),
                   "failed": int(summ.group("failed")),
                   "ignored": int(summ.group("ignored")),
                   "cargo_status": summ.group("status")}

    # 编译失败/超时归类
    if not tests and proc.returncode != 0:
        infra = "error[E" in raw or "could not compile" in raw
        klass = "INFRA_ERROR" if infra else "FAIL"
        return {"tests": [], "summary": {}, "exit_code": proc.returncode,
                "class": klass, "duration_s": duration,
                "note": "no test lines parsed; see raw log",
                "cmd": " ".join(cmd), "raw": raw}
    # 反例修复（2026-11-20 DSH-B，contract-observations #1）：
    # 此前"零测试 + exit 0"会命中 else 分支判为 PASS——空跑不是验收证据。
    # 现在无测试行（即使 exit 0、summary 也空）一律 NOT_RUN。
    if not tests:
        return {"tests": [], "summary": summary, "exit_code": proc.returncode,
                "class": "NOT_RUN", "duration_s": duration,
                "note": "no test lines parsed despite exit 0 "
                        "(zero-test counterexample fix: empty run is not evidence)",
                "cmd": " ".join(cmd), "raw": raw}
    # summary 与解析出的测试行交叉校验：cargo 声称的 passed 必须与可见测试行一致，
    # 否则视为分类不可信（防"静默无匹配过滤"被记 PASS）。
    parsed_pass = sum(1 for t in tests if t["class"] in ("PASS", "KNOWN_FAIL"))
    claimed_pass = summary.get("passed")
    if claimed_pass is not None and claimed_pass != parsed_pass:
        return {"tests": tests, "summary": summary, "exit_code": proc.returncode,
                "class": "INFRA_ERROR", "duration_s": duration,
                "note": f"summary.passed={claimed_pass} != parsed={parsed_pass}; "
                        "result not trustworthy",
                "cmd": " ".join(cmd), "raw": raw}
    return {"tests": tests, "summary": summary, "exit_code": proc.returncode,
            "class": "FAIL" if summary.get("failed", 0) else "PASS",
            "duration_s": duration, "cmd": " ".join(cmd), "raw": raw}


def record(repo: Path, out_dir: Path, suite: str, result: dict) -> Path:
    """归档：raw log + structured json + digest。

    result 含 "raw"（原始 cargo 输出）时，.log 存原始输出（真实证据），
    结构化 JSON 中剔除 raw 后落 .json；否则退化为旧行为（.log 存 JSON）。
    """
    out_dir.mkdir(parents=True, exist_ok=True)
    stamp = time.strftime("%Y%m%dT%H%M%S")
    raw_path = out_dir / f"{suite}-{stamp}.log"
    raw_text = result.get("raw")
    if raw_text is not None:
        raw_path.write_text(raw_text)
        stored = {k: v for k, v in result.items() if k != "raw"}
    else:
        raw_path.write_text(json.dumps(result, ensure_ascii=False, indent=1))
        stored = result
    packed = {"suite": suite, "base_sha": sha(repo), "generated_at": stamp,
              "rustc": subprocess.run(["rustc", "--version"], capture_output=True,
                                      text=True).stdout.strip(),
              "cargo": subprocess.run(["cargo", "--version"], capture_output=True,
                                      text=True).stdout.strip(),
              "result": stored, "raw_log": str(raw_path),
              "artifact_digest": file_digest(raw_path)}
    jpath = out_dir / f"{suite}-{stamp}.json"
    jpath.write_text(json.dumps(packed, ensure_ascii=False, indent=1))
    return jpath


def main(argv=None) -> int:
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)

    r = sub.add_parser("run")
    r.add_argument("--repo", default=".")
    r.add_argument("--suite", required=True)
    r.add_argument("--package", default="sqlrustgo-storage")
    r.add_argument("--filter", action="append", default=[])
    r.add_argument("--out", default="evidence")
    r.add_argument("--target-dir", default=None)
    r.add_argument("--timeout", type=int, default=1800)
    r.add_argument("--test", default=None,
                   help="集成测试目标名（--test <name>）；缺省跑 --lib")

    m = sub.add_parser("list")
    m.add_argument("--manifest", required=True)

    a = ap.parse_args(argv)
    if a.cmd == "list":
        data = json.loads(Path(a.manifest).read_text())
        for c in data["cases"]:
            print(f"{c['id']:16s} {c['suite']:6s} {c['status']:10s} {c['desc'][:60]}")
        return 0

    repo = Path(a.repo).resolve()
    res = run_suite(repo, a.package, a.filter,
                    target_dir=a.target_dir, timeout=a.timeout,
                    test_target=getattr(a, "test", None))
    path = record(repo, Path(a.out), a.suite, res)
    print(json.dumps({"suite": a.suite, "class": res["class"],
                      "summary": res.get("summary"),
                      "duration_s": res["duration_s"],
                      "evidence": str(path)}, ensure_ascii=False))
    return 0 if res["class"] == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
