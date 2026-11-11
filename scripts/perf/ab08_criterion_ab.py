#!/usr/bin/env python3
"""AB-08 criterion A/B pipeline (#5116 / ALPHA_TO_BETA_GATE_PLAN §AB-08).

Contract (v4.1.0 Alpha→Beta):
  run     --tag T [--benches a,b | --core] [--samples N]
          correctness pre-gate per bench (`--test` single iteration), then
          criterion measurement (`--save-baseline T --sample-size N`, N>=5).
          Collects criterion JSON into benchmarks/results/ab08-run-<T>.json
          including the fixed-env fingerprint (rustc/cargo/host/os/ncpu).
  compare --run benchmarks/results/ab08-run-<T>.json --baseline benchmarks/baselines/<file>.json
          FAIL-CLOSED: baseline missing, bench missing, sample count < --min-samples,
          or env fingerprint drift (override: --allow-env-drift) all => exit 1.
          Regression thresholds (gate plan AB-08): mean point drop > 10% => FAIL;
          tail proxy: median-estimate point increase > 20% => FAIL.
          criterion 0.5 emits no p99 in estimates.json; the median point is the
          committed tail proxy -- deviation disclosed in the run/compare JSON.
  promote --run ... --baseline benchmarks/baselines/<file>.json [--force]
          freeze a passing run into a committed baseline file.

Exit codes: 0 pass; 1 fail/baseline-missing; 2 correctness pre-gate NOT RUN
(never measured perf on a bench binary that failed its own smoke test).
"""

import argparse
import glob
import json
import os
import platform
import socket
import subprocess
import sys
from datetime import datetime, timezone

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(SCRIPT_DIR, "..", ".."))
CRITERION_DIR = os.environ.get("AB08_CRITERION_DIR") or os.path.join(
    os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target")), "criterion"
)
RESULTS_DIR = os.path.join(ROOT, "benchmarks", "results")
BASELINE_SCHEMA = "ab08-baseline-v1"
RUN_SCHEMA = "ab08-run-v1"
CORE_BENCHES = [
    "lexer_bench",
    "parser_bench",
    "bench_insert",
    "bench_cbo",
    "bench_expr_binding",
    "bench_columnar",
]
DROP_THRESHOLD = 0.10   # mean point drop -> FAIL
TAIL_THRESHOLD = 0.20   # median point increase -> FAIL


def sh(argv):
    return subprocess.run(argv, cwd=ROOT, capture_output=True, text=True)


def cargo():
    exe = os.environ.get("CARGO", "cargo")
    if shutil_which(exe) is None and os.path.exists(os.path.expanduser("~/.cargo/bin/cargo")):
        exe = os.path.expanduser("~/.cargo/bin/cargo")
    return exe


def shutil_which(x):
    import shutil
    return shutil.which(x)


def env_fingerprint():
    rustc = cargo().replace("cargo", "rustc") if not cargo().endswith("cargo") else "rustc"
    rc = sh([rustc, "-vV"])
    cc = sh([cargo(), "--version"])
    return {
        "hostname": socket.gethostname(),
        "rustc": (rc.stdout or rc.stderr).strip().splitlines()[0] if (rc.stdout or rc.stderr) else "unknown",
        "cargo": (cc.stdout or cc.stderr).strip(),
        "system": platform.system(),
        "release": platform.release(),
        "machine": platform.machine(),
        "ncpu": os.cpu_count(),
        "criterion_dir": CRITERION_DIR,
    }


def now_iso():
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def parse_estimates(path):
    """Map criterion estimates.json -> flat metric dict (tolerant)."""
    try:
        with open(path, encoding="utf-8") as fh:
            data = json.load(fh)
    except (OSError, ValueError):
        return None
    out = {}
    for key in ("mean", "median", "slope"):
        block = data.get(key) or {}
        point = block.get("point_estimate")
        ci = (block.get("confidence_interval") or {})
        if point is not None:
            out[key + "_point"] = point
            out[key + "_ci_low"] = ci.get("lower_bound")
            out[key + "_ci_high"] = ci.get("upper_bound")
    return out or None


def collect(tag, min_mtime=None):
    metrics = {}
    pattern = os.path.join(CRITERION_DIR, "**", tag, "estimates.json")
    for path in sorted(glob.glob(pattern, recursive=True)):
        if min_mtime is not None and os.path.getmtime(path) < min_mtime:
            continue
        rel = os.path.relpath(path, CRITERION_DIR)
        bench_id = rel.split(os.sep + tag + os.sep)[0]
        parsed = parse_estimates(path)
        if parsed:
            metrics[bench_id] = parsed
    return metrics


def run_benches(benches, tag, samples, smoke=True):
    import time as _time
    metrics = {}
    groups = {}
    for bench in benches:
        if smoke:
            pre = sh([cargo(), "bench", "--bench", bench, "--", "--test"])
            if pre.returncode != 0:
                print(f"PRE-GATE NOT RUN: {bench} --test failed\n{pre.stderr[-800:]}")
                sys.exit(2)
        t0 = _time.time() - 2.0
        run = sh([cargo(), "bench", "--bench", bench, "--",
                  "--save-baseline", tag, "--sample-size", str(samples)])
        if run.returncode != 0:
            print(f"MEASURE FAIL: {bench} exit {run.returncode}\n{run.stderr[-800:]}")
            sys.exit(2)
        fresh = collect(tag, min_mtime=t0)
        if not fresh:
            print(f"COLLECT FAIL: no fresh criterion JSON for bench {bench}")
            sys.exit(2)
        groups[bench] = sorted(fresh)
        metrics.update(fresh)
    return metrics, groups


def write_json(path, payload):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as fh:
        json.dump(payload, fh, indent=2, sort_keys=True)
        fh.write("\n")
    print("wrote " + os.path.relpath(path, ROOT))


def cmd_run(args):
    benches = CORE_BENCHES if args.core else args.benches.split(",")
    if args.samples < 5:
        print("REFUSED: gate plan AB-08 requires >=5 samples")
        sys.exit(1)
    metrics, groups = run_benches(benches, args.tag, args.samples, smoke=not args.no_smoke)
    payload = {
        "schema": RUN_SCHEMA,
        "generated_utc": now_iso(),
        "tag": args.tag,
        "samples_requested": args.samples,
        "env": env_fingerprint(),
        "deviations": ["criterion-0.5-no-p99: median point committed as tail proxy"],
        "bench_groups": groups,
        "benches": metrics,
    }
    write_json(os.path.join(RESULTS_DIR, f"ab08-run-{args.tag}.json"), payload)


def cmd_promote(args):
    with open(args.run, encoding="utf-8") as fh:
        run = json.load(fh)
    if run.get("schema") != RUN_SCHEMA:
        print("REFUSED: not an ab08 run json")
        sys.exit(1)
    if os.path.exists(args.baseline) and not args.force:
        print("REFUSED: baseline exists (use --force)")
        sys.exit(1)
    payload = {
        "schema": BASELINE_SCHEMA,
        "generated_utc": now_iso(),
        "source_run": os.path.basename(args.run),
        "env": run["env"],
        "deviations": run.get("deviations", []),
        "benches": run["benches"],
    }
    write_json(args.baseline, payload)


def compare(run, baseline, min_samples, allow_env_drift):
    failures = []
    env_b, env_r = baseline.get("env", {}), run.get("env", {})
    if not allow_env_drift and (env_b.get("hostname"), env_b.get("rustc"), env_b.get("cargo")) != \
            (env_r.get("hostname"), env_r.get("rustc"), env_r.get("cargo")):
        failures.append("ENV_DRIFT: baseline host/toolchain != run (use --allow-env-drift)")
    for bench_id, base in sorted(baseline.get("benches", {}).items()):
        new = run.get("benches", {}).get(bench_id)
        if new is None:
            failures.append(f"FAIL-CLOSED: bench missing in run: {bench_id}")
            continue
        mean_b, mean_n = base.get("mean_point"), new.get("mean_point")
        if mean_b and mean_n is not None and (mean_b - mean_n) / mean_b > DROP_THRESHOLD:
            failures.append(f"REGRESSION {bench_id}: mean drop {(1 - mean_n / mean_b):.1%} > {DROP_THRESHOLD:.0%}")
        med_b, med_n = base.get("median_point"), new.get("median_point")
        if med_b and med_n is not None and (med_n - med_b) / med_b > TAIL_THRESHOLD:
            failures.append(f"TAIL {bench_id}: median +{(med_n / med_b - 1):.1%} > {TAIL_THRESHOLD:.0%} (p99 proxy)")
    extra = sorted(set(run.get("benches", {})) - set(baseline.get("benches", {})))
    return failures, extra


def cmd_compare(args):
    if not os.path.exists(args.baseline):
        print(f"FAIL-CLOSED: baseline missing: {args.baseline}")
        sys.exit(1)
    with open(args.run, encoding="utf-8") as fh:
        run = json.load(fh)
    with open(args.baseline, encoding="utf-8") as fh:
        baseline = json.load(fh)
    if run.get("samples_requested", 0) < args.min_samples:
        print(f"FAIL-CLOSED: samples {run.get('samples_requested')} < {args.min_samples}")
        sys.exit(1)
    failures, extra = compare(run, baseline, args.min_samples, args.allow_env_drift)
    report = {
        "schema": "ab08-compare-v1",
        "generated_utc": now_iso(),
        "run": os.path.basename(args.run),
        "baseline": os.path.basename(args.baseline),
        "verdict": "PASS" if not failures else "FAIL",
        "failures": failures,
        "extra_benches_in_run": extra,
    }
    write_json(os.path.join(RESULTS_DIR, f"ab08-compare-{os.path.basename(args.run)}"), report)
    for f in failures:
        print(f)
    if failures:
        sys.exit(1)
    print("AB-08 compare: PASS (drop<=10%, median-tail<=+20%, fail-closed gaps: none)")


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    r = sub.add_parser("run")
    r.add_argument("--tag", required=True)
    g = r.add_mutually_exclusive_group(required=True)
    g.add_argument("--benches", default="")
    g.add_argument("--core", action="store_true")
    r.add_argument("--samples", type=int, default=30)
    r.add_argument("--no-smoke", action="store_true")
    r.set_defaults(func=cmd_run)
    p = sub.add_parser("promote")
    p.add_argument("--run", required=True)
    p.add_argument("--baseline", required=True)
    p.add_argument("--force", action="store_true")
    p.set_defaults(func=cmd_promote)
    c = sub.add_parser("compare")
    c.add_argument("--run", required=True)
    c.add_argument("--baseline", required=True)
    c.add_argument("--min-samples", type=int, default=10)
    c.add_argument("--allow-env-drift", action="store_true")
    c.set_defaults(func=cmd_compare)
    args = ap.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
