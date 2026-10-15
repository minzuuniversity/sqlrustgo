#!/usr/bin/env python3
"""A01 audit manifest tool: freeze / run / verify (Issue #5223).

Single implementation of the audit/v4.1.0 manifest contract; see README.md.
Supports legacy schema_version 1 (runs/<run_id>) and iaia410-manifest/v1
(<run_id> directly under audit/v4.1.0, per IAIA-410 SSOT section 5).
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
SCHEMA_VERSION_V2 = "iaia410-manifest/v1"
V2_HASH_EXCLUDED = ("manifest_sha256", "timestamp_started", "timestamp_finished")
V2_RUN_SUBDIRS = ("commands", "datasets", "findings", "traces")


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


def manifest_sha256_of(manifest: dict) -> str:
    payload = {k: v for k, v in manifest.items() if k not in V2_HASH_EXCLUDED}
    canonical = json.dumps(
        payload, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    )
    return "sha256:" + sha256_bytes(canonical.encode("utf-8"))


def run_dir_for(run_id: str) -> Path:
    for candidate in (AUDIT_DIR / run_id, RUNS_DIR / run_id):
        if (candidate / "manifest.yaml").is_file():
            return candidate
    die(
        f"manifest not found for run {run_id} "
        f"(looked in {AUDIT_DIR / run_id} and {RUNS_DIR / run_id})"
    )


def load_manifest(run_id: str) -> dict:
    path = run_dir_for(run_id) / "manifest.yaml"
    with path.open() as f:
        m = yaml.safe_load(f)
    if not isinstance(m, dict):
        die("manifest is not a mapping")
    return m


def save_manifest(run_dir: Path, m: dict) -> None:
    with (run_dir / "manifest.yaml").open("w") as f:
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
    if m.get("schema_version") == SCHEMA_VERSION_V2:
        _run_v2(args, m)
        return
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
    save_manifest(RUNS_DIR / args.run_id, m)
    print(f"evidence_hash={m['evidence_hash']}")
    if not all_ok:
        die("one or more commands failed; manifest written for forensic use", code=1)


def cmd_verify(args: argparse.Namespace) -> None:
    m = load_manifest(args.run_id)
    if m.get("schema_version") == SCHEMA_VERSION_V2:
        _verify_v2(args, m)
        return
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
    save_manifest(run_dir, m)
    print(f"scaffolded {run_dir / 'manifest.yaml'} (frozen_commit={frozen})")


def _run_v2(args: argparse.Namespace, m: dict) -> None:
    run_dir = run_dir_for(args.run_id)
    repo = m["repository"]
    head = git("rev-parse", "HEAD")
    if head != repo["commit"]:
        die(f"HEAD {head} != repository.commit {repo['commit']}; refusing to run")
    lock_hash = sha256_file(REPO_ROOT / "Cargo.lock")
    if lock_hash != repo["cargo_lock_sha256"]:
        die(
            f"Cargo.lock hash mismatch: "
            f"manifest={repo['cargo_lock_sha256']} actual={lock_hash}"
        )
    dirty = git("status", "--porcelain=v1", "--untracked-files=no")
    if dirty:
        die(f"worktree has tracked modifications; refusing to run:\n{dirty}")

    cmds_dir = run_dir / "commands"
    cmds_dir.mkdir(parents=True, exist_ok=True)
    all_ok = True
    for c in m.get("commands", []):
        cid = str(c["id"])
        cwd = (REPO_ROOT / c.get("cwd", ".")).resolve()
        if not cwd.is_dir():
            die(f"command {cid}: cwd does not exist: {c.get('cwd')}")
        started = utc_now()
        try:
            proc = subprocess.run(
                list(c["argv"]),
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
        (cmds_dir / f"{cid}.stdout.log").write_bytes(out)
        (cmds_dir / f"{cid}.stderr.log").write_bytes(err)
        meta = {
            "schema_version": "iaia410-command-meta/v1",
            "command_id": cid,
            "name": c.get("name"),
            "argv": list(c["argv"]),
            "cwd": c.get("cwd", "."),
            "timeout_sec": c.get("timeout_sec", 600),
            "started_at": started,
            "finished_at": finished,
            "exit_code": rc,
        }
        with (cmds_dir / f"{cid}.meta.yaml").open("w") as f:
            yaml.safe_dump(meta, f, sort_keys=False, allow_unicode=True, width=100)
        c["started_at"] = started
        c["finished_at"] = finished
        c["exit_code"] = rc
        c["stdout"] = f"commands/{cid}.stdout.log"
        c["stderr"] = f"commands/{cid}.stderr.log"
        c["stdout_sha256"] = sha256_bytes(out)
        c["stderr_sha256"] = sha256_bytes(err)
        status = "ok" if rc == 0 else f"FAIL({rc})"
        print(f"[{status}] {cid} {c.get('name', '')}")
        if rc != 0:
            all_ok = False
        save_manifest(run_dir, m)

    m["timestamp_finished"] = utc_now()
    m["manifest_sha256"] = manifest_sha256_of(m)
    save_manifest(run_dir, m)
    print(f"manifest_sha256={m['manifest_sha256']}")
    if not all_ok:
        die("one or more commands failed; manifest written for forensic use", code=1)


def _verify_v2(args: argparse.Namespace, m: dict) -> None:
    run_dir = run_dir_for(args.run_id)
    for c in m.get("commands", []):
        cid = str(c["id"])
        for key in ("stdout", "stderr"):
            rel = c.get(key)
            if not rel:
                die(f"command {cid}: missing {key} path; run it first")
            p = run_dir / rel
            if not p.is_file():
                die(f"missing log: {p}")
            actual = sha256_file(p)
            expected = c.get(f"{key}_sha256")
            if actual != expected:
                die(
                    f"{cid}.{key} hash mismatch: "
                    f"manifest={expected} actual={actual}"
                )
        meta = run_dir / "commands" / f"{cid}.meta.yaml"
        if not meta.is_file():
            die(f"missing meta: {meta}")
    recomputed = manifest_sha256_of(m)
    if recomputed != m.get("manifest_sha256"):
        die(
            f"manifest_sha256 mismatch: "
            f"manifest={m.get('manifest_sha256')} recomputed={recomputed}"
        )
    sums = run_dir / "SHA256SUMS"
    if not sums.is_file():
        die("SHA256SUMS missing; run `audit_run.py sha256sums --run-id` first")
    for line in sums.read_text().splitlines():
        if not line.strip():
            continue
        digest, sep, rel = line.partition("  ")
        if not sep:
            die(f"malformed SHA256SUMS line: {line!r}")
        p = run_dir / rel
        if not p.is_file() or sha256_file(p) != digest:
            die(f"SHA256SUMS mismatch for {rel}")
    print(f"verify OK: {args.run_id} manifest_sha256={recomputed}")


def workspace_features() -> list:
    out = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    names = set()
    if out.returncode == 0:
        for pkg in json.loads(out.stdout).get("packages", []):
            names.update(pkg.get("features", {}))
    return sorted(names)


def v2_environment(profile: str, reference_dbms: list) -> dict:
    mem_bytes = 0
    try:
        for line in Path("/proc/meminfo").read_text().splitlines():
            if line.startswith("MemTotal:"):
                mem_bytes = int(line.split()[1]) * 1024
                break
    except (OSError, IndexError, ValueError):
        pass
    cpu_model = ""
    try:
        for line in Path("/proc/cpuinfo").read_text().splitlines():
            if line.lower().startswith("model name"):
                cpu_model = line.split(":", 1)[1].strip()
                break
    except OSError:
        pass
    try:
        filesystem = (
            subprocess.run(
                ["df", "--output=fstype", str(REPO_ROOT)],
                capture_output=True,
                text=True,
                check=False,
            )
            .stdout.split()[-1]
        )
    except (OSError, IndexError):
        filesystem = ""
    return {
        "os": platform.system().lower(),
        "arch": platform.machine(),
        "cpu": f"{os.cpu_count()}x {cpu_model}".strip(),
        "memory_bytes": mem_bytes,
        "filesystem": filesystem,
        "rustc": subprocess.run(
            ["rustc", "--version"], capture_output=True, text=True, check=False
        ).stdout.strip(),
        "cargo": subprocess.run(
            ["cargo", "--version"], capture_output=True, text=True, check=False
        ).stdout.strip(),
        "features": workspace_features(),
        "profile": profile,
        "reference_dbms": reference_dbms,
        "process_pollution_check": "traces/env_pollution_pre.txt",
    }


def cmd_scaffold_v2(args: argparse.Namespace) -> None:
    run_dir = AUDIT_DIR / args.run_id
    if run_dir.exists():
        die(f"already exists: {run_dir}")
    commit = (args.commit or git("rev-parse", "HEAD")).lower()
    if len(commit) != 40 or any(ch not in "0123456789abcdef" for ch in commit):
        die(f"commit must be 40-char lowercase hex, got: {commit!r}")
    head = git("rev-parse", "HEAD")
    if head != commit:
        die(f"HEAD {head} != --commit {commit}; check out the assessed commit first")
    dirty = git("status", "--porcelain=v1", "--untracked-files=no")
    if dirty:
        die(f"worktree has tracked modifications:\n{dirty}")
    porcelain = git("status", "--porcelain")
    if porcelain.strip():
        die("worktree is not clean (untracked files present); commit or remove first")
    raw_commands = yaml.safe_load(Path(args.commands_yaml).read_text())
    if not isinstance(raw_commands, list) or not raw_commands:
        die("commands YAML must be a non-empty list")
    commands = []
    for i, c in enumerate(raw_commands, start=1):
        cid = str(c.get("id") or i)
        if cid.isdigit():
            cid = cid.zfill(3)
        commands.append(
            {
                "id": cid,
                "name": c["name"],
                "argv": [str(a) for a in c["argv"]],
                "cwd": c.get("cwd", "."),
                "timeout_sec": int(c.get("timeout_sec", 600)),
            }
        )
    reference_dbms = []
    for spec in args.reference_dbms:
        name, _, version = spec.partition(":")
        reference_dbms.append({"name": name, "version": version})
    m = {
        "schema_version": SCHEMA_VERSION_V2,
        "run_id": args.run_id,
        "work_package": args.work_package,
        "source_agent": args.source_agent,
        "source_run": args.source_run,
        "timestamp_started": utc_now(),
        "timestamp_finished": None,
        "conflict_resolution": args.conflict_resolution,
        "repository": {
            "canonical_remote": args.canonical_remote
            or git("remote", "get-url", "origin"),
            "commit": commit,
            "branch_or_detached": git("branch", "--show-current") or "detached",
            "status_porcelain_sha256": sha256_bytes(porcelain.encode("utf-8")),
            "cargo_lock_sha256": sha256_file(REPO_ROOT / "Cargo.lock"),
        },
        "environment": v2_environment(args.profile, reference_dbms),
        "datasets": [],
        "commands": commands,
        "manifest_sha256": None,
    }
    run_dir.mkdir(parents=True)
    for sub in V2_RUN_SUBDIRS:
        (run_dir / sub).mkdir()
    save_manifest(run_dir, m)
    print(f"scaffolded {run_dir / 'manifest.yaml'} (commit={commit})")


def cmd_sha256sums(args: argparse.Namespace) -> None:
    run_dir = run_dir_for(args.run_id)
    files = sorted(
        (p.relative_to(run_dir).as_posix(), p)
        for p in run_dir.rglob("*")
        if p.is_file() and p.name != "SHA256SUMS"
    )
    lines = [f"{sha256_file(p)}  {rel}" for rel, p in files]
    (run_dir / "SHA256SUMS").write_text("\n".join(lines) + "\n")
    print(f"wrote SHA256SUMS ({len(lines)} files) for {args.run_id}")


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

    p = sub.add_parser(
        "scaffold-v2", help="create an iaia410-manifest/v1 run skeleton"
    )
    p.add_argument("--run-id", required=True)
    p.add_argument("--work-package", required=True)
    p.add_argument("--source-agent", required=True)
    p.add_argument("--source-run", required=True)
    p.add_argument("--commands-yaml", required=True)
    p.add_argument(
        "--commit", default=None, help="40-hex assessed commit (default: HEAD)"
    )
    p.add_argument("--canonical-remote", default=None)
    p.add_argument("--conflict-resolution", default="none")
    p.add_argument("--profile", default="debug", choices=["debug", "release"])
    p.add_argument("--reference-dbms", action="append", default=[], metavar="NAME:VERSION")
    p.set_defaults(func=cmd_scaffold_v2)

    p = sub.add_parser("sha256sums", help="write SHA256SUMS for a v2 run directory")
    p.add_argument("--run-id", required=True)
    p.set_defaults(func=cmd_sha256sums)

    args = ap.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
