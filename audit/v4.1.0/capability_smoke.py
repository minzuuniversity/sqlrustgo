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
import glob
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
import threading
import tempfile
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

# NULL 形状判据：一行真 NULL、一行字面量 'NULL'。
# 判据口径（2026-11-06 实测纠正，原口径是错的，见下）：
#   ✗ 旧口径「mysql 客户端 --batch 下真 NULL 渲染成 \\N，与字面量 'NULL' 可区分」——**实测不成立**：
#     同一条 MariaDB 客户端打**参考服务端**（MariaDB 12.3.2）时，两行同样都打印 `NULL`。
#     把客户端渲染当成被测缺陷，等于拿客户端行为给被测对象定罪（run 20261106-cap-smoke-001 里
#     我就这么误报了一条 null_ambiguous_on_wire，现已撤）。
#   ✓ 新口径：只看**结果集行包的字节**。SQL NULL 必须是长度编码 NULL(0xFB)，字面量 'NULL'
#     必须是 0x04 4e 55 4c 4c。客户端渲染只在「与参考服务端不一致」时才算被测缺陷。
NULL_DDL = "CREATE TABLE smoke.nn (id INT PRIMARY KEY, s VARCHAR(32))"
NULL_SQL = "SELECT id,s FROM smoke.nn ORDER BY id;"

# =============================================================================
# 结果集**包级**判据（NULL 编码）：只认字节，不认客户端打印出来的字符串。
# 为什么必须下沉到包级 —— 这是一次实测纠偏，不是风格问题：
#   run 20261106-cap-smoke-001 里我把「客户端把真 NULL 与字面量 'NULL' 都打印成 NULL」
#   记成被测缺陷（null_ambiguous_on_wire），期望写的是「--batch 下真 NULL 应为 \N」。
#   现场补了一台**参考服务端**（MariaDB 12.3.2，同一条客户端、同一条 SQL）复测：
#     客户端侧：两行同样都打印 `NULL` ⇒ 那是客户端渲染，与被测对象无关；
#     包级侧：真 NULL = `01 31 fb`，字面量 = `01 33 04 4e 55 4c 4c` ⇒ 协议层分得清。
#   被测服务端的行包与参考服务端**逐字节同形** ⇒ 那条 finding 是我自己的探针错，已撤。
#   教训：**期望值必须有一台参考实现兜底**，否则判据会把「我记错的约定」当成被测缺陷。
#   参考实现由 reference_control() 现场起，不作为前置假设（起不来 ⇒ available=false，判据不加戏）。
# =============================================================================


def split_packets(stream: bytes) -> list:
    """MySQL 包帧：3 字节小端长度 + 1 字节序列号 + 负载。返回负载列表（原字节）。"""
    out, buf = [], stream
    while len(buf) >= 4:
        ln = int.from_bytes(buf[0:3], "little")
        if len(buf) < 4 + ln:
            break
        out.append(buf[4:4 + ln])
        buf = buf[4 + ln:]
    return out


def wire_row_packets(server_stream: bytes) -> list:
    """两个 EOF(0xFE 且负载 <9) 之间的包 = 结果集行包。列定义/OK/ERR 进不来。"""
    pkts = split_packets(server_stream)
    eof = [i for i, p in enumerate(pkts) if p[:1] == b"\xfe" and len(p) < 9]
    if len(eof) < 2:
        return []
    return pkts[eof[0] + 1:eof[1]]


def wire_fields(payload: bytes) -> list:
    """行负载 → 字段列表；NULL(0xFB) 解成 None，其余解成原始字节。
    判据与反向变异夹具**共用这一份实现**（夹具里不许另写解析器，否则夹具会先于判据坏）。"""
    out, i = [], 0
    while i < len(payload):
        b0 = payload[i]
        if b0 < 0xFB:
            out.append(payload[i + 1:i + 1 + b0])
            i += 1 + b0
        elif b0 == 0xFB:
            out.append(None)
            i += 1
        elif b0 == 0xFC:
            ln = int.from_bytes(payload[i + 1:i + 3], "little")
            out.append(payload[i + 3:i + 3 + ln])
            i += 3 + ln
        elif b0 == 0xFD:
            ln = int.from_bytes(payload[i + 1:i + 4], "little")
            out.append(payload[i + 4:i + 4 + ln])
            i += 4 + ln
        elif b0 == 0xFE:
            ln = int.from_bytes(payload[i + 1:i + 8], "little")
            out.append(payload[i + 8:i + 8 + ln])
            i += 8 + ln
        else:      # 0xFF 不是合法 lenenc 前缀：整段吞掉，让判据去红
            out.append(payload[i + 1:])
            i = len(payload)
    return out


def wire_rows_from_obs(obs: dict) -> dict:
    """obs 里存**行包十六进制**（归档里能直接看见字节），判据现解。
    返回 {id: [除 id 外的字段...]}；字段值 = None(0xFB) 或 bytes。"""
    out = {}
    for rid, hexstr in (obs.get("wire_rows_hex") or {}).items():
        try:
            fields = wire_fields(bytes.fromhex(hexstr))
        except ValueError:
            fields = []
        out[rid] = fields[1:] if len(fields) > 1 else []
    return out


def row_payload(fields: list) -> bytes:
    """lenenc **编码器**：只有 --selftest 的反向变异夹具用它，判据用的是 wire_fields（解码器）。
    故意让夹具与判据走两套实现：任一边写错，夹具与判据就会互相顶穿，不会假绿。"""
    buf = b""
    for f in fields:
        if f is None:
            buf += b"\xfb"
            continue
        b = f if isinstance(f, bytes) else str(f).encode()
        if len(b) < 251:
            buf += bytes([len(b)]) + b
        elif len(b) < (1 << 16):
            buf += b"\xfc" + len(b).to_bytes(2, "little") + b
        elif len(b) < (1 << 24):
            buf += b"\xfd" + len(b).to_bytes(3, "little") + b
        else:
            buf += b"\xfe" + len(b).to_bytes(8, "little") + b
    return buf


def judge_wire(obs: dict) -> list:
    """NULL 的协议层判据。抓不到包 ⇒ 单独一条 finding，措辞是「没能核对」而不是「被测对象坏」。"""
    if not obs.get("wire_probed"):
        return []
    if obs.get("wire_capture_error"):
        return ["wire_not_observed: 包级探针没抓到结果集（%s）⇒ 这一维只能记「没能核对」，不得写成已验证"
                % str(obs.get("wire_capture_error"))[:120]]
    d = wire_rows_from_obs(obs)
    null_row = d.get("1") or []
    lit_row = d.get("2") or []
    ref = obs.get("wire_reference") or {}
    f = []
    if not null_row or null_row[0] is not None:
        f.append("wire_null_not_lenenc_null: id=1 插入的是 SQL NULL，行包里 s 字段却是 %r（期望 0xFB）；"
                 "参考服务端同题包=%s"
                 % (null_row[0] if null_row else "缺行",
                    (ref.get("rows_hex") or {}).get("1", "-")))
    if not lit_row or lit_row[0] != b"NULL":
        f.append("wire_literal_null_misencoded: id=2 插入的是字面量 'NULL'，行包里 s 字段却是 %r"
                 "（期望 04 4e 55 4c 4c）" % (lit_row[0] if lit_row else "缺行"))
    if ref.get("available") and obs.get("client_batch_both_null") and not ref.get("client_batch_both_null"):
        f.append("client_visible_null_ambiguity_vs_reference: 同一条客户端在参考服务端上分得开 NULL/'NULL'，"
                 "在被测服务端上分不开（参考=%r 被测=%r）" % (ref.get("client_raw"), obs.get("client_raw")))
    return f


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
    findings.extend(judge_wire(obs))
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

    # ---- 包级（NULL 编码）反向变异：夹具用 row_payload **编码**，判据用 wire_fields **解码** ----
    # 这组臂是 run 20261106-cap-smoke-001 那次误报的**遗物**：我当时的判据把「客户端渲染歧义」
    # 当成被测缺陷，现场补一台参考服务端复测后证明是我的期望错了。W4 就是防止我再犯的臂。
    HEX_OK = {"1": row_payload([b"1", None]).hex(),
              "2": row_payload([b"2", b"NULL"]).hex()}
    REF = {"available": True, "client_batch_both_null": True,
           "client_raw": "1\tNULL\n2\tNULL", "rows_hex": HEX_OK}

    def _w(extra):
        o = _healthy_obs()
        o["null_probed"] = True
        o["null_predicate_ok"] = True
        o["wire_probed"] = True
        o["wire_rows_hex"] = dict(HEX_OK)
        o["client_raw"] = "1\tNULL\n2\tNULL"
        o["client_batch_both_null"] = False
        o["wire_reference"] = dict(REF)
        o.update(extra)
        return o

    w1 = judge_scenario(_w({}))
    arms.append(("W1 包级 NULL=0xFB 且字面量=04 4e 55 4c 4c ⇒ 必须绿", w1["verdict"] == "PASS",
                 w1["findings"][:2]))
    ok += 1 if arms[-1][1] else 0

    hx2 = dict(HEX_OK)
    hx2["1"] = row_payload([b"1", b"NULL"]).hex()
    w2 = judge_scenario(_w({"wire_rows_hex": hx2}))
    arms.append(("W2 SQL NULL 被编码成字面量字节 ⇒ 必须红", w2["verdict"] == "FAIL", w2["findings"][:1]))
    ok += 1 if arms[-1][1] else 0

    hx3 = dict(HEX_OK)
    hx3["2"] = row_payload([b"2", None]).hex()
    w3 = judge_scenario(_w({"wire_rows_hex": hx3}))
    arms.append(("W3 字面量 'NULL' 被编码成 0xFB ⇒ 必须红", w3["verdict"] == "FAIL", w3["findings"][:1]))
    ok += 1 if arms[-1][1] else 0

    w4 = judge_scenario(_w({"client_batch_both_null": True}))
    arms.append(("W4 客户端两行都打印 NULL 但参考服务端同样 ⇒ 必须绿（run-001 我误报的那一型）",
                 w4["verdict"] == "PASS", w4["findings"][:2]))
    ok += 1 if arms[-1][1] else 0

    w5 = judge_scenario(_w({"client_batch_both_null": True,
                            "wire_reference": dict(REF, client_batch_both_null=False)}))
    arms.append(("W5 参考服务端分得开而被测分不开 ⇒ 必须红", w5["verdict"] == "FAIL", w5["findings"][:1]))
    ok += 1 if arms[-1][1] else 0

    w6 = judge_scenario(_w({"wire_rows_hex": {},
                            "wire_capture_error": "代理没等到客户端连接（timeout）"}))
    arms.append(("W6 包级探针没抓到 ⇒ 必须红且措辞是「没能核对」",
                 w6["verdict"] == "FAIL" and any("没能核对" in x for x in w6["findings"]),
                 w6["findings"][:1]))
    ok += 1 if arms[-1][1] else 0

    w7 = _healthy_obs()
    w7["wire_reference"] = dict(REF)
    g7 = judge_scenario(w7)
    arms.append(("W7 包级探针没跑 ⇒ 不得凭空报 NULL 缺陷", g7["verdict"] == "PASS", g7["findings"][:2]))
    ok += 1 if arms[-1][1] else 0

    le = (wire_fields(row_payload([b"a", None, b"bb"])) == [b"a", None, b"bb"]
          and wire_fields(b"\xfc\x05\x00" + b"x" * 5) == [b"xxxxx"]
          and wire_fields(b"\xfd\x03\x00\x00" + b"yzw") == [b"yzw"])
    arms.append(("W8 lenenc 编解码自洽（1B/0xFC/0xFD/0xFB）", le is True, [str(le)]))
    ok += 1 if arms[-1][1] else 0

    # W9 包帧切分本身也要被变异覆盖：run 002 首跑就是死在这里（wire_row_packets 里变量名写错，
    # 异常被线程吞掉，判据只报「没能核对」——诚实但没有夹具挡着，下次坏成别的样就没人发现）。
    def _stream(pkts):
        out = b""
        for i, pl in enumerate(pkts):
            out += len(pl).to_bytes(3, "little") + bytes([i & 0xFF]) + pl
        return out

    _eof = b"\xfe\x00\x00\x00\x00"
    _framed = _stream([b"\x0a\x05\x00\x00\x01",                 # handshake
                       b"\x00\x00\x00\x02\x00\x00\x00\x01\x00",  # OK（登录）
                       b"\x01\x16",                              # 列数
                       b"\x03def\x02nn\x00\x01s",                # 列定义
                       _eof,
                       row_payload([b"1", None]),
                       row_payload([b"2", b"NULL"]),
                       _eof])
    rp = wire_row_packets(_framed)
    trunc = _framed[: len(_framed) - len(_eof)]                 # 少一个 EOF ⇒ 切不出行包
    arms.append(("W9 包帧切分：只在两个 EOF 之间取行包；EOF 不齐 ⇒ 切不出（不得假绿）",
                 len(rp) == 2 and wire_fields(rp[0]) == [b"1", None]
                 and wire_fields(rp[1]) == [b"2", b"NULL"]
                 and len(wire_row_packets(trunc)) == 0,
                 ["rows=%d trunc=%d" % (len(rp), len(wire_row_packets(trunc)))]))
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
    """NULL 形状探针：客户端渲染只作**观测**记录（渲染歧义本身不是被测缺陷，见 §「包级判据」注释），
    服务端 IS NULL / = 'NULL' 谓词计数才是判据。真正的缺陷判据在 judge_wire（看包级字节）。"""
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


# =============================================================================
# 包级探针：TCP 代理抓 server→client 的结果集行包字节
# =============================================================================

def _pump(src, dst, sink, record: bool):
    try:
        while True:
            b = src.recv(65536)
            if not b:
                break
            if record:
                sink.append(b)
            dst.sendall(b)
    except Exception:
        pass
    finally:
        try:
            dst.shutdown(socket.SHUT_WR)
        except Exception:
            pass


def capture_wire(my: str, up_port: int, query: str, wait_sec: float = 25.0) -> dict:
    """在同一条客户端与被测服务端之间插一个 TCP 代理，只转发一次连接，把 server→client 的
    字节流按 MySQL 包帧切开，取结果集行包（两个 EOF 之间的包），按**行 id → 行负载 hex**存进 obs。
    抓不到（超时/查询报错/没有结果集）⇒ 只写 wire_capture_error，判据据此记「没能核对」。"""
    out = {"wire_probed": False}
    lst = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    try:
        lst.bind(("127.0.0.1", 0))
        lst.listen(1)
        proxy_port = lst.getsockname()[1]
    except OSError as e:
        out["wire_capture_error"] = "代理监听失败 %s" % e
        return out

    box = {"err": "", "rows": {}, "raw": "", "rc": None}

    def serve_once():
        try:
            lst.settimeout(wait_sec)
            c, _ = lst.accept()
        except Exception as e:
            box["err"] = "代理没等到客户端连接（%s）" % e
            return
        try:
            u = socket.create_connection(("127.0.0.1", up_port), timeout=5)
        except Exception as e:
            box["err"] = "代理连上游失败（%s）" % e
            c.close()
            return
        cs, us = [], []
        t1 = threading.Thread(target=_pump, args=(c, u, cs, True), daemon=True)
        t2 = threading.Thread(target=_pump, args=(u, c, us, True), daemon=True)
        t1.start()
        t2.start()
        for t in (t1, t2):
            t.join(timeout=wait_sec)
        box["accepted"] = True
        stream = b"".join(us)
        pkts = wire_row_packets(stream)
        rows = {}
        for p in pkts:
            f = wire_fields(p)
            if not f:
                continue
            key = f[0].decode("utf-8", "replace") if f[0] is not None else "?"
            rows[key] = p.hex()
        box["rows"] = rows
        box["stream_bytes"] = len(stream)
        box["row_pkts"] = len(pkts)
        c.close()
        u.close()

    # 客户端必须在**主线程**跑：代理的 accept() 要等客户端连上来，把客户端也放线程里、
    # 又排在 accept 之后，就是自己等自己（run 002 首跑实测：代理超时 ⇒ wire_not_observed）。
    th = threading.Thread(target=serve_once, daemon=True)
    th.start()
    time.sleep(0.3)
    rc, raw, err = q(my, proxy_port, query)
    box["rc"], box["raw"], box["client_err"] = rc, raw, err
    th.join(timeout=wait_sec + 15)
    lst.close()

    out["wire_probed"] = True
    if box["err"]:
        out["wire_capture_error"] = box["err"]
        return out
    if box["rc"]:
        out["wire_capture_error"] = "客户端经代理查询 rc=%s %s" % (
            box.get("rc"), str(box.get("client_err", "")).strip()[:120])
        return out
    if not box["rows"]:
        out["wire_capture_error"] = "代理抓到 %s 字节的行包区但没有可解析的行包（stream=%s pkts=%s）" % (
            box.get("stream_bytes"), box.get("stream_bytes"), box.get("row_pkts"))
        return out
    out["wire_rows_hex"] = box["rows"]
    out["wire_row_packet_count"] = box["row_pkts"]
    m = {r[0]: (r[1] if len(r) > 1 else None) for r in cell_rows(box["raw"]) if r}
    out["client_raw"] = box["raw"].strip()[:200]
    out["client_batch_both_null"] = (m.get("1") == m.get("2"))
    return out


def mariadbd_bin() -> str:
    cands = [shutil.which("mariadbd"), shutil.which("mysqld")]
    cands += sorted(glob.glob("/opt/homebrew/Cellar/mariadb/*/bin/mariadbd"), reverse=True)
    cands += sorted(glob.glob("/opt/homebrew/bin/mariadbd"), reverse=True)
    for c in cands:
        if c and os.access(c, os.X_OK):
            return c
    return ""


def _install_db(bd: str) -> str:
    d = pathlib.Path(bd).parent
    for name in ("mariadb-install-db", "mysql_install_db"):
        p = d / name
        if p.is_file() and os.access(p, os.X_OK):
            return str(p)
    return shutil.which("mariadb-install-db") or shutil.which("mysql_install_db") or ""


def reference_control(work: pathlib.Path, my: str) -> dict:
    """现场起一台**参考服务端**（MariaDB），给 NULL 判据一个「期望值」兜底。
    起不来 ⇒ available=false，判据不加戏（不得因为参考实现缺位就把被测判红）。"""
    out = {"available": False}
    bd = mariadbd_bin()
    if not bd:
        out["error"] = "本机没有 mariadbd/mysqld ⇒ 参考实现缺位，NULL 判据只保留包级字节口径"
        return out
    out["mariadbd"] = bd
    inst = _install_db(bd)
    if not inst:
        out["error"] = "找不到 mariadb-install-db/mysql_install_db ⇒ 参考实现缺位"
        return out
    ref = work / "reference_mariadb"
    if ref.exists():
        shutil.rmtree(ref, ignore_errors=True)
    ref.mkdir(parents=True)
    # **必须绝对路径**：mariadb-install-db 会按自己的 basedir 解析相对 datadir，
    # 相对路径会让它去 /opt/homebrew/Cellar/mariadb/<ver>/test_results/... 找目录（run 002 首跑实测 rc=1）。
    ref = ref.resolve()
    port = free_port()
    # socket / pid 必须放**短路径**：MariaDB 的 AF_UNIX 路径上限 103 字符，
    # 落在仓内 work 下会超限（run 002 首跑实测：`The socket file path is too long (> 103)` ⇒ 服务端 Aborting，
    # 表现成「TCP 20s 未就绪」，看着像端口问题，其实是路径问题）。
    short = pathlib.Path(tempfile.mkdtemp(prefix="srg410ref_"))
    sock = str(short / "s.sock")
    pidfile = str(short / "s.pid")
    ir = subprocess.run([inst, "--datadir=%s" % ref, "--auth-root-authentication-method=normal"],
                        capture_output=True, timeout=180)
    if ir.returncode != 0:
        out["error"] = "install-db rc=%d %s" % (ir.returncode,
                                                ir.stderr.decode("utf-8", "replace").strip()[-200:])
        return out
    proc = subprocess.Popen(
        [bd, "--datadir=%s" % ref, "--socket=%s" % sock, "--port=%d" % port,
         "--bind-address=127.0.0.1", "--pid-file=%s" % pidfile,
         "--log-error=%s" % (ref / "error.log")],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        up = False
        for _ in range(40):
            if q(my, port, "SELECT 1")[0] == 0:
                up = True
                break
            time.sleep(0.5)
        if not up:
            tail = ""
            try:
                tail = (ref / "error.log").read_text(errors="replace").strip().splitlines()[-3:]
            except Exception:
                pass
            out["error"] = "参考服务端 20s 内 TCP 未就绪（见 %s）：%s" % (ref / "error.log", " | ".join(tail))
            return out
        # 被测链里 `smoke` 库是 run_scenario 建的；参考服务端必须自己建，否则 NULL_DDL
        # 会因「未知数据库」失败，参考侧什么也观测不到（run 002 实测：available=True 但 rows={}）。
        setup = [q(my, port, "CREATE DATABASE IF NOT EXISTS smoke;")[0],
                 q(my, port, NULL_DDL)[0],
                 q(my, port, "INSERT INTO smoke.nn VALUES (1, NULL), (2, 'NULL');")[0]]
        if any(rc != 0 for rc in setup):
            out["setup_error"] = "参考服务端建表/写入 rc=%s" % setup
        cap = capture_wire(my, port, NULL_SQL)
        out["available"] = True
        out["client_raw"] = cap.get("client_raw", "")
        out["client_batch_both_null"] = cap.get("client_batch_both_null")
        out["rows_hex"] = cap.get("wire_rows_hex", {})
        if cap.get("wire_capture_error"):
            out["capture_error"] = cap["wire_capture_error"]
        ref_client = q(my, port, NULL_SQL)
        out["client_direct_raw"] = ref_client[1].strip()[:200]
        return out
    finally:
        try:
            proc.send_signal(signal.SIGTERM)
            proc.wait(timeout=15)
        except Exception:
            try:
                proc.kill()
            except Exception:
                pass
        shutil.rmtree(short, ignore_errors=True)


def run_scenario(binary: pathlib.Path, my: str, work: pathlib.Path, storage: str,
                 wal_sync: str, tag: str, do_backup_restore: bool,
                 reference: dict = None) -> dict:
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
    # 包级探针：同一条 SQL 走一次代理，抓结果集行包字节（NULL 必须是 0xFB）
    # 只在 NULL 探针表真建成时跑：探针表没建成（binary 臂就是这样）时不冒充「跑过」，
    # 也不把「探针自己没跑」写成 wire_not_observed —— DDL 没生效已经是独立 finding。
    if obs.get("null_probed"):
        obs.update(capture_wire(my, port, NULL_SQL))
    else:
        obs["wire_skipped"] = "NULL 探针表未建成（%s）⇒ 包级探针没跑" % (
            obs.get("null_probe_ddl_error") or obs.get("null_probe_insert_error")
            or obs.get("null_probe_select_error") or "无错误文本")
    if reference is not None:
        obs["wire_reference"] = reference

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
    # 参考实现（MariaDB）现场起一次，给 NULL 判据一个期望值兜底；起不来则判据自动退到字节口径。
    reference = None if args.no_reference else reference_control(work, my)
    if reference is not None:
        print("reference_control: available=%s mariadbd=%s client_both_null=%s rows=%s error=%s"
              % (reference.get("available"), reference.get("mariadbd", "-"),
                 reference.get("client_batch_both_null"),
                 {k: v[:24] for k, v in (reference.get("rows_hex") or {}).items()},
                 str(reference.get("error") or reference.get("setup_error")
                     or reference.get("capture_error", ""))[:200]),
              flush=True)
    prod = run_scenario(SERVER_BIN, my, work, "file", "every", "prod", True, reference)
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
           "wire_reference_control": (
               {k: reference.get(k) for k in ("available", "mariadbd", "client_raw",
                                              "client_batch_both_null", "rows_hex",
                                              "client_direct_raw", "capture_error",
                                              "setup_error", "error")}
               if reference else None),
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
    ap.add_argument("--no-reference", action="store_true",
                    help="不现场起参考服务端（MariaDB）。缺了参考兜底，NULL 判据只看包级字节，"
                         "客户端渲染歧义不再算 finding。")
    a = ap.parse_args(argv)
    if a.selftest:
        return run_selftest()
    if a.mode == "run":
        return cmd_run(a)
    return cmd_static(a)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
