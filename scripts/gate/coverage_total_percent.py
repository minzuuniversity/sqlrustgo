#!/usr/bin/env python3
"""Extract the TOTAL **line**-coverage percentage from a `cargo llvm-cov` report.

Reads the report on stdin and prints the percentage (e.g. "86.46") on stdout.
Prints nothing when the report does not contain one — a measurement that did
not happen must never be reported as a number.

Why this is not `grep -oE '[0-9]+\\.[0-9]+%' | tail -1`
---------------------------------------------------------
`cargo llvm-cov` prints a table whose TOTAL row carries one percentage per
group:

    Filename      Regions  Missed Regions  Cover  Functions  Missed Functions  Executed  Lines  Missed Lines  Cover  Branches  Missed Branches  Cover
    TOTAL             1278              76  94.05%      1236                2  98.35%   15987            76  95.28%         0                0  -

The old extraction took the *last* percentage in the row and called it line
coverage. That is only accidentally right: in this repository the Branches
group is always `-` or `0%` (Rust's MIR has no `brancwitch`, so llvm-cov
records no branch regions for it), so the last percentage happened to be the
Lines one. The moment a crate does carry branch regions, the same code
silently reports **branch** coverage while calling it line coverage.

The other shape that was tried — "walk back from the header's Lines-Cover
column until a cell parses as a percentage" — fails for the same reason: it
lands on the Branches cell when that cell is a percentage.

How this one works
------------------
The table's header names each group twice: once as the group header and once
inside the merged "Missed <Group>" label. So a group starts at exactly the
token whose successor is "Missed":

    Regions   Missed Regions   Cover   Functions  Missed Functions  Executed   Lines  Missed Lines  Cover   Branches  Missed Branches  Cover
    ^group 0  ^group 1                  ^group 2                          ^group 3

Each group occupies three cells in the data row (total, missed, percentage),
and the percentage is the last of the three. From that the Lines group is
addressed directly, instead of being inferred from which cells happen to
look like percentages.

Every step that could go wrong ends the extraction with **no output** rather
than a plausible-looking wrong number, and the result is cross-checked
against the group's position in the row's percentage sequence. A report whose
shape this script does not recognise is reported as "no coverage", which the
calling gate turns into a failure — the right direction for a measurement.

`--selftest` runs the fixtures below; they are the shape this script is
allowed to accept, and they are what stops the ambiguity from coming back.
"""

import re
import sys

PERCENT_RE = re.compile(r"([0-9]+(?:\.[0-9]+)?)%")
# A group header is the token right before a "Missed <Group>" label.
MISSED = "Missed"


def _split_cells(line):
    return line.split()


def _find_groups(header):
    """Return the header indices at which each group starts.

    A group starts at the token whose successor is "Missed" — that is the
    group name, immediately followed by the merged "Missed <Group>" label.
    """
    starts = []
    for i, tok in enumerate(header[:-1]):
        if header[i + 1] == MISSED:
            starts.append(i)
    return starts


def extract(report):
    """Return the Lines coverage percentage as a string, or None."""
    header = None
    total = None
    for line in report.splitlines():
        cells = _split_cells(line)
        if not cells:
            continue
        if header is None and cells[0] == "Filename" and "Lines" in cells:
            header = cells
        elif cells[0] == "TOTAL":
            total = cells

    if header is None or total is None:
        return None

    starts = _find_groups(header)
    if len(starts) < 3:
        # Need at least Regions / Functions / Lines to address the Lines
        # group by name rather than by position.
        return None

    # Which group is the Lines group?
    lines_group = None
    for g, start in enumerate(starts):
        if header[start] == "Lines":
            lines_group = g
            break
    if lines_group is None:
        return None

    # Group starts are evenly spaced (Filename, then one header per group).
    steps = {starts[i + 1] - starts[i] for i in range(len(starts) - 1)}
    if len(steps) != 1:
        return None

    # Data row: one filename cell, then `cells_per_group` cells per group.
    groups = len(starts)
    body = len(total) - 1
    if body <= 0 or body % groups != 0:
        return None
    cells_per_group = body // groups
    if cells_per_group < 2:
        # A group is (total, missed, percentage) — at least three cells.
        return None

    # The percentage is the last cell of its group.
    idx = 1 + cells_per_group * lines_group + (cells_per_group - 1)
    if idx >= len(total):
        return None
    m = PERCENT_RE.fullmatch(total[idx])
    if not m:
        return None
    value = m.group(1)

    # Cross-check: the Lines group is the (lines_group + 1)-th group to carry
    # a percentage, so the same value must come out of the row's percentage
    # sequence. If the two disagree, this report's shape is not the one this
    # script understands — say "no coverage" instead of picking a side.
    percents = [pm.group(1) for cell in total for pm in [PERCENT_RE.fullmatch(cell)] if pm]
    if len(percents) > lines_group:
        if percents[lines_group] != value:
            return None
    else:
        return None

    return value


SELFTEST_CASES = [
    (
        "normal report (Branches is '-') -> Lines Cover",
        """Filename      Regions  Missed Regions  Cover  Functions  Missed Functions  Executed  Lines  Missed Lines  Cover  Branches  Missed Branches  Cover
src/lib.rs        1278              76  94.05%      1236                2  98.35%   15987            76  95.28%         0                0  -
TOTAL             1278              76  94.05%      1236                2  98.35%   15987            76  95.28%         0                0  -
""",
        "95.28",
    ),
    (
        "Branches IS a percentage -> must still be Lines Cover, not Branches",
        """Filename      Regions  Missed Regions  Cover  Functions  Missed Functions  Executed  Lines  Missed Lines  Cover  Branches  Missed Branches  Cover
src/lib.rs        1278              76  94.05%      1236                2  98.35%   15987            76  95.28%        512               5  99.02%
TOTAL             1278              76  94.05%      1236                2  98.35%   15987            76  95.28%        512               5  99.02%
""",
        "95.28",
    ),
    (
        "no TOTAL row -> no number",
        """Filename      Regions  Missed Regions  Cover
src/lib.rs        1278              76  94.05%
""",
        None,
    ),
    (
        "unknown shape (data row does not divide into the header's groups) -> no number",
        """Filename      Regions  Missed Regions  Cover  Functions  Missed Functions  Executed  Lines  Missed Lines  Cover  Branches  Missed Branches  Cover
TOTAL             1278              76  94.05%      1236                2  98.35%   15987            76  95.28%
""",
        None,
    ),
    (
        "empty report -> no number",
        "",
        None,
    ),
]


def selftest():
    failures = 0
    for name, report, expected in SELFTEST_CASES:
        got = extract(report)
        if got != expected:
            failures += 1
            print("  [FAIL] %s: expected %r, got %r" % (name, expected, got))
        else:
            print("  [PASS] %s" % name)
    if failures:
        print("coverage_total_percent selftest: %d/%d failed" % (failures, len(SELFTEST_CASES)))
        return 1
    print("coverage_total_percent selftest: %d/%d passed" % (len(SELFTEST_CASES), len(SELFTEST_CASES)))
    return 0


def main(argv):
    if len(argv) > 1 and argv[1] == "--selftest":
        return selftest()
    if len(argv) > 1:
        print("usage: %s [--selftest]  (reads a report on stdin)" % argv[0], file=sys.stderr)
        return 2
    value = extract(sys.stdin.read())
    if value is not None:
        print(value)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
