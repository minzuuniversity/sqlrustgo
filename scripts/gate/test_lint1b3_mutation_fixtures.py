#!/usr/bin/env python3
"""LINT-1 批次 3 的反向变异夹具（2026-11-08，看板行 LINT-1）。

批次 3 把 `clippy::useless_vec`（"useless use of `vec!`"）从 21 处清零：
`crates/storage/src/engine.rs` 12 处 + `crates/storage/src/page.rs` 4 处是调用实参的
`&vec![…]`（改 `&[…]`），`crates/storage/src/bplus_tree/index.rs` 5 处是 `let x = vec![…]`
绑定（改数组字面量）。归档 run 里那条计数命令给出的 0 必须**有鉴别力**：
计数管道若看不见这个反模式，"21⇒0" 就只是没测。

三臂（口径与批次 2 夹具一致，三个空转坑都已避开）：
  A) 现状实测：该 crate 该类 = 0（跑之前先 bump mtime，保证被审计 target 是重新分析）；
  M) 往既有 test target 注入反模式探针（把 `&vec![…]` 传给收 `&[i32]` 的函数），
     同类计数必须回弹 ≥1 且点名注入文件；
  E) 恢复后复跑：回到 0，且恢复是**逐字节一致** + 显式 bump mtime
     （`shutil.copy2` 连 mtime 一起还原 ⇒ cargo 会重放陈旧诊断，批次 2 实测踩过）。

每一处读数都对着当前源码核一遍：报出来的那一行必须真的含 `vec![`，
否则按陈旧诊断剔掉（批次 2 实测报过一条行号超出文件长度的陈旧警告）。

命名 test_* ⇒ 不进 `scripts/gate/check_*.sh` 的 glob ⇒ 门禁分母 168 不动。
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
LINT = "clippy::useless_vec"
CLASS_HEAD = "useless use of `vec!`"

PROBE = """
fn lint1b3_probe_sink(xs: &[i32]) -> usize {
    xs.len()
}

#[test]
fn lint1b3_mutation_probe_useless_vec() {
    assert_eq!(lint1b3_probe_sink(&vec![1, 2, 3]), 3);
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
        if "vec![" not in cur:
            stale.append(f"{path}:{lineno} 当前该行不含 vec![：{cur!r}")
            continue
        ok.append(f"{path}:{lineno} {cur}")
    return ok, stale


def count_class(scope: list[str] | None = None) -> tuple[int, str]:
    """跑 clippy（只开目标 lint），返回该类**在当前源码里成立**的处数 + 原始输出。

    口径三条硬规定（批次 2 逐个实测出来的坑）：
    1. `-A clippy::all -W <目标类>`，不是 `-A warnings -W <目标类>`（后者把 `-W` 吃掉 ⇒ 恒 0）。
    2. scope 是**参数列表**（`["--test", name]`），不能把 `"--test foo"` 当一个 argv。
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

    backup = os.path.join(tempfile.mkdtemp(prefix="lint1b3-"), "orig.rs")
    shutil.copy2(TARGET_FILE, backup)

    try:
        print("== A) 基线：该类必须实测为 0 ==")
        os.utime(TARGET_FILE, None)
        base, _ = count_class()
        print(f"   基线计数 = {base}")
        if base != 0:
            fail(f"批次 3 后该类应为 0，实测 {base} ⇒ 未清零")

        print("== M) 注入反模式探针（把 &vec![…] 传给收 &[i32] 的函数）==")
        original = open(TARGET_FILE, encoding="utf-8").read()
        with open(TARGET_FILE, "w", encoding="utf-8") as f:
            f.write(original + PROBE)
        if open(TARGET_FILE, encoding="utf-8").read() == original:
            fail("注入未改动文件（空转）⇒ 夹具无效")
        mutated, err = count_class(scope=["--test", TARGET])
        print(f"   注入后计数 = {mutated}")
        if mutated < 1:
            fail("注入反模式后计数没有回弹 ⇒ 归档 run 里那条计数命令对这个反模式**没有鉴别力**")
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
