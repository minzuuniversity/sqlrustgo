#!/usr/bin/env python3
"""test_kanban_render.py — kanban_render.py 的夹具（含反向变异）。

纪律：AGENTS §2.5 —— 门禁/判据必须能"把问题条件去掉就变绿"，
所以这里既测'坏输入必须红'，也测'好输入必须绿'（否则夹具本身可能永远红）。

退出码：0=全部通过；1=有断言失败；2=夹具没能运行（INCOMPLETE，不得当 PASS）。
"""
import json
import os
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(HERE, "kanban_render.py")
MD = os.path.join(HERE, "..", "..", "docs", "releases", "v4.1.0",
                  "KANBAN_v4.1.0_ALPHA.md")

results = []


def run(args, cwd=None):
    p = subprocess.run([sys.executable, SCRIPT] + args,
                       capture_output=True, text=True)
    return p.returncode, p.stdout, p.stderr


def case(name, fn):
    try:
        fn()
        results.append((name, "PASS", ""))
    except AssertionError as exc:
        results.append((name, "FAIL", str(exc)))
    except Exception as exc:  # noqa: BLE001
        results.append((name, "INCOMPLETE", "%s: %s" % (type(exc).__name__, exc)))


def sandbox():
    """Copy the real board into a temp dir so mutations never touch the repo."""
    tmp = tempfile.mkdtemp(prefix="kanban_fixture_")
    shutil.copy(MD, os.path.join(tmp, "KANBAN_v4.1.0_ALPHA.md"))
    shutil.copy(os.path.splitext(MD)[0] + ".jsonl",
                os.path.join(tmp, "KANBAN_v4.1.0_ALPHA.jsonl"))
    return os.path.join(tmp, "KANBAN_v4.1.0_ALPHA.md")


def read_jsonl(md):
    return [json.loads(l) for l in open(os.path.splitext(md)[0] + ".jsonl",
                                        encoding="utf-8") if l.strip()]


def write_jsonl(md, rows):
    with open(os.path.splitext(md)[0] + ".jsonl", "w", encoding="utf-8") as fh:
        for r in rows:
            fh.write(json.dumps(r, ensure_ascii=False) + "\n")


def t_baseline_green():
    md = sandbox()
    rc, out, err = run(["check", md])
    assert rc == 0, "clean board must be green, got rc=%d err=%s" % (rc, err)
    rc2, out2, err2 = run(["render", md])
    assert rc2 == 0, "render failed rc=%d %s" % (rc2, err2)
    rc3, _, err3 = run(["check", md])
    assert rc3 == 0, "render is not idempotent: %s" % err3


def t_reverse_mutation_drop_row():
    """删一行登记 ⇒ 必须红（这是本判据的牙齿）。"""
    md = sandbox()
    rows = read_jsonl(md)
    write_jsonl(md, rows[:-1])
    rc, _, err = run(["check", md])
    assert rc == 1, "dropping a ledger row must be red, got rc=%d" % rc
    assert "drifted" in err, "error must name the failure: %r" % err[:200]
    # 反向变异的反向：恢复 ⇒ 必须重新变绿
    write_jsonl(md, rows)
    rc2, _, _ = run(["check", md])
    assert rc2 == 0, "restored board must go green again, got rc=%d" % rc2


def t_reverse_mutation_edit_status():
    md = sandbox()
    rows = read_jsonl(md)
    for r in rows:
        if r["status"] == "未开始":
            r["status"] = "完成"
            r["evidence"] = "fabricated-evidence"
            break
    write_jsonl(md, rows)
    rc, _, err = run(["check", md])
    assert rc == 1, "status text drift must be red, rc=%d" % rc


def t_status_vocabulary_closed():
    md = sandbox()
    rows = read_jsonl(md)
    rows[0]["status"] = "已搞定"
    write_jsonl(md, rows)
    rc, _, err = run(["check", md])
    assert rc == 1, "out-of-vocabulary status must be refused, rc=%d" % rc
    assert "not in" in err, "must explain: %r" % err[:200]


def t_evidence_less_done_downgraded():
    """AFP §1：无证据的'完成'不成立，自动降级为待复核，而不是放行。"""
    md = sandbox()
    rows = read_jsonl(md)
    for r in rows:
        if r["status"] == "完成":
            r["evidence"] = ""
    write_jsonl(md, rows)
    rc, out, err = run(["counts", md])
    assert rc == 0, "counts must still run: %s" % err
    data = json.loads(out)
    assert data["by_status"].get("完成", 0) == 0, \
        "完成 without evidence must not count as 完成: %s" % data
    assert data["by_status"].get("待复核", 0) >= 1, \
        "downgrade to 待复核 missing: %s" % data


def t_missing_machine_copy():
    md = sandbox()
    os.remove(os.path.splitext(md)[0] + ".jsonl")
    rc, _, err = run(["check", md])
    assert rc == 1, "missing JSONL must not pass, rc=%d" % rc
    assert "INCOMPLETE" in err, "must be reported as INCOMPLETE: %r" % err[:200]


def t_missing_marker_refuses_to_guess():
    md = sandbox()
    text = open(md, encoding="utf-8").read()
    text = text.replace("<!-- LEDGER:BEGIN", "<!-- NOPE:BEGIN")
    text = text.replace("<!-- LEDGER:END -->", "<!-- NOPE:END -->")
    open(md, "w", encoding="utf-8").write(text)
    rc, _, err = run(["render", md])
    assert rc == 1, "no marker => refuse, rc=%d" % rc
    assert "refusing to guess" in err, "must say why: %r" % err[:200]


def t_duplicate_id():
    md = sandbox()
    rows = read_jsonl(md)
    dup = dict(rows[0])
    write_jsonl(md, rows + [dup])
    rc, _, err = run(["check", md])
    assert rc == 1, "duplicate ids must be red, rc=%d" % rc


def t_bad_json_fails_closed():
    md = sandbox()
    with open(os.path.splitext(md)[0] + ".jsonl", "w", encoding="utf-8") as fh:
        fh.write('{"id": "X", broken json\n')
    rc, _, err = run(["check", md])
    assert rc == 1, "bad json must fail closed, rc=%d" % rc
    assert "INCOMPLETE" in err, "must be INCOMPLETE not silent: %r" % err[:200]


def main():
    if not os.path.isfile(SCRIPT) or not os.path.isfile(MD):
        print("INCOMPLETE: script or board missing (script=%s md=%s)"
              % (os.path.isfile(SCRIPT), os.path.isfile(MD)))
        return 2
    for name, fn in [
        ("baseline_green_and_idempotent", t_baseline_green),
        ("rev_drop_row_must_red", t_reverse_mutation_drop_row),
        ("rev_status_text_must_red", t_reverse_mutation_edit_status),
        ("status_vocabulary_closed", t_status_vocabulary_closed),
        ("evidence_less_done_downgraded", t_evidence_less_done_downgraded),
        ("missing_machine_copy", t_missing_machine_copy),
        ("missing_marker_refuses_to_guess", t_missing_marker_refuses_to_guess),
        ("duplicate_id", t_duplicate_id),
        ("bad_json_fails_closed", t_bad_json_fails_closed),
    ]:
        case(name, fn)
    bad = 0
    incomplete = 0
    for name, verdict, detail in results:
        print("%-4s %s%s" % (verdict, name, (" :: " + detail) if detail else ""))
        if verdict == "FAIL":
            bad += 1
        elif verdict == "INCOMPLETE":
            incomplete += 1
    print("total=%d PASS=%d FAIL=%d INCOMPLETE=%d"
          % (len(results), len(results) - bad - incomplete, bad, incomplete))
    if incomplete:
        return 2
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
