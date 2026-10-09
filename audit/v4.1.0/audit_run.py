#!/usr/bin/env python3
"""A01 audit manifest tool: freeze / run / verify (Issue #5223).

Single implementation of the audit/v4.1.0 manifest contract; see README.md.
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import os
import platform
import socket
import subprocess
import sys
from pathlib import Path

import yaml

REPO_ROOT = Path(__file__).resolve().parents[2]
AUDIT_DIR = REPO_ROOT / "audit" / "v4.1.0"
RUNS_DIR = AUDIT_DIR / "runs"
COMMIT_FILE = AUDIT_DIR / "commit.txt"
SCHEMA_VERSION = 1
RECORDED_ENV_VARS = ("CARGO_TARGET_DIR", "RUSTFLAGS", "CARGO_TERM_COLOR")


def die(msg: str, code: int = 2) -> "NoReturn":  # type: ignore[name-defined]
    print(f"audit_run: {msg}", file=sys.stderr)
    sys.exit(code)


def utc_now() -> str:
    return _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def git(*args: str) -> str:
    out = subprocess.run(
        ["git", *args], cwd=REPO_ROOT, capture_output=True, text=True, check=True
    )
    return out.stdout.strip()


def evidence_hash_of(manifest: dict) -> str:
    results = sorted(
        (
            {
                "id": r["id"],
                "exit_code": r["exit_code"],
                "stdout_sha256": r["stdout_sha256"],
                "stderr_sha256": r["stderr_sha256"],
            }
            for r in manifest.get("results", [])
        ),
        key=lambda r: r["id"],
    )
    canonical = json.dumps(
        {
            "schema_version": manifest["schema_version"],
            "run_id": manifest["run_id"],
            "frozen_commit": manifest["frozen_commit"],
            "artifacts": manifest["artifacts"],
            "results": results,
        },
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=True,
    )
    return "sha256:" + sha256_bytes(canonical.encode("utf-8"))


def load_manifest(run_id: str) -> dict:
    path = RUNS_DIR / run_id / "manifest.yaml"
    if not path.is_file():
        die(f"manifest not found: {path}")
    with path.open() as f:
        m = yaml.safe_load(f)
    if not isinstance(m, dict):
        die("manifest is not a mapping")
    return m


def save_manifest(run_id: str, m: dict) -> None:
    path = RUNS_DIR / run_id / "manifest.yaml"
    with path.open("w") as f:
        yaml.safe_dump(m, f, sort_keys=False, allow_unicode=True, width=100)


def cmd_freeze(args: argparse.Namespace) -> None:
    commit = args.commit or git("rev-parse", "HEAD")
    if len(commit) != 40 or any(c not in "0123456789abcdef" for c in commit):
        die(f"commit must be 40-char lowercase hex, got: {commit!r}")
    dirty = git("status", "--porcelain")
    if dirty and not args.allow_dirty:
        die("worktree is dirty; commit/stash first or pass --allow-dirty")
    try:
        base = git("rev-parse", args.base)
        subprocess.run(
            ["git", "merge-base", "--is-ancestor", commit, base],
            cwd=REPO_ROOT,
            check=True,
        )
    except subprocess.CalledProcessError:
        die(f"commit {commit} is not an ancestor of {args.base}")
    COMMIT_FILE.write_text(commit + "\n")
    print(f"frozen_commit={commit} worktree_clean={not bool(dirty)}")
    print(f"wrote {COMMIT_FILE.relative_to(REPO_ROOT)}")


def cmd_run(args: argparse.Namespace) -> None:
    m = load_manifest(args.run_id)
    if m.get("schema_version") != SCHEMA_VERSION:
        die(f"unsupported schema_version: {m.get('schema_version')}")
    head = git("rev-parse", "HEAD")
    frozen = m.get("frozen_commit", "")
    if head != frozen:
        die(f"HEAD {head} != frozen_commit {frozen}; refusing to run")
    lock_path = REPO_ROOT / "Cargo.lock"
    lock_hash = sha256_file(lock_path)
    declared = (m.get("artifacts") or {}).get("cargo_lock_sha256")
    if declared and declared != lock_hash:
        die(f"Cargo.lock hash mismatch: manifest={declared} actual={lock_hash}")

    logs_dir = RUNS_DIR / args.run_id / "logs"
    logs_dir.mkdir(parents=True, exist_ok=True)
    results = []
    all_ok = True
    for i, c in enumerate(m.get("commands", []), start=1):
        cid = c["id"]
        cwd = (REPO_ROOT / c.get("cwd", ".")).resolve()
        if not cwd.is_dir():
            die(f"command {cid}: cwd does not exist: {c.get('cwd')}")
        started = utc_now()
        try:
            proc = subprocess.run(
                ["bash", "-c", c["cmd"]],
                cwd=cwd,
                capture_output=True,
                timeout=c.get("timeout_sec", 600),
            )
            rc, out, err = proc.returncode, proc.stdout, proc.stderr
        except subprocess.TimeoutExpired as e:
            rc = 124
            out = e.stdout or b""
            err = (e.stderr or b"") + b"\nTIMEOUT\n"
        finished = utc_now()
        stdout_name = f"{i:03d}-{cid}.stdout"
        stderr_name = f"{i:03d}-{cid}.stderr"
        (logs_dir / stdout_name).write_bytes(out)
        (logs_dir / stderr_name).write_bytes(err)
        results.append(
            {
                "id": cid,
                "started_at": started,
                "finished_at": finished,
                "exit_code": rc,
                "stdout_sha256": sha256_bytes(out),
                "stderr_sha256": sha256_bytes(err),
                "stdout_path": f"logs/{stdout_name}",
                "stderr_path": f"logs/{stderr_name}",
            }
        )
        status = "ok" if rc == 0 else f"FAIL({rc})"
        print(f"[{status}] {cid}")
        if rc != 0:
            all_ok = False

    m["results"] = results
    m["evidence_hash"] = evidence_hash_of(m)
    m["completed_at"] = utc_now()
    save_manifest(args.run_id, m)
    print(f"evidence_hash={m['evidence_hash']}")
    if not all_ok:
        die("one or more commands failed; manifest written for forensic use", code=1)


def cmd_verify(args: argparse.Namespace) -> None:
    m = load_manifest(args.run_id)
    if "results" not in m or "evidence_hash" not in m:
        die("manifest has no results/evidence_hash; run it first")
    for r in m["results"]:
        for kind in ("stdout", "stderr"):
            p = RUNS_DIR / args.run_id / r[f"{kind}_path"]
            if not p.is_file():
                die(f"missing log: {p}")
            actual = sha256_file(p)
            if actual != r[f"{kind}_sha256"]:
                die(
                    f"{r['id']}.{kind} hash mismatch: "
                    f"manifest={r[f'{kind}_sha256']} actual={actual}"
                )
    recomputed = evidence_hash_of(m)
    if recomputed != m["evidence_hash"]:
        die(f"evidence_hash mismatch: manifest={m['evidence_hash']} recomputed={recomputed}")
    print(f"verify OK: {args.run_id} evidence_hash={recomputed}")


def cmd_env_snapshot() -> dict:
    return {
        "os": platform.system(),
        "arch": platform.machine(),
        "hostname": socket.gethostname(),
        "rustc": subprocess.run(
            ["rustc", "--version"], capture_output=True, text=True, check=False
        ).stdout.strip(),
        "cargo": subprocess.run(
            ["cargo", "--version"], capture_output=True, text=True, check=False
        ).stdout.strip(),
        "env_vars": {k: os.environ[k] for k in RECORDED_ENV_VARS if k in os.environ},
        "worktree_clean": not bool(git("status", "--porcelain")),
    }


def cmd_scaffold(args: argparse.Namespace) -> None:
    run_dir = RUNS_DIR / args.run_id
    if (run_dir / "manifest.yaml").exists():
        die(f"already exists: {run_dir}")
    commit_file = COMMIT_FILE if COMMIT_FILE.is_file() else None
    frozen = (commit_file.read_text().strip() if commit_file else git("rev-parse", "HEAD"))
    if git("rev-parse", "HEAD") != frozen:
        die(f"HEAD != frozen_commit ({frozen}); checkout the freeze first")
    m = {
        "schema_version": SCHEMA_VERSION,
        "run_id": args.run_id,
        "source_agent": args.source_agent,
        "frozen_commit": frozen,
        "environment": cmd_env_snapshot(),
        "artifacts": {
            "cargo_lock_sha256": sha256_file(REPO_ROOT / "Cargo.lock"),
            "dataset_hashes": {},
        },
        "commands": yaml.safe_load(Path(args.commands_yaml).read_text()),
    }
    run_dir.mkdir(parents=True, exist_ok=True)
    save_manifest(args.run_id, m)
    print(f"scaffolded {run_dir / 'manifest.yaml'} (frozen_commit={frozen})")


def main() -> None:
    ap = argparse.ArgumentParser(prog="audit_run", description=__doc__)
    sub = ap.add_subparsers(dest="cmd", required=True)

    p = sub.add_parser("freeze", help="write audit/v4.1.0/commit.txt")
    p.add_argument("--commit", default=None, help="40-hex commit (default: HEAD)")
    p.add_argument("--base", default="origin/develop/v4.1.0")
    p.add_argument("--allow-dirty", action="store_true")
    p.set_defaults(func=cmd_freeze)

    p = sub.add_parser("run", help="execute manifest commands and record results")
    p.add_argument("--run-id", required=True)
    p.set_defaults(func=cmd_run)

    p = sub.add_parser("verify", help="recompute evidence_hash from committed logs")
    p.add_argument("--run-id", required=True)
    p.set_defaults(func=cmd_verify)

    p = sub.add_parser("scaffold", help="create run skeleton from a commands YAML")
    p.add_argument("--run-id", required=True)
    p.add_argument("--source-agent", required=True)
    p.add_argument("--commands-yaml", required=True)
    p.set_defaults(func=cmd_scaffold)

    args = ap.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
