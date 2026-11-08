#!/usr/bin/env python3
"""LINT-1 批次 4 的反向变异夹具（2026-11-08，看板行 LINT-1）。

批次 4 把 `clippy::ptr_arg`（"writing `&PathBuf` instead of `&Path` involves a new object
where a copy would be sufficient"）从 11 处清零：全部是 `fn f(p: &PathBuf)` 参数声明
（`crates/storage/examples/tpch_import.rs` 7、`crates/storage/tests/multi_db_crash_recovery_5192.rs` 2、
`crates/storage/tests/repro_5250_autocommit_commit.rs` 1、`crates/storage/benches/storage_benchmark.rs` 1）。

三臂（口径与批次 2/3 夹具一致；三个空转坑——标志组合、argv 拆分、缓存重放——逐条避开）：
  A) 现状实测：storage crate（含 tests/examples/benches 全目标）该类 = 0；
  M) 往既有 test target 注入反模式探针（`fn …(&PathBuf)` 且函数体只用 `join`/`display`
     ——正是 ptr_arg 认定「改 `&Path` 就够」的形态），计数必须回弹 ≥1 且点名注入文件；
  E) 恢复后复跑：回到 0，恢复逐字节一致 + 显式 bump mtime。

每处读数对着当前源码核：报出来的那一行必须真含 `&PathBuf`，否则按陈旧诊断剔掉。
命名 test_* ⇒ 不占 `scripts/gate/check_*.sh` 的 glob ⇒ 门禁分母 168 不动。
"""
from __future__ import annotations

import os
import re
import shutil
import subprocess
import sys
import tempfile

TARGET_FILE = "crates/storage/tests/pk_rebuild_scoped_5592.rs"
TARGET = "pk_rebuild_scoped_5592"
CRATE = "sqlrustgo-storage"
LINT = "clippy::ptr_arg"
CLASS_HEAD = "writing `&PathBuf` instead of `&Path`"

PROBE = """
fn lint1b4_probe_sink(p: &std::path::PathBuf) -> String {
    p.join("nested-under-probe").display().to_string()
}

#[test]
fn lint1b4_mutation_probe_ptr_arg_pathbuf() {
    let pb = std::path::PathBuf::from("lint1b4-probe-root");
    assert!(lint1b4_probe_sink(&pb).contains("nested-under-probe"));
}
"""


def current_instances(pairs: list[tuple[str, int]]) -> tuple[list[str], list[str]]:
    """把 clippy 报的每一处**对着当前源码**核一遍（防缓存重放）。"""
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
        if "PathBuf" not in cur or "&" not in cur:
            stale.append(f"{path}:{lineno} 当前该行不是 `&…PathBuf` 参数：{cur!r}")
            continue
        ok.append(f"{path}:{lineno} {cur}")
    return ok, stale


def count_class(scope: list[str] | None = None) -> tuple[int, str]:
    """跑 clippy（只开目标 lint），返回该类**在当前源码里成立**的处数 + 原始输出。

    三条硬规定（批次 2 实测出来的坑，这里逐条避开）：
    1. `-A clippy::all -W <目标类>`（`-A warnings` 会把后面的 `-W` 吃掉 ⇒ 恒 0 假绿）。
    2. scope 是参数列表（`["--test", name]`），不能把 `"--test foo"` 当一个 argv。
    3. 读数对着当前源码核（`current_instances`），否则测的是缓存重放。
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

    backup = os.path.join(tempfile.mkdtemp(prefix="lint1b4-"), "orig.rs")
    shutil.copy2(TARGET_FILE, backup)

    try:
        print("== A) 基线：该类必须实测为 0 ==")
        os.utime(TARGET_FILE, None)
        base, _ = count_class()
        print(f"   基线计数 = {base}")
        if base != 0:
            fail(f"批次 4 后该类应为 0，实测 {base} ⇒ 未清零")

        print("== M) 注入反模式探针（fn …(&PathBuf)，函数体只用 join/display）==")
        original = open(TARGET_FILE, encoding="utf-8").read()
        with open(TARGET_FILE, "w", encoding="utf-8") as f:
            f.write(original + PROBE)
        if open(TARGET_FILE, encoding="utf-8").read() == original:
            fail("注入未改动文件（空转）⇒ 夹具无效")
        mutated, err = count_class(scope=["--test", TARGET])
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
        after, _ = count_class(scope=["--test", TARGET])
        print(f"   恢复后计数 = {after}")
        if after != 0:
            fail(f"恢复后应回到 0，实测 {after}")
    finally:
        shutil.copy2(backup, TARGET_FILE)

    print("OK: 计数判据有鉴别力（基线 0；注入反模式即回弹并点名文件；恢复后回 0）")


if __name__ == "__main__":
    main()
