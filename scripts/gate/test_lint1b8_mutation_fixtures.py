#!/usr/bin/env python3
"""LINT-1 批次 8 的反向变异夹具（2026-11-09，看板行 LINT-1）。

批次 8 把 rustc 的 `unused_must_use` 类（clippy inventory 里显示为
"unused `std::result::Result` that must be used"）从 6 处清零：全部在测试里，
形态是吞掉返回 Result 的调用——`storage.insert("users", vec![]);`、
spawn 闭包尾 `c.commit_lsn(i)` 配 `h.join().unwrap()`（单层解包：只解 JoinHandle，
Result 被丢 ⇒ 提交失败测试仍绿，真缺陷）、`c.set_default_schema("main".to_string());`。
修法是测试加固（错误必须浮出），不是把警告消掉。

三臂（计数管线与归档 run 的 inventory 臂**同一条**：默认 lints，按类文本计数——
本类是 rustc lint，`-A clippy::all` 过滤配方对它同样无效，批次 7 的教训直接适用）：
  A) 现状实测：storage + catalog 两 crate 全目标该类 = 0；
  M) 往 group_commit.rs 的 `mod tests` 内注入**修复前形态的镜像探针**（spawn 闭包尾
     返回 Result + 单层 join().unwrap()），计数必须回弹 ≥1 且点名注入文件；
  E) 恢复后复跑：回到 0，恢复逐字节一致 + 显式 bump mtime。

陈旧诊断核：诊断渲染块里的源码片段必须与当前文件该行逐字一致才算数（批次 7 实测
bump mtime 挡不住缓存诊断重放，片段对质是唯一硬判据）。命名 test_* ⇒ 不占
`check_*` glob ⇒ 分母 168 不动。
"""
from __future__ import annotations

import os
import re
import shutil
import subprocess
import sys
import tempfile

TARGET_FILE = "crates/storage/src/wal/group_commit.rs"
TARGET = "group_commit"
CRATES = ["-p", "sqlrustgo-storage", "-p", "sqlrustgo-catalog"]
CLASS_HEAD = "unused `std::result::Result` that must be used"

# 修复前形态的镜像：spawn 闭包尾返回 Result，join 臂单层 unwrap ⇒ Result 被丢。
PROBE = """
    #[allow(dead_code)]
    fn lint1b8_probe_shape(coord: std::sync::Arc<GroupCommitCoordinator<CountingWal>>) {
        let handles: Vec<_> = (0..2)
            .map(|i| {
                let c = coord.clone();
                std::thread::spawn(move || c.commit_lsn(i as u64))
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
    }
"""

# 陈旧诊断核法：clippy 渲染块里的源码片段（`NNNN | code`）必须与当前文件该行**逐字一致**
# 才算现行实锤——批次 7 实测 bump mtime 也挡不住缓存诊断重放，片段对质是唯一硬判据。
def _snippet_ok(path: str, lineno: int, snippet: str) -> bool:
    if not os.path.isfile(path):
        return False
    lines = open(path, encoding="utf-8").read().splitlines()
    if lineno > len(lines):
        return False
    return lines[lineno - 1].strip() == snippet.strip()


def current_instances(pairs: list[tuple[str, int, str]]) -> tuple[list[str], list[str]]:
    ok, stale = [], []
    for path, lineno, snippet in pairs:
        if not _snippet_ok(path, lineno, snippet):
            stale.append(f"{path}:{lineno} 当前该行与诊断片段不一致（重放/陈旧）：{snippet!r}")
            continue
        ok.append(f"{path}:{lineno} {snippet}")
    return ok, stale


def count_class() -> tuple[int, str]:
    """跑默认 lints 的 clippy（与归档 run inventory 同一管线），返回该类**在当前源码里成立**
    的处数 + 原始输出。按类首文本精确计数，不做任何 allow/warn 过滤（本类是 rustc lint，
    过滤配方无效）。
    """
    proc = subprocess.run(
        [
            "cargo",
            "clippy",
            *CRATES,
            "--all-targets",
            "--all-features",
            "--keep-going",
        ],
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        fail(f"clippy 自身 rc={proc.returncode}，计数不可用：{(proc.stderr or proc.stdout)[-300:]}")
    out = proc.stderr + proc.stdout
    # 状态机：类首行 ⇒ 记类；任意缩进的 `--> path:line:col` ⇒ 记位；随后**首条**同号片段行
    # ⇒ 成对。其余 `warning:` 行复位状态（防串类）。渲染缩进宽度可变，故只按 \s* 匹配。
    pairs: list[tuple[str, int, str]] = []
    want_loc = False
    loc: tuple[str, int] | None = None
    for ln in out.splitlines():
        if ln.startswith("warning: "):
            want_loc = ln.startswith("warning: " + CLASS_HEAD)
            loc = None
            continue
        if want_loc and loc is None:
            m = re.match(r"\s*-->\s+(\S+):(\d+):\d+", ln)
            if m:
                loc = (m.group(1), int(m.group(2)))
            continue
        if loc is not None:
            m = re.match(r"\s*(\d+) \| (.*)$", ln)
            if m and int(m.group(1)) == loc[1]:
                pairs.append((loc[0], loc[1], m.group(2)))
                loc = None
                want_loc = False
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

    backup = os.path.join(tempfile.mkdtemp(prefix="lint1b8-"), "orig.rs")
    shutil.copy2(TARGET_FILE, backup)

    try:
        print("== A) 基线：该类必须实测为 0 ==")
        os.utime(TARGET_FILE, None)
        base, _ = count_class()
        print(f"   基线计数 = {base}")
        if base != 0:
            fail(f"批次 8 后该类应为 0，实测 {base} ⇒ 未清零")

        print("== M) 注入反模式探针（闭包尾 Result + 单层 join().unwrap()）==")
        original = open(TARGET_FILE, encoding="utf-8").read()
        # 探针注入 mod tests 内部：CountingWal 等测试私有符号只在该作用域可见。
        anchor = "mod tests {"
        pos = original.index(anchor) + len(anchor)
        mutated_src = original[:pos] + PROBE + original[pos:]
        with open(TARGET_FILE, "w", encoding="utf-8") as f:
            f.write(mutated_src)
        if open(TARGET_FILE, encoding="utf-8").read() == original:
            fail("注入未改动文件（空转）⇒ 夹具无效")
        mutated, err = count_class()
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
        after, _ = count_class()
        print(f"   恢复后计数 = {after}")
        if after != 0:
            fail(f"恢复后应回到 0，实测 {after}")
    finally:
        shutil.copy2(backup, TARGET_FILE)
        os.utime(TARGET_FILE, None)

    print("OK: 计数判据有鉴别力（基线 0；注入反模式即回弹并点名文件；恢复后回 0）")


if __name__ == "__main__":
    main()
