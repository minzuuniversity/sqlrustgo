#!/usr/bin/env python3
"""refactor_api_surface.py — 公开 API 面的快照与比对工具（**不是门禁**）。

用途
====
R1「契约先行」为 REF-01/02/03 的拆分定下的**最强验收判据**是
「拆分前后 `pub` 面逐字节不变」。本工具把那条判据变成**可执行**的：

    # 拆分前
    python3 audit/v4.1.0/refactor_api_surface.py snapshot \
        --files crates/executor/src/expr/mod.rs \
                crates/executor/src/stored_proc.rs \
                crates/executor/src/trigger.rs \
        --out /tmp/pub-before.txt

    # 拆分后
    python3 audit/v4.1.0/refactor_api_surface.py snapshot \
        --files crates/executor/src/expr/mod.rs \
                crates/executor/src/stored_proc.rs \
                crates/executor/src/trigger.rs \
        --out /tmp/pub-after.txt

    python3 audit/v4.1.0/refactor_api_surface.py diff \
        --before /tmp/pub-before.txt --after /tmp/pub-after.txt

**它为什么不是门禁**：本仓的门禁分母固定 168（`scripts/gate/check_*.sh` +
`check_*.py`），新判据一律**挂靠既有门禁**，不新增 `check_*`。新工具只放
`audit/v4.1.0/`（AGENTS.md 的约定）。本文件**不在** 168 分母里，也不改变任何
门禁的判定逻辑 —— 它是 R2 开工前的**人工执行**工具。

它抓什么
========
1. `pub` 项的**名字**（fn / struct / enum / trait / type / const / static）；
2. 每个 `pub` 项的**所属文件**（子模块化后名字会散到多个文件里，归属变了也算破坏）；
3. `pub` 项的**签名行**（fn 的参数与返回类型、type/struct 的定义头）——
   名字不变但签名变了**也是**破坏 API；
4. `pub(crate)` 可见性的**变化**（`pub` → `pub(crate)` 是**收窄**，
   会破坏外部调用方 ⇒ 必须报出来）。

它**不**抓什么
============
- 函数体内部逻辑（那是行为判据的事，靠特征测试）；
- `#[cfg(...)]` 条件编译分支内的项（按源码文本抓，不做 cfg 求值）。

用法
====
    snapshot --files <路径...> [--out <文件>]
    diff --before <文件> --after <文件>
    --selftest

退出码：0 = 通过；1 = 有差异（diff 模式）或自测失败；2 = 用法错。
"""

from __future__ import annotations

import argparse
import difflib
import os
import pathlib
import re
import sys

# --- 抓取规则 -----------------------------------------------------------------
#
# 一行一个 `pub` 项，格式：
#     <相对路径>\t<可见性>\t<种类>\t<名字>\t<签名>
#
# 签名取「从 `pub` 开始到行尾」的那一段，压掉空白后比较。

VIS_RE = re.compile(
    r"^\s*(pub(?:\([^)]*\))?)\s+"
    r"(async\s+|const\s+|unsafe\s+|extern\s+\"[^\"]*\"\s+)*"
    r"(fn|struct|enum|trait|type|const|static)\s+"
    r"([A-Za-z0-9_]+)"
    r"(.*)$"
)

# `impl` 块里的方法：`pub fn` 出现在缩进里，也要抓 —— 它是外部 API 的一部分。
IMPL_SCOPE_RE = re.compile(r"^\s*impl\b")
MOD_RE = re.compile(r"^\s*(pub(?:\([^)]*\))?\s+)?mod\s+([A-Za-z0-9_]+)\s*;")


def normalize_ws(text: str) -> str:
    """把签名的空白归一化，使**排版差异**不被当成 API 变更。

    rustfmt 与人手排版会在 `:` `,` `&` `(` `)` `->` 两侧留/不留空格，
    这些都不改变签名。归一化后 `( x : i32 )` 与 `(x: i32)` 必须相等。
    """
    t = re.sub(r"\s+", " ", text.strip())
    # 去掉标点两侧的空白：`: , & ( ) < > =` 与箭头
    t = re.sub(r"\s*([:,&()=<>])\s*", r"\1", t)
    t = re.sub(r"\s*->\s*", "->", t)
    # 归一化后可能把 `&` 与类型名粘在一起的可读性问题还原成有空格的形式
    t = re.sub(r"(&)([A-Za-z_])", r"\1 \2", t)
    return re.sub(r"\s+", " ", t).strip()


def iter_pub_items(lines):
    """产出 (可见性, 种类, 名字, 签名) 四元组。

    **多行签名**要拼起来：`pub fn f(\n    a: i32,\n) -> R {` 的签名是
    `(a: i32,) -> R`，不是 `(`。所以这里在括号配平前一直续行。
    """
    i = 0
    n = len(lines)
    while i < n:
        raw = lines[i]
        m = VIS_RE.match(raw)
        if not m:
            i += 1
            continue
        vis, _, kind, name, rest = m.groups()
        # 续行直到圆括号配平（fn 的参数表）或遇到 `;`（type/const 的定义头）
        buf = rest
        j = i
        while _unbalanced(buf) and j + 1 < n:
            j += 1
            buf += " " + lines[j]
        sig = normalize_ws(buf.split("{")[0].strip().rstrip(";").strip())
        yield (normalize_ws(vis), kind, name, sig)
        i = j + 1


def _unbalanced(text: str) -> bool:
    """fn 的参数表是否还没闭合（需要续行）。"""
    depth = text.count("(") - text.count(")")
    return depth > 0


def module_roots(path, root):
    """一个「门面文件」对应的模块根。

    - `.../expr/mod.rs` ⇒ 根是 `.../expr/`（子模块化后 pub 项落到其子文件里）；
    - `.../stored_proc.rs` ⇒ 根是**这个文件本身** + `.../stored_proc/`
      （拆分前的形态是单文件，拆分后是目录 ⇒ 两种形态都要覆盖，
      否则「拆分前采的快照」与「拆分后采的快照」不可比）。
    """
    p = pathlib.Path(path)
    if not p.is_absolute():
        p = pathlib.Path(root) / p
    if p.name == "mod.rs":
        # 门面文件 `.../expr/mod.rs` ⇒ 模块根是 `.../expr/` 这个**目录**
        return [], [p.parent]
    # 单文件门面 `.../stored_proc.rs` ⇒ 覆盖「文件本身」+「同名目录」
    return [p], [p.parent / p.stem]


def collect(files, root):
    """收集给定门面文件对应的模块的 pub 面。"""
    out = {}
    for f in files:
        p = pathlib.Path(f)
        if not p.is_absolute():
            p = pathlib.Path(root) / p
        if not p.exists():
            raise SystemExit("refactor_api_surface: 文件不存在：%s" % f)
        files_to_scan, dirs_to_scan = module_roots(p, root)
        targets = list(files_to_scan)
        for d in dirs_to_scan:
            if d.is_dir():
                targets += sorted(d.rglob("*.rs"))
        for src in targets:
            if not src.is_file():
                continue
            rel = os.path.relpath(src, root)
            text = src.read_text(encoding="utf-8", errors="replace")
            for vis, kind, name, sig in iter_pub_items(text.split("\n")):
                key = "%s::%s" % (rel, name)
                out.setdefault(key, []).append((vis, kind, sig))
    return out


def render(surface):
    """把收集结果渲染成稳定的、可逐行 diff 的文本。"""
    lines = []
    for key in sorted(surface):
        for vis, kind, sig in sorted(surface[key]):
            lines.append("%s\t%s\t%s\t%s" % (key, vis, kind, sig))
    return "\n".join(lines) + ("\n" if lines else "")


def snapshot_text(files, root):
    return render(collect(files, root))


def do_diff(before_path, after_path):
    before = pathlib.Path(before_path).read_text(encoding="utf-8").splitlines()
    after = pathlib.Path(after_path).read_text(encoding="utf-8").splitlines()
    if before == after:
        print("refactor_api_surface: 公开 API 面**未变** ✓（%d 项）" % len(before))
        return 0
    print("refactor_api_surface: 公开 API 面**变了** ✗", file=sys.stderr)
    # `difflib.unified_diff` 是**生成器** —— 必须先物化成列表，
    # 否则下面的 for 循环把它耗尽后 added/removed 永远是空的
    # （计数会报「0 项 / 0 项」，而 diff 里明明有变化）。
    diff = list(difflib.unified_diff(
        before, after, fromfile=before_path, tofile=after_path, lineterm="", n=1
    ))
    for line in diff:
        print(line, file=sys.stderr)
    added = [l for l in diff if l.startswith("+") and not l.startswith("+++")]
    removed = [l for l in diff if l.startswith("-") and not l.startswith("---")]
    print("  新增 %d 项 / 消失或变更 %d 项" % (len(added), len(removed)), file=sys.stderr)
    return 1


# --- 自测 ---------------------------------------------------------------------

SELFTEST_CASES = [
    (
        "普通 pub fn 被抓到",
        "pub fn foo(a: i32) -> bool { true }\n",
        1,
    ),
    (
        "私有 fn 不被抓到",
        "fn hidden() {}\n",
        0,
    ),
    (
        "pub(crate) 被抓到且可见性正确",
        "pub(crate) fn internal() {}\n",
        1,
    ),
    (
        "pub struct 被抓到",
        "pub struct S { a: i32 }\n",
        1,
    ),
    (
        "impl 块里的 pub fn 被抓到",
        "impl S {\n    pub fn method(&self) {}\n}\n",
        1,
    ),
    (
        "私有 impl 里的 pub fn 仍被抓（它是外部 API）",
        "impl S {\n    pub fn method2(&self) {}\n}\n",
        1,
    ),
    (
        "非 pub 的 struct 不被抓到",
        "struct Private {}\n",
        0,
    ),
    (
        "pub const 被抓到",
        "pub const MAX: usize = 3;\n",
        1,
    ),
    (
        "注释里的 pub fn 不被当成代码",
        "// pub fn commented_out() {}\n",
        0,
    ),
]


def selftest():
    failures = 0
    for name, src, want in SELFTEST_CASES:
        got = len(list(iter_pub_items(src.split("\n"))))
        if got == want:
            print("  [PASS] %s" % name)
        else:
            failures += 1
            print("  [FAIL] %s: 期望 %d 项，实得 %d" % (name, want, got))

    # 签名归一化：空白差异不算 API 变更
    a = list(iter_pub_items(["pub fn f(x: i32) -> i32 { x }"]))
    b = list(iter_pub_items(["pub   fn  f( x : i32 )  ->  i32 { x }"]))
    if a == b:
        print("  [PASS] 签名归一化：空白差异不算变更")
    else:
        failures += 1
        print("  [FAIL] 签名归一化：%r != %r" % (a, b))

    # 签名真变了要能看出来
    a = list(iter_pub_items(["pub fn f(x: i32) -> i32 { x }"]))
    b = list(iter_pub_items(["pub fn f(x: i64) -> i32 { 0 }"]))
    if a != b:
        print("  [PASS] 参数类型变更能被识别")
    else:
        failures += 1
        print("  [FAIL] 参数类型变更没被识别")

    # 可见性收窄要能看出来
    a = list(iter_pub_items(["pub fn f() {}"]))
    b = list(iter_pub_items(["pub(crate) fn f() {}"]))
    if a != b:
        print("  [PASS] 可见性收窄能被识别")
    else:
        failures += 1
        print("  [FAIL] 可见性收窄没被识别")

    # 反向变异：证明「diff 模式真的抓得住」—— 否则这个工具可能只是在
    # 打印 diff 而没有判定力。三条都必须是 rc=1。
    import tempfile
    with tempfile.TemporaryDirectory() as td:
        base = pathlib.Path(td) / "a.txt"
        mut = pathlib.Path(td) / "b.txt"
        base.write_text("m\tpub\tfn\t(x: i32)->i32\n", encoding="utf-8")
        # 变异 1：删掉一项
        mut.write_text("", encoding="utf-8")
        rc = do_diff(str(base), str(mut))
        if rc == 1:
            print("  [PASS] 反向变异：删掉一个 pub 项能被抓住")
        else:
            failures += 1
            print("  [FAIL] 反向变异：删掉一个 pub 项**没被**抓住")
        # 变异 2：只改签名
        mut.write_text("m\tpub\tfn\t(x: i64)->i32\n", encoding="utf-8")
        rc = do_diff(str(base), str(mut))
        if rc == 1:
            print("  [PASS] 反向变异：改签名能被抓住")
        else:
            failures += 1
            print("  [FAIL] 反向变异：改签名**没被**抓住")
        # 变异 3：可见性收窄
        mut.write_text("m\tpub(crate)\tfn\t(x: i32)->i32\n", encoding="utf-8")
        rc = do_diff(str(base), str(mut))
        if rc == 1:
            print("  [PASS] 反向变异：可见性收窄能被抓住")
        else:
            failures += 1
            print("  [FAIL] 反向变异：可见性收窄**没被**抓住")
        # 对照：完全相同 ⇒ rc=0
        mut.write_text(base.read_text(encoding="utf-8"), encoding="utf-8")
        rc = do_diff(str(base), str(mut))
        if rc == 0:
            print("  [PASS] 对照：完全相同不报差异")
        else:
            failures += 1
            print("  [FAIL] 对照：完全相同却报了差异")

    total = len(SELFTEST_CASES) + 3 + 4
    if failures:
        print("refactor_api_surface selftest: %d/%d failed" % (failures, total))
        return 1
    print("refactor_api_surface selftest: %d/%d passed" % (total, total))
    return 0


def main(argv):
    ap = argparse.ArgumentParser(
        description="公开 API 面的快照与比对（R1 契约先行的验收工具；不是门禁）")
    ap.add_argument("--repo", default=".", help="仓库根（相对路径以此为基准）")
    sub = ap.add_subparsers(dest="cmd")

    sp = sub.add_parser("snapshot", help="采集 pub 面并写到文件")
    sp.add_argument("--files", nargs="+", required=True, help="门面文件（其所在目录会被整体扫描）")
    sp.add_argument("--out", default=None, help="输出文件（缺省打到 stdout）")

    dp = sub.add_parser("diff", help="比对两份快照")
    dp.add_argument("--before", required=True)
    dp.add_argument("--after", required=True)

    ap.add_argument("--selftest", action="store_true")

    args = ap.parse_args(argv)
    if args.selftest:
        return selftest()
    if args.cmd == "snapshot":
        root = os.path.abspath(args.repo)
        text = snapshot_text(args.files, root)
        if args.out:
            pathlib.Path(args.out).write_text(text, encoding="utf-8")
            print("wrote %s (%d 项)" % (args.out, len(text.splitlines()) - (1 if text else 0)))
        else:
            sys.stdout.write(text)
        return 0
    if args.cmd == "diff":
        return do_diff(args.before, args.after)
    ap.print_help()
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
