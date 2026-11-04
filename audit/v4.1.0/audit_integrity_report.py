#!/usr/bin/env python3
"""audit_integrity_report.py —— 把仓里**每一条审计记录**的"声称 vs 在场证据"摊开。

不是门禁（不叫 check_*，不进 STAGE_CONFIG 阶段清单），只做度量：
`check_audit_manifest.sh` 会告诉你"哪几条 verify 失败"，
但它不回答一个更重要的问题：**失败的形状是什么**。

三类缺陷的形状完全不同：

  hollow   —— manifest 声称 N 条命令 exit 0，但**没有记录命令行**、
              或日志根本不在场 ⇒ 这条记录在结构上就不可复核。
  unsealed —— manifest_sha256 为 None（脚手架建了，没封板）。
  partial  —— 日志在场但命令元数据(meta)缺失。

用法: python3 audit/v4.1.0/audit_integrity_report.py [--json]
退出码恒 0（度量工具不判红不判绿；判定是门禁与人的事）。
"""
from __future__ import annotations

import json
import pathlib
import sys

import yaml

ROOT = pathlib.Path(__file__).resolve().parents[2]
AUDIT = ROOT / "audit" / "v4.1.0"


def classify(run_dir: pathlib.Path) -> dict:
    mf = run_dir / "manifest.yaml"
    if not mf.exists():
        return {"run_id": run_dir.name, "verdict": "no-manifest"}
    m = yaml.safe_load(mf.read_text(encoding="utf-8")) or {}
    cmds = [c for c in (m.get("commands") or []) if isinstance(c, dict)]

    declared = len(cmds)
    with_cmd = sum(1 for c in cmds if (c.get("cmd") or c.get("command")))
    zero_claims = sum(1 for c in cmds if c.get("exit_code") == 0)

    schema = str(m.get("schema_version") or "")
    if schema.startswith("iaia410"):
        log_dirs = [run_dir / "commands"]
        log_pat = ("*.stdout.log", "*.log")
        meta_pat = ("*.meta.yaml",)
    else:
        log_dirs = [run_dir / "logs"]
        log_pat = ("*.stdout",)
        meta_pat = ()

    logs = sum(1 for d in log_dirs if d.is_dir() for pat in log_pat for _ in d.glob(pat))
    metas = sum(1 for d in log_dirs if d.is_dir() for pat in meta_pat for _ in d.glob(pat))
    sealed = bool(m.get("manifest_sha256") or m.get("evidence_hash"))

    defects = []
    if declared == 0:
        defects.append("empty-commands")
    if declared and with_cmd == 0:
        defects.append("no-cmd-recorded")
    if declared and logs == 0:
        defects.append("no-logs")
    if schema.startswith("iaia410") and declared and metas == 0:
        defects.append("no-meta")
    if not sealed:
        defects.append("unsealed")

    verdict = "sound" if not defects else ("hollow" if ("no-cmd-recorded" in defects or "no-logs" in defects) else "partial")
    sums = run_dir / "SHA256SUMS"
    placeholder = sums.exists() and "placeholder" in sums.read_text(encoding="utf-8", errors="replace").lower()
    if placeholder:
        defects.append("sha256sums-placeholder")
        verdict = "hollow"

    return {
        "run_id": run_dir.name,
        "schema": schema,
        "declared_cmds": declared,
        "cmd_recorded": with_cmd,
        "exit0_claims": zero_claims,
        "logs": logs,
        "metas": metas,
        "sealed": sealed,
        "defects": defects,
        "verdict": verdict,
    }


def main() -> int:
    rows = []
    for d in sorted(AUDIT.iterdir()):
        if d.is_dir() and (d.name.startswith("iaia410-")):
            rows.append(classify(d))
    runs = AUDIT / "runs"
    if runs.is_dir():
        for d in sorted(runs.iterdir()):
            if d.is_dir():
                rows.append(classify(d))

    if "--json" in sys.argv:
        print(json.dumps({"rows": rows}, ensure_ascii=False, indent=1))
        return 0
    print(f"{'run_id':<40} {'schema':<24} {'decl':>4} {'cmd':>4} {'logs':>5} {'meta':>5} sealed  verdict   defects")
    for r in rows:
        print(f"{r['run_id']:<40} {str(r.get('schema')):<24} {r.get('declared_cmds',0):>4} "
              f"{r.get('cmd_recorded',0):>4} {r.get('logs',0):>5} {r.get('metas',0):>5} "
              f"{str(r.get('sealed')):<6} {r.get('verdict',''):<8} {','.join(r.get('defects',[]))}")
    n = {}
    for r in rows:
        n[r["verdict"]] = n.get(r["verdict"], 0) + 1
    print("\n合计:", json.dumps(n, ensure_ascii=False), "| records =", len(rows))
    return 0


if __name__ == "__main__":
    sys.exit(main())
