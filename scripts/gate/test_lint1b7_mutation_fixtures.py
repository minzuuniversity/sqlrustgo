#!/usr/bin/env python3
"""LINT-1 批次 7 的反向变异夹具（2026-11-09，看板行 LINT-1）。

批次 7 把 `clippy::needless_borrow`（"the borrowed expression implements the required traits"）
从 4 处清零：全部在 `crates/storage/src/io_delay.rs` 的测试里，
形态是 `from_lookup(&lookup(…))`（`from_lookup<F: Fn(…)>` 按值收）⇒ 去掉外层 `&`。

三臂（计数管线与归档 run 的 inventory 臂**同一条**：默认 lints，按类文本计数）：
  A) 现状实测：storage crate 全目标该类 = 0；
  M) 往文件末尾注入「`from_lookup(&get)`」闭包重新绑定的镜像反模式探针，
     计数必须回弹 ≥1 且点名注入文件；
  E) 恢复后复跑：回到 0，恢复逐字节一致 + 显式 bump mtime。

为什么不用批次 2–6 的 `-A clippy::all -W <lint>` 过滤配方：本批实测（clippy 1.97）该配方
对 needless_borrow 这一类**不触发**（过滤跑 0 条、默认跑 1 条，同树同命令其余部分一字不差），
group-allow 把这条 specific-warn 吞了。计数判据改用与 inventory 一致的默认 lints 管线，
按类首文本精确计数——判据口径反而与归档 run 完全同源。
每处读数对着当前源码核：报出来的那一行必须真的还带 `(&` 传参形态，
否则按陈旧诊断剔掉。命名 test_* ⇒ 不占 `check_*` glob ⇒ 分母 168 不动。
"""
from __future__ import annotations

import os
import re
import shutil
import subprocess
import sys
import tempfile

TARGET_FILE = "crates/storage/src/io_delay.rs"
TARGET = "io_delay"
CRATE = "sqlrustgo-storage"
CLASS_HEAD = "the borrowed expression implements the required traits"

PROBE = """
#[allow(dead_code)]
fn lint1b7_probe_shape() {
    let pairs: [(&str, &str); 0] = [];
    let get = |k: &str| pairs.iter().find(|(a, _)| *a == k).map(|(_, b)| (*b).to_string());
    let _ = IoDelayConfig::from_lookup(&get);
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
        if "(&" not in cur:
            stale.append(f"{path}:{lineno} 当前该行不再带 (& 传参形态：{cur!r}")
            continue
        ok.append(f"{path}:{lineno} {cur}")
    return ok, stale


def count_class(scope: list[str] | None = None) -> tuple[int, str]:
    """跑默认 lints 的 clippy（与归档 run inventory 同一管线），返回该类**在当前源码里成立**
    的处数 + 原始输出。按类首文本精确计数，不做任何 allow/warn 过滤（实测该过滤对本类失效）。
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
        if ln.startswith("warning: " + CLASS_HEAD):
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

    backup = os.path.join(tempfile.mkdtemp(prefix="lint1b7-"), "orig.rs")
    shutil.copy2(TARGET_FILE, backup)

    try:
        print("== A) 基线：该类必须实测为 0 ==")
        os.utime(TARGET_FILE, None)
        base, _ = count_class()
        print(f"   基线计数 = {base}")
        if base != 0:
            fail(f"批次 7 后该类应为 0，实测 {base} ⇒ 未清零")

        print("== M) 注入反模式探针（from_lookup(&get) 闭包重新绑定）==")
        original = open(TARGET_FILE, encoding="utf-8").read()
        with open(TARGET_FILE, "w", encoding="utf-8") as f:
            f.write(original + PROBE)
        if open(TARGET_FILE, encoding="utf-8").read() == original:
            fail("注入未改动文件（空转）⇒ 夹具无效")
        mutated, err = count_class(scope=["--lib"])
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
        after, _ = count_class(scope=["--lib"])
        print(f"   恢复后计数 = {after}")
        if after != 0:
            fail(f"恢复后应回到 0，实测 {after}")
    finally:
        shutil.copy2(backup, TARGET_FILE)
        os.utime(TARGET_FILE, None)

    print("OK: 计数判据有鉴别力（基线 0；注入反模式即回弹并点名文件；恢复后回 0）")


if __name__ == "__main__":
    main()
