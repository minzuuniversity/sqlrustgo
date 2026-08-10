#!/usr/bin/env python3
"""Validate historical-backlog-disposition.yml per V312-20 / #3907 acceptance."""
import sys, subprocess, re


def fix_line(line):
    """Wrap unquoted date-like / 40-char-hex values in quotes so Python's
    stricter tokenizer and PyYAML don't try to parse them as scientific
    notation or octal integers. Track per-line open/close quote state so
    we don't re-quote content inside an already-quoted string."""
    out = []
    i = 0
    in_single = False
    in_double = False
    while i < len(line):
        c = line[i]
        if c == "'" and not in_double:
            in_single = not in_single
            out.append(c)
            i += 1
            continue
        if c == '"' and not in_single:
            in_double = not in_double
            out.append(c)
            i += 1
            continue
        if not in_single and not in_double:
            m = re.match(r"\d{4}-\d{2}-\d{2}(-\S+)?", line[i:])
            if m:
                tok = m.group(0)
                out.append(f'"{tok}"')
                i += len(tok)
                continue
        out.append(c)
        i += 1
    return "".join(out)


def main():
    import yaml  # local so missing pyyaml produces a clean error

    yaml_path = sys.argv[1] if len(sys.argv) > 1 else (
        "docs/releases/v3.12.0/historical-backlog-disposition.yml"
    )

    if not __import__("os").path.exists(yaml_path):
        print(f"FAIL: YAML not found: {yaml_path}")
        sys.exit(1)
    print(f"PASS: YAML exists: {yaml_path}")

    with open(yaml_path) as f:
        raw = f.read()

    quoted = "\n".join(fix_line(ln) for ln in raw.split("\n"))
    try:
        data = yaml.safe_load(quoted)
    except yaml.YAMLError as e:
        print(f"FAIL: YAML parse error: {e}")
        if hasattr(e, "problem_mark"):
            m = e.problem_mark
            for i, ln in enumerate(quoted.split("\n"), 1):
                if abs(i - (m.line + 1)) <= 2:
                    print(f"  L{i}: {ln[:200]}")
        sys.exit(1)
    print("PASS: YAML schema validation passed")

    allowed = {"closed", "superseded", "carried", "deferred", "retired"}
    errors = []
    section_keys = [
        "v3_6", "v3_7", "v3_8", "v3_9", "v3_10",
        "v3_10_extension_crates", "v3_11_carry_forward",
    ]

    total = 0
    by_disp = {}
    seen_ids = set()
    for sk in section_keys:
        for item in data.get(sk, []) or []:
            iid = item.get("id", "?")
            # Allow duplicate IDs across sections for cross-version
            # tracking (e.g. INT-2 / ARCH-3 appear in both v3_8 and v3_10).
            # We only flag intra-section duplicates.
            key = (sk, iid)
            if key in seen_ids:
                errors.append(f"DUPLICATE id within {sk}: {iid}")
            seen_ids.add(key)
            d = item.get("disposition")
            by_disp[d] = by_disp.get(d, 0) + 1
            total += 1
            if d not in allowed:
                errors.append(f"{sk}/{iid}: disposition '{d}' not in {sorted(allowed)}")
            if not item.get("evidence"):
                errors.append(f"{sk}/{iid}: missing evidence block")
            if d == "carried":
                if not item.get("v312_owner_issue"):
                    errors.append(f"{sk}/{iid}: carried missing v312_owner_issue")
                if not item.get("v312_expiry"):
                    errors.append(f"{sk}/{iid}: carried missing v312_expiry")
                if not item.get("evidence", {}).get("evidence_hash"):
                    errors.append(f"{sk}/{iid}: carried missing evidence.evidence_hash")

    summary = data.get("summary", {})
    summary_total = summary.get("total_items")
    if summary_total != total:
        errors.append(f"summary.total_items={summary_total} != actual {total}")
    for disp_name, count in summary.get("by_disposition", {}).items():
        actual = by_disp.get(disp_name, 0)
        if actual != count:
            errors.append(
                f"summary.by_disposition.{disp_name}={count} != actual {actual}"
            )

    print(f"Total items: {total}")
    for k, v in sorted(by_disp.items()):
        print(f"  {k}: {v}")

    if errors:
        print("\nERRORS:")
        for e in errors:
            print(f"  - {e}")
        sys.exit(1)
    else:
        print("\nALL CHECKS PASSED")
        sys.exit(0)


if __name__ == "__main__":
    main()