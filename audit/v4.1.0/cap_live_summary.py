#!/usr/bin/env python3
"""capability_smoke 的 run json 摘要器：只读现成产物，不重新观测。

把 audit_run 里的一条命令变成「可核对的结论」，而不是把几十 KB 的 json 塞进运行日志。

退出码（与门禁三态一致，本脚本不改判工具自己的 verdict）：
  0  关键判据全部成立
  2  INCOMPLETE：json 缺失 / 缺字段 / 参考服务端不可用 / 包级探针没跑或没抓到 ⇒「没能核对」
  3  关键判据不成立（三段读回不等、wal 空、restore 没落盘、判别臂没红、
     被测与参考服务端的结果集包级字节不等）
INCOMPLETE 优先于 FAIL：没观测到的东西既不能算通过，也不能算被测的缺陷。

为什么 NULL 判据必须带参考对照（run-001 的教训，写死在这里防止再犯）：
  我曾把「客户端把 SQL NULL 和字面量 'NULL' 都打印成 NULL」判成缺陷（WIRE-1）。
  后来现场起 mariadbd 做参考，参考服务端在同一个客户端下也是 `1\\tNULL / 2\\tNULL`，
  而结果集包级字节 0xFB vs 04 4e 55 4c 4c 两边分得开 ⇒ 那是我探针的错，不是被测的错。
  判据只有「我期望的值」而没有参考实现兜底时，探针自己就能把误报坐实。
  所以这里 `wire_rows_hex` 与 `rows_hex` 不等 ⇒ 真缺陷（3），
  而参考侧不可用 ⇒ 只能 INCOMPLETE（2），不许退回单侧断言。
"""
import argparse
import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[2]


def main(argv) -> int:
    ap = argparse.ArgumentParser(prog="cap_live_summary.py")
    ap.add_argument("--json", default="test_results/capability_smoke/run-archive-002/capability_smoke.json")
    a = ap.parse_args(argv)
    p = pathlib.Path(a.json)
    if not p.is_absolute():
        p = REPO / p
    if not p.is_file():
        print("INCOMPLETE: 找不到 %s（上一命令没跑成，或路径写错）" % p, file=sys.stderr)
        return 2
    try:
        d = json.loads(p.read_text(encoding="utf-8"))
    except Exception as exc:  # 读不动就是没核对，不是被测红
        print("INCOMPLETE: json 解析失败 %s: %s" % (p, exc), file=sys.stderr)
        return 2
    if "prod" not in d or "obs" not in d["prod"] or "judge" not in d["prod"]:
        print("INCOMPLETE: json 缺 prod.obs / prod.judge 字段（%s）" % p, file=sys.stderr)
        return 2

    obs = d["prod"]["obs"]
    ref = d.get("wire_reference_control")

    incomplete = []
    if not ref:
        incomplete.append("json 里没有 wire_reference_control 字段：这一 run 没起参考服务端，NULL 只能看包级字节")
    else:
        if not ref.get("available"):
            incomplete.append("参考服务端不可用：%s" % (ref.get("error") or ref.get("setup_error") or "无 error 文本"))
        for key in ("capture_error", "setup_error", "error"):
            if ref.get(key):
                incomplete.append("参考侧 %s=%s" % (key, ref[key]))
        if not ref.get("rows_hex"):
            incomplete.append("参考侧 rows_hex 为空：没有字节对照基准")
    if not obs.get("wire_probed"):
        incomplete.append("被测侧包级探针没跑（wire_probed=false）⇒ NULL 这一维没能核对")
    if obs.get("wire_capture_error"):
        incomplete.append("被测侧包级抓包失败：%s" % obs["wire_capture_error"])
    if not obs.get("null_probed"):
        incomplete.append("被测侧没插入 NULL 行（null_probed=false）⇒ NULL 判据没跑")

    out = {
        "kind": "cap_live_summary",
        "json": str(p.relative_to(REPO)) if p.is_relative_to(REPO) else str(p),
        "git_head": d.get("git_head"),
        "bin": d.get("bin"),
        "expected_rows": d.get("expected_rows"),
        "prod_storage": [obs.get("storage"), obs.get("wal_sync")],
        "prod_ddl_errors": len(obs.get("ddl_errors") or []),
        "prod_insert_errors": len(obs.get("insert_errors") or []),
        "wal_nonempty": obs.get("wal_nonempty"),
        "table_files_nonempty": obs.get("table_files_nonempty"),
        "stage_eq_insert_vs_crash": obs.get("select_before") == obs.get("crash_readback"),
        "stage_eq_crash_vs_restore": obs.get("crash_readback") == obs.get("restore_readback"),
        "restore_readback_rows": len(obs.get("restore_readback") or []),
        "backup_rc": obs.get("backup_rc"),
        "backup_rows": obs.get("backup_rows"),
        "restore_rc": obs.get("restore_rc"),
        "restore_files_nonempty": bool(obs.get("restored_files")) and all(
            n > 0 for _, n in obs.get("restored_files") or []),
        "restore_wrote_nothing": obs.get("restore_wrote_nothing"),
        "wire_probed": obs.get("wire_probed"),
        "wire_rows_hex": obs.get("wire_rows_hex"),
        "reference_rows_hex": (ref or {}).get("rows_hex"),
        "wire_byte_identical": bool(ref) and bool(ref.get("rows_hex"))
                              and obs.get("wire_rows_hex") == ref.get("rows_hex"),
        "client_batch_both_null_prod": obs.get("client_batch_both_null"),
        "client_batch_both_null_reference": (ref or {}).get("client_batch_both_null"),
        "client_raw_prod": obs.get("client_raw"),
        "client_raw_reference": (ref or {}).get("client_raw"),
        "null_predicate_ok": obs.get("null_predicate_ok"),
        "null_predicate_detail": obs.get("null_predicate_detail"),
        "prod_findings": d["prod"]["judge"]["findings"],
        "prod_verdict": d["prod"]["judge"]["verdict"],
        "arm_findings_n": len((d.get("arm_nonproduction") or {}).get("judge", {}).get("findings") or []),
        "arm_verdict": (d.get("arm_nonproduction") or {}).get("judge", {}).get("verdict"),
        "discriminator": d.get("discriminator"),
        "overall": d.get("overall"),
    }
    print(json.dumps(out, ensure_ascii=False, indent=1))

    bad = []
    if not out["stage_eq_insert_vs_crash"]:
        bad.append("崩溃后读回与插入值逐值不等")
    if not out["stage_eq_crash_vs_restore"]:
        bad.append("恢复后读回与崩溃前逐值不等")
    if not obs.get("wal_nonempty"):
        bad.append("WAL 文件为空（假落库）")
    if not obs.get("table_files_nonempty"):
        bad.append("表数据文件为空")
    if obs.get("restore_wrote_nothing"):
        bad.append("restore 报成功但什么都没写")
    if not out["restore_files_nonempty"]:
        bad.append("restore 之后没有非空文件落盘")
    if obs.get("backup_rows") != d.get("expected_rows"):
        bad.append("backup_rows=%s 与 expected_rows=%s 不等" % (obs.get("backup_rows"), d.get("expected_rows")))
    disc = d.get("discriminator") or {}
    if not disc.get("discriminating"):
        bad.append("判别臂没红：这套判据失去判别力（%s）" % disc.get("why"))
    if d["prod"]["judge"]["findings"]:
        bad.append("生产链判据红：%s" % d["prod"]["judge"]["findings"])
    if ref and ref.get("rows_hex") and not out["wire_byte_identical"]:
        bad.append("被测结果集包级字节与参考服务端不等：prod=%s ref=%s"
                   % (obs.get("wire_rows_hex"), ref.get("rows_hex")))
    if incomplete:
        print("NOT-OK: INCOMPLETE —— " + "; ".join(incomplete), file=sys.stderr)
        return 2
    if bad:
        print("NOT-OK: FAIL —— " + "; ".join(bad), file=sys.stderr)
        return 3
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
