#!/usr/bin/env python3
"""CI 执行根集合声明（P0-5）的机器核对 + 自带反向变异。

背景：`gate_executability` 用隐式 glob 扫 `.gitea/workflows/*.yml`，而仓里还留着
13 个 `.github/workflows/*.yml`（GitHub Actions 在本仓永不运行）与一个冒充 workflow 的
`.md`。任何人引用这些文件当「CI 已覆盖」都是空承诺。P0-5 把根集合变成显式声明：
`docs/releases/v4.1.0/CI_EXECUTION_ROOTS.jsonl`。

判据（任一不成立 ⇒ rc=1；输入取不到 ⇒ rc=2=INCOMPLETE，绝不等于通过）：
  R1 声明头存在且 glob 集合未被改小（防止"把声明改窄就变绿"）
  R2 声明行 path 唯一且文件存在
  R3 磁盘上匹配 glob 的文件集合 == 声明集合（漏声明 / 幽灵声明都红）
  R4 archive-only 必须带横幅；live 不许带存档横幅
  R5 执行目录里不许有非 *.yml/*.yaml（冒充 workflow 直接红）
  R6 168h SOAK 只能有一个 canonical，且必须等于声明头里的 soak_canonical
  R7 与门禁耦合：`gate_executability --json` 报告的真实调用点所在 workflow，
     必须在声明里且是 live 类（声明若与门禁实际扫描脱钩 ⇒ 声明只是装饰）
  R8 live 目录下的行不许被改判成 archive-only（防止重新分类绕过 R7）

mutation-selftest：CTRL 绿；M1..M7 七类注入必须各自变红。

不进门禁分母（audit/ 探针，不命名 check_*，不进 STAGE_CONFIG 阶段清单）。
"""
from __future__ import annotations

import argparse
import glob
import json
import pathlib
import shutil
import subprocess
import sys
import tempfile

REAL_ROOT = pathlib.Path(__file__).resolve().parents[2]
DECL_REL = "docs/releases/v4.1.0/CI_EXECUTION_ROOTS.jsonl"
BANNER_MARK = "ARCHIVE-ONLY — 仅存档、不执行"
EXPECTED_LIVE_GLOBS = [".gitea/workflows/*.yml", ".gitea/workflows/*.yaml"]
EXPECTED_ARCHIVE_GLOBS = [".github/workflows/*.yml", ".github/workflows/*.yaml"]
LIVE_CLASSES = {"live", "canonical-soak", "superseded-soak", "liveness-probe"}
IMPOSTOR_EXTS = (".md", ".markdown", ".txt", ".sh", ".bak")


def load_decl(path: pathlib.Path) -> tuple[dict, list[dict], list[str]]:
    fails: list[str] = []
    header: dict = {}
    rows: list[dict] = []
    for i, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        try:
            rec = json.loads(line)
        except ValueError as exc:
            fails.append(f"R0 声明 {path.name}:{i} 不是合法 json: {exc}")
            continue
        k = rec.get("kind")
        if k == "ci_root_declaration":
            if header:
                fails.append("R1 声明头出现多次")
            header = rec
        elif k == "ci_root":
            rows.append(rec)
    if not header:
        fails.append("R1 缺声明头（kind=ci_root_declaration）")
    return header, rows, fails


def match_glob(rel: str, patterns: list[str]) -> bool:
    return any(glob.fnmatch.fnmatch(rel, p) for p in patterns)


def check(root: pathlib.Path, decl_path: pathlib.Path) -> int:
    import glob as _glob
    globals()["glob"] = _glob
    fails: list[str] = []
    notes: list[str] = []
    if not decl_path.is_file():
        print(json.dumps({"kind": "ci_roots", "result": "INCOMPLETE",
                          "why": f"缺声明文件 {decl_path}"}, ensure_ascii=False))
        return 2
    header, rows, fails = load_decl(decl_path)
    if header:
        for key, want in (("live_globs", EXPECTED_LIVE_GLOBS),
                          ("archive_globs", EXPECTED_ARCHIVE_GLOBS)):
            if sorted(header.get(key) or []) != sorted(want):
                fails.append(f"R1 {key} 被改动：声明={sorted(header.get(key) or [])} 期望={sorted(want)}")
    live_globs = header.get("live_globs") or EXPECTED_LIVE_GLOBS
    arch_globs = header.get("archive_globs") or EXPECTED_ARCHIVE_GLOBS

    # R2
    seen: set[str] = set()
    for r in rows:
        p = r.get("path") or ""
        if p in seen:
            fails.append(f"R2 声明重复: {p}")
        seen.add(p)
        if not (root / p).is_file():
            fails.append(f"R2 声明里的文件不存在: {p}")

    # R3
    disk = set()
    for pat in live_globs + arch_globs:
        for p in (root / pat).parent.glob(pathlib.PurePosixPath(pat).name):
            disk.add(str(pathlib.PurePosixPath(p.parent.relative_to(root), p.name)))
    declared = {r["path"] for r in rows}
    undeclared = sorted(disk - declared)
    phantom = sorted(declared - disk)
    if undeclared:
        fails.append(f"R3 磁盘上有但没声明（幽灵执行点未登记）: {undeclared}")
    if phantom:
        fails.append(f"R3 声明了但磁盘上没有（陈旧声明）: {phantom}")

    # R4 / R8
    for r in rows:
        p = pathlib.Path(r["path"])
        cls = r.get("class")
        is_live_dir = match_glob(r["path"], live_globs)
        if is_live_dir and cls not in LIVE_CLASSES:
            fails.append(f"R8 执行目录里的行被改判成 {cls}: {r['path']}")
            continue
        if not (root / r["path"]).is_file():
            continue
        text = (root / r["path"]).read_text(encoding="utf-8", errors="replace")
        has_banner = BANNER_MARK in text
        if cls == "archive-only" and not has_banner:
            fails.append(f"R4 archive-only 却没写存档横幅: {r['path']}")
        if cls in LIVE_CLASSES and has_banner and match_glob(r["path"], live_globs):
            fails.append(f"R4 live 文件里出现存档横幅（分类与内容矛盾）: {r['path']}")

    # R5 冒充 workflow
    for d in {str(pathlib.PurePosixPath(p).parent) for p in live_globs}:
        dirp = root / d
        if not dirp.is_dir():
            fails.append(f"R5 声明的执行目录不存在: {d}")
            continue
        bad = sorted(f.name for f in dirp.iterdir()
                     if f.suffix.lower() in IMPOSTOR_EXTS or f.name.endswith((".yml.bak", ".yml.disabled")))
        if bad:
            fails.append(f"R5 执行目录里有冒充 workflow 的文件: {[d + '/' + b for b in bad]}")

    # R6 soak canonical
    canon = [r for r in rows if r.get("class") == "canonical-soak"]
    if len(canon) != 1:
        fails.append(f"R6 canonical 168h SOAK 入口必须恰好 1 个，实测 {len(canon)}")
    elif header.get("soak_canonical") != canon[0]["path"]:
        fails.append(f"R6 声明头 soak_canonical={header.get('soak_canonical')} "
                     f"与行级 canonical={canon[0]['path']} 不一致")

    # R7 与门禁实际扫描耦合（只在真实仓做；沙箱里没有 STAGE_CONFIG 等依赖）
    used_by_gate: list[str] = []
    if root == REAL_ROOT:
        proc = subprocess.run([sys.executable, "scripts/gate/gate_executability.py", "--json"],
                              capture_output=True, text=True, cwd=str(root))
        blob = proc.stdout
        start = blob.find("{")
        data = None
        if start >= 0:
            try:
                data = json.loads(blob[start:])
            except ValueError:
                data = None
        if data is None:
            fails.append(f"R7 gate_executability --json 解析失败 rc={proc.returncode} ⇒ 无法核对耦合")
        else:
            roots_map = data.get("roots") or {}
            if not roots_map:
                fails.append("R7 gate_executability 报告 0 个调用点 ⇒ 声明无法与之耦合（INCOMPLETE 语义，按红处理）")
            for wfname in {v.split(":")[0] for lst in roots_map.values() for v in lst}:
                used_by_gate.append(wfname)
                rel = ".gitea/workflows/" + wfname
                row = next((r for r in rows if r["path"] == rel), None)
                if row is None:
                    fails.append(f"R7 门禁实际调用的 workflow 未声明: {rel}")
                elif row.get("class") not in LIVE_CLASSES:
                    fails.append(f"R7 门禁实际调用的 workflow 被声明成 {row.get('class')}: {rel}")
            notes.append(f"gate 调用点所在 workflow：{sorted(set(used_by_gate))}")
    else:
        notes.append("gate-coupling(R7): SKIPPED（沙箱没有 gate 依赖，不据此声称已核对）")

    out = {"kind": "ci_roots", "repo_root": str(root), "decl": str(decl_path),
           "declared": len(rows), "on_disk": len(disk),
           "archive_only": sum(1 for r in rows if r.get("class") == "archive-only"),
           "live": sum(1 for r in rows if r.get("class") in LIVE_CLASSES),
           "gate_used_workflows": sorted(set(used_by_gate)),
           "notes": notes, "fails": len(fails)}
    out["result"] = "OK" if not fails else "FAIL"
    print(json.dumps(out, ensure_ascii=False))
    for f in fails:
        print("FAIL:", f)
    return 0 if not fails else 1


def selftest(args) -> int:
    real_decl = REAL_ROOT / DECL_REL
    n_ok = n_bad = 0

    def expect(label: str, want_rc0: bool, rc: int, out: str, want_rule: str = "") -> None:
        nonlocal n_ok, n_bad
        fails = [l[6:].strip() for l in out.splitlines() if l.startswith("FAIL:")]
        got_ok = (rc == 0) == want_rc0
        # 只 rc 变红不够：必须命中这一臂该抓的那条判据，否则是在靠副作用蒙对
        if want_rule:
            got_ok = got_ok and any(f.startswith(want_rule) for f in fails)
        tag = fails[0] if fails else ""
        print(f"ARM {label} want_rc0={want_rc0} want_rule={want_rule or '-'} "
              f"rc={rc} fails={len(fails)} ok={1 if got_ok else 0} {tag[:150]}")
        if got_ok:
            n_ok += 1
        else:
            n_bad += 1

    def fresh_recs() -> list[dict]:
        return [json.loads(l) for l in real_decl.read_text(encoding="utf-8").splitlines() if l.strip()]

    def run(root: pathlib.Path, decl: pathlib.Path) -> tuple[int, str]:
        import io
        buf = io.StringIO()
        old = sys.stdout
        sys.stdout = buf
        try:
            rc = check(root, decl)
        finally:
            sys.stdout = old
        return rc, buf.getvalue()

    with tempfile.TemporaryDirectory(prefix="srg-ciroots-") as td:
        td = pathlib.Path(td)

        def sandbox(name: str) -> tuple[pathlib.Path, pathlib.Path]:
            s = td / name
            (s / ".gitea/workflows").mkdir(parents=True)
            (s / ".github/workflows").mkdir(parents=True)
            for d in (".gitea/workflows", ".github/workflows"):
                for f in (REAL_ROOT / d).iterdir():
                    shutil.copy2(f, s / d / f.name)
            (s / DECL_REL).parent.mkdir(parents=True)
            shutil.copy2(real_decl, s / DECL_REL)
            return s, s / DECL_REL

        # CTRL：真实仓真实声明 ⇒ 必须绿
        rc, out = run(REAL_ROOT, real_decl)
        expect("CTRL", True, rc, out)

        # M1 磁盘上多一个未声明的 archive workflow
        s, d = sandbox("m1")
        (s / ".github/workflows/ghost-new.yml").write_text(
            "name: ghost\non: push\njobs: {}\n", encoding="utf-8")
        rc, out = run(s, d)
        expect("M1-undeclared-workflow", False, rc, out, want_rule="R3")

        # M2 声明里留着一个已被删掉的文件（陈旧声明）
        s, d = sandbox("m2")
        victim = next((s / ".github/workflows").glob("*.yml"))
        victim.unlink()
        rc, out = run(s, d)
        expect("M2-phantom-declaration", False, rc, out, want_rule="R2")

        # M3 摘掉某个 archive-only 文件的存档横幅
        s, d = sandbox("m3")
        victim = next((s / ".github/workflows").glob("*.yml"))
        txt = (s / ".github/workflows" / victim.name).read_text(encoding="utf-8")
        (s / ".github/workflows" / victim.name).write_text(
            "\n".join(l for l in txt.splitlines() if BANNER_MARK not in l) + "\n", encoding="utf-8")
        rc, out = run(s, d)
        expect("M3-banner-stripped", False, rc, out, want_rule="R4")

        # M4 往执行目录塞一个冒充 workflow 的 .md
        s, d = sandbox("m4")
        (s / ".gitea/workflows/ci-test.md").write_text("ci test notes\n", encoding="utf-8")
        rc, out = run(s, d)
        expect("M4-impostor-md", False, rc, out, want_rule="R5")

        # M5 把门禁真正调用的 workflow 改判成 archive-only（分类绕过硬判据）
        recs = fresh_recs()
        hit = False
        for r in recs:
            if r.get("path") == ".gitea/workflows/gate.yml":
                r["class"] = "archive-only"
                hit = True
        assert hit, "声明里没有 gate.yml，声明本身已漂"
        d5 = td / "m5.jsonl"
        d5.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in recs), encoding="utf-8")
        rc, out = run(REAL_ROOT, d5)
        expect("M5-reclass-live-as-archive", False, rc, out, want_rule="R8")

        # M6 删掉一条声明行（漏登记）
        d6 = td / "m6.jsonl"
        recs6 = [r for r in fresh_recs() if r.get("kind") != "ci_root"
                 or r.get("path") != ".gitea/workflows/gate.yml"]
        d6.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in recs6), encoding="utf-8")
        rc, out = run(REAL_ROOT, d6)
        expect("M6-dropped-declaration-row", False, rc, out, want_rule="R3")

        # M7 把 soak_canonical 指向已被取代的那份
        d7 = td / "m7.jsonl"
        recs7 = []
        for r in fresh_recs():
            r = dict(r)
            if r.get("kind") == "ci_root_declaration":
                r["soak_canonical"] = ".gitea/workflows/z6g4-168h-soak.yml"
            recs7.append(r)
        d7.write_text("".join(json.dumps(r, ensure_ascii=False) + "\n" for r in recs7), encoding="utf-8")
        rc, out = run(REAL_ROOT, d7)
        expect("M7-soak-canonical-moved", False, rc, out, want_rule="R6")

    print(f"probe_ci_roots mutation-selftest: ok={n_ok} bad={n_bad}")
    if n_bad:
        print("probe_ci_roots mutation-selftest: 判据可疑（有臂未按预期）")
        return 1
    print("probe_ci_roots mutation-selftest: OK (CTRL 绿；M1..M7 均红)")
    return 0


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("mode", choices=["check", "mutation-selftest"])
    ap.add_argument("--repo-root", default=str(REAL_ROOT))
    ap.add_argument("--declaration", default="")
    args = ap.parse_args(argv)
    root = pathlib.Path(args.repo_root).resolve()
    decl = pathlib.Path(args.declaration) if args.declaration else root / DECL_REL
    if args.mode == "mutation-selftest":
        return selftest(args)
    return check(root, decl)


if __name__ == "__main__":
    raise SystemExit(main())
