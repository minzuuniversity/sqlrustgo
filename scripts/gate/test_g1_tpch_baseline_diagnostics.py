#!/usr/bin/env python3
"""G1 诊断补丁的夹具：证明「实算哈希」在门禁输出里可见，且既有语义一字未变。

WHY（2026-11-18 实测，见看板行 GA-TPCH1 的验收条件 ①）
--------------------------------------------------------------------------------
权威轮 20261117-ga-gates-013 的第 13 项 `check_g1_tpch_baseline.sh` 判 FAIL，
但归档 stdout 里只有一句 `hash mismatch`，**没有实算哈希**，也没有任何一行
说明「哪条语句变了」。核对源码后机理闭合：

  1. Rust 侧 `compare_or_disclose()` 用 `assert_eq!(run.hash, expected, "…")`。
     Rust 的 assert_eq! 宏**总是**先打印
         left: "<实算>"
        right: "<基线>"
     再追加自定义消息 `G1 FAIL: TPC-H hash mismatch / expected: … / actual: …`。
     ⇒ 实算哈希本来就在测试输出里。
  2. 门禁 step 3 走 `run_tpch_test()` helper：它把测试输出**全量落盘**到临时
     文件，再 `tail -10 "${out}"` 显示 ⇒ Rust panic 的 `left:` 那一行被截掉；
     紧接着 rc != 0 分支里的 `rm -f "${out}"` 又把**完整输出销毁** ⇒
     「实算是多少」被销毁两次，事后连取证都做不到。
  3. 门禁 step 4 调 `tpch_hash_compare.py --check`，它自己再跑一遍测试；
     `classify()` 的 MISMATCH 分支返回**空 payload** ⇒ 实算哈希被**第三次**丢弃。

⇒ 一次失败被丢掉两次诊断信息。本夹具钉住修复后的三条性质：

  A. mismatch 时门禁输出**含实算哈希**（64 位），并含定位命令；
  B. 既有输出**一字未动**（tail -10 仍在）、退出码语义不变（0/1/2 三态）；
  C. 诚实性三态不受影响：PASS 仍 PASS、NOT-RUN 仍 INCOMPLETE 且**不得**说成
     hash mismatch（这是 `test_g1_tpch_baseline_honesty.py` 的既有断言）。

零编译：用一个 stub `cargo` 顶替真编译器，输出预设的测试输出。
本夹具**不改任何门禁判据**，只验证诊断信息是否可见。

用法：python3 scripts/gate/test_g1_tpch_baseline_diagnostics.py
退出码：0 = 全部通过；1 = 有断言失败（fail-closed）。
"""
from __future__ import annotations

import importlib.util
import os
import shutil
import stat
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parents[1]
COMPARE_PY = HERE / "tpch_hash_compare.py"
GATE_SH = HERE / "check_g1_tpch_baseline.sh"

def _baseline_hash() -> str:
    """从真实的基线文件读期望哈希 —— 门禁 step 4 比较的就是它，
    stub 必须报同一个值，PASS 路径才可能成立。读不到就退回一个 64 位常量。"""
    try:
        import json
        data = json.loads((REPO_ROOT / "tests" / "tpch_hashes_v380.json").read_text())
        h = data.get("tpc_h_hash_sha256", "")
        if isinstance(h, str) and len(h) == 64:
            return h
    except Exception:
        pass
    return "a" * 64


BASE_HASH = _baseline_hash()          # 基线（right:）——门禁自己也会读它
# 实算（left:）—— 恰好 64 位十六进制，且必定与基线不同；缺一位就不是哈希。
COMPUTED_HASH = ("f" if BASE_HASH[0] != "f" else "e") + BASE_HASH[1:]

FAILURES: list[str] = []


def check(name: str, cond: bool, detail: str = "") -> None:
    if cond:
        print(f"  [OK]   {name}")
    else:
        print(f"  [FAIL] {name} {detail}")
        FAILURES.append(name)


def load_compare():
    spec = importlib.util.spec_from_file_location("tpch_hash_compare", COMPARE_PY)
    mod = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(mod)
    return mod


def stub_outputs(mode: str) -> tuple[str, str, int]:
    """返回 (ok 输出, hash 输出, hash 目标的退出码)。"""
    ok_tail = (
        "running 7 tests\n"
        "test common::tpch_wire_scope::tests::wire_scope_and_exclusions_cover_every_standard_query ... ok\n"
        "test tpch_gate_completes ... ok\n"
        "\n"
        "test result: ok. 40 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.31s\n"
    )
    if mode == "pass":
        hash_body = (
            "running 50 tests\n"
            "test tpch_hash_matches_v380_baseline ... ok\n"
            "test tpch_hash_test_runs_in_under_5_minutes ... ok\n"
            "\n"
            "test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.82s\n"
            f"G1 PASS: TPC-H 22/22 baseline hash matches ({BASE_HASH})\n"
        )
        rc = 0
    elif mode == "notrun":
        hash_body = (
            "running 50 tests\n"
            "TPCH-NOT-RUN: tpch_hash_test reason=fixture_missing "
            "detail=no TPC-H fixture found (set TPCH_DATA_DIR to a directory with the 8 .tbl files)\n"
            "\n"
            "test result: ok. 50 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s\n"
        )
        rc = 0
    else:
        # 真实布局：panic 段落在 failures 之前 —— tail -10 恰好把 left:/right: 吃掉，
        # 这正是 013 轮「诊断黑障」的成因。
        hash_body = (
            "running 50 tests\n"
            "test tpch_hash_matches_v380_baseline ... FAILED\n"
            "test tpch_hash_test_runs_in_under_5_minutes ... FAILED\n"
            "\n"
            "failures:\n"
            "\n"
            "---- tpch_hash_matches_v380_baseline stdout ----\n"
            "assertion failed: `(left == right)`\n"
            f"  left: \"{COMPUTED_HASH}\"\n"
            f" right: \"{BASE_HASH}\"\n"
            "G1 FAIL: TPC-H hash mismatch\n"
            f"  expected: {BASE_HASH}\n"
            f"  actual:   {COMPUTED_HASH}\n"
            "\n"
            "---- tpch_hash_test_runs_in_under_5_minutes stdout ----\n"
            "assertion failed: `(left == right)`\n"
            f"  left: \"{COMPUTED_HASH}\"\n"
            f" right: \"{BASE_HASH}\"\n"
            "G1 FAIL: TPC-H hash mismatch\n"
            f"  expected: {BASE_HASH}\n"
            f"  actual:   {COMPUTED_HASH}\n"
            "\n"
            "failures:\n"
            "    tpch_hash_matches_v380_baseline\n"
            "    tpch_hash_test_runs_in_under_5_minutes\n"
            "\n"
            "test result: FAILED. 48 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; "
            "finished in 7.82s\n"
            "\n"
            "error: test failed, to rerun pass `--test tpch_hash_test`\n"
        )
        rc = 101
    return ok_tail, hash_body, rc


def run_gate(mode: str, tmp_root: Path) -> subprocess.CompletedProcess:
    bindir = tmp_root / "bin"
    bindir.mkdir(parents=True, exist_ok=True)
    outdir = tmp_root / "out"
    outdir.mkdir(parents=True, exist_ok=True)
    ok_tail, hash_body, hash_rc = stub_outputs(mode)
    (outdir / "ok.txt").write_text(ok_tail)
    (outdir / "hash.txt").write_text(hash_body)

    cargo = bindir / "cargo"
    cargo.write_text(
        "#!/usr/bin/env bash\n"
        "set -u\n"
        'target=""\n'
        "next=0\n"
        'for a in "$@"; do\n'
        '  if [ "$next" = 1 ]; then target="$a"; next=0; continue; fi\n'
        '  if [ "$a" = "--test" ]; then next=1; fi\n'
        "done\n"
        'if [ "${target}" = "tpch_hash_test" ]; then\n'
        '  cat "${STUB_OUT_DIR}/hash.txt"\n'
        '  exit "${STUB_HASH_RC}"\n'
        "fi\n"
        'cat "${STUB_OUT_DIR}/ok.txt"\n'
        "exit 0\n"
    )
    cargo.chmod(cargo.stat().st_mode | stat.S_IEXEC | stat.S_IXGRP | stat.S_IXOTH)

    fixture = tmp_root / "fixture"
    fixture.mkdir(parents=True, exist_ok=True)
    for tbl in ("customer", "lineitem", "nation", "orders", "part", "partsupp",
                "region", "supplier"):
        (fixture / f"{tbl}.tbl").write_text("")

    env = dict(os.environ)
    env["PATH"] = f"{bindir}{os.pathsep}{env.get('PATH', '')}"
    env["TPCH_DATA_DIR"] = str(fixture)
    env["STUB_OUT_DIR"] = str(outdir)
    env["STUB_HASH_RC"] = str(hash_rc)
    return subprocess.run(
        ["bash", str(GATE_SH)],
        cwd=str(REPO_ROOT),
        env=env,
        capture_output=True,
        text=True,
        timeout=600,
    )


def main() -> int:
    mod = load_compare()
    tmp_root = Path(tempfile.mkdtemp(prefix="g1-diag-fixture-"))
    try:
        # ---- A. 比较器侧：MISMATCH 态必须带出实算哈希 ----
        print("A. tpch_hash_compare MISMATCH 态携带实算哈希")
        drift_left = (
            b"assertion failed: `(left == right)`\n"
            b"  left: \"" + COMPUTED_HASH.encode() + b"\"\n"
            b" right: \"" + BASE_HASH.encode() + b"\"\n"
            b"G1 FAIL: TPC-H hash mismatch\n"
        )
        verdict, payload = mod.classify(drift_left)[:2]
        check("左值式（left:）⇒ MISMATCH 且取出实算哈希",
              verdict == "MISMATCH" and payload == COMPUTED_HASH,
              f"got {verdict!r}/{payload!r}")

        drift_actual = (
            b"G1 FAIL: TPC-H hash mismatch\n"
            b"  expected: " + BASE_HASH.encode() + b"\n"
            b"  actual:   " + COMPUTED_HASH.encode() + b"\n"
        )
        verdict, payload = mod.classify(drift_actual)[:2]
        check("右值式（actual:）⇒ MISMATCH 且取出实算哈希",
              verdict == "MISMATCH" and payload == COMPUTED_HASH,
              f"got {verdict!r}/{payload!r}")

        drift_bare = b"G1 FAIL: TPC-H hash mismatch\n"
        verdict, payload = mod.classify(drift_bare)[:2]
        check("取不到实算哈希时 ⇒ MISMATCH + 空 payload（不猜、不补造）",
              verdict == "MISMATCH" and payload == "",
              f"got {verdict!r}/{payload!r}")

        good = b"G1 PASS: TPC-H 22/22 baseline hash matches (" + BASE_HASH.encode() + b")\n"
        verdict, payload = mod.classify(good)[:2]
        check("PASS 态判定与取值不变",
              verdict == "OK" and payload == BASE_HASH,
              f"got {verdict!r}/{payload!r}")

        not_run = (b"TPCH-NOT-RUN: tpch_hash_test reason=fixture_missing detail=x\n"
                   b"test result: ok. 50 passed; 0 failed\n")
        verdict, payload = mod.classify(not_run)[:2]
        check("NOT_RUN 态判定与 reason 不变",
              verdict == "NOT_RUN" and payload == "fixture_missing",
              f"got {verdict!r}/{payload!r}")

        verdict, payload = mod.classify(b"test result: ok. 1 passed\n")[:2]
        check("UNDISCLOSED 态不变", verdict == "UNDISCLOSED", f"got {verdict!r}")

        # ---- B. 门禁侧：mismatch 时实算哈希可见，既有输出一字未动 ----
        print("B. check_g1_tpch_baseline.sh 在 mismatch 时给出实算哈希")
        proc = run_gate("mismatch", tmp_root)
        out = proc.stdout + proc.stderr
        check("退出码 = 1（FAIL，不是 0 也不是 2）", proc.returncode == 1,
              f"got {proc.returncode}")
        check("输出含实算哈希（64 位）", COMPUTED_HASH in out,
              "computed hash not found in the gate output")
        check("输出含基线哈希（64 位）", BASE_HASH in out,
              "baseline hash not found in the gate output")
        check("输出含「computed vs baseline」诊断段",
              "computed vs baseline" in out)
        check("输出含定位命令 v380_candidate_dump",
              "v380_candidate_dump" in out)
        check("既有 tail 输出仍在（test result 行）",
              "test result: FAILED. 48 passed; 2 failed" in out)

        # ---- C. 门禁侧：PASS 路径一字未变 ----
        print("C. PASS 路径与三态语义不变")
        proc = run_gate("pass", tmp_root)
        out = proc.stdout + proc.stderr
        check("PASS ⇒ 退出码 0", proc.returncode == 0, f"got {proc.returncode}")
        check("PASS ⇒ 输出含 G1 PASS", "G1 PASS" in out)
        check("PASS ⇒ 不含诊断段（没有失败就不该有）",
              "computed vs baseline" not in out)

        # ---- D. 门禁侧：NOT-RUN 仍是 INCOMPLETE，不得说成 hash mismatch ----
        print("D. NOT-RUN 三态不变")
        proc = run_gate("notrun", tmp_root)
        out = proc.stdout + proc.stderr
        check("NOT-RUN ⇒ 退出码 2（INCOMPLETE）", proc.returncode == 2,
              f"got {proc.returncode}")
        check("NOT-RUN ⇒ 输出含 INCOMPLETE", "INCOMPLETE" in out)
        check("NOT-RUN ⇒ 不得出现 hash mismatch", "hash mismatch" not in out)
        check("NOT-RUN ⇒ 点名原因 fixture", "fixture" in out.lower())
    finally:
        shutil.rmtree(tmp_root, ignore_errors=True)

    print()
    if FAILURES:
        print(f"FAILED ({len(FAILURES)}): {', '.join(FAILURES)}")
        return 1
    print("OK: G1 诊断夹具全部通过")
    return 0


if __name__ == "__main__":
    sys.exit(main())
