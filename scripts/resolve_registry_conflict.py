#!/usr/bin/env python3
"""Resolve an append-only conflict in v4.1_fix_pr_test_registry.json.

Two PRs that each append one entry to `entries` collide at the same
place. The two sides share a single `{` / `}`, so splicing both bodies
together yields ONE object holding both sets of keys. JSON accepts
duplicate keys silently — last one wins — so the earlier entry vanishes
and `json.load` still reports success. This script emits one complete
object per side, then validates the candidate string *in memory* with an
object_pairs_hook that rejects duplicate keys, and only writes once the
result parses clean. A failed validation therefore leaves the conflicted
file untouched instead of corrupting it.

Usage:  python3 scripts/resolve_registry_conflict.py <path>
"""
import collections
import json
import sys

HEAD = "<<<<<<<"
BASE = "======="
TAIL = ">>>>>>>"


def split_sides(raw: str):
    """Return (prefix, ours, theirs, suffix) around one conflict hunk."""
    lines = raw.split("\n")

    def marker_index(prefix):
        return next(i for i, l in enumerate(lines) if l.startswith(prefix))

    h = marker_index(HEAD)
    m = marker_index(BASE)
    t = marker_index(TAIL)
    if not h < m < t:
        raise SystemExit(f"markers out of order: {h} {m} {t}")

    return (
        lines[:h],
        lines[h + 1 : m],
        lines[m + 1 : t],
        lines[t + 1 :],
    )


def join_bodies(ours, theirs) -> list:
    """Close the entry the shared `{` opened, then open the next one.

    The conflict hunk sits *inside* one object: the common prefix ends
    with `    {` and the common suffix is `    }`. Concatenating the two
    bodies directly would leave both sets of keys in a single object,
    where JSON's duplicate-key rule silently keeps the last one.

    The `},` on the closing line already separates the two entries — no
    comma belongs after the last field of `ours`, and `theirs` (now the
    final entry, closed by the common suffix) must not end in one either.
    """
    ours = [l for l in ours if l.strip()]
    theirs = [l for l in theirs if l.strip()]
    if not ours or not theirs:
        raise SystemExit("one conflict side is empty — resolve by hand")
    if ours[-1].rstrip().endswith(","):
        raise SystemExit("ours already ends in a comma — check the hunk shape")
    if theirs[-1].rstrip().endswith(","):
        theirs[-1] = theirs[-1].rstrip()[:-1]
    return ours + ["    },", "    {"] + theirs


def main() -> int:
    path = sys.argv[1]
    raw = open(path).read()
    if HEAD not in raw:
        print("no conflict markers — nothing to do")
        return 0

    prefix, ours, theirs, suffix = split_sides(raw)

    # Ours (the PR under review) goes first, then the side already on
    # develop — both are kept, in PR order as the conflict presents them.
    candidate_lines = prefix + join_bodies(ours, theirs) + suffix
    candidate = "\n".join(candidate_lines)

    def no_dupes(pairs):
        counts = collections.Counter(k for k, _ in pairs)
        dupes = sorted(k for k, n in counts.items() if n > 1)
        if dupes:
            raise ValueError(f"duplicate keys: {dupes}")
        return dict(pairs)

    try:
        data = json.loads(candidate, object_pairs_hook=no_dupes)
    except ValueError as exc:
        print(f"validation failed, file left untouched: {exc}")
        return 1

    prs = [e["pr"] for e in data["entries"]]
    if len(prs) != len(set(prs)):
        print(f"validation failed, file left untouched: duplicate pr {prs}")
        return 1

    open(path, "w").write(candidate)
    print(f"strict JSON OK — {len(prs)} entries, tail: {prs[-4:]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
