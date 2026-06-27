#!/usr/bin/env python3
"""
extract_soak_report.py — Parse mysqlslap + procfs metrics into a standardized SoakReport JSON.

Usage:
    python3 scripts/soak/extract_soak_report.py \\
        --metrics soak_results/metrics.csv \\
        --mysqlslap-output soak_results/mysqlslap.log \\
        --level 30m \\
        --concurrency 16 \\
        [--output soak_results/SoakReport.json]

Output: 14-field JSON SoakReport (see SPEC.md)
"""
import argparse
import json
import os
import re
import sys


def parse_metrics_csv(path: str) -> tuple[int | None, int | None, int | None, int | None, int]:
    """
    Parse procfs CSV and return (rss_baseline, rss_final, fd_baseline, fd_final, num_samples).
    """
    if not os.path.exists(path):
        return None, None, None, None, 0

    with open(path) as f:
        lines = f.readlines()

    data_lines = [l.strip() for l in lines if l.strip() and not l.startswith("timestamp")]
    num_samples = len(data_lines)
    if num_samples == 0:
        return None, None, None, None, 0

    def parse_row(parts: list[str]) -> tuple[int, int]:
        rss_kb = int(parts[1]) if len(parts) > 1 else 0
        fd_count = int(parts[2]) if len(parts) > 2 else 0
        return rss_kb * 1024, fd_count  # KB → bytes

    rss_b, fd_b = parse_row(data_lines[0].split(","))
    rss_f, fd_f = parse_row(data_lines[-1].split(","))
    return rss_b, rss_f, fd_b, fd_f, num_samples


def parse_mysqlslap_output(path: str, concurrency: int) -> tuple[int, int, float, float]:
    """
    Parse mysqlslap verbose log and return (queries_executed, errors, p50_ms, p99_ms).
    """
    if not os.path.exists(path):
        return 0, 0, 0.0, 0.0

    content = open(path).read()
    queries_executed = 0
    errors = 0
    p50 = 0.0
    p99 = 0.0

    # mysqlslap verbose output format:
    # Benchmark
    #   Running for query 1 of 22 iterations...
    #   etc.
    # Or:
    #   Average...: 1.234 ms
    #   P95/99...: etc.

    # Extract iterations
    iter_match = re.search(r'Iterations:\s*(\d+)', content, re.IGNORECASE)
    iterations = int(iter_match.group(1)) if iter_match else 0

    # Extract queries per iteration
    qpi_match = re.search(r'number-of-queries[=:]\s*(\d+)', content, re.IGNORECASE)
    queries_per_iter = int(qpi_match.group(1)) if qpi_match else 1

    # Extract concurrency
    conc_match = re.search(r'Concurrency[=:]\s*(\d+)', content, re.IGNORECASE)
    actual_conc = int(conc_match.group(1)) if conc_match else concurrency

    queries_executed = iterations * queries_per_iter * actual_conc

    # Extract latency
    # Format: "Average = 1.234 ms" or "avg = 1.234 ms"
    avg_match = re.search(
        r'(?:Average|avg)\s*[=:]\s*(\d+\.?\d*)\s*ms', content, re.IGNORECASE
    )
    if avg_match:
        p50 = float(avg_match.group(1))

    # p50: "p50 = 1.000 ms" or "50th percentile = 1.000 ms"
    p50_match = re.search(
        r'(?:p50|50(?:th percentile)?)\s*[=:]\s*(\d+\.?\d*)\s*ms',
        content, re.IGNORECASE
    )
    if p50_match:
        p50 = float(p50_match.group(1))

    # p99: "p99 = 2.000 ms" or "99th percentile = 2.000 ms"
    p99_match = re.search(
        r'(?:p99|99(?:th percentile)?)\s*[=:]\s*(\d+\.?\d*)\s*ms',
        content, re.IGNORECASE
    )
    if p99_match:
        p99 = float(p99_match.group(1))

    # Extract error count
    err_match = re.search(r'errors:\s*(\d+)', content, re.IGNORECASE)
    if err_match:
        errors = int(err_match.group(1))

    return queries_executed, errors, p50, p99


def build_report(
    metrics_csv: str,
    slap_log: str,
    level: str,
    concurrency: int,
    duration_seconds: int,
) -> dict:
    rss_baseline, rss_final, fd_baseline, fd_final, num_samples = parse_metrics_csv(metrics_csv)
    queries, errors, p50, p99 = parse_mysqlslap_output(slap_log, concurrency)

    memory_growth_pct = 0.0
    if rss_baseline and rss_final and rss_baseline > 0:
        memory_growth_pct = ((rss_final - rss_baseline) / rss_baseline) * 100.0

    fd_growth = (fd_final - fd_baseline) if (fd_baseline is not None and fd_final is not None) else 0

    alert_triggered = memory_growth_pct >= 10.0 or fd_growth >= 5

    return {
        "level": level,
        "duration_seconds": duration_seconds,
        "concurrency": concurrency,
        "queries_executed": queries,
        "errors": errors,
        "memory_baseline_bytes": rss_baseline or 0,
        "memory_final_bytes": rss_final or 0,
        "memory_growth_pct": round(memory_growth_pct, 4),
        "fd_baseline": fd_baseline or 0,
        "fd_final": fd_final or 0,
        "fd_growth": fd_growth,
        "p50_latency_ms": round(p50, 4),
        "p99_latency_ms": round(p99, 4),
        "alert_triggered": alert_triggered,
        "num_metrics_samples": num_samples,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description="Extract SoakReport JSON from soak test outputs")
    parser.add_argument("--metrics", required=True,
                        help="Path to procfs metrics CSV (from sample_metrics.sh)")
    parser.add_argument("--mysqlslap-output", required=True,
                        help="Path to mysqlslap stdout log")
    parser.add_argument("--level", required=True,
                        help="Soak level label (e.g. 30m, 4h)")
    parser.add_argument("--concurrency", type=int, default=16,
                        help="Concurrency level (default: 16)")
    parser.add_argument("--duration", type=int, default=0,
                        help="Wall-clock duration in seconds (default: 0)")
    parser.add_argument("--output", default=None,
                        help="Output JSON path (default: stdout)")
    args = parser.parse_args()

    report = build_report(
        metrics_csv=args.metrics,
        slap_log=args.mysqlslap_output,
        level=args.level,
        concurrency=args.concurrency,
        duration_seconds=args.duration,
    )

    json_str = json.dumps(report, indent=2)

    if args.output:
        with open(args.output, "w") as f:
            f.write(json_str + "\n")
        print(f"SoakReport written to {args.output}")
    else:
        print(json_str)


if __name__ == "__main__":
    main()
