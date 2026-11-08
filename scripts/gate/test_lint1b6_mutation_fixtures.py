#!/usr/bin/env python3
"""LINT-1 批次 6 的反向变异夹具（2026-11-08，看板行 LINT-1）。

批次 6 把 `clippy::manual_clamp`（"clamp-like pattern without using clamp function"）
从 5 处清零：全部在 `crates/bench/examples/serial_vs_parallel_bench.rs`，
形态是 `.min(H).max(L)`（H > L）⇒ 改 `.clamp(L, H)`。

三臂（口径与批次 2/3/4/5 夹具一致；三个空转坑逐条避开）：
  A) 现状实测：bench crate（含 examples 全目标）该类 = 0；
  M) 往既有 example 文件注入反模式探针（一个 `x.min(9).max(2)`），
     计数必须回弹 ≥1 且点名注入文件；
  E) 恢复后复跑：回到 0，恢复逐字节一致 + 显式 bump mtime。

每处读数对着当前源码核：报出来的那一行必须真的还带 `.min(` + `.max(` 形态，
否则按陈旧诊断剔掉。命名 test_* ⇒ 不占 `check_*` glob ⇒ 分母 168 不动。
"""
from __future__ import annotations

import os
import re
import shutil
import subprocess
import sys
import tempfile

TARGET_FILE = "crates/bench/examples/serial_vs_parallel_bench.rs"
# example 的 target 名 = 文件去扩展名（注入/恢复两臂的口径）
TARGET = "serial_vs_parallel_bench"
CRATE = "sqlrustgo-bench"
LINT = "clippy::manual_clamp"
CLASS_HEAD = "clamp-like pattern without using clamp function"

PROBE = """
#[allow(dead_code)]
fn lint1b6_probe_shape(x: usize) -> usize {
    x.min(9).max(2)
}
"""


def current_instances(pairs: list[tuple[str, int]]) -> tuple[list[str], list[str]]:
    ok, stale = [], []
    for path, lineno in pairs:
        if not os.path.isfile(path):
            stale.append(f"{path}:{lineno} 文件不存在")
            continue
        lines = open(path, encoding="utf-8").read().splitlines()
        if lineno > len(lines):
            stale.append(f"{path}:{lineno} 行号超出当前文件长度 {len(lines)} ⇒ 陈旧诊断")
            continue
        cur = lines[lineno - 1].strip()
        if ".min(" not in cur or ".max(" not in cur:
            stale.append(f"{path}:{lineno} 当前该行不再带 .min/.max 成对形态：{cur!r}")
            continue
        ok.append(f"{path}:{lineno} {cur}")
    return ok, stale


def count_class(scope: list[str] | None = None) -> tuple[int, str]:
    """跑 clippy（只开目标 lint），返回该类**在当前源码里成立**的处数 + 原始输出。

    三条硬规定（批次 2 实测的坑）：`-A clippy::all -W <类>`；scope 是参数列表；
    读数对着当前源码核。
    """
    proc = subprocess.run(
        [
            "cargo",
            "clippy",
            "-p",
            CRATE,
            *(scope or ["--all-targets"]),
            "--all-features",
            "--keep-going",
            "--",
            "-A",
            "clippy::all",
            "-W",
            LINT,
        ],
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        fail(f"clippy 自身 rc={proc.returncode}，计数不可用：{(proc.stderr or proc.stdout)[-300:]}")
    out = proc.stderr + proc.stdout
    pairs: list[tuple[str, int]] = []
    pending = False
    for ln in out.splitlines():
        if ln.startswith("warning:") and CLASS_HEAD in ln:
            pending = True
            continue
        if pending:
            m = re.match(r"\s*-->\s+(\S+):(\d+):\d+", ln)
            if m:
                pairs.append((m.group(1), int(m.group(2))))
                pending = False
    ok, stale = current_instances(pairs)
    for s in stale:
        print(f"   陈旧诊断核掉：{s}")
    return len(ok), out


def fail(msg: str) -> None:
    print(f"FAIL: {msg}")
    sys.exit(1)


def main() -> None:
    if not os.path.isfile(TARGET_FILE):
        fail(f"目标文件不存在：{TARGET_FILE}（必须在仓库根运行）")

    backup = os.path.join(tempfile.mkdtemp(prefix="lint1b6-"), "orig.rs")
    shutil.copy2(TARGET_FILE, backup)

    try:
        print("== A) 基线：该类必须实测为 0 ==")
        os.utime(TARGET_FILE, None)
        base, _ = count_class()
        print(f"   基线计数 = {base}")
        if base != 0:
            fail(f"批次 6 后该类应为 0，实测 {base} ⇒ 未清零")

        print("== M) 注入反模式探针（x.min(9).max(2)）==")
        original = open(TARGET_FILE, encoding="utf-8").read()
        with open(TARGET_FILE, "w", encoding="utf-8") as f:
            f.write(original + PROBE)
        if open(TARGET_FILE, encoding="utf-8").read() == original:
            fail("注入未改动文件（空转）⇒ 夹具无效")
        mutated, err = count_class(scope=["--example", TARGET])
        print(f"   注入后计数 = {mutated}")
        if mutated < 1:
            fail("注入反模式后计数没有回弹 ⇒ 计数命令对这个反模式**没有鉴别力**")
        if TARGET_FILE not in err:
            fail(f"计数没有点名注入文件 {TARGET_FILE}")
        print(f"   点名到位：{TARGET_FILE}")

        print("== E) 恢复后复跑（确认恢复干净）==")
        shutil.copy2(backup, TARGET_FILE)
        if open(TARGET_FILE, "rb").read() != open(backup, "rb").read():
            fail("恢复后文件与备份不一致")
        os.utime(TARGET_FILE, None)
        after, _ = count_class(scope=["--example", TARGET])
        print(f"   恢复后计数 = {after}")
        if after != 0:
            fail(f"恢复后应回到 0，实测 {after}")
    finally:
        shutil.copy2(backup, TARGET_FILE)

    print("OK: 计数判据有鉴别力（基线 0；注入反模式即回弹并点名文件；恢复后回 0）")


if __name__ == "__main__":
    main()
