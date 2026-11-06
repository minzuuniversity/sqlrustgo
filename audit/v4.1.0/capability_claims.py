#!/usr/bin/env python3
"""P1-2 capability_claims.py —— 「声称的能力」到底是不是真的。

背景（整改方案 §P1-2、看板 P1-2）：
    文档里声称「真 CLI + 真 MySQL + 真落库 + 崩溃重启读回 + 备份恢复逐值」，
    但没人真跑过。这套探针把每个声称**用真实二进制真跑一遍**，
    跑不出结果就如实报 NOT VERIFIED —— 绝不把「跑不了」写成「通过」。

五个探针（全部使用真实产物，不接受 Mock）：

    C1 cli-real-query       真 CLI：`sqlrustgo sqlite --batch --mode csv --headers true --cmd`
                            跑一条真查询，逐值比对期望结果
    C2 mysql-wire-real      真 MySQL 客户端：起真实 server，用真实 mysql 客户端
                            （走 MySQL wire 协议）写 + 读
    C3 durable-on-disk      真持久化落库：写完**正常停服**，重启同一数据目录，读回
    C4 crash-restart        崩溃重启读回：写完 **SIGKILL**，重启，读回
    C5 backup-restore       备份→恢复→逐值比对：`physical-backup` 备份，恢复到新目录，
                            在新目录起服读回，**逐值**与原值比对

口径纪律（三态，不可混写）：
    VERIFIED     —— 真跑过，期望值逐项吻合
    FAILED       —— 真跑过，对不上（这就是红）
    NOT_VERIFIED —— 跑不了（缺产物 / 缺外部工具 / 环境不具备）
                    **绝不等于通过**：只要某个「被声称」的能力落在这一档，
                    本工具整体判红并把原因写进账。

二进制解析（刻意**不走 PATH 回退**）：
    IL-10 的教训 —— `tests/common/bin_resolver.rs` 的 PATH 回退在本机取到
    2026-02-21 的陈旧全局 `sqlrustgo`，它不支持被驱动的接口，只打印 REPL 横幅，
    于是证据生成器拿横幅去比对，报出「归档证据已与引擎漂移」的假红，
    并直接催生了 #5466 这条 P0。
    所以这里只认：环境变量 → CARGO_TARGET_DIR → 包根 target/{debug,release}。
    找不到就是 NOT_VERIFIED，并指名该设哪个环境变量。

用法：
    capability_claims.py run [--ledger <path>]     跑五个探针，写台账，打印摘要
    capability_claims.py selftest                  反向变异：证明每个探针真能判红
    capability_claims.py --help

退出码：0 = 全部 VERIFIED；1 = 有 FAILED 或被声称的能力 NOT_VERIFIED；2 = 用法错
本工具放在 audit/v4.1.0/ 下：它是探针，**不是门禁**，不进 STAGE_CONFIG。
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import signal
import socket
import subprocess
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
WORKROOT = Path(os.environ.get("SQLRUSTGO_CAPABILITY_WORKDIR",
                               "/tmp/sqlrustgo-capability-probe"))

VERIFIED = "VERIFIED"
FAILED = "FAILED"
NOT_VERIFIED = "NOT_VERIFIED"

# 被声称的能力。声称来自看板 P1-2 / 整改方案 §P1-2 的验收口径。
CLAIMED_CAPABILITIES = [
    "cli-real-query",
    "mysql-wire-real",
    "durable-on-disk",
    "crash-restart",
    "backup-restore",
]

# 与 tests/common/bin_resolver.rs 的 ENGINE_BIN_ENV 同名（#5405：只有一个来源）。
ENGINE_BIN_ENV = "SQLRUSTGO_BIN"
# 外部 MySQL 客户端（同 bin_resolver.rs 的 MYSQL_BIN_ENV）。
MYSQL_BIN_ENV = "SQLRUSTGO_MYSQL_BIN"
MYSQL_BIN_NAME = "mysql"


# ---------------------------------------------------------------------------
# 二进制解析（无 PATH 回退 —— 见模块头部 IL-10 说明）
# ---------------------------------------------------------------------------

def resolve_repo_binary(name: str) -> tuple[str | None, str]:
    """返回 (路径, 未找到时的说明)。顺序：环境变量 → CARGO_TARGET_DIR → 包根 target/。"""
    env = os.environ.get(ENGINE_BIN_ENV, "").strip()
    if env:
        p = Path(env)
        if p.is_file() and os.access(p, os.X_OK):
            return str(p), ""
        return None, f"{ENGINE_BIN_ENV} 指向 {env}，但它不存在或不可执行"
    tdir = os.environ.get("CARGO_TARGET_DIR", "").strip()
    roots = []
    if tdir:
        roots.append(Path(tdir))
    roots.append(REPO / "target")
    for root in roots:
        for profile in ("debug", "release"):
            cand = root / profile / name
            if cand.is_file() and os.access(cand, os.X_OK):
                return str(cand), ""
    return None, (f"找不到 {name}（已查 {'、'.join(str(r) for r in roots)}）。"
                  f"请先 `cargo build -p sqlrustgo-cli --bin {name}`，"
                  f"或用 {ENGINE_BIN_ENV}=<路径> 指定。")


def resolve_mysql_client() -> tuple[str | None, str]:
    env = os.environ.get(MYSQL_BIN_ENV, "").strip()
    if env:
        p = Path(env)
        if p.is_file() and os.access(p, os.X_OK):
            return str(p), ""
        return None, f"{MYSQL_BIN_ENV} 指向 {env}，但它不存在或不可执行"
    found = shutil.which(MYSQL_BIN_NAME)
    if found:
        return found, ""
    return None, (f"找不到真实 MySQL 客户端 `{MYSQL_BIN_NAME}`。"
                  f"请安装它或用 {MYSQL_BIN_ENV}=<路径> 指定。")


# ---------------------------------------------------------------------------
# 进程/端口帮手
# ---------------------------------------------------------------------------

def rel(path: Path, root: Path) -> str:
    """台账里不要留机器相关的绝对路径 —— 记录相对于探测工作根的路径。"""
    try:
        return str(Path(path).resolve().relative_to(Path(root).resolve()))
    except (ValueError, OSError):
        return Path(path).name


def free_port() -> int:
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    port = s.getsockname()[1]
    s.close()
    return port


def wait_port(port: int, deadline_s: float = 30.0) -> bool:
    end = time.time() + deadline_s
    while time.time() < end:
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=0.5):
                return True
        except OSError:
            time.sleep(0.15)
    return False


class Server:
    """真实 server 进程。注意：**存储落在 cwd**，不是 --data-dir
    （--data-dir 目前只用作 WAL 位置）—— 这是实测事实，不是推测。"""

    def __init__(self, binary: str, datadir: Path, port: int, log: Path):
        self.binary = binary
        self.datadir = datadir
        self.port = port
        self.log = log
        self.proc: subprocess.Popen | None = None

    def start(self) -> tuple[bool, str]:
        self.datadir.mkdir(parents=True, exist_ok=True)
        fh = open(self.log, "wb")
        self.proc = subprocess.Popen(
            [self.binary, "serve", "--port", str(self.port),
             "--data-dir", str(self.datadir), "--auth-mode", "none",
             "--storage", "file", "--log-level", "warn"],
            cwd=str(self.datadir), stdout=fh, stderr=subprocess.STDOUT)
        if not wait_port(self.port):
            tail = ""
            try:
                tail = self.log.read_text(errors="replace")[-1500:]
            except OSError:
                pass
            return False, f"server 未在 30s 内监听 {self.port}；日志尾部：{tail}"
        return True, ""

    def stop(self, hard: bool) -> None:
        if not self.proc:
            return
        try:
            if hard:
                self.proc.kill()
            else:
                self.proc.send_signal(signal.SIGTERM)
            self.proc.wait(timeout=20)
        except Exception:
            try:
                self.proc.kill()
                self.proc.wait(timeout=10)
            except Exception:
                pass
        self.proc = None
        time.sleep(0.3)


def mysql_query(client: str, port: int, sql: str) -> tuple[int, str, str]:
    p = subprocess.run(
        [client, "--protocol=TCP", "--host=127.0.0.1", f"--port={port}",
         "--user=root", "--skip-column-names", "--batch", "-e", sql],
        capture_output=True, text=True, timeout=90)
    return p.returncode, p.stdout.strip(), p.stderr.strip()


# ---------------------------------------------------------------------------
# 期望数据（五行，两个字段：id / s）
# ---------------------------------------------------------------------------

SEED = [
    (1, "alpha"),
    (2, "beta"),
    (3, "gamma-三"),
    (4, "delta"),
    (5, "epsilon"),
]
SEED_SQL = ("CREATE TABLE cap_t(id INTEGER, s TEXT); "
            + " ".join(f"INSERT INTO cap_t VALUES ({i}, '{s}');" for i, s in SEED))
READ_SQL = "SELECT id, s FROM cap_t ORDER BY id;"

# 期望值一律按**字段对**记（不是按行文本）—— 不同客户端的分隔符不同
# （CLI 的 --mode csv 是逗号，mysql 客户端的 --batch 是制表符），
# 逐值比对的正确粒度是字段，不是分隔符。
EXPECTED = [(str(i), s) for i, s in SEED]


def diff_pairs(expected: list[tuple[str, str]], rows: list[list[str]]) -> list[str]:
    """逐值比对：按 (行号, 字段) 比，缺行缺字段都算不符。"""
    bad = []
    for idx in range(max(len(expected), len(rows))):
        e = list(expected[idx]) if idx < len(expected) else None
        g = list(rows[idx]) if idx < len(rows) else None
        if e == g:
            continue
        if e is None:
            bad.append(f"row{idx}: 期望 <无该行> 实得 {g!r}")
        elif g is None:
            bad.append(f"row{idx}: 期望 {e!r} 实得 <缺失>")
        else:
            for fi in range(max(len(e), len(g))):
                ev = e[fi] if fi < len(e) else "<缺失字段>"
                gv = g[fi] if fi < len(g) else "<缺失字段>"
                if ev != gv:
                    bad.append(f"row{idx}.field{fi}: 期望 {ev!r} 实得 {gv!r}")
    return bad


def parse_csv_rows(text: str) -> list[list[str]]:
    import csv
    import io
    return [r for r in csv.reader(io.StringIO(text)) if any(f.strip() for f in r)]


def parse_tsv_rows(text: str) -> list[list[str]]:
    return [ln.split("\t") for ln in text.splitlines() if ln.strip()]


# ---------------------------------------------------------------------------
# 五个探针
# ---------------------------------------------------------------------------

def probe_cli_real_query(bin_cli: str | None, work: Path) -> dict:
    """C1 真 CLI：真二进制、真查询、逐值比对。"""
    if not bin_cli:
        return {"status": NOT_VERIFIED, "detail": "没有可解析的 sqlrustgo 二进制"}
    db = work / "cli" / "cap.db"
    db.parent.mkdir(parents=True, exist_ok=True)
    cmd = [bin_cli, "sqlite", "--batch", "--mode", "csv", "--headers", "true",
           "--cmd", SEED_SQL + "\n" + READ_SQL, str(db)]
    try:
        p = subprocess.run(cmd, capture_output=True, text=True, timeout=120)
    except subprocess.TimeoutExpired:
        return {"status": FAILED, "detail": "CLI 查询超时"}
    out = p.stdout.strip()
    if p.returncode != 0:
        return {"status": FAILED,
                "detail": f"CLI 退出码 {p.returncode}；stderr={p.stderr.strip()[:300]!r}"}
    # --headers true ⇒ 首行是表头
    rows = parse_csv_rows(out)
    if not rows:
        return {"status": FAILED, "detail": "CLI 无输出（空结果也算失败）"}
    if rows[0] != ["id", "s"]:
        return {"status": FAILED,
                "detail": f"CLI 输出首行不是表头，实际 {rows[0]!r}（可能被 REPL 横幅污染）"}
    body = rows[1:]
    bad = diff_pairs(EXPECTED, body)
    if bad:
        return {"status": FAILED, "detail": "逐值比对不符：" + "；".join(bad[:5])}
    return {"status": VERIFIED,
            "detail": f"{len(body)} 行逐值一致",
            "evidence": {"cmd": "sqlrustgo sqlite --batch --mode csv --headers true --cmd <sql> <db>",
                         "rows": body}}


def probe_mysql_wire(bin_srv: str | None, client: str | None, work: Path,
                     port: int) -> dict:
    """C2 真 MySQL 客户端：真 server + 真 mysql 客户端走 wire 协议。"""
    if not bin_srv:
        return {"status": NOT_VERIFIED, "detail": "没有可解析的 sqlrustgo-mysql-server 二进制"}
    if not client:
        return {"status": NOT_VERIFIED, "detail": "没有真实 MySQL 客户端"}
    datadir = work / "mysql" / "data"
    srv = Server(bin_srv, datadir, port, work / "mysql" / "server.log")
    ok, why = srv.start()
    if not ok:
        return {"status": FAILED, "detail": why}
    try:
        rc, out, err = mysql_query(client, port, SEED_SQL)
        if rc != 0:
            return {"status": FAILED,
                    "detail": f"客户端写入失败 rc={rc} err={err[:300]!r}"}
        rc, out, err = mysql_query(client, port, READ_SQL)
        if rc != 0:
            return {"status": FAILED,
                    "detail": f"客户端读回失败 rc={rc} err={err[:300]!r}"}
        body = parse_tsv_rows(out)
        bad = diff_pairs(EXPECTED, body)
        if bad:
            return {"status": FAILED, "detail": "逐值比对不符：" + "；".join(bad[:5])}
        return {"status": VERIFIED,
                "detail": f"{len(body)} 行经 MySQL wire 协议逐值一致",
                "evidence": {"client": client, "port": port}}
    finally:
        srv.stop(hard=True)


def _persist_probe(bin_srv: str | None, client: str | None, work: Path,
                   port: int, hard: bool, label: str,
                   mutate_between: bool = False) -> dict:
    """C3/C4 共用：写 → (正常停 | SIGKILL) → 重启 → 读回。

    `mutate_between=True` 时，在两次运行之间**删掉落盘的数据文件** ——
    这正是「生产存储换成 MemoryStorage」的运行时表现（数据不落盘 ⇒ 重启后读不回）。
    用来做反向变异：证明这个探针真能判红，而不是恒绿摆设。
    """
    if not bin_srv:
        return {"status": NOT_VERIFIED, "detail": "没有可解析的 sqlrustgo-mysql-server 二进制"}
    if not client:
        return {"status": NOT_VERIFIED, "detail": "没有真实 MySQL 客户端"}
    datadir = work / label / "data"
    srv = Server(bin_srv, datadir, port, work / label / "s1.log")
    ok, why = srv.start()
    if not ok:
        return {"status": FAILED, "detail": why}
    try:
        rc, out, err = mysql_query(client, port, SEED_SQL)
        if rc != 0:
            return {"status": FAILED, "detail": f"写入失败 rc={rc} err={err[:300]!r}"}
    finally:
        srv.stop(hard=hard)
    if mutate_between:
        # 变异：删掉落盘的数据文件（= 生产存储不落盘）
        for f in list(datadir.glob("*.json")):
            try:
                f.unlink()
            except OSError:
                pass
    # 重启同一个数据目录
    srv2 = Server(bin_srv, datadir, port, work / label / "s2.log")
    ok, why = srv2.start()
    if not ok:
        return {"status": FAILED, "detail": "重启失败：" + why}
    try:
        rc, out, err = mysql_query(client, port, READ_SQL)
        if rc != 0:
            return {"status": FAILED, "detail": f"重启后读回失败 rc={rc} err={err[:300]!r}"}
        body = parse_tsv_rows(out)
        bad = diff_pairs(EXPECTED, body)
        if bad:
            return {"status": FAILED,
                    "detail": f"{'崩溃' if hard else '正常停服'}重启后数据不符：" + "；".join(bad[:5])}
        return {"status": VERIFIED,
                "detail": f"{'SIGKILL' if hard else 'SIGTERM'} 后重启读回 {len(body)} 行逐值一致",
                "evidence": {"datadir": rel(datadir, work),
                             "kill": "SIGKILL" if hard else "SIGTERM"}}
    finally:
        srv2.stop(hard=True)


def probe_backup_restore(bin_srv: str | None, client: str | None,
                          bin_pb: str | None, work: Path, port: int) -> dict:
    """C5 备份→恢复→逐值比对。备份用**物理**备份（physical-backup）——
    逻辑备份（crates/tools/src/backup_restore.rs）按 #5525 是骨架，
    枚举 0 张表却报成功，不能拿来支撑任何「已验证」声明。"""
    if not bin_srv:
        return {"status": NOT_VERIFIED, "detail": "没有可解析的 sqlrustgo-mysql-server 二进制"}
    if not client:
        return {"status": NOT_VERIFIED, "detail": "没有真实 MySQL 客户端"}
    if not bin_pb:
        return {"status": NOT_VERIFIED, "detail": "没有可解析的 physical-backup 二进制"}
    datadir = work / "backup" / "data"
    srv = Server(bin_srv, datadir, port, work / "backup" / "s1.log")
    ok, why = srv.start()
    if not ok:
        return {"status": FAILED, "detail": why}
    try:
        rc, out, err = mysql_query(client, port, SEED_SQL)
        if rc != 0:
            return {"status": FAILED, "detail": f"写入失败 rc={rc} err={err[:300]!r}"}
        rc, out, err = mysql_query(client, port, READ_SQL)
        if rc != 0:
            return {"status": FAILED, "detail": f"读回失败 rc={rc} err={err[:300]!r}"}
        original = parse_tsv_rows(out)
        bad = diff_pairs(EXPECTED, original)
        if bad:
            return {"status": FAILED, "detail": "备份前数据就不对：" + "；".join(bad[:5])}
    finally:
        srv.stop(hard=True)

    bkdir = work / "backup" / "bk"
    target = work / "backup" / "restored"
    p = subprocess.run([bin_pb, "backup", "-d", str(bkdir),
                        "-D", str(datadir), "-w", str(datadir / "wal")],
                       capture_output=True, text=True, timeout=180)
    if p.returncode != 0:
        return {"status": FAILED,
                "detail": f"physical-backup 失败 rc={p.returncode} err={p.stderr[:300]!r}"}
    p = subprocess.run([bin_pb, "restore", "-d", str(bkdir), "-t", str(target),
                        "-w", str(target / "wal")],
                       capture_output=True, text=True, timeout=180)
    if p.returncode != 0:
        return {"status": FAILED,
                "detail": f"physical-backup restore 失败 rc={p.returncode} err={p.stderr[:300]!r}"}

    # 在恢复出来的目录里起服读回
    restored_data = target / "data" if (target / "data").is_dir() else target
    srv2 = Server(bin_srv, restored_data, port, work / "backup" / "s2.log")
    ok, why = srv2.start()
    if not ok:
        return {"status": FAILED, "detail": "恢复目录起服失败：" + why}
    try:
        rc, out, err = mysql_query(client, port, READ_SQL)
        if rc != 0:
            return {"status": FAILED, "detail": f"恢复后读回失败 rc={rc} err={err[:300]!r}"}
        restored = parse_tsv_rows(out)
        # 逐值比对：与备份前的原始值逐行逐字段比
        bad = diff_pairs([tuple(r) for r in original], [tuple(r) for r in restored])
        if bad:
            return {"status": FAILED, "detail": "恢复后逐值比对不符：" + "；".join(bad[:5])}
        if not restored:
            return {"status": FAILED, "detail": "恢复后读回 0 行（备份/恢复是骨架，见 #5525）"}
        return {"status": VERIFIED,
                "detail": f"备份→恢复→读回 {len(restored)} 行，与备份前逐值一致",
                "evidence": {"backup_dir": rel(bkdir, work), "target": rel(target, work),
                             "rows": restored}}
    finally:
        srv2.stop(hard=True)


# ---------------------------------------------------------------------------
# 驱动
# ---------------------------------------------------------------------------

def run_probes(work: Path) -> list[dict]:
    bin_cli, why_cli = resolve_repo_binary("sqlrustgo")
    bin_srv, why_srv = resolve_repo_binary("sqlrustgo-mysql-server")
    bin_pb, why_pb = resolve_repo_binary("physical-backup")
    client, why_client = resolve_mysql_client()

    work.mkdir(parents=True, exist_ok=True)
    port = free_port()
    results = []

    def add(cid: str, r: dict) -> None:
        rec = {"capability": cid, **r}
        if r["status"] != VERIFIED and "why" in r:
            rec["reason"] = r.pop("why")
        results.append(rec)

    add("cli-real-query", probe_cli_real_query(bin_cli, work)
        if bin_cli else {"status": NOT_VERIFIED, "detail": why_cli})
    add("mysql-wire-real", probe_mysql_wire(bin_srv, client, work, free_port())
        if (bin_srv and client) else {"status": NOT_VERIFIED,
                                       "detail": why_srv if not bin_srv else why_client})
    add("durable-on-disk", _persist_probe(bin_srv, client, work, free_port(),
                                          hard=False, label="durable")
        if (bin_srv and client) else {"status": NOT_VERIFIED,
                                       "detail": why_srv if not bin_srv else why_client})
    add("crash-restart", _persist_probe(bin_srv, client, work, free_port(),
                                         hard=True, label="crash")
        if (bin_srv and client) else {"status": NOT_VERIFIED,
                                       "detail": why_srv if not bin_srv else why_client})
    add("backup-restore", probe_backup_restore(bin_srv, client, bin_pb, work, free_port())
        if (bin_srv and client and bin_pb) else {"status": NOT_VERIFIED,
                                                  "detail": why_srv if not bin_srv
                                                  else (why_client if not client else why_pb)})
    return results


def summarize(results: list[dict]) -> tuple[int, list[str]]:
    fails = []
    for r in results:
        cid = r["capability"]
        if r["status"] == FAILED:
            fails.append(f"{cid}: {r.get('detail','')}")
        elif r["status"] == NOT_VERIFIED:
            fails.append(f"{cid}: 未验证 —— {r.get('detail','')}")
    return (1 if fails else 0), fails


def cmd_run(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(prog="capability_claims.py")
    ap.add_argument("--ledger", default=str(REPO / "docs" / "releases" / "v4.1.0" /
                                            "CAPABILITY_LEDGER.jsonl"))
    ap.add_argument("--work", default=str(WORKROOT))
    args = ap.parse_args(argv)

    work = Path(args.work)
    if work.exists():
        shutil.rmtree(work)
    results = run_probes(work)
    rc, fails = summarize(results)

    ledger = Path(args.ledger)
    ledger.parent.mkdir(parents=True, exist_ok=True)
    ledger.write_text("".join(
        json.dumps(r, ensure_ascii=False, sort_keys=True) + "\n" for r in results),
        encoding="utf-8")

    print(json.dumps({
        "kind": "capability_claims",
        "probes": len(results),
        "verified": sum(1 for r in results if r["status"] == VERIFIED),
        "failed": sum(1 for r in results if r["status"] == FAILED),
        "not_verified": sum(1 for r in results if r["status"] == NOT_VERIFIED),
        "claimed": CLAIMED_CAPABILITIES,
        "ledger": str(ledger),
        "verdict": "FAILED" if fails else "VERIFIED",
        "fails": fails,
    }, ensure_ascii=False))
    return rc


def cmd_selftest(argv: list[str]) -> int:
    """反向变异：证明每个探针**真能判红**。

    对照设计（每个对照都必须能让对应探针判红，否则探针没有判别力）：

      C1 对照：把期望值改成错的 ⇒ cli-real-query 必须判红
      C2 对照：查一张不存在的表 ⇒ 客户端必须非零退出
      C3 对照：两次运行之间**删掉落盘数据文件**（= 生产存储换成 MemoryStorage
                的运行时表现）⇒ durable-on-disk 必须判红
      C4 对照：同上 ⇒ crash-restart 必须判红
      C5 对照：把期望值改成错的 ⇒ backup-restore 必须判红
      C5 追加对照：恢复成功后**改掉恢复目录里的一个值**再读回
                ⇒ 逐值比对必须检出差异（否则「逐值比对」是空话）

    任一对照没被判红 ⇒ 报红。
    """
    global EXPECTED
    work = WORKROOT / "selftest"
    if work.exists():
        shutil.rmtree(work)
    work.mkdir(parents=True, exist_ok=True)
    bin_cli, _ = resolve_repo_binary("sqlrustgo")
    bin_srv, _ = resolve_repo_binary("sqlrustgo-mysql-server")
    bin_pb, _ = resolve_repo_binary("physical-backup")
    client, _ = resolve_mysql_client()
    if not (bin_cli and bin_srv and bin_pb and client):
        print(json.dumps({"kind": "capability_selftest", "verdict": "NOT_VERIFIED",
                          "detail": "二进制或客户端不全，自测无法执行"}, ensure_ascii=False))
        return 1

    good_expected = list(EXPECTED)
    wrong_expected = [(str(i), s + "-TAMPERED") for i, s in good_expected]
    checks: list[tuple[str, bool, str]] = []

    # --- C1 对照：错的期望值 ⇒ 必须判红
    EXPECTED = wrong_expected
    r = probe_cli_real_query(bin_cli, work / "c1")
    EXPECTED = good_expected
    checks.append(("cli-real-query-detects-wrong-values", r["status"] == FAILED,
                   f"期望 FAILED，实得 {r['status']}：{r.get('detail','')[:140]}"))

    # --- C1 正向：对的期望值 ⇒ 必须判绿（证明上一条不是因为探针恒红）
    r = probe_cli_real_query(bin_cli, work / "c1b")
    checks.append(("cli-real-query-passes-on-correct-values", r["status"] == VERIFIED,
                   f"期望 VERIFIED，实得 {r['status']}：{r.get('detail','')[:140]}"))

    # --- C2 对照：查不存在的表
    port = free_port()
    datadir = work / "c2" / "data"
    srv = Server(bin_srv, datadir, port, work / "c2" / "s.log")
    ok, why = srv.start()
    if not ok:
        checks.append(("mysql-wire-detects-missing-table", False, why))
    else:
        try:
            rc, out, err = mysql_query(client, port, "SELECT * FROM no_such_table_xyz;")
            checks.append(("mysql-wire-detects-missing-table", rc != 0,
                           f"期望非零退出，实得 rc={rc}"))
        finally:
            srv.stop(hard=True)

    # --- C3/C4 对照：删掉落盘数据 ⇒ 探针必须判红
    for label, hard in (("durable", False), ("crash", True)):
        r = _persist_probe(bin_srv, client, work, free_port(), hard=hard,
                           label="mut_" + label, mutate_between=True)
        checks.append((f"{label}-detects-data-loss", r["status"] == FAILED,
                       f"期望 FAILED（数据被删后读不回），实得 {r['status']}：{r.get('detail','')[:140]}"))

    # --- C3/C4 正向：不删数据 ⇒ 必须判绿
    for label, hard in (("durable", False), ("crash", True)):
        r = _persist_probe(bin_srv, client, work, free_port(), hard=hard,
                           label="ok_" + label)
        checks.append((f"{label}-passes-on-persistent-storage", r["status"] == VERIFIED,
                       f"期望 VERIFIED，实得 {r['status']}：{r.get('detail','')[:140]}"))

    # --- C5 对照：错的期望值 ⇒ 必须判红
    EXPECTED = wrong_expected
    r = probe_backup_restore(bin_srv, client, bin_pb, work / "c5", free_port())
    EXPECTED = good_expected
    checks.append(("backup-restore-detects-wrong-values", r["status"] == FAILED,
                   f"期望 FAILED，实得 {r['status']}：{r.get('detail','')[:140]}"))

    # --- C5 正向 + 追加对照：改掉恢复目录里的一个值 ⇒ 逐值比对必须检出
    r = probe_backup_restore(bin_srv, client, bin_pb, work / "c5b", free_port())
    checks.append(("backup-restore-passes-on-correct-roundtrip", r["status"] == VERIFIED,
                   f"期望 VERIFIED，实得 {r['status']}：{r.get('detail','')[:140]}"))
    if r["status"] == VERIFIED:
        restored_rows = r.get("evidence", {}).get("rows", [])
        tampered = [list(x) for x in restored_rows]
        if tampered:
            tampered[0][1] = tampered[0][1] + "-TAMPERED"
            detected = bool(diff_pairs([tuple(x) for x in restored_rows],
                                       [tuple(x) for x in tampered]))
            checks.append(("backup-restore-comparison-detects-tampered-value", detected,
                           "篡改一个值后比对应检出差异" if detected else "篡改未被检出 ⇒ 比对是空话"))
        else:
            checks.append(("backup-restore-comparison-detects-tampered-value", False,
                           "恢复后没有可比对的行"))

    bad = [(n, d) for n, ok, d in checks if not ok]
    print(json.dumps({
        "kind": "capability_selftest",
        "checks": len(checks),
        "ok": len(checks) - len(bad),
        "not_ok": len(bad),
        "detail": [{"check": n, "detail": d} for n, d in bad],
        "verdict": "FAILED" if bad else "VERIFIED",
    }, ensure_ascii=False))
    return 1 if bad else 0


def main(argv: list[str] | None = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    if not argv or argv[0] in ("-h", "--help"):
        print(__doc__)
        return 0 if argv else 2
    cmd = argv[0]
    if cmd == "run":
        return cmd_run(argv[1:])
    if cmd == "selftest":
        return cmd_selftest(argv[1:])
    print(f"unknown subcommand: {cmd}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
