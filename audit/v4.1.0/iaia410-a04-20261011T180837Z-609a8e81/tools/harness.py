#!/usr/bin/env python3
"""A04 SQL-semantics + MySQL-protocol differential harness (SSOT IAIA-410 §7.2).

For each corpus case, runs setup+query through a REAL MySQL client (pymysql)
via a raw-wire TCP proxy against BOTH the sqlrustgo subject and the reference
MySQL, then compares per SSOT §7.2:
  - column names, column types, row count, value-by-value
  - NULL positions and 3-valued logic
  - order (if ORDER BY) / set-multiset (else)
  - error category (code), side effects
  - incomparable reference -> not_comparable + rationale

Outputs (under --out):
  corpus.json           (copy of the corpus actually run)
  cases/<id>_.{subj,ref}.wire   (raw wire bytes, per §7.2/acceptance-3)
  cases/<id>.json      (captured subject+ref results + comparison)
  report.json          (all cases + verdicts, machine-readable)
  report.md            (human summary)
"""
import argparse, json, os, subprocess, sys, time
from collections import Counter
import pymysql

HARNESS_DIR = os.path.dirname(os.path.abspath(__file__))
PROXY = os.path.join(HARNESS_DIR, "wireproxy.py")
SUBJECT = dict(host="127.0.0.1", port=3309, user="root")
REF = dict(host="127.0.0.1", port=3306, user="root", password="root123")
REF_DB = "a04_diff"
SUBJ_DB = "default"


def start_proxy(lport, target, logfile):
    if os.path.exists(logfile):
        os.remove(logfile)
    return subprocess.Popen(
        [sys.executable, PROXY, str(lport), target[0], str(target[1]), logfile],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def connect_retry(port, user, password=None, retries=8, delay=0.4):
    last = None
    for _ in range(retries):
        kw = dict(host="127.0.0.1", port=port, user=user, connect_timeout=25, read_timeout=90)
        if password:
            kw["password"] = password
        try:
            return pymysql.connect(**kw)
        except Exception as e:
            last = e
            time.sleep(delay)
    raise last


def run_sql(conn, sql):
    cur = conn.cursor()
    try:
        cur.execute(sql)
        if cur.description:
            cols = [d[0] for d in cur.description]
            types = [d[1] for d in cur.description]
            rows = [tuple(r) for r in cur.fetchall()]
            return {"ok": True, "kind": "resultset", "columns": cols, "types": types, "rows": rows}
        return {"ok": True, "kind": "ok", "columns": [], "types": [], "rows": [], "affected": cur.rowcount}
    except pymysql.MySQLError as e:
        return {"ok": False, "kind": "error",
                "error_code": e.args[0],
                "error_msg": str(e.args[1] if len(e.args) > 1 else e)}
    finally:
        cur.close()


def run_role(role, case, outdir, port, target):
    tag = "subj" if role == "subject" else "ref"
    casedir = os.path.join(outdir, "cases")
    os.makedirs(casedir, exist_ok=True)
    logfile = os.path.join(casedir, "%s_.%s.wire" % (case["id"], tag))
    proxy = start_proxy(port, target, logfile)
    res = {"case_id": case["id"], "role": role, "port": port}
    conn = None
    try:
        password = (REF if role == "ref" else SUBJECT).get("password")
        conn = connect_retry(port, target[2], password)
        if role == "ref":
            run_sql(conn, "USE `%s`" % REF_DB)
        setup = []
        for stmt in case.get("setup", []):
            setup.append({"stmt": stmt, "res": run_sql(conn, stmt)})
        res["setup"] = setup
        res["query"] = run_sql(conn, case["query"])
    except Exception as e:
        res["transport_error"] = str(e)
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


def norm_row(row):
    return tuple("NULL" if x is None else str(x) for x in row)


def compare_query(sres, rres, order_sensitive):
    sk, rk = sres.get("kind"), rres.get("kind")
    if sk == "error" and rk == "error":
        same = sres["error_code"] == rres["error_code"]
        if same:
            return "match", "both error, same code %d" % sres["error_code"], []
        return "mismatch", "both error, DIFFERENT codes: subj=%d ref=%d" % (
            sres["error_code"], rres["error_code"]), ["error_code"]
    if (sk == "error") != (rk == "error"):
        bad = sres if sk == "error" else rres
        who = "subject" if sk == "error" else "reference"
        return "mismatch", "%s errored (code %s) but other returned rows: %s" % (
            who, bad.get("error_code"), str(bad.get("error_msg"))[:80]), ["error_presence"]
    if sk == "ok" and rk == "ok":
        same = sres.get("affected") == rres.get("affected")
        return ("match" if same else "mismatch"), "side-effect(affected): subj=%s ref=%s" % (
            sres.get("affected"), rres.get("affected")), ([] if same else ["affected"])
    if sk != rk:
        return "mismatch", "result kind differs: subj=%s ref=%s" % (sk, rk), ["kind"]
    details, aspects = [], []
    if sres["columns"] != rres["columns"]:
        details.append("column names differ: subj=%s ref=%s" % (sres["columns"], rres["columns"]))
        aspects.append("column_name")
    if sres["types"] != rres["types"]:
        details.append("column types differ: subj=%s ref=%s" % (sres["types"], rres["types"]))
        aspects.append("column_type")
    srows = [norm_row(r) for r in sres["rows"]]
    rrows = [norm_row(r) for r in rres["rows"]]
    if len(srows) != len(rrows):
        details.append("rowcount differs: subj=%d ref=%d" % (len(srows), len(rrows)))
        aspects.append("rowcount")
    elif order_sensitive:
        for i, (a, b) in enumerate(zip(srows, rrows)):
            if a != b:
                details.append("row[%d] differs: subj=%s ref=%s" % (i, a, b))
                aspects.append("value")
                break
    else:
        sa, ra = sorted(srows), sorted(rrows)
        if sa != ra:
            ca, cb = Counter(srows), Counter(rrows)
            details.append("multiset differs (same count %d): only_subj=%s only_ref=%s" % (
                len(srows), list(ca - cb)[:4], list(cb - ca)[:4]))
            aspects.append("value")
    return ("match" if not details else "mismatch"), (
        "; ".join(details) if details else "identical (names/types/rows)"), aspects


def compare_case(subj, ref, case):
    if subj.get("transport_error") or ref.get("transport_error"):
        return "not_comparable", "transport error: subj=%s ref=%s" % (
            subj.get("transport_error"), ref.get("transport_error")), []
    # setup divergence -> state not comparable
    setup_notes, setup_ok = [], True
    for ss, rs in zip(subj.get("setup", []), ref.get("setup", [])):
        so, ro = ss["res"].get("kind"), rs["res"].get("kind")
        if so != ro:
            setup_ok = False
            setup_notes.append("setup DIVERGED on %r: subj_kind=%s ref_kind=%s (subj_err=%s)" % (
                ss["stmt"][:50], so, ro, (ss["res"].get("error_msg") if so == "error" else "")))
        elif so == "ok" and ss["res"].get("affected") != rs["res"].get("affected"):
            setup_ok = False
            setup_notes.append("setup affected differs on %r: subj=%s ref=%s" % (
                ss["stmt"][:50], ss["res"].get("affected"), rs["res"].get("affected")))
    qv, qd, qa = compare_query(subj["query"], ref["query"], case.get("order_sensitive", False))
    if not setup_ok:
        return "not_comparable", "SETUP DIVERGED (state not comparable): " + " | ".join(setup_notes) + " || query: " + qd, ["setup"] + qa
    return qv, qd, qa


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--corpus", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--limit", type=int, default=0, help="run only first N cases (0=all)")
    a = ap.parse_args()
    outdir = a.out
    os.makedirs(os.path.join(outdir, "cases"), exist_ok=True)
    corpus = json.load(open(a.corpus))
    cases = corpus["cases"][:a.limit] if a.limit else corpus["cases"]

    # record corpus actually run + seed
    with open(os.path.join(outdir, "corpus.json"), "w") as f:
        json.dump({"seed": corpus.get("seed"), "corpus_sha256": corpus.get("corpus_sha256"),
                   "cases": cases}, f, indent=2, ensure_ascii=False)
    subj_target = (SUBJECT["host"], SUBJECT["port"], SUBJECT["user"])
    ref_target = (REF["host"], REF["port"], REF["user"])
    report = {"corpus_sha256": corpus.get("corpus_sha256"), "seed": corpus.get("seed"), "cases": []}
    for idx, case in enumerate(cases):
        sp = 14500 + idx
        rp = 14600 + idx
        t0 = time.time()
        subj = run_role("subject", case, outdir, sp, subj_target)
        ref = run_role("ref", case, outdir, rp, ref_target)
        verdict, detail, aspects = compare_case(subj, ref, case)
        rec = {"id": case["id"], "domain": case.get("domain"), "query": case["query"],
               "order_sensitive": case.get("order_sensitive", False),
               "verdict": verdict, "detail": detail, "aspects": aspects,
               "subj": subj, "ref": ref, "elapsed_s": round(time.time() - t0, 2),
               "wire": {"subj": "cases/%s_.subj.wire" % case["id"],
                        "ref": "cases/%s_.ref.wire" % case["id"]}}
        report["cases"].append(rec)
        print("[%s] %-10s %s -> %s  (%s)" % (case["id"], case.get("domain", "?"),
                                            case["query"][:40].replace("\n", " "), verdict, detail[:110]))
        with open(os.path.join(outdir, "cases", case["id"] + ".json"), "w") as f:
            json.dump(rec, f, indent=2, ensure_ascii=False, default=str)
    report["summary"] = {v: sum(1 for c in report["cases"] if c["verdict"] == v)
                         for v in ("match", "mismatch", "not_comparable")}
    with open(os.path.join(outdir, "report.json"), "w") as f:
        json.dump(report, f, indent=2, ensure_ascii=False, default=str)
    write_md(report, os.path.join(outdir, "report.md"))
    print("\n== SUMMARY ==", report["summary"])


def write_md(report, path):
    lines = ["# A04 SQL/protocol differential report", "",
             "corpus_sha256: `%s`  seed: `%s`" % (report["corpus_sha256"], report["seed"]), "",
             "| verdict | count |", "|---|---|"]
    for k, v in report["summary"].items():
        lines.append("| %s | %d |" % (k, v))
    lines += ["", "## Cases", "",
              "| id | domain | verdict | detail |", "|---|---|---|---|"]
    for c in report["cases"]:
        det = c["detail"].replace("|", "/").replace("\n", " ")
        lines.append("| %s | %s | %s | %s |" % (c["id"], c.get("domain", "?"), c["verdict"], det))
    with open(path, "w") as f:
        f.write("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
