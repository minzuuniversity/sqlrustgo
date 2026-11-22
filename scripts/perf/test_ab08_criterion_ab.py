#!/usr/bin/env python3
"""AB-08 pipeline self-test — the fail-closed guards must actually fire.

A guard that never fires is a guard that does not exist. This script drives
`ab08_criterion_ab.py` with synthetic run/baseline payloads and asserts the
verdict for each arm:

* **positive** — an unchanged run compares PASS;
* **mean regression** — a run whose mean drops >10% is rejected;
* **median tail** — a run whose median grows >20% is rejected (the p99 proxy,
  criterion 0.5 has no p99);
* **missing bench** — a bench present in the baseline but absent from the run
  is rejected (fail-closed, never "no data ⇒ fine");
* **environment drift** — a different host/toolchain is rejected;
* **too few samples** — a run below the sample floor is rejected;
* **host load** — a run taken while the host was busy is rejected;
* **missing baseline** — a missing baseline file is rejected;
* **promote refuses a run with NOT-RUN benches** and **promote refuses a run
  taken under load**.

Run it:

    python3 scripts/perf/test_ab08_criterion_ab.py

Exit code 0 = every arm behaved as specified. The synthetic payloads live in
a temporary directory; `AB08_RESULTS_DIR` keeps the pipeline's own reports
out of the repository.
"""

import json
import os
import subprocess
import sys
import tempfile

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PIPELINE = os.path.join(SCRIPT_DIR, "ab08_criterion_ab.py")

RUN_SCHEMA = "ab08-run-v1"
BASELINE_SCHEMA = "ab08-baseline-v1"


def env(host="host-a", rustc="rustc 1.97.0", load=None):
    return {
        "hostname": host,
        "rustc": rustc,
        "cargo": "cargo 1.97.0",
        "ncpu": 10,
        "load": load or {"load1": 0.2, "other_build_processes": 0},
    }


def bench(mean, median):
    return {"mean_point": mean, "median_point": median}


def run_payload(benches, host="host-a", load=None, samples=30, not_run=None):
    return {
        "schema": RUN_SCHEMA,
        "generated_utc": "2026-11-12T00:00:00Z",
        "tag": "selftest",
        "samples_requested": samples,
        "env": env(host, load=load),
        "deviations": [],
        "bench_groups": {},
        "benches": benches,
        "not_run": not_run or {},
    }


def baseline_payload(benches, host="host-a"):
    return {
        "schema": BASELINE_SCHEMA,
        "generated_utc": "2026-11-12T00:00:00Z",
        "source_run": "ab08-run-selftest.json",
        "env": env(host),
        "deviations": [],
        "benches": benches,
    }


def write(path, payload):
    with open(path, "w", encoding="utf-8") as fh:
        json.dump(payload, fh, indent=2)
    return path


def run_pipeline(argv, results_dir):
    env_vars = dict(os.environ)
    env_vars["AB08_RESULTS_DIR"] = results_dir
    return subprocess.run(
        [sys.executable, PIPELINE] + argv,
        capture_output=True,
        text=True,
        env=env_vars,
    )


def check(name, condition, detail=""):
    status = "ok  " if condition else "FAIL"
    print(f"[{status}] {name}" + (f" — {detail}" if detail and not condition else ""))
    return bool(condition)


def main():
    failures = 0
    with tempfile.TemporaryDirectory(prefix="ab08-selftest-") as tmp:
        results_dir = os.path.join(tmp, "results")
        os.makedirs(results_dir, exist_ok=True)

        run_path = os.path.join(tmp, "run.json")
        base_path = os.path.join(tmp, "baseline.json")

        # --- positive: an unchanged run compares PASS -------------------------
        benches = {"lexer_bench": bench(100.0, 100.0)}
        write(run_path, run_payload(benches))
        write(base_path, baseline_payload(benches))
        out = run_pipeline(
            ["compare", "--run", run_path, "--baseline", base_path, "--min-samples", "10"],
            results_dir,
        )
        failures += not check(
            "positive: unchanged run ⇒ PASS",
            out.returncode == 0 and "AB-08 compare: PASS" in out.stdout,
            out.stdout.strip() + out.stderr.strip(),
        )

        # --- mean regression --------------------------------------------------
        write(run_path, run_payload({"lexer_bench": bench(80.0, 100.0)}))
        out = run_pipeline(
            ["compare", "--run", run_path, "--baseline", base_path, "--min-samples", "10"],
            results_dir,
        )
        failures += not check(
            "regression: mean drop 20% ⇒ rejected",
            out.returncode == 1 and "REGRESSION" in out.stdout,
            out.stdout.strip(),
        )

        # --- median tail ------------------------------------------------------
        write(run_path, run_payload({"lexer_bench": bench(100.0, 130.0)}))
        out = run_pipeline(
            ["compare", "--run", run_path, "--baseline", base_path, "--min-samples", "10"],
            results_dir,
        )
        failures += not check(
            "tail: median +30% ⇒ rejected",
            out.returncode == 1 and "TAIL" in out.stdout,
            out.stdout.strip(),
        )

        # --- bench missing from the run --------------------------------------
        write(run_path, run_payload({}))
        out = run_pipeline(
            ["compare", "--run", run_path, "--baseline", base_path, "--min-samples", "10"],
            results_dir,
        )
        failures += not check(
            "missing bench ⇒ fail-closed",
            out.returncode == 1 and "FAIL-CLOSED" in out.stdout,
            out.stdout.strip(),
        )

        # --- environment drift ------------------------------------------------
        write(run_path, run_payload(benches, host="host-b"))
        out = run_pipeline(
            ["compare", "--run", run_path, "--baseline", base_path, "--min-samples", "10"],
            results_dir,
        )
        failures += not check(
            "environment drift ⇒ rejected",
            out.returncode == 1 and "ENV_DRIFT" in out.stdout,
            out.stdout.strip(),
        )

        # --- too few samples --------------------------------------------------
        write(run_path, run_payload(benches, samples=5))
        out = run_pipeline(
            ["compare", "--run", run_path, "--baseline", base_path, "--min-samples", "10"],
            results_dir,
        )
        failures += not check(
            "samples below the floor ⇒ rejected",
            out.returncode == 1 and "FAIL-CLOSED" in out.stdout,
            out.stdout.strip(),
        )

        # --- host load --------------------------------------------------------
        write(run_path, run_payload(benches, load={"load1": 0.2, "other_build_processes": 4}))
        out = run_pipeline(
            ["compare", "--run", run_path, "--baseline", base_path, "--min-samples", "10"],
            results_dir,
        )
        failures += not check(
            "run taken under host load ⇒ rejected",
            out.returncode == 1 and "LOAD_NOISE" in out.stdout,
            out.stdout.strip(),
        )

        # --- missing baseline file --------------------------------------------
        out = run_pipeline(
            ["compare", "--run", run_path, "--baseline", os.path.join(tmp, "nope.json"),
             "--min-samples", "10"],
            results_dir,
        )
        failures += not check(
            "missing baseline ⇒ rejected",
            out.returncode == 1 and "FAIL-CLOSED" in out.stdout,
            out.stdout.strip(),
        )

        # --- promote refuses a run with NOT-RUN benches ----------------------
        write(run_path, run_payload(benches, not_run={"bench_cbo": "measurement timed out"}))
        out = run_pipeline(
            ["promote", "--run", run_path, "--baseline", os.path.join(tmp, "promoted.json")],
            results_dir,
        )
        failures += not check(
            "promote refuses a NOT-RUN bench",
            out.returncode == 1 and "REFUSED" in out.stdout,
            out.stdout.strip(),
        )

        # --- promote refuses a run taken under load ---------------------------
        write(run_path, run_payload(benches, load={"load1": 0.2, "other_build_processes": 4}))
        out = run_pipeline(
            ["promote", "--run", run_path, "--baseline", os.path.join(tmp, "promoted.json")],
            results_dir,
        )
        failures += not check(
            "promote refuses a run taken under load",
            out.returncode == 1 and "REFUSED" in out.stdout,
            out.stdout.strip(),
        )

        # --- promote accepts a clean run --------------------------------------
        write(run_path, run_payload(benches))
        out = run_pipeline(
            ["promote", "--run", run_path, "--baseline", os.path.join(tmp, "promoted.json")],
            results_dir,
        )
        failures += not check(
            "promote accepts a clean, idle-host run",
            out.returncode == 0 and os.path.isfile(os.path.join(tmp, "promoted.json")),
            out.stdout.strip() + out.stderr.strip(),
        )

        # --- group names that collide across bench targets ---------------------
        # The 2026-11-12 core run measured a group called
        # `parser_batch_100_select` in **both** `bench_insert` and
        # `parser_bench`. Keying the metrics by the bare group name dropped
        # one of the two measurements and let a comparison attribute one
        # bench's numbers to another. The key must carry the bench.
        import importlib.util

        spec = importlib.util.spec_from_file_location("ab08_pipeline", PIPELINE)
        pipeline = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(pipeline)
        key_insert = pipeline.metric_key("bench_insert", "parser_batch_100_select")
        key_parser = pipeline.metric_key("parser_bench", "parser_batch_100_select")
        failures += not check(
            "colliding group names in different benches ⇒ distinct keys",
            key_insert != key_parser,
            f"{key_insert} == {key_parser}",
        )

        write(run_path, run_payload({
            key_insert: bench(80.0, 80.0),
            key_parser: bench(100.0, 100.0),
        }))
        write(base_path, baseline_payload({
            key_insert: bench(100.0, 100.0),
            key_parser: bench(100.0, 100.0),
        }))
        out = run_pipeline(
            ["compare", "--run", run_path, "--baseline", base_path, "--min-samples", "10"],
            results_dir,
        )
        failures += not check(
            "collision: only the regressed bench is reported",
            out.returncode == 1
            and out.stdout.count("REGRESSION") == 1
            and key_insert in out.stdout,
            out.stdout.strip(),
        )

    if failures:
        print(f"ab08 selftest: {failures} arm(s) failed")
        return 1
    print("ab08 selftest: all arms behaved as specified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
