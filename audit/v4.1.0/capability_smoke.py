#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""capability_smoke.py —— 用真 CLI + 真 server + 真落库 核对「存储语义已验证」这类声称（P1-2，探针，不是门禁）。

它存在的理由（看板 P1-2 验收，GOVERNANCE_REMEDIATION_PLAN_2026-11-04.md:352）：
  「报告都好看、一测就出问题」的直接机制是**声称与使用面脱节**——
  文档说持久化/崩溃恢复/备份恢复已验证，却从来没有一条**从 mysql 客户端打进真 server、
  真落盘、kill -9 后读回、备份→恢复→逐值比对**的链路上跑过（#5525 的三条假成功就是这一类）。
  本工具把这条链路做成机器判定，并**自带判别力自检**：
  把生产存储换成非生产（`--storage binary`）时必须变红；红不了就说明这条 smoke 是装饰。

与既有门禁的关系（分母恒 168，本工具不新增门禁、不改任何 scripts/gate/check_*）：
  * `check_correctness_tests_production_storage.sh` 判的是**源码装配块**（lib.rs 的 match arm）；
  * 本工具判的是**运行时使用面**（真进程、真字节落盘、真崩溃、真恢复后逐值相等）；
  * 两者互补：装配块绿不代表使用面绿（本次实测即为证）。

三种模式：
  run [--out DIR]      真跑：生产链 + 非生产判别臂（需要 target/debug 产物与 mysql 客户端）
  static               静态普查：声称验证存储语义的测试里，多少只在内存/Mock 引擎上跑
  --selftest           离线反向变异：证明逐值比对、落盘非空、假恢复、判别力四条判据真的会红
退出码：0 = 生产链全绿**且**判别臂变红（判据有判别力）；1 = 有 finding；2 = INCOMPLETE（没能核对）。
  「跑不动」永远不是「通过」：产物缺失、mysql 缺失、起不来、超时 ⇒ 2，并写清缺什么。
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import re
import shutil
import signal
import socket
import subprocess
import sys
import time

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[1]     # audit/v4.1.0/ -> 仓库根

SERVER_BIN = REPO / "target/debug/sqlrustgo-mysql-server"
BACKUP_BIN = REPO / "target/debug/physical-backup"
CLAIM_DOCS = REPO / "docs/releases/v4.1.0"

# 已知的「声称验证存储语义」的关键词（用于静态普查；宁可宽，判据口径写在输出里）
SEMANTIC_KEYS = ("crash", "recover", "restore", "backup", "persist", "durabil", "wal", "kill -9", "restart")
# 逐值比对的载荷：CJK、负 BIGINT、内嵌引号/注释符、空串——都是「只测 ASCII 就绿」会漏的形状
ROWS = [
    (1, "名称-1", 1007, "备注 1 -- 边界"),
    (2, "名称-2", 2007, "备注 2 /* x */"),
    (3, "名称-3", -3007, ""),          # 空串必须按空串存回去（插 NULL 是另一条判据，见下）
    (4, "名称-4", 9223372036854775806, "tab\t内嵌"),
    (5, "名称-5", 0, "quote'sq\"dq"),
]
TABLE_DDL = ("CREATE TABLE smoke.t (id INT PRIMARY KEY, name VARCHAR(64), "
             "amount BIGINT, note VARCHAR(128))")
SELECT_SQL = "SELECT id,name,amount,note FROM smoke.t ORDER BY id;"

# NULL 形状判据：一行真 NULL、一行字面量 'NULL'。真 mysql 客户端在 --batch 下把 NULL 渲染成 `\N`，
# 把字面量渲染成 `NULL`。两者若渲染成同一个 token，客户端就无法区分「无值」和「值为 NULL 四个字符」，
# ETL/备份回读必然串味。这一条与「服务端 IS NULL 谓词对不对」是两件事，分别判定。
NULL_DDL = "CREATE TABLE smoke.nn (id INT PRIMARY KEY, s VARCHAR(32))"
NULL_SQL = "SELECT id,s FROM smoke.nn ORDER BY id;"


def sha(p: pathlib.Path) -> str:
    return hashlib.sha256(p.read_bytes()).hexdigest() if p.is_file() else "MISSING"


# =============================================================================
# 判据（一份实现，主流程与 --selftest 共用）
# =============================================================================

def cell_rows(text: str) -> list:
    """把 mysql --batch -N 的输出解析成行列表（\\N -> None，制表符分列）。

    `--batch` 会把字段里的制表符/换行/反斜杠**转义**（`tab` 输出成 `tab\\t`），
    所以必须单程解码，否则「值里含 tab」这一形状会被误判成不等（假红）。
    单程扫描而不是逐个 replace：先 replace 会把「字面反斜杠 + t」误解成制表符（假绿）。
    """
    out = []
    for line in text.splitlines():
        if not line.strip():
            continue
        out.append([decode_batch(c) for c in line.split("\t")])
    return out


_BATCH_ESC = {"0": "\0", "n": "\n", "r": "\r", "t": "\t", "b": "\b", "\\": "\\", "s": " "}


def decode_batch(field: str):
    if field == "\\N":
        return None
    if "\\" not in field:
        return field
    buf, i = [], 0
    while i < len(field):
        ch = field[i]
        if ch == "\\" and i + 1 < len(field):
            nxt = field[i + 1]
            buf.append(_BATCH_ESC.get(nxt, nxt))
            i += 2
            continue
        buf.append(ch)
        i += 1
    return "".join(buf)


def expected_rows() -> list:
    rows = []
    for i, name, amount, note in ROWS:
        rows.append([str(i), name, str(amount), note if note else ""])
    return rows


def diff_rows(expected: list, actual: list) -> list:
    """逐值（cell-level）比对。缺行/多行/单元格不等都是 finding。"""
    f = []
    if len(actual) != len(expected):
        f.append("row_count: expected=%d actual=%d" % (len(expected), len(actual)))
    for idx in range(max(len(expected), len(actual))):
        e = expected[idx] if idx < len(expected) else None
        a = actual[idx] if idx < len(actual) else None
        if e is None:
            f.append("row[%d] extra: %r" % (idx, a))
            continue
        if a is None:
            f.append("row[%d] missing: expected %r" % (idx, e))
            continue
        if len(a) != len(e):
            f.append("row[%d] arity: expected=%d actual=%d" % (idx, len(e), len(a)))
            continue
        for c, (ev, av) in enumerate(zip(e, a)):
            if (ev or "") != (av or ""):
                f.append("row[%d].col[%d]: expected=%r actual=%r" % (idx, c, ev, av))
    return f


def nonzero_files(root: pathlib.Path) -> list:
    """落盘真身：返回 (相对路径, 字节数) 且**非空**的文件。空文件不算落库。"""
    out = []
    if not root.is_dir():
        return out
    for p in sorted(root.rglob("*")):
        if p.is_file() and p.stat().st_size > 0:
            out.append((str(p.relative_to(root)), p.stat().st_size))
    return out


def judge_scenario(obs: dict) -> dict:
    """obs 里是观测到的事实；返回逐项判定 + 该链路的 verdict。"""
    findings = []
    for stmt in obs.get("ddl_errors", []):
        findings.append("ddl_error: %s" % stmt)
    for e in obs.get("insert_errors", []):
        findings.append("insert_error: %s" % e)
    for label in ("select_before", "crash_readback", "restore_readback"):
        if label in obs:
            d = diff_rows(expected_rows(), obs[label])
            if d:
                findings.append("%s_mismatch: %s" % (label, "; ".join(d[:8])))
    if not obs.get("wal_nonempty"):
        findings.append("wal_not_persisted: sqlrustgo.wal 缺失或为空")
    if not obs.get("table_files_nonempty"):
        findings.append("table_files_not_persisted: 表数据文件缺失或全为空")
    if obs.get("backup_rows", 0) != len(ROWS):
        findings.append("backup_rows: expected=%d actual=%s" % (len(ROWS), obs.get("backup_rows")))
    if obs.get("restore_wrote_nothing"):
        findings.append("restore_wrote_nothing: restore 报成功但没有落盘（#5525 假成功形状）")
    if obs.get("null_probed") and obs.get("null_wire_ambiguous"):
        findings.append("null_ambiguous_on_wire: SQL NULL 与字面量 'NULL' 在结果集里渲染成同一个 token，"
                        "客户端无法区分（mysql --batch 下真 NULL 应为 \\N）：%s"
                        % obs.get("null_wire_detail", ""))
    if obs.get("null_probed") and obs.get("null_predicate_ok") is False:
        findings.append("null_predicate_wrong: 服务端 IS NULL / = 'NULL' 谓词计数不对 %s"
                        % obs.get("null_predicate_detail", ""))
    if obs.get("server_up") is not True:
        findings.append("server_not_ready")
    return {"findings": findings, "verdict": "PASS" if not findings else "FAIL"}


def judge_discriminator(prod: dict, arm: dict) -> dict:
    """判别臂必须比生产链**更红**，否则这条 smoke 没有判别力（=它无法发现被换掉的存储）。"""
    pf, af = len(prod["findings"]), len(arm["findings"])
    if af == 0:
        return {"discriminating": False, "why": "非生产臂没有变红：判据失去判别力", "prod": pf, "arm": af}
    if af <= pf:
        return {"discriminating": False, "why": "非生产臂不比生产链更红（arm=%d<=prod=%d）" % (af, pf),
                "prod": pf, "arm": af}
    return {"discriminating": True, "why": "换成非生产存储后判据变红（arm=%d > prod=%d）" % (af, pf),
            "prod": pf, "arm": af}


def overall(prod: dict, arm: dict, disc: dict) -> dict:
    if disc["discriminating"] and prod["verdict"] == "PASS":
        return {"verdict": "PASS", "findings": 0}
    n = len(prod["findings"]) + len(arm["findings"]) + (0 if disc["discriminating"] else 1)
    return {"verdict": "FAIL", "findings": n}


# =============================================================================
# 离线反向变异（不需要起服务）
# =============================================================================

def _healthy_obs() -> dict:
    return {
        "server_up": True,
        "ddl_errors": [],
        "select_before": expected_rows(),
        "crash_readback": expected_rows(),
        "restore_readback": expected_rows(),
        "wal_nonempty": True,
        "table_files_nonempty": True,
        "backup_rows": len(ROWS),
        "restore_wrote_nothing": False,
    }


def run_selftest() -> int:
    arms, ok = [], 0

    healthy = judge_scenario(_healthy_obs())
    arms.append(("H0 健康链必须绿", healthy["verdict"] == "PASS", healthy["findings"][:2]))
    ok += 1 if arms[-1][1] else 0

    m1 = _healthy_obs(); m1["crash_readback"] = expected_rows()[:-1]
    a1 = judge_scenario(m1)
    arms.append(("M1 崩溃后少一行 ⇒ 必须红", a1["verdict"] == "FAIL", a1["findings"][:2]))
    ok += 1 if arms[-1][1] else 0

    m2 = _healthy_obs()
    m2["restore_readback"] = [r[:] for r in expected_rows()]
    m2["restore_readback"][2][2] = "3007"          # 符号被吞
    m2["restore_readback"][4][3] = "quote'sq"      # 引号被截
    a2 = judge_scenario(m2)
    arms.append(("M2 恢复后逐值被改 ⇒ 必须红", a2["verdict"] == "FAIL", a2["findings"][:2]))
    ok += 1 if arms[-1][1] else 0

    m3 = _healthy_obs(); m3["restore_wrote_nothing"] = True; m3["table_files_nonempty"] = False
    m3["restore_readback"] = []
    a3 = judge_scenario(m3)
    arms.append(("M3 restore 报成功但没落盘（#5525 形状）⇒ 必须红", a3["verdict"] == "FAIL", a3["findings"][:2]))
    ok += 1 if arms[-1][1] else 0

    m4 = _healthy_obs(); m4["wal_nonempty"] = False
    a4 = judge_scenario(m4)
    arms.append(("M4 WAL 文件为空（假落库）⇒ 必须红", a4["verdict"] == "FAIL", a4["findings"][:2]))
    ok += 1 if arms[-1][1] else 0

    m5 = _healthy_obs(); m5["ddl_errors"] = ["CREATE DATABASE smoke: OK but no-op"]
    a5 = judge_scenario(m5)
    arms.append(("M5 DDL 报成功但未生效（binary 臂实测形状）⇒ 必须红", a5["verdict"] == "FAIL", a5["findings"][:2]))
    ok += 1 if arms[-1][1] else 0

    m6 = _healthy_obs(); m6["null_probed"] = True; m6["null_wire_ambiguous"] = True
    m6["null_predicate_ok"] = True; m6["null_wire_detail"] = "both='NULL'"
    a6 = judge_scenario(m6)
    arms.append(("M6 NULL 与 'NULL' 渲染同一个 token ⇒ 必须红", a6["verdict"] == "FAIL", a6["findings"][:1]))
    ok += 1 if arms[-1][1] else 0

    m7 = _healthy_obs(); m7["null_probed"] = True; m7["null_wire_ambiguous"] = False
    m7["null_predicate_ok"] = False; m7["null_predicate_detail"] = "is_null=0 want 1"
    a7 = judge_scenario(m7)
    arms.append(("M7 服务端 IS NULL 谓词计数不对 ⇒ 必须红", a7["verdict"] == "FAIL", a7["findings"][:1]))
    ok += 1 if arms[-1][1] else 0

    disc_good = judge_discriminator(healthy, judge_scenario(m1))
    arms.append(("D1 判别臂更红 ⇒ 判定有判别力", disc_good["discriminating"] is True, [disc_good["why"]]))
    ok += 1 if arms[-1][1] else 0

    disc_bad = judge_discriminator(healthy, healthy)
    arms.append(("D2 判别臂没变红 ⇒ 必须判『失去判别力』", disc_bad["discriminating"] is False,
                 [disc_bad["why"]]))
    ok += 1 if arms[-1][1] else 0

    ov_bad = overall(healthy, judge_scenario(healthy), disc_bad)
    arms.append(("D3 判别力丢了 ⇒ 整体不得判 PASS", ov_bad["verdict"] == "FAIL", [json.dumps(ov_bad)]))
    ok += 1 if arms[-1][1] else 0

    print(json.dumps({"kind": "capability_smoke_selftest", "arms": len(arms),
                      "discriminating": ok,
                      "detail": [[a[0], "OK" if a[1] else "NOT-DISCRIMINATING", a[2]] for a in arms]},
                     ensure_ascii=False, indent=1))
    return 0 if ok == len(arms) else 1


# =============================================================================
# 真跑
# =============================================================================

def free_port() -> int:
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    p = s.getsockname()[1]
    s.close()
    return p


def mysql_bin() -> str:
    for cand in (shutil.which("mysql"), "/opt/homebrew/bin/mysql", "/usr/local/bin/mysql",
                 shutil.which("mariadb")):
        if cand and os.access(cand, os.X_OK):
            return cand
    return ""


class Server:
    def __init__(self, binary: pathlib.Path, port: int, data_dir: pathlib.Path,
                 storage: str, wal_sync: str, log: pathlib.Path):
        self.p = None
        self.log = log
        cmd = [str(binary), "--log-level", "warn", "serve", "--host", "127.0.0.1",
               "--port", str(port), "--data-dir", str(data_dir),
               "--storage", storage, "--wal-sync", wal_sync]
        self.p = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        self.outfile = open(log, "wb")
        # 不阻塞读：把 server 输出灌进日志，避免管道塞满导致假死
        import threading
        threading.Thread(target=lambda: [self.outfile.write(l) for l in self.p.stdout],
                         daemon=True).start()

    def ready(self, my: str, port: int, tries: int = 40) -> bool:
        for _ in range(tries):
            try:
                r = subprocess.run([my, "-h", "127.0.0.1", "-P", str(port), "-u", "root",
                                    "--batch", "-N", "-e", "SELECT 1"],
                                   capture_output=True, timeout=5)
                if r.returncode == 0:
                    return True
            except subprocess.TimeoutExpired:
                pass
            time.sleep(0.5)
        return False

    def kill9(self):
        if self.p and self.p.poll() is None:
            self.p.send_signal(signal.SIGKILL)
            self.p.wait(timeout=10)
        try:
            self.outfile.close()
        except Exception:
            pass


def q(my: str, port: int, sql: str, timeout: int = 20):
    try:
        r = subprocess.run([my, "-h", "127.0.0.1", "-P", str(port), "-u", "root",
                            "--batch", "-N", "-e", sql], capture_output=True, timeout=timeout)
        return r.returncode, r.stdout.decode("utf-8", "replace"), r.stderr.decode("utf-8", "replace")
    except subprocess.TimeoutExpired:
        return 124, "", "TIMEOUT after %ss" % timeout


def probe_null_shape(my: str, port: int) -> dict:
    """NULL 形状探针：真 NULL 与字面量 'NULL' 必须在结果集里可区分，且服务端谓词要数对。"""
    out = {"null_probed": False}
    rc, _, err = q(my, port, NULL_DDL)
    if rc != 0:
        out["null_probe_ddl_error"] = err.strip()[:160]
        return out
    rc, _, err = q(my, port, "INSERT INTO smoke.nn VALUES (1, NULL), (2, 'NULL');")
    if rc != 0:
        out["null_probe_insert_error"] = err.strip()[:160]
        return out
    rc, raw, err = q(my, port, NULL_SQL)
    if rc != 0:
        out["null_probe_select_error"] = err.strip()[:160]
        return out
    out["null_probed"] = True
    out["null_wire_raw"] = raw.strip()[:200]
    m = {r[0]: (r[1] if len(r) > 1 else None) for r in cell_rows(raw) if r}
    a, b = m.get("1"), m.get("2")
    out["null_wire_detail"] = "null_row=%r literal_row=%r" % (a, b)
    out["null_wire_ambiguous"] = (a == b)
    c1 = q(my, port, "SELECT COUNT(*) FROM smoke.nn WHERE s IS NULL;")
    c2 = q(my, port, "SELECT COUNT(*) FROM smoke.nn WHERE s = 'NULL';")
    out["null_predicate_detail"] = "is_null=%r eq_literal_NULL=%r" % (c1[1].strip()[:16], c2[1].strip()[:16])
    out["null_predicate_ok"] = (c1[1].strip() == "1" and c2[1].strip() == "1")
    return out


def run_scenario(binary: pathlib.Path, my: str, work: pathlib.Path, storage: str,
                 wal_sync: str, tag: str, do_backup_restore: bool) -> dict:
    data = work / ("data_%s" % tag)
    if data.exists():
        shutil.rmtree(data)
    data.mkdir(parents=True)
    obs = {"tag": tag, "storage": storage, "wal_sync": wal_sync}
    port = free_port()
    srv = Server(binary, port, data, storage, wal_sync, work / ("server_%s.log" % tag))
    obs["server_up"] = srv.ready(my, port)
    if not obs["server_up"]:
        obs["server_log_tail"] = (work / ("server_%s.log" % tag)).read_text(errors="replace")[-800:]
        return obs

    ddl = []
    rc, _, err = q(my, port, "CREATE DATABASE IF NOT EXISTS smoke;")
    if rc != 0:
        ddl.append("CREATE DATABASE rc=%d %s" % (rc, err.strip()[:200]))
    else:
        # 关键判据：DDL 报成功就必须真的生效（binary 臂实测：CREATE DATABASE 返回 OK 但 SHOW DATABASES 里没有）
        rc2, out2, _ = q(my, port, "SHOW DATABASES;")
        if "smoke" not in out2:
            ddl.append("CREATE DATABASE 报成功但未生效：SHOW DATABASES=%r" % out2.split())
    rc, _, err = q(my, port, TABLE_DDL)
    if rc != 0:
        ddl.append("CREATE TABLE rc=%d %s" % (rc, err.strip()[:200]))
    obs["ddl_errors"] = ddl
    ins_errs = []
    for i, name, amount, note in ROWS:
        note_sql = "'" + note.replace("\\", "\\\\").replace("'", "''") + "'"
        sql = ("INSERT INTO smoke.t VALUES (%d, '%s', %d, %s);" % (i, name.replace("'", "''"), amount, note_sql))
        rc, _, err = q(my, port, sql)
        if rc != 0:
            ins_errs.append("INSERT id=%d rc=%d %s" % (i, rc, err.strip()[:160]))
    obs["insert_errors"] = ins_errs
    obs["wal_nonempty"] = (data / "sqlrustgo.wal").is_file() and (data / "sqlrustgo.wal").stat().st_size > 0
    tf = [f for f in nonzero_files(data) if "smoke" in f[0]]
    obs["table_files_nonempty"] = bool(tf)
    obs["data_files"] = nonzero_files(data)[:12]

    _, out, _ = q(my, port, SELECT_SQL)
    obs["select_before"] = cell_rows(out)
    obs.update(probe_null_shape(my, port))

    # 真崩溃：kill -9，然后同一 data dir 重启，逐值读回
    srv.kill9()
    time.sleep(1)
    srv2 = Server(binary, port, data, storage, wal_sync, work / ("server_%s_r2.log" % tag))
    if not srv2.ready(my, port):
        obs["server_up"] = False
        obs["restart_after_crash_failed"] = True
        return obs
    obs["crash_restarted"] = True
    _, out, err = q(my, port, SELECT_SQL)
    obs["crash_readback"] = cell_rows(out)
    if err.strip():
        obs["crash_readback_error"] = err.strip()[:200]

    if do_backup_restore:
        bk = work / ("bk_%s" % tag)
        if bk.exists():
            shutil.rmtree(bk)
        bk.mkdir(parents=True)
        rb = subprocess.run([str(binary), "--log-level", "warn", "backup", str(bk),
                             "--database", "smoke", "--data-dir", str(data)],
                            capture_output=True, timeout=60)
        obs["backup_rc"] = rb.returncode
        obs["backup_stdout"] = rb.stdout.decode("utf-8", "replace")[-400:]
        stems = sorted(p.name for p in bk.glob("*.data.json"))
        obs["backup_files"] = nonzero_files(bk)
        payload = bk / stems[0] if stems else None
        rows_backed = 0
        if payload and payload.stat().st_size > 0:
            txt = payload.read_text(errors="replace")
            for i, name, amount, note in ROWS:
                if str(amount) in txt and ("名称-%d" % i) in txt:
                    rows_backed += 1
        obs["backup_rows"] = rows_backed
        if payload:
            stem = payload.name[: -len(".data.json")]
            data2 = work / ("data_%s_restored" % tag)
            if data2.exists():
                shutil.rmtree(data2)
            data2.mkdir(parents=True)
            rr = subprocess.run([str(binary), "--log-level", "warn", "restore", stem,
                                 "--backup-dir", str(bk), "--database", "smoke",
                                 "--data-dir", str(data2), "--drop-first"],
                                capture_output=True, timeout=60)
            obs["restore_rc"] = rr.returncode
            obs["restore_stdout"] = rr.stdout.decode("utf-8", "replace")[-400:]
            wrote = nonzero_files(data2)
            obs["restored_files"] = wrote[:12]
            obs["restore_wrote_nothing"] = (rr.returncode == 0 and not wrote)
            srv.kill9()
            time.sleep(1)
            port3 = free_port()
            srv3 = Server(binary, port3, data2, "file", "every", work / ("server_%s_r3.log" % tag))
            if not srv3.ready(my, port3):
                obs["server_up"] = False
                obs["restore_restart_failed"] = True
                return obs
            _, out3, err3 = q(my, port3, SELECT_SQL)
            obs["restore_readback"] = cell_rows(out3)
            if err3.strip():
                obs["restore_readback_error"] = err3.strip()[:200]
            srv3.kill9()
    else:
        obs["backup_rows"] = len(ROWS)      # 判别臂不测备份（它连 DDL 都不成立）
        obs["restore_readback"] = expected_rows()
        obs["restore_wrote_nothing"] = False
    srv.kill9()
    obs["server_up"] = True
    return obs


def cmd_run(args) -> int:
    missing = [str(p) for p in (SERVER_BIN,) if not p.is_file()]
    my = mysql_bin()
    if not my:
        print("INCOMPLETE: 找不到 mysql/mariadb 客户端（用 PATH 或 /opt/homebrew/bin/mysql）", file=sys.stderr)
        return 2
    if missing:
        print("INCOMPLETE: 缺产物 %s；先跑 cargo build -p sqlrustgo-mysql-server --bin sqlrustgo-mysql-server"
              % ", ".join(missing), file=sys.stderr)
        return 2
    work = pathlib.Path(args.out) if args.out else (REPO / "test_results/capability_smoke/manual")
    work.mkdir(parents=True, exist_ok=True)
    t0 = time.time()
    prod = run_scenario(SERVER_BIN, my, work, "file", "every", "prod", True)
    arm = run_scenario(SERVER_BIN, my, work, "binary", "every", "arm_binary", False)
    jp, ja = judge_scenario(prod), judge_scenario(arm)
    disc = judge_discriminator(jp, ja)
    ov = overall(jp, ja, disc)
    rec = {"kind": "capability_smoke", "at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
           "seconds": round(time.time() - t0, 1),
           "bin": {"server": str(SERVER_BIN.relative_to(REPO)), "sha256": sha(SERVER_BIN),
                   "mysql_client": my},
           "git_head": subprocess.run(["git", "-C", str(REPO), "rev-parse", "HEAD"],
                                      capture_output=True, text=True).stdout.strip(),
           "expected_rows": len(ROWS),
           "prod": {"obs": prod, "judge": jp},
           "arm_nonproduction": {"obs": arm, "judge": ja},
           "discriminator": disc, "overall": ov}
    (work / "capability_smoke.json").write_text(json.dumps(rec, ensure_ascii=False, indent=1))
    print(json.dumps({"kind": "capability_smoke_summary", "out": str(work),
                      "prod_findings": jp["findings"][:6],
                      "arm_findings": ja["findings"][:6],
                      "prod_n": len(jp["findings"]), "arm_n": len(ja["findings"]),
                      "discriminator": disc, "overall": ov}, ensure_ascii=False, indent=1))
    print("full report: %s" % (work / "capability_smoke.json"))
    return 0 if ov["verdict"] == "PASS" else 1


# =============================================================================
# 静态普查：声称验证存储语义的测试里有多少只在 Mock/内存引擎上跑
# =============================================================================

def cmd_static(args) -> int:
    roots = [REPO / "tests"] + list((REPO / "crates").glob("*/tests")) if (REPO / "crates").is_dir() else []
    files = []
    for r in roots:
        if r.is_dir():
            files += [p for p in r.rglob("*.rs")]
    mock_only, production, ambiguous = [], [], []
    for p in files:
        try:
            txt = p.read_text(errors="replace")
        except Exception:
            continue
        low = txt.lower()
        if not any(k in low for k in SEMANTIC_KEYS):
            continue
        has_mock = "MemoryStorage" in txt
        has_prod = ("CARGO_BIN_EXE" in txt) or ("FileStorage" in txt) or ("FileBackedWalManager" in txt) \
            or ("physical-backup" in txt) or ("physical_backup" in txt)
        name = str(p.relative_to(REPO))
        if has_prod and not has_mock:
            production.append(name)
        elif has_mock and not has_prod:
            mock_only.append(name)
        elif has_prod and has_mock:
            ambiguous.append(name)
    # 「Mock 不得支撑已验证声明」：文档行里点名了某个 mock-only 测试，又写了「已验证」这类结论
    claim_re = re.compile(r"(已验证|验证通过|已覆盖|已闭环)")
    doc_hits = []
    if CLAIM_DOCS.is_dir():
        for d in CLAIM_DOCS.rglob("*.md"):
            for ln, line in enumerate(d.read_text(errors="replace").splitlines(), 1):
                if not claim_re.search(line):
                    continue
                for m in mock_only:
                    if pathlib.Path(m).stem in line:
                        doc_hits.append({"doc": str(d.relative_to(REPO)), "line": ln,
                                         "mock_test": m, "text": line.strip()[:160]})
    out = {"kind": "capability_smoke_static",
           "scanned_test_files_matching_semantic_keys": len(production) + len(mock_only) + len(ambiguous),
           "production_backed": len(production), "mock_only": len(mock_only),
           "mixed": len(ambiguous),
           "mock_only_files": mock_only[:40], "mixed_files": ambiguous[:20],
           "claim_lines_naming_mock_only_test": doc_hits[:20],
           "note": "普查口径：文件名/内容含 %s；production=引用真 server 或 FileStorage/WAL 装配；mock_only=只用 MemoryStorage"
                   % ("/".join(SEMANTIC_KEYS))}
    print(json.dumps(out, ensure_ascii=False, indent=1))
    return 0


def main(argv) -> int:
    ap = argparse.ArgumentParser(prog="capability_smoke.py")
    ap.add_argument("mode", nargs="?", default="run", choices=["run", "static"])
    ap.add_argument("--out")
    ap.add_argument("--selftest", action="store_true")
    a = ap.parse_args(argv)
    if a.selftest:
        return run_selftest()
    if a.mode == "run":
        return cmd_run(a)
    return cmd_static(a)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
