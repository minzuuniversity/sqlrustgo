#!/usr/bin/env python3
"""A04 prepared-statement (COM_STMT_PREPARE) differential.

pymysql does not expose COM_STMT_PREPARE, so this uses mysql-connector-python
(prepared=True) through a raw-wire proxy, comparing the sqlrustgo subject vs
reference MySQL on the same prepared statements.
"""
import json, os, subprocess, sys, time
import mysql.connector

PROXY = "/tmp/a04/wireproxy.py"
REF_DB = "a04_diff"

CASES = [
    {"id": "A04-PREP-001", "sql": "SELECT %s AS a, %s AS b", "params": [1, "x"], "setup": []},
    {"id": "A04-PREP-002", "sql": "SELECT %s + 1 AS p, %s AS s", "params": [5, "z"], "setup": []},
    {"id": "A04-PREP-003", "sql": "SELECT id FROM pp_t WHERE id = %s ORDER BY id",
     "params": [2],
     "setup": ["DROP TABLE IF EXISTS pp_t", "CREATE TABLE pp_t (id INT)",
               "INSERT INTO pp_t VALUES (1),(2),(3)"]},
]


def start_proxy(lport, host, port, logfile):
    if os.path.exists(logfile):
        os.remove(logfile)
    return subprocess.Popen([sys.executable, PROXY, str(lport), host, str(port), logfile],
                             stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def run_prepared(role, case, outdir, lport):
    casedir = os.path.join(outdir, "cases")
    os.makedirs(casedir, exist_ok=True)
    tag = "subj" if role == "subject" else "ref"
    logfile = os.path.join(casedir, "%s_.%s.wire" % (case["id"], tag))
    host, port = ("127.0.0.1", 3309) if role == "subject" else ("127.0.0.1", 3306)
    proxy = start_proxy(lport, host, port, logfile)
    res = {"case_id": case["id"], "role": role}
    conn = None
    time.sleep(0.4)
    try:
        kw = dict(host="127.0.0.1", port=lport, user="root", connect_timeout=25)
        if role == "ref":
            kw["password"] = "root123"
        conn = mysql.connector.connect(**kw)
        if role == "ref":
            cur = conn.cursor()
            cur.execute("USE `%s`" % REF_DB)
            cur.close()
            for stmt in case["setup"]:
                c2 = conn.cursor()
                c2.execute(stmt)
                c2.close()
        cur = conn.cursor(prepared=True)
        try:
            cur.execute(case["sql"], tuple(case["params"]))
            res["kind"] = "resultset"
            res["columns"] = [d[0] for d in cur.description]
            res["types"] = [d[1] for d in cur.description] if cur.description else []
            res["rows"] = [tuple(r) for r in cur.fetchall()]
        except mysql.connector.Error as e:
            res["kind"] = "error"
            res["error_code"] = e.errno
            res["error_msg"] = e.msg
        cur.close()
    except Exception as e:
        res["kind"] = "transport_error"
        res["error_msg"] = str(e)
    finally:
        if conn:
            try:
                conn.close()
            except Exception:
                pass
        try:
            proxy.wait(timeout=25)
        except Exception:
            proxy.kill()
    return res


def compare(s, r):
    sk, rk = s.get("kind"), r.get("kind")
    if sk == "error" and rk == "error":
        same = s["error_code"] == r["error_code"]
        return ("match" if same else "mismatch"), "both error code subj=%s ref=%s" % (
            s["error_code"], r["error_code"])
    if (sk == "error") != (rk == "error"):
        e = s if sk == "error" else r
        who = "subject" if sk == "error" else "reference"
        return "mismatch", "%s errored (%s) other returned rows" % (who, e.get("error_code"))
    if sk != rk:
        return "mismatch", "kind differs subj=%s ref=%s" % (sk, rk)
    # both resultset
    det = []
    if s["columns"] != r["columns"]:
        det.append("cols subj=%s ref=%s" % (s["columns"], r["columns"]))
    if s["types"] != r["types"]:
        det.append("types subj=%s ref=%s" % (s["types"], r["types"]))
    sn = [tuple("NULL" if x is None else str(x) for x in row) for row in s["rows"]]
    rn = [tuple("NULL" if x is None else str(x) for x in row) for row in r["rows"]]
    if len(sn) != len(rn):
        det.append("rowcount subj=%d ref=%d" % (len(sn), len(rn)))
    elif sorted(sn) != sorted(rn):
        det.append("rows differ")
    return ("match" if not det else "mismatch"), "; ".join(det) or "identical"


def main():
    outdir = sys.argv[1] if len(sys.argv) > 1 else "/tmp/a04/prepared"
    report = {"cases": []}
    for idx, case in enumerate(CASES):
        sp, rp = 14700 + idx, 14800 + idx
        s = run_prepared("subject", case, outdir, sp)
        r = run_prepared("ref", case, outdir, rp)
        verdict, detail = compare(s, r)
        rec = {"id": case["id"], "domain": "prepared", "sql": case["sql"],
               "params": case["params"], "verdict": verdict, "detail": detail,
               "subj": s, "ref": r,
               "wire": {"subj": "cases/%s_.subj.wire" % case["id"],
                        "ref": "cases/%s_.ref.wire" % case["id"]}}
        report["cases"].append(rec)
        print("[%s] %s -> %s  (%s)" % (case["id"], case["sql"], verdict, detail))
        with open(os.path.join(outdir, "cases", case["id"] + ".json"), "w") as f:
            json.dump(rec, f, indent=2, ensure_ascii=False, default=str)
    report["summary"] = {v: sum(1 for c in report["cases"] if c["verdict"] == v)
                         for v in ("match", "mismatch", "not_comparable")}
    with open(os.path.join(outdir, "report.json"), "w") as f:
        json.dump(report, f, indent=2, ensure_ascii=False, default=str)
    print("\n== PREPARED SUMMARY ==", report["summary"])


if __name__ == "__main__":
    main()
