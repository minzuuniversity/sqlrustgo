#!/usr/bin/env python3
"""AB-07 soak heartbeat emitter (#5115 / ALPHA_TO_BETA_GATE_PLAN §AB-07).

Sidecar observer for soak runs: appends one JSONL record per interval while
the watched soak process is alive, then a final exit record. Does NOT modify
the soak runner itself (additive contract; wiring happens in the evidence
commands manifest, not in gate scripts).

Records (one JSON object per line in --out):
  {"event":"start","ts":...,"pid":...,"interval":...}
  {"event":"beat","ts":...,"pid":...,"alive":true,"log_bytes":...,"log_lines":...}
  {"event":"exit","ts":...,"pid":...,"last_alive_ts":...}

Exit codes: 0 normal end; 2 bad usage (pid not given / unwritable out).
"""

import argparse
import json
import os
import subprocess
import sys
import time
from datetime import datetime, timezone


def ts_now():
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def pid_alive(pid):
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    # signal 0 succeeds on unreaped zombies; ps state separates real liveness
    r = subprocess.run(["ps", "-p", str(pid), "-o", "state="],
                       capture_output=True, text=True)
    state = (r.stdout or "").strip()
    if not state:
        return False
    return not any(ch == "Z" for ch in state)


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--pid", type=int, required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--interval", type=float, default=10.0)
    ap.add_argument("--log", default="", help="optional soak log to track size/line growth")
    ap.add_argument("--max-life", type=float, default=0.0,
                    help="stop beating after this many seconds (0 = until pid gone)")
    args = ap.parse_args()

    if args.pid <= 0:
        print("usage error: bad pid")
        sys.exit(2)
    try:
        fh = open(args.out, "a", encoding="utf-8")
    except OSError as exc:
        print(f"usage error: cannot write {args.out}: {exc}")
        sys.exit(2)

    def emit(rec):
        fh.write(json.dumps(rec, sort_keys=True) + "\n")
        fh.flush()

    started = time.monotonic()
    emit({"event": "start", "ts": ts_now(), "pid": args.pid, "interval": args.interval})
    last_alive = ts_now()
    while True:
        alive = pid_alive(args.pid)
        if not alive:
            break
        rec = {"event": "beat", "ts": ts_now(), "pid": args.pid, "alive": True}
        if args.log and os.path.exists(args.log):
            rec["log_bytes"] = os.path.getsize(args.log)
            try:
                with open(args.log, "rb") as lh:
                    rec["log_lines"] = sum(1 for _ in lh)
            except OSError:
                pass
        emit(rec)
        last_alive = rec["ts"]
        if args.max_life and (time.monotonic() - started) >= args.max_life:
            emit({"event": "timeout", "ts": ts_now(), "pid": args.pid,
                  "max_life": args.max_life})
            break
        time.sleep(args.interval)
    emit({"event": "exit", "ts": ts_now(), "pid": args.pid, "last_alive_ts": last_alive})
    fh.close()


if __name__ == "__main__":
    main()
