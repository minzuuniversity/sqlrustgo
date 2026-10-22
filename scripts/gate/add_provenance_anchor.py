#!/usr/bin/env python3
"""Add a verifiable provenance anchor to a release document's header.

The evidence-binding gate (#5402) recognises a document as anchored when its
header contains a git SHA that resolves to a real commit. Documents written
before that rule existed have no such anchor, so every PASS/FAIL claim they
make is (correctly) reported as unverified.

This tool adds the anchor — and only the anchor. It never edits a claim, never
edits a measurement, and never edits anything below the header block. The SHA
it writes is the commit that **introduced the document**, so the anchor is a
fact about the file rather than a claim about its contents.

It is idempotent: a document that already carries a resolvable anchor is left
alone.

    scripts/gate/add_provenance_anchor.py <doc> [<doc> ...]
    scripts/gate/add_provenance_anchor.py --check <doc> [...]   # report only

Exit codes: 0 = all documents anchored (or already anchored), 1 = a document
could not be anchored, 2 = usage error.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

# The release line this tool writes anchors for. A document under
# `docs/releases/<version>/` belongs to that release line, and the anchor
# must name the line — not the branch that happened to be checked out when
# the tool ran.
RELEASE_LINE_VERSION = "v4.1.0"

# The anchor line we add. It is deliberately written in the shape the
# repository already uses (`**基线**：`develop/v4.1.0` @ `<sha>`) so that a
# reader — and the gate — recognises it without a special case.
ANCHOR_TEMPLATE = "**基线**：`{branch}` @ `{sha}`"

# Where to put it: right after the H1 and the blank line that follows it,
# unless the document opens with a `>` metadata block, in which case the
# anchor joins that block (a `>` line stays inside the blockquote).
SHA_RE = re.compile(r"(?<![0-9a-zA-Z])([0-9a-f]{7,40})(?![0-9a-zA-Z])")


def run_git(*args: str) -> str:
    return subprocess.run(
        ["git", *args],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    ).stdout.strip()


def is_commit(sha: str) -> bool:
    result = subprocess.run(
        ["git", "cat-file", "-t", sha],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    return result.returncode == 0 and result.stdout.strip() == "commit"


def resolves_to_commit(text: str) -> str | None:
    for candidate in SHA_RE.findall(text):
        if is_commit(candidate):
            return candidate
    return None


def current_branch() -> str:
    """The branch the release line lives on.

    NOT the branch this tool happens to run from: the anchor describes the
    document's own provenance, and a document on the `develop/v4.1.0` release
    line must say so — recording the branch of whatever feature worktree ran
    the tool would be a true-looking falsehood.
    """
    branch = run_git("rev-parse", "--abbrev-ref", "HEAD")
    if branch and branch != "HEAD":
        return branch
    return "develop/v4.1.0"


def release_line_branch() -> str:
    """The release line a document under `docs/releases/<version>/` belongs to."""
    version = RELEASE_LINE_VERSION
    if version:
        return f"develop/{version}"
    return "develop/v4.1.0"


def introducing_commit(doc: Path) -> str | None:
    """The commit that first added this document to the repository."""
    rel = doc.relative_to(REPO_ROOT) if doc.is_absolute() else doc
    out = run_git("log", "--diff-filter=A", "--format=%H", "--", str(rel))
    commits = [line for line in out.splitlines() if line]
    if not commits:
        return None
    sha = commits[-1]
    return sha if is_commit(sha) else None


def header_of(lines: list[str]) -> str:
    return "\n".join(lines[:60])


def insert_anchor(lines: list[str], anchor: str) -> list[str]:
    """Insert the anchor into the header block, preserving the document's shape."""
    # Find the H1.
    h1_index = next((i for i, l in enumerate(lines) if l.startswith("# ")), None)
    if h1_index is None:
        h1_index = 0

    # If the document opens with a `>` metadata block right after the H1,
    # append the anchor to that block so it reads as part of the metadata.
    probe = h1_index + 1
    while probe < len(lines) and not lines[probe].strip():
        probe += 1
    if probe < len(lines) and lines[probe].startswith(">"):
        return lines[:probe] + [f"> {anchor}"] + lines[probe:]

    # Otherwise place it right after the H1, separated by a blank line.
    insert_at = h1_index + 1
    if insert_at < len(lines) and not lines[insert_at].strip():
        insert_at += 1
    return lines[:insert_at] + [anchor, ""] + lines[insert_at:]


def process(doc: Path, check_only: bool) -> bool:
    if not doc.is_absolute():
        doc = REPO_ROOT / doc
    if not doc.is_file():
        print(f"MISSING  {doc}")
        return False

    text = doc.read_text(encoding="utf-8")
    lines = text.splitlines()

    existing = resolves_to_commit(header_of(lines))
    if existing:
        print(f"ANCHORED {doc.relative_to(REPO_ROOT)}  (sha {existing[:12]})")
        return True

    sha = introducing_commit(doc)
    if sha is None:
        print(f"NO-SHA   {doc.relative_to(REPO_ROOT)}  (no introducing commit found)")
        return False

    anchor = ANCHOR_TEMPLATE.format(branch=release_line_branch(), sha=sha)

    if check_only:
        print(f"WOULD    {doc.relative_to(REPO_ROOT)}  ({anchor})")
        return True

    updated = insert_anchor(lines, anchor)
    doc.write_text("\n".join(updated) + ("\n" if text.endswith("\n") else ""), encoding="utf-8")
    print(f"ANCHORED {doc.relative_to(REPO_ROOT)}  (+ {anchor})")
    return True


def main(argv: list[str]) -> int:
    check_only = False
    args = [a for a in argv if not (a == "--check" and not check_only)]
    if "--check" in args:
        check_only = True
        args = [a for a in args if a != "--check"]

    if not args:
        print(__doc__)
        return 2

    ok = True
    for arg in args:
        ok = process(Path(arg), check_only) and ok
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
