#!/usr/bin/env python3
"""Mutation test runner for v4.1.0 fix PRs.

Reads v4.1_mutation_specs.json, applies each mutation, runs the registered
test target, records PASS (killed) / FAIL (survived). Restores file on
every iteration so subsequent mutations see clean code.
"""
import json, subprocess, sys, os, tempfile, time, pathlib

REPO = pathlib.Path(__file__).resolve().parent.parent.parent
SPEC_PATH = REPO / "tests/baseline/v4.1_mutation_specs.json"

# Logs go under the repo's own target/ dir, which sits on the workspace
# volume (external NVMe). Not /tmp or /var/folders: those live on
# /System/Volumes/Data, which runs near full, and a mutation log plus
# cargo's build chatter will fill it. Override with MUT_LOG_DIR.
LOG_DIR = pathlib.Path(
    os.environ.get("MUT_LOG_DIR", str(REPO / "target" / "mut-logs")))
LOG_DIR.mkdir(parents=True, exist_ok=True)


def run_cargo(pkg: str, target: str, timeout: int = None) -> tuple:
    """Run cargo test for one (pkg, target). Returns (rc, log_path).

    `target == "--lib"` runs the crate's in-source unit tests (used for
    fixes whose contract tests live in a `#[cfg(test)] mod tests`).

    Both streams go to the same log: cargo writes warnings to stderr but
    the `test result:` / `FAILED` lines to stdout, and we need both to
    diagnose a surviving mutation.

    Timeout defaults to `MUT_TIMEOUT` (3600s). A mutation in a leaf crate
    such as `sqlrustgo-storage` rebuilds every dependent in the
    workspace, and the `sqlrustgo-mysql-server` binary alone needs ~4min
    cold; the previous 1200s default killed the run mid-build, which
    looks identical to a SURVIVED mutation but is no verdict at all.
    """
    log = tempfile.NamedTemporaryFile(
        mode="w", delete=False, suffix=".log", dir=str(LOG_DIR),
        prefix=f"mut-{pkg}-{target}-")
    log.close()
    if timeout is None:
        timeout = int(os.environ.get("MUT_TIMEOUT", "3600"))
    if target == "--lib":
        argv = ["cargo", "test", "-p", pkg, "--lib", "--"]
    else:
        argv = ["cargo", "test", "-p", pkg, "--test", target, "--"]
    argv.append("--test-threads=1")
    with open(log.name, "w") as out:
        proc = subprocess.run(
            argv,
            cwd=str(REPO),
            stdout=out,
            stderr=subprocess.STDOUT,
            timeout=timeout,
        )
    return proc.returncode, log.name


def cargo_targets() -> set:
    """Set of (package, target_name) pairs that actually exist.

    Used to reject a spec whose pkg/target is wrong BEFORE running cargo.
    Without this, `cargo test -p <bogus>` exits non-zero and the runner
    would report a false KILLED — a fabrication.
    """
    meta = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version=1"], cwd=str(REPO)))
    out = set()
    for pkg in meta["packages"]:
        for t in pkg["targets"]:
            if "test" in t["kind"] or "bin" in t["kind"]:
                out.add((pkg["name"], t["name"]))
    return out


CARGO_TARGETS = None


def mutate(spec: dict) -> dict:
    """Apply one mutation spec, run its test, restore the file."""
    global CARGO_TARGETS
    # Non-Rust / non-cargo entries cannot be mutation-tested here.
    if (spec["pkg"] in ("__file__", "__script__", "REVIEW")
            or spec["target"] == "REVIEW"
            or not spec["file"].endswith(".rs")):
        return {
            "pr": spec["pr"], "outcome": "REVIEW",
            "reason": "no cargo target: docs/script/mod-hookup/perf-equivalent entry"
        }
    if CARGO_TARGETS is None:
        CARGO_TARGETS = cargo_targets()
    if spec["target"] == "--lib":
        if spec["pkg"] not in {p for p, _ in CARGO_TARGETS}:
            return {
                "pr": spec["pr"], "outcome": "INVALID",
                "reason": f"package {spec['pkg']} not in workspace — spec is wrong"
            }
    elif (spec["pkg"], spec["target"]) not in CARGO_TARGETS:
        return {
            "pr": spec["pr"], "outcome": "INVALID",
            "reason": f"({spec['pkg']}, {spec['target']}) is not a cargo target — "
                      f"spec is wrong; refusing to report a verdict"
        }
    fp = REPO / spec["file"]
    if not fp.exists():
        return {
            "pr": spec["pr"], "outcome": "SKIP",
            "reason": f"file not found: {spec['file']}"
        }
    backup = fp.read_bytes()
    try:
        content = fp.read_text()
        if spec["find"] not in content:
            return {
                "pr": spec["pr"], "outcome": "SKIP",
                "reason": "find pattern not present in file (already mutated or reverted elsewhere)"
            }
        mutated = content.replace(spec["find"], spec["replace"], 1)
        if mutated == content:
            return {
                "pr": spec["pr"], "outcome": "SKIP",
                "reason": "replace() no-op"
            }
        fp.write_text(mutated)

        try:
            rc, log_path = run_cargo(spec["pkg"], spec["target"])
        except subprocess.TimeoutExpired:
            # A build/test that never finished yields no verdict at all.
            # Reporting SURVIVED here would be a fabrication: the test
            # never ran, so nothing was proven either way.
            return {
                "pr": spec["pr"],
                "fix": spec["fix_desc"],
                "file": spec["file"],
                "pkg": spec["pkg"],
                "target": spec["target"],
                "outcome": "TIMEOUT",
                "detail": "cargo test exceeded MUT_TIMEOUT; no verdict — "
                          "raise MUT_TIMEOUT and rerun",
            }
        if rc != 0:
            outcome = "KILLED"
            detail = f"test failed under mutation (rc={rc})"
        else:
            outcome = "SURVIVED"
            detail = f"test PASSED under mutation (fix may be hollow)"
        return {
            "pr": spec["pr"],
            "fix": spec["fix_desc"],
            "file": spec["file"],
            "pkg": spec["pkg"],
            "target": spec["target"],
            "outcome": outcome,
            "detail": detail,
            "log": log_path,
        }
    finally:
        fp.write_bytes(backup)


def main():
    if not SPEC_PATH.exists():
        print(f"spec file not found: {SPEC_PATH}", file=sys.stderr)
        sys.exit(2)
    spec = json.loads(SPEC_PATH.read_text())
    results = []
    t0 = time.time()
    only_pending = "--pending" in sys.argv
    only_pr = None
    if "--pr" in sys.argv:
        only_pr = int(sys.argv[sys.argv.index("--pr") + 1])
    for entry in spec["mutations"]:
        if only_pending and entry.get("outcome") != "PENDING":
            continue
        if only_pr is not None and entry["pr"] != only_pr:
            continue
        print(f"=== PR #{entry['pr']}: {entry['fix_desc'][:60]} ===")
        r = mutate(entry)
        results.append(r)
        print(f"    outcome: {r['outcome']} — {r.get('detail', r.get('reason', ''))}")
        if r.get("outcome") in ("KILLED", "SURVIVED") and "log" in r:
            # Surface the assertion that fired, so the verdict is auditable.
            try:
                lines = open(r["log"], errors="replace").read().splitlines()
                for ln in lines:
                    if "FAILED" in ln or "test result:" in ln:
                        print(f"      | {ln.strip()[:150]}")
            except OSError:
                pass
    elapsed = time.time() - t0
    killed = sum(1 for r in results if r["outcome"] == "KILLED")
    survived = sum(1 for r in results if r["outcome"] == "SURVIVED")
    skipped = sum(1 for r in results if r["outcome"] == "SKIP")
    review = sum(1 for r in results if r["outcome"] == "REVIEW")
    timedout = sum(1 for r in results if r["outcome"] == "TIMEOUT")
    print()
    print(f"=== Summary ===")
    print(f"  total:    {len(results)}")
    print(f"  KILLED:   {killed}")
    print(f"  SURVIVED: {survived}")
    print(f"  SKIPPED:  {skipped}")
    print(f"  REVIEW:   {review}")
    print(f"  TIMEOUT:  {timedout}")
    print(f"  elapsed:  {elapsed:.0f}s")
    out_path = REPO / "tests/baseline/v4.1_mutation_results.json"
    out_path.write_text(json.dumps({
        "schema_version": 1,
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%S"),
        "results": results,
    }, indent=2))
    print(f"  written: {out_path.relative_to(REPO)}")


if __name__ == "__main__":
    main()