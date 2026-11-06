#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""binmode_probe.py —— 把 `--storage binary` 下的「报成功但没生效」逐语句摆到桌面上（探针，不是门禁）。

为什么单独一个探针：capability_smoke 的 binary 判别臂在**建表**这一步就失败，
后面的语句（默认库建表、触发器）根本走不到，于是「触发器被静默跳过」这一维只有
本地 scratch 日志（未入库）。本探针专走那条路：DDL 失败也要继续往下试，
并把服务端日志里的自认行一起归档。

判据口径：**语句返回 rc=0 且后续能观测到效果**才算生效；只看 rc 不算（假成功正是 rc=0）。
服务端起不来 ⇒ rc=2（INCOMPLETE，没能核对），绝不把没观测到写成没问题。

用法：/usr/bin/python3 audit/v4.1.0/binmode_probe.py [--out test_results/binmode]
退出码：0 = 观测完成（**有无缺陷看 JSON，不看退出码**）/ 2 = 没能观测。
"""
from __future__ import annotations

import argparse
import json
import pathlib
import hashlib
import re
import signal
import shutil
import subprocess
import sys
import time

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[1]
SERVER_BIN = REPO / "target/debug/sqlrustgo-mysql-server"
MY = "/opt/homebrew/bin/mysql"


def bin_provenance(p: pathlib.Path) -> dict:
    """把**被测二进制本身**钉成可核对的 provenance（路径 + sha256 + mtime）。
    跨线复测时这台机器上同时存在多条线的 target/debug，光写路径会被误读成
    「测的是本线代码」⇒ 必须有哈希，且报告里要写清该二进制来自哪条线。"""
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for blk in iter(lambda: f.read(1 << 20), b""):
            h.update(blk)
    st = p.stat()
    return {"path": str(p), "sha256": h.hexdigest(),
            "size": st.st_size, "mtime_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(st.st_mtime))}

sys.path.insert(0, str(HERE))
from capability_smoke import free_port, q  # noqa: E402  复用同一套查询/空闲端口实现，避免两套口径


def main(argv) -> int:
    ap = argparse.ArgumentParser(prog="binmode_probe.py")
    ap.add_argument("--out", default=str(REPO / "test_results/binmode"))
    ap.add_argument("--server-bin", default=str(SERVER_BIN),
                    help="被测服务端二进制；跨线复测时指向**另一条线**的 target/debug，"
                         "provenance 会记下它的 sha256，别把结论记成本线的")
    a = ap.parse_args(argv)
    out = pathlib.Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    srv = pathlib.Path(a.server_bin)
    if not srv.is_file():
        print("INCOMPLETE: 缺 %s" % srv, file=sys.stderr)
        return 2
    prov = bin_provenance(srv)
    data = out / "data"
    if data.exists():
        shutil.rmtree(data, ignore_errors=True)   # 只删自己刚建的 scratch 子目录
    data.mkdir(parents=True)
    port = free_port()
    log = out / "server_info.log"
    # 不复用 Server 的起停：它固定 --log-level warn，而这里要的就是服务端自己在 info 级写下的
    # 自认行（`TriggerExecutor::new: ... silently skipped`）。就绪判定仍走同一份 q()。
    # 上一版这里把服务端输出丢进 DEVNULL，却仍去读 server_info.log ⇒ 「自认行」这一维
    # 其实**没观测过**（实测：log 文件根本不存在，hits 恒空）。改成落盘再判。
    logf = open(log, "wb")
    proc = subprocess.Popen([str(srv), "--log-level", "info", "serve",
                             "--host", "127.0.0.1", "--port", str(port),
                             "--data-dir", str(data), "--storage", "binary", "--wal-sync", "every"],
                            stdout=logf, stderr=subprocess.STDOUT)
    up = False
    for _ in range(40):
        if q(MY, port, "SELECT 1")[0] == 0:
            up = True
            break
        time.sleep(0.5)
    if not up:
        proc.kill()
        print("INCOMPLETE: binary 模式服务端 20s 未就绪（见 %s）" % log, file=sys.stderr)
        return 2

    steps = []

    def run(label: str, sql: str):
        rc, o, e = q(MY, port, sql)
        steps.append({"label": label, "sql": sql, "rc": rc,
                      "stdout": o.strip()[:300], "stderr": e.strip()[:300]})
        return rc, o, e

    run("show_databases_before", "SHOW DATABASES;")
    run("create_database", "CREATE DATABASE IF NOT EXISTS probeb;")
    run("show_databases_after", "SHOW DATABASES;")
    run("create_table_in_new_db", "CREATE TABLE probeb.t (id INT PRIMARY KEY, v VARCHAR(16));")
    run("create_table_default_db", "CREATE TABLE pb_t2 (id INT PRIMARY KEY, v VARCHAR(16));")
    run("insert_default_db", "INSERT INTO pb_t2 VALUES (1,'orig'),(2,'orig');")
    run("create_trigger", "CREATE TRIGGER pb_trig BEFORE INSERT ON pb_t2 "
                          "FOR EACH ROW SET NEW.v = 'fired';")
    run("insert_after_trigger", "INSERT INTO pb_t2 VALUES (3,'orig');")
    run("select_after_trigger", "SELECT id,v FROM pb_t2 ORDER BY id;")
    run("drop_trigger", "DROP TRIGGER pb_trig;")
    run("insert_after_drop", "INSERT INTO pb_t2 VALUES (4,'orig');")
    run("select_final", "SELECT id,v FROM pb_t2 ORDER BY id;")

    try:
        proc.send_signal(signal.SIGTERM)
        proc.wait(timeout=10)
    except Exception:
        proc.kill()
    logf.close()
    txt = log.read_text(errors="replace") if log.is_file() else ""
    if not txt.strip():
        print("INCOMPLETE: 服务端日志为空 ⇒ 『自认行』这一维没能核对", file=sys.stderr)
        return 2
    hits = [ln.strip()[:220] for ln in txt.splitlines()
            if re.search(r"TriggerExecutor|silently skipped|no WAL enabled", ln)]
    rows = {}
    for s in steps:
        if s["label"] == "select_final":
            for line in s["stdout"].splitlines():
                parts = line.split("\t")
                if len(parts) == 2:
                    rows[parts[0]] = parts[1]
    fired = [k for k, v in rows.items() if v == "fired"]

    def rc_of(label: str) -> int:
        return next(s["rc"] for s in steps if s["label"] == label)

    def out_of(label: str) -> str:
        return next(s["stdout"] for s in steps if s["label"] == label)

    # 「假成功」的机器口径：语句 rc=0，但客户端侧能观测到它没生效。三条各自独立可判。
    false_success = []
    if rc_of("create_database") == 0 and "probeb" not in out_of("show_databases_after").split():
        false_success.append("CREATE DATABASE rc=0 但 SHOW DATABASES 仍只有 %s"
                             % out_of("show_databases_after").split())
    if rc_of("create_trigger") == 0 and not fired:
        false_success.append("CREATE TRIGGER rc=0 但 INSERT 后 NEW.v 未被改写（触发器没执行；final_rows=%s）"
                             % rows)
    if rc_of("create_trigger") == 0 and rc_of("drop_trigger") != 0:
        false_success.append("CREATE TRIGGER rc=0 但 DROP TRIGGER rc=%d（%s）⇒ 触发器从未登记"
                             % (rc_of("drop_trigger"),
                                " ".join(out_of("drop_trigger").split())
                                or " ".join(next(s["stderr"] for s in steps
                                                  if s["label"] == "drop_trigger").split())[:120]))
    summary = {
        "kind": "binmode_probe",
        "storage": "binary",
        "server_bin": prov["path"],
        "server_sha256": prov["sha256"],
        "server_size": prov["size"],
        "server_mtime_utc": prov["mtime_utc"],
        "server_bin_is_this_line": str(srv.resolve().parent) == str((REPO / "target/debug").resolve()),
        "port": port,
        "steps": steps,
        "create_database_rc": next(s["rc"] for s in steps if s["label"] == "create_database"),
        "show_databases_before": next(s["stdout"] for s in steps if s["label"] == "show_databases_before").split(),
        "show_databases_after": next(s["stdout"] for s in steps if s["label"] == "show_databases_after").split(),
        "create_database_effective": "probeb" in next(
            s["stdout"] for s in steps if s["label"] == "show_databases_after").split(),
        "trigger_self_admission_log_lines": hits,
        "final_rows": rows,
        "trigger_actually_fired_rows": fired,
        "create_trigger_rc": rc_of("create_trigger"),
        "drop_trigger_rc": rc_of("drop_trigger"),
        "false_success_count": len(false_success),
        "false_success": false_success,
    }
    print(json.dumps(summary, ensure_ascii=False, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
