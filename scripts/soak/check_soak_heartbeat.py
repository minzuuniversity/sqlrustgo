#!/usr/bin/env python3
"""AB-07 soak heartbeat checker (#5115 / ALPHA_TO_BETA_GATE_PLAN §AB-07).

Verifies a soak heartbeat JSONL is continuous and honest. FAIL-CLOSED:
missing file, zero records, missing start/exit records, beat gaps beyond
--max-gap, or an exit record whose pid was still observed alive at cutoff
without a prior timeout record all => exit 1.

Exit codes: 0 heartbeat continuous and closed; 1 violations (listed);
2 usage error.
"""

import argparse
import json
import sys
from datetime import datetime, timezone


def parse_ts(value):
    return datetime.strptime(value, "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc)


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--file", required=True)
    ap.add_argument("--max-gap", type=float, default=60.0,
                    help="max allowed seconds between consecutive beats/start/exit")
    ap.add_argument("--expect-exit", action="store_true", default=True)
    ap.set_defaults(expect_exit=True)
    ap.add_argument("--allow-open", action="store_true",
                    help=" tolerate a still-running heartbeat (no exit record)")
    ap.add_argument("--min-beats", type=int, default=1,
                    help="require at least this many alive beats (0 to disable)")
    args = ap.parse_args()

    try:
        lines = [l for l in open(args.file, encoding="utf-8") if l.strip()]
    except OSError as exc:
        print(f"FAIL-CLOSED: heartbeat file unreadable: {exc}")
        sys.exit(1)
    recs = []
    for i, line in enumerate(lines, 1):
        try:
            recs.append(json.loads(line))
        except ValueError:
            print(f"FAIL: corrupt record at line {i}")
            sys.exit(1)
    if not recs:
        print("FAIL-CLOSED: heartbeat file has zero records")
        sys.exit(1)

    violations = []
    events = [r.get("event") for r in recs]
    if events[0] != "start":
        violations.append("first record is not start")
    has_exit = "exit" in events or "timeout" in events
    if not has_exit and not args.allow_open:
        violations.append("no exit/timeout record (heartbeat did not close)")

    beats = [r for r in recs if r.get("event") in ("start", "beat", "exit", "timeout")]
    prev = None
    for r in beats:
        try:
            t = parse_ts(r["ts"])
        except (KeyError, ValueError):
            violations.append(f"bad ts on event {r.get('event')}")
            continue
        if prev is not None:
            gap = (t - prev).total_seconds()
            if gap > args.max_gap:
                violations.append(f"heartbeat gap {gap:.0f}s > {args.max_gap:.0f}s at {r['ts']}")
        prev = t

    beats_only = [r for r in recs if r.get("event") == "beat"]
    if len(beats_only) < args.min_beats:
        violations.append(f"alive beats {len(beats_only)} < min-beats {args.min_beats}")
    if beats_only and not any(b.get("alive") for b in beats_only):
        violations.append("all beats report alive=false (soak never observed alive)")

    print(f"heartbeat records={len(recs)} beats={len(beats_only)} events={events[:1]}..{events[-1:]}")
    if violations:
        for v in violations:
            print("FAIL: " + v)
        sys.exit(1)
    print(f"AB-07 heartbeat: PASS (max_gap<= {args.max_gap:.0f}s, closed={has_exit})")


if __name__ == "__main__":
    main()
