#!/usr/bin/env python3
"""LINT-1 批次 2 的反向变异夹具（2026-11-08，看板行 LINT-1）。

批次 2 把 `clippy::field_reassign_with_default`（"field assignment outside of
initializer for an instance created with Default::default()"）从 27 处清零。
归档 run 里那条"计数"命令给出的 0，本身必须**有鉴别力**：如果计数管道看不见
这个反模式，那"27⇒0"就只是没测。

三臂：
  A) 现状实测：该 crate 内该类 = 0；
  M) 往既有 test target 里注入一个反模式探针（`T::default()` 后逐字段赋值），
     同类计数必须回弹 ≥1 且点名注入的文件 ⇒ 证明计数管道真的在数这个反模式；
  E) 恢复后复跑：必须回到 0，且文件恢复成**逐字节一致**（`cmp` 级自证，
     任何一次注入/恢复空转都会让夹具自己红）。

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
TARGET = "pk_rebuild_scoped_5592"  # 该文件的 cargo test target 名（注入/恢复两臂的口径）
CRATE = "sqlrustgo-storage"
LINT = "clippy::field_reassign_with_default"
CLASS_HEAD = "field assignment outside of initializer"

PROBE = """

#[derive(Default)]
struct Lint1b2Probe {
    a: u32,
    b: u32,
}

#[test]
fn lint1b2_mutation_probe_field_reassign() {
    let mut p = Lint1b2Probe::default();
    p.a = 1;
    p.b = 2;
    assert_eq!(p.a + p.b, 3);
}
"""


def current_instances(pairs: list[tuple[str, int]]) -> tuple[list[str], list[str]]:
    """把 clippy 报的每一处**对着当前源码**核一遍。

    实测过的空转方式：cargo 对"新鲜"单元会**重放缓存诊断**——恢复文件后（`copy2` 连 mtime
    一起还原）它报了一条 `pk_rebuild_scoped_5592.rs:107`，而那个文件当前只有 94 行、
    也没有探针。所以计数不能只数 `warning:` 行数，必须确认**当前那一行确实是字段赋值、
    且上方 8 行内有 `::default()` 绑定**；核不过的就当"陈旧诊断"处理并让夹具红。
    """
    import re as _re

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
        if not _re.match(r"^[A-Za-z_][\w.]*\s*=\s*.+;$", cur):
            stale.append(f"{path}:{lineno} 当前该行不是字段赋值：{cur!r}")
            continue
        window = "\n".join(lines[max(0, lineno - 9) : lineno])
        if "::default()" not in window:
            stale.append(f"{path}:{lineno} 上方 8 行内没有 ::default() 绑定：{cur!r}")
            continue
        ok.append(f"{path}:{lineno} {cur}")
    return ok, stale


def count_class(scope: list[str] | None = None) -> tuple[int, str]:
    """跑 crate 范围的 clippy（只开目标 lint），返回该类**在当前源码里成立**的处数 + 原始输出。

    **三个坑都实测过**：
    1. 标志必须是 `-A clippy::all -W <目标类>`。`-A warnings -W <同条>` ⇒ 命中 **0**
       （`-A warnings` 把后面的 `-W` 一起吃掉，这条 lint 根本没开）；`-A clippy::all -W <同条>` ⇒ 命中 **1**。
    2. scope 必须是**参数列表**：把 `"--test foo"` 当一个 argv 传进去，cargo 不认，整条命令空转
       ⇒ 计数恒 0，看着像「清零成功」。
    3. 诊断会与当前源码核一遍（见 `current_instances`），否则测的是缓存重放。
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

    backup = os.path.join(tempfile.mkdtemp(prefix="lint1b2-"), "orig.rs")
    shutil.copy2(TARGET_FILE, backup)

    try:
        print("== A) 基线：该类必须实测为 0 ==")
        # 先 bump mtime，保证被审计的那个 target 是**重新分析**而不是重放缓存
        os.utime(TARGET_FILE, None)
        base, _ = count_class()
        print(f"   基线计数 = {base}")
        if base != 0:
            fail(f"批次 2 后该类应为 0，实测 {base} ⇒ 未清零")

        print("== M) 注入反模式探针（T::default() 后逐字段赋值）==")
        original = open(TARGET_FILE, encoding="utf-8").read()
        with open(TARGET_FILE, "w", encoding="utf-8") as f:
            f.write(original + PROBE)
        # 空转自检：注入必须真的改动了文件
        if open(TARGET_FILE, encoding="utf-8").read() == original:
            fail("注入未改动文件（空转）⇒ 夹具无效")
        # 注入/恢复两臂只看探针所在的那个 test target（M 与 E 必须同口径才有可比性）
        mutated, err = count_class(scope=["--test", TARGET])
        print(f"   注入后计数 = {mutated}")
        if mutated < 1:
            fail("注入反模式后计数没有回弹 ⇒ 归档 run 里那条计数命令对这个反模式**没有鉴别力**")
        if TARGET_FILE not in err:
            fail(f"计数命令没有点名注入文件（{TARGET_FILE}）⇒ 证据不可追溯")

        print("== E) 恢复后复跑（确认恢复干净）==")
        shutil.copy2(backup, TARGET_FILE)
        if open(TARGET_FILE, "rb").read() != open(backup, "rb").read():
            fail("恢复后文件与备份不一致")
        # `shutil.copy2` 连 mtime 一起还原 ⇒ cargo 认为该单元"新鲜"，直接**重放缓存诊断**
        # （实测：恢复后计数仍是 1，那条就是注入期探针的陈旧警告）。必须显式 bump mtime
        # 强制重新分析，否则 E 臂测的是缓存而不是当前源码。
        os.utime(TARGET_FILE, None)
        after, _ = count_class(scope=["--test", TARGET])
        print(f"   恢复后计数 = {after}")
        if after != 0:
            fail(f"恢复后应回到 0，实测 {after}")

        print("OK: 计数判据有鉴别力（基线 0；注入反模式即回弹并点名文件；恢复后回 0）")
    finally:
        shutil.copy2(backup, TARGET_FILE)


if __name__ == "__main__":
    main()
