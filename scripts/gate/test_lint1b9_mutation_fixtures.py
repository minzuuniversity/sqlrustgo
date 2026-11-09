#!/usr/bin/env python3
"""LINT-1 批次 9 反向变异夹具(2026-11-09)。

批次 9 清掉的是 16 个纯语法/机械类(长尾 A 组,26 处)。判据口径与归档 run inventory 臂
同源:默认 lints 全 workspace 管线,按类首文本精确计数,不做任何 allow/warn 过滤。

三站:
  A) 基线:16 个类首文本在整树计数必须全 0(批次 9 后)。
  M) 注入:往 `crates/storage/src/io_delay.rs` 的 `mod tests` 内注入**三种代表形态**探针
     (manual RangeInclusive::contains / length comparison to one / unwrap-on-Ok literal),
     对应三个类首文本的计数必须各回弹 ≥1,且点名注入文件。
  E) 恢复:逐字节还原后复跑,回到全 0。

陈旧诊断剔除:每处读数的诊断渲染片段(`NNNN | code`)必须与当前文件该行逐字一致
(批次 7/8 实测 bump mtime 挡不住缓存重放,片段对质是唯一硬判据)。
"""
import os
import re
import subprocess
import sys
import tempfile

TARGET_FILE = "crates/storage/src/io_delay.rs"
ANCHOR = "mod tests {"

CLASS_HEADS = [
    "used `unwrap()` on `Ok` value",
    "used `unwrap_err()` on `Err` value",
    "unnecessary use of `get(&tx_id).is_none()`",
    "unnecessary use of `get(&TxId::new(2)).is_none()`",
    "casting to the same type is unnecessary (`i32` -> `i32`)",
    "casting to the same type is unnecessary (`u64` -> `u64`)",
    "useless conversion to the same type: `std::string::String`",
    "length comparison to one",
    "manual `RangeInclusive::contains` implementation",
    "manual `Range::contains` implementation",
    "using `.clone()` on a double reference, which returns `&str` instead of cloning the inner type",
    "this expression creates a reference which is immediately dereferenced by the compiler",
    "redundant redefinition of a binding `scan_time`",
    "duplicated attribute",
    "can be more succinctly written as a byte str",
    "writing `&mut Vec` instead of `&mut [_]` involves a new object where a slice will do",
]

PROBE = """
    #[test]
    fn lint1b9_probe_shape() {
        // 与批次 9 清掉的三处代表形态完全同构:
        let s = 2usize;
        assert!(s >= 1 && s <= 4); // manual `RangeInclusive::contains`
        let v: Vec<u8> = vec![7];
        assert!(v.len() >= 1); // length comparison to one
        let r: Result<i32, String> = Ok(9);
        assert_eq!(r.unwrap(), 9); // used `unwrap()` on `Ok` value
    }
"""

PROBE_CLASSES = [
    "manual `RangeInclusive::contains` implementation",
    "length comparison to one",
    "used `unwrap()` on `Ok` value",
]


def _snippet_ok(path: str, lineno: int, snippet: str) -> bool:
    if not os.path.isfile(path):
        return False
    lines = open(path, encoding="utf-8").read().splitlines()
    if lineno > len(lines):
        return False
    return lines[lineno - 1].strip() == snippet.strip()


def count_class() -> tuple[dict, list]:
    """默认 lints 管线(与归档 inventory 同一命令),返回 {类首文本: 现行实锤处数}。

    状态机:类首行 ⇒ 记类;任意缩进 `--> path:line:col` ⇒ 记位;随后**首条**同号片段行
    ⇒ 成对。其余 `warning:` 行复位状态。读数按片段对质核新鲜度,陈旧重放剔掉。
    """
    proc = subprocess.run(
        [
            "cargo",
            "clippy",
            "--workspace",
            "--all-features",
            "--all-targets",
            "--keep-going",
        ],
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        fail(f"clippy 自身 rc={proc.returncode},计数不可用:{(proc.stderr or proc.stdout)[-300:]}")
    out = proc.stderr + proc.stdout
    pairs: list[tuple[str, int, str, str]] = []
    cur_head: str | None = None
    loc: tuple[str, int] | None = None
    for ln in out.splitlines():
        if ln.startswith("warning: "):
            head = re.sub(r"\s+\[.*$", "", ln[len("warning: ") :].strip())
            cur_head = head if head in CLASS_HEADS else None
            loc = None
            continue
        if cur_head is not None and loc is None:
            m = re.match(r"\s*-->\s+(\S+):(\d+):\d+", ln)
            if m:
                loc = (m.group(1), int(m.group(2)))
            continue
        if loc is not None:
            m = re.match(r"\s*(\d+) \| (.*)$", ln)
            if m and int(m.group(1)) == loc[1]:
                pairs.append((cur_head, loc[0], loc[1], m.group(2)))
                loc = None
    counts: dict = {h: 0 for h in CLASS_HEADS}
    sites = []
    for head, path, lineno, snippet in pairs:
        if not _snippet_ok(path, lineno, snippet):
            print(f"   陈旧诊断核掉:{path}:{lineno} {head!r}")
            continue
        counts[head] += 1
        sites.append((head, path, lineno))
    return counts, sites


def fail(msg: str) -> None:
    print(f"FAIL: {msg}")
    sys.exit(1)


def main() -> None:
    if not os.path.isfile(TARGET_FILE):
        fail(f"目标文件不存在:{TARGET_FILE}(必须在仓库根运行)")

    backup = os.path.join(tempfile.mkdtemp(prefix="lint1b9-"), "orig.rs")
    with open(TARGET_FILE, "rb") as f:
        orig = f.read()
    with open(backup, "wb") as f:
        f.write(orig)

    try:
        print("== A) 基线:16 类必须全 0 ==")
        base, _sites = count_class()
        nonzero = {k: v for k, v in base.items() if v}
        print(f"   基线非零类 = {nonzero or '无'}")
        if nonzero:
            fail(f"批次 9 后这 15 类应为 0,实测 {nonzero}")

        print("== M) 注入三形态代表探针(manual contains / length>=1 / unwrap-on-Ok) ==")
        src = orig.decode("utf-8")
        idx = src.index(ANCHOR) + len(ANCHOR)
        open(TARGET_FILE, "w", encoding="utf-8").write(src[:idx] + PROBE + src[idx:])
        os.utime(TARGET_FILE, None)
        mid, msites = count_class()
        bad = [c for c in PROBE_CLASSES if mid.get(c, 0) < 1]
        if bad:
            fail(f"注入后这些类未回弹:{bad};实测 {mid}")
        named = {p for _h, p, _l in msites if _h in PROBE_CLASSES}
        if named != {TARGET_FILE}:
            fail(f"回弹读数应点名注入文件 {TARGET_FILE},实测点名为 {named}")
        print(f"   注入后回弹:{ {c: mid[c] for c in PROBE_CLASSES} },点名 {named}")

        print("== E) 恢复后复跑 ==")
        with open(TARGET_FILE, "wb") as f:
            f.write(orig)
        os.utime(TARGET_FILE, None)
        end, _s2 = count_class()
        nonzero = {k: v for k, v in end.items() if v}
        if nonzero:
            fail(f"恢复后应回 0,实测 {nonzero}")
        now = open(TARGET_FILE, "rb").read()
        if now != orig:
            fail("恢复后文件与原始不一致(非逐字节)")
        print("OK: 判据有鉴别力(16 类基线 0;注入三形态各回弹 ≥1;恢复回 0 且逐字节一致)")
    finally:
        with open(TARGET_FILE, "wb") as f:
            f.write(orig)


if __name__ == "__main__":
    main()
