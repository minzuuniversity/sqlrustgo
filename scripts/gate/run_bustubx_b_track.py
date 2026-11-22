#!/usr/bin/env python3
"""RC-B1 Phase 2 — BustubX-EDU B-track differential runner (#5114).

Reads `tests/compat/bustubx_edu_b_track/manifest.yml`, runs `seed.sql` +
each `exercises/*.sql` against SQLite and SQLRustGo, and compares:

  * exit code
  * normalized rows (sorted; pipe-separated `--mode list`)
  * row-count + SHA256 of the normalized row set
  * for expected-failure cases: SQLRustGo must surface a non-empty error

The committed `oracle/*.expected` files are ALSO checked against a live
SQLite run. A stale oracle is a failure, not a pass: the gate must not
silently drift away from its pinned reference.

Exit codes (three-state, fail-closed):
  0  PASS       every case compared equal and the oracle is fresh
  1  FAIL       at least one case/oracle/mutation failed
  2  INCOMPLETE sqlite3 / SQLRustGo binary / manifest / oracle unavailable

This is a probe/gate, not a new `check_*`: the caller is the existing
`scripts/gate/check_bustubx_b_track_v312.sh`, so the 168-gate denominator
does not change.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
DEFAULT_ROOT = REPO / "tests" / "compat" / "bustubx_edu_b_track"
RUN_TIMEOUT = 120


def die(msg: str, code: int = 2) -> "NoReturn":  # type: ignore[name-defined]
    print(f"btrack: {msg}", file=sys.stderr)
    sys.exit(code)


def resolve_bin(explicit: str | None) -> str | None:
    if explicit:
        return explicit if Path(explicit).is_file() else None
    env = os.environ.get("SQLRUSTGO_BIN")
    if env and Path(env).is_file():
        return env
    candidates = [
        REPO / "target" / "debug" / "sqlrustgo",
        REPO / "target" / "release" / "sqlrustgo",
    ]
    target_dir = os.environ.get("CARGO_TARGET_DIR")
    if target_dir:
        candidates.insert(0, Path(target_dir) / "debug" / "sqlrustgo")
        candidates.insert(1, Path(target_dir) / "release" / "sqlrustgo")
    for c in candidates:
        if c.is_file():
            return str(c)
    found = shutil.which("sqlrustgo")
    return found


def run(cmd: list[str], stdin: str, cwd: Path | None = None) -> subprocess.CompletedProcess:
    return subprocess.run(
        cmd, input=stdin, cwd=str(cwd) if cwd else None,
        capture_output=True, text=True, timeout=RUN_TIMEOUT,
    )


def parse_expected(path: Path) -> tuple[str, int, list[str]]:
    stderr, exit_code, rows = "", None, []
    in_stdout = False
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("STDERR: "):
            stderr = line[len("STDERR: "):]
        elif line.startswith("EXIT: "):
            exit_code = int(line[len("EXIT: "):].strip())
        elif line.strip() == "--- stdout ---":
            in_stdout = True
        elif in_stdout:
            rows.append(line)
    if exit_code is None:
        raise ValueError(f"{path}: missing EXIT line")
    return stderr, exit_code, rows


def norm_rows(stdout: str) -> list[str]:
    return sorted(r.rstrip("\r") for r in stdout.splitlines() if r.strip())


def sha_rows(rows: list[str]) -> str:
    payload = "\n".join(rows).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def run_sqlrustgo(binary: str, sql: str) -> tuple[int, str, str]:
    tmp = tempfile.mkdtemp(prefix="btrack-rsg-")
    try:
        # A fresh directory per case: the CLI treats the path as a DB and
        # would otherwise see the previous case's tables (#5114 seed is
        # idempotent only on a clean DB).
        p = run([binary, "sqlite", "--batch", "--continue-on-error",
                 "--mode", "list", "--headers", "false", tmp], sql)
        return p.returncode, p.stdout, p.stderr
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def compare_case(case: dict, root: Path, sqlite_bin: str, rsg_bin: str) -> dict:
    name = f"{case.get('week', '?')}/{case.get('exercise', '?')}"
    exercise = root / case["exercise"]
    expected = root / case["expected"]
    seed = root / "seed.sql"
    if not exercise.is_file():
        return {"case": name, "status": "FAIL", "reason": f"missing exercise {exercise}"}
    if not expected.is_file():
        return {"case": name, "status": "FAIL", "reason": f"missing oracle {expected}"}
    try:
        exp_stderr, exp_exit, exp_rows = parse_expected(expected)
    except Exception as e:
        return {"case": name, "status": "FAIL", "reason": f"bad oracle: {e}"}

    sql = seed.read_text(encoding="utf-8") + "\n" + exercise.read_text(encoding="utf-8")

    try:
        sq = run([sqlite_bin, ":memory:"], sql)
    except FileNotFoundError:
        return {"case": name, "status": "INCOMPLETE", "reason": f"sqlite3 not found: {sqlite_bin}"}
    except subprocess.TimeoutExpired:
        return {"case": name, "status": "FAIL", "reason": "sqlite3 timeout"}

    sq_rows = norm_rows(sq.stdout)
    exp_norm = norm_rows("\n".join(exp_rows))
    if sq.returncode != exp_exit or sq_rows != exp_norm:
        return {
            "case": name, "status": "ORACLE_STALE",
            "reason": "live SQLite no longer matches committed oracle",
            "oracle_exit": exp_exit, "sqlite_exit": sq.returncode,
            "oracle_rows": len(exp_norm), "sqlite_rows": len(sq_rows),
            "oracle_sha256": sha_rows(exp_norm), "sqlite_sha256": sha_rows(sq_rows),
        }

    try:
        rc, out, err = run_sqlrustgo(rsg_bin, sql)
    except FileNotFoundError:
        return {"case": name, "status": "INCOMPLETE", "reason": f"sqlrustgo not found: {rsg_bin}"}
    except subprocess.TimeoutExpired:
        return {"case": name, "status": "FAIL", "reason": "sqlrustgo timeout"}

    rsg_rows = norm_rows(out)
    base = {
        "case": name,
        "oracle_exit": exp_exit,
        "sqlrustgo_exit": rc,
        "oracle_rows": len(exp_norm),
        "sqlrustgo_rows": len(rsg_rows),
        "oracle_sha256": sha_rows(exp_norm),
        "sqlrustgo_sha256": sha_rows(rsg_rows),
    }
    if rc != exp_exit:
        base.update(status="FAIL", reason=f"exit mismatch: sqlite={exp_exit} sqlrustgo={rc}")
        return base
    if rsg_rows != exp_norm:
        base.update(status="FAIL",
                    reason=f"row-set mismatch: sqlite={len(exp_norm)} sqlrustgo={len(rsg_rows)}")
        return base
    if exp_exit != 0 and not err.strip():
        base.update(status="FAIL", reason="expected failure but SQLRustGo produced no error")
        return base
    if exp_exit == 0 and any(line.startswith("Error:") for line in err.splitlines()):
        base.update(status="FAIL", reason="SQLRustGo reported an error on a success case")
        return base
    base.update(status="PASS", reason="row-count + sha256 + exit match")
    return base


def selftest(root: Path, sqlite_bin: str, rsg_bin: str) -> int:
    """Prove the runner can fail: 1 tampered oracle, 1 missing case, 1 no binary."""
    fails: list[str] = []
    with tempfile.TemporaryDirectory(prefix="btrack-selftest-") as td:
        clone = Path(td) / "bustubx_edu_b_track"
        shutil.copytree(root, clone)
        manifest = clone / "manifest.yml"
        import yaml
        data = yaml.safe_load(manifest.read_text(encoding="utf-8"))
        # M1: append a bogus row to the first oracle -> ORACLE_STALE/FAIL
        first = clone / data["cases"][0]["expected"]
        first.write_text(first.read_text(encoding="utf-8") + "\n999|bogus\n", encoding="utf-8")
        r = main(argv=["--root", str(clone), "--bin", rsg_bin, "--sqlite", sqlite_bin,
                       "--json-out", str(Path(td) / "m1.json")])
        if r == 0:
            fails.append("M1 tampered oracle still PASS")
        # M2: remove a case exercise -> FAIL
        shutil.copytree(root, clone / "m2", dirs_exist_ok=True)
        (clone / "m2" / "exercises" / "week01_setup.sql").unlink()
        r = main(argv=["--root", str(clone / "m2"), "--bin", rsg_bin, "--sqlite", sqlite_bin,
                       "--json-out", str(Path(td) / "m2.json")])
        if r == 0:
            fails.append("M2 missing exercise still PASS")
        # M3: missing binary -> INCOMPLETE (2), never PASS.
        # Run in a subprocess: main() calls sys.exit(2) for a missing
        # binary, which must not abort the selftest harness itself.
        p = subprocess.run(
            [sys.executable, __file__, "--root", str(root),
             "--bin", "/nonexistent/sqlrustgo", "--sqlite", sqlite_bin],
            capture_output=True, text=True, timeout=RUN_TIMEOUT,
        )
        if p.returncode != 2:
            fails.append(f"M3 missing binary returned {p.returncode}, expected 2")
    if fails:
        for f in fails:
            print(f"SELFTEST FAIL: {f}", file=sys.stderr)
        return 1
    print("btrack selftest: OK (M1 tampered oracle -> fail, M2 missing case -> fail, "
          "M3 missing binary -> INCOMPLETE)")
    return 0


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--root", default=str(DEFAULT_ROOT))
    ap.add_argument("--bin", default=None, help="SQLRustGo binary")
    ap.add_argument("--sqlite", default=os.environ.get("SQLITE_BIN", "sqlite3"))
    ap.add_argument("--json-out", default=None)
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args(argv)

    root = Path(args.root)
    manifest = root / "manifest.yml"
    if not manifest.is_file():
        die(f"manifest not found: {manifest}")
    try:
        import yaml
        data = yaml.safe_load(manifest.read_text(encoding="utf-8"))
    except Exception as e:
        die(f"cannot parse manifest: {e}")
    cases = data.get("cases") or []
    if not cases:
        die("manifest has no cases")

    rsg_bin = resolve_bin(args.bin)
    if args.selftest:
        if not rsg_bin:
            die("selftest needs a SQLRustGo binary (set SQLRUSTGO_BIN or build target/debug/sqlrustgo)")
        return selftest(root, args.sqlite, rsg_bin)
    if not rsg_bin:
        die("SQLRustGo binary not found (set SQLRUSTGO_BIN or build target/debug/sqlrustgo)")

    results = []
    for case in cases:
        try:
            results.append(compare_case(case, root, args.sqlite, rsg_bin))
        except Exception as e:
            results.append({"case": str(case), "status": "FAIL", "reason": f"runner exception: {e}"})

    by_status: dict[str, int] = {}
    for r in results:
        by_status[r["status"]] = by_status.get(r["status"], 0) + 1
    print("=== RC-B1 BustubX-EDU B-track Phase 2 ===")
    print(f"  root:      {root}")
    print(f"  sqlite:    {args.sqlite}")
    print(f"  sqlrustgo: {rsg_bin}")
    print(f"  cases:     {len(results)}")
    for r in results:
        print(f"  [{r['status']}] {r['case']}: {r.get('reason','')}")
        if r["status"] in ("FAIL", "ORACLE_STALE"):
            for k in ("oracle_rows", "sqlrustgo_rows", "oracle_sha256", "sqlrustgo_sha256"):
                if k in r:
                    print(f"      {k}: {r[k]}")
    print(f"  summary: {by_status}")

    report = {
        "kind": "bustubx_b_track_phase2",
        "manifest_sha256": hashlib.sha256(manifest.read_bytes()).hexdigest(),
        "cases": results,
        "summary": by_status,
    }
    if args.json_out:
        Path(args.json_out).write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n",
                                      encoding="utf-8")

    if any(r["status"] == "INCOMPLETE" for r in results):
        return 2
    return 0 if all(r["status"] == "PASS" for r in results) else 1


if __name__ == "__main__":
    sys.exit(main())
