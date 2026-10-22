#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Static analyzer behind `scripts/gate/check_cli_binary_names.sh` — #5405.

THE INVARIANT
=============

A build artifact's name must have **exactly one** source of truth, and that
source must be the manifest that produces it.

  C1  no two workspace packages declare the same `[[bin]] name`
      (a duplicate name is a cargo filename collision waiting to happen)

  C2  every `env!("CARGO_BIN_EXE_<name>")` names a `[[bin]]` of the
      **package that owns the file** — cargo only sets that variable for
      the current package's own binaries, so a name borrowed from a sibling
      package is a compile error waiting to happen

  C3  every `target/<profile>/<name>` literal names a binary that some
      workspace package actually produces (or a declared external tool)

  C4  every candidate name in the shared resolver
      (`tests/common/bin_resolver.rs`) is either a workspace bin or a
      declared external tool — and the external-tool list is read from
      that same file, so "what is not a workspace bin" has one source

WHY
===

#5405: `tests/e2e/sqlrustgo_cli_soak_e2e_test.rs` looked for
`target/debug/sqlrustgo-cli`. No bin is called `sqlrustgo-cli` — that is
the **package** name of `crates/sqlrustgo-cli`, whose `[[bin]] name` is
`sqlrustgo`. The test therefore never found its binary, and it only
survived because the target had never actually been executed.

The typo is not the real problem. The real problem is that "what is this
binary called" had **two** answers — the package name and the bin name —
and the test tree was free to pick either one. These checks remove that
freedom: the manifest is the only place a bin name may come from, and
everywhere else is cross-checked against it.

EXIT CODES
==========

  0  no violation
  1  at least one violation
  2  usage error / a required input is missing
"""

import argparse
import json
import os
import re
import sys

# Directories that never contain first-party sources worth scanning.
SKIP_DIRS = {
    ".git",
    ".worktrees",
    ".mavis-tmp",
    "archive",
    "node_modules",
    "target",
}

SCAN_SUFFIXES = (".rs", ".sh", ".yml", ".yaml")

# Cargo's own sub-directories under `target/`. They are directories, not
# binaries — `target/debug/deps` is where cargo puts the test executables.
CARGO_SUBDIRS = {
    ".fingerprint",
    "build",
    "deps",
    "incremental",
}

# A file may opt out of C3 with this marker in its first 20 lines.
#
# It exists for exactly one reason: **the checker needs sample paths to
# test itself against**. `tests/common/bin_resolver.rs` is the C6/C7
# detector — it must carry deliberately fake paths (`target/debug/x`) to
# prove the detector has discriminating power. Requiring that file to be
# free of them would make the detector untestable.
#
# The marker is explicit, auditable (a visible string in the source, not a
# hidden path list), and scoped per file. Every exempt file is listed in
# the gate output so a reviewer can see them.
C3_EXEMPT_MARKER = "cli-bin-allow-samples"

SECTION_RE = re.compile(r"^\s*\[([^\]]+)\]\s*$")
ARRAY_SECTION_RE = re.compile(r"^\s*\[\[([^\]]+)\]\]\s*$")
NAME_IN_SECTION_RE = re.compile(r'(?m)^\s*name\s*=\s*"([^"]+)"')

CONST_STR_RE = re.compile(
    r'(?m)^\s*(?:pub\s+)?const\s+([A-Za-z_][A-Za-z0-9_]*)\s*:\s*&str\s*=\s*"([^"]*)"\s*;'
)
CONST_SLICE_RE = re.compile(
    r"(?m)^\s*(?:pub\s+)?const\s+([A-Za-z_][A-Za-z0-9_]*)\s*:\s*&\[&str\]\s*=\s*&\[([^\]]*)\]\s*;"
)
RESOLVER_CALL_RE = re.compile(
    r"\b(resolve_binary|resolve_external_tool)\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*,\s*&\[([^\]]*)\]\s*\)"
)
CARGO_BIN_EXE_RE = re.compile(r"CARGO_BIN_EXE_([A-Za-z0-9_.\-]+)")
TARGET_LITERAL_RE = re.compile(r"target/(?:debug|release)/([A-Za-z0-9_.\-]+)")


def iter_sections(text):
    """Yield ``(kind, name, body)`` for every ``[x]`` / ``[[x]]`` section.

    ``kind`` is ``"table"`` or ``"array"``. ``body`` is the list of lines
    belonging to the section.
    """
    header = None
    body = []
    for line in text.splitlines():
        m = ARRAY_SECTION_RE.match(line)
        if m:
            if header is not None:
                yield header[0], header[1], body
            header = ("array", m.group(1))
            body = []
            continue
        m = SECTION_RE.match(line)
        if m:
            if header is not None:
                yield header[0], header[1], body
            header = ("table", m.group(1))
            body = []
            continue
        if header is not None:
            body.append(line)
    if header is not None:
        yield header[0], header[1], body


def package_name_of(text):
    """The ``[package] name = "..."`` of a manifest, or ``None``."""
    for kind, name, body in iter_sections(text):
        if kind == "table" and name == "package":
            m = NAME_IN_SECTION_RE.search("\n".join(body))
            if m:
                return m.group(1)
    return None


def bin_names_of(text):
    """Every ``[[bin]] name = "..."`` of a manifest."""
    out = []
    for kind, name, body in iter_sections(text):
        if kind == "array" and name == "bin":
            m = NAME_IN_SECTION_RE.search("\n".join(body))
            if m:
                out.append(m.group(1))
    return out


def workspace_members(text):
    """The ``[workspace] members = [...]`` list of a manifest."""
    for kind, name, body in iter_sections(text):
        if kind == "table" and name == "workspace":
            m = re.search(r"members\s*=\s*\[(.*?)\]", "\n".join(body), re.S)
            if m:
                return re.findall(r'"([^"]+)"', m.group(1))
    return []


def collect_packages(root):
    """``{package_name: {"dir": rel_dir, "bins": [bin names]}}``."""
    root_manifest = os.path.join(root, "Cargo.toml")
    with open(root_manifest, errors="replace") as fh:
        root_text = fh.read()

    dirs = ["."] + workspace_members(root_text)
    packages = {}
    for rel in dirs:
        manifest = os.path.join(root, rel, "Cargo.toml")
        if not os.path.isfile(manifest):
            continue
        with open(manifest, errors="replace") as fh:
            text = fh.read()
        pkg = package_name_of(text)
        if pkg is None:
            continue
        packages[pkg] = {"dir": rel, "bins": bin_names_of(text)}
    return packages


def bin_owners(packages):
    """``{bin_name: [packages declaring it]}``."""
    owners = {}
    for pkg, info in packages.items():
        for b in info["bins"]:
            owners.setdefault(b, []).append(pkg)
    return owners


def iter_source_files(root):
    """Yield ``(rel_path, abs_path)`` for every scannable source file."""
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = sorted(d for d in dirnames if d not in SKIP_DIRS)
        for fn in sorted(filenames):
            if not fn.endswith(SCAN_SUFFIXES):
                continue
            abs_path = os.path.join(dirpath, fn)
            rel = os.path.relpath(abs_path, root)
            yield rel, abs_path


def owning_package(rel_file, pkg_by_dir):
    """The package that owns ``rel_file`` (nearest ancestor manifest)."""
    parts = rel_file.replace(os.sep, "/").split("/")
    for i in range(len(parts), 0, -1):
        d = "/".join(parts[:i])
        if d in pkg_by_dir:
            return pkg_by_dir[d]
    return None


def is_comment_line(line, suffix):
    """True when ``line`` is (part of) a comment rather than code.

    A binary path inside a comment is **prose**: "this used to look for
    ``target/debug/sqlrustgo-cli``" describes a past state, it does not
    resolve anything. Flagging it would punish the very documentation
    that explains the bug.
    """
    stripped = line.strip()
    if not stripped:
        return True
    if suffix in (".sh", ".yml", ".yaml"):
        return stripped.startswith("#")
    # Rust: line comments, doc comments, and block-comment bodies.
    if stripped.startswith("//") or stripped.startswith("*") or stripped.startswith("/*"):
        return True
    return False


def code_lines(src, suffix):
    """The lines of ``src`` that are not comments."""
    return [ln for ln in src.splitlines() if not is_comment_line(ln, suffix)]


def has_c3_exemption(abs_path):
    """True when the file opted out of C3 with the documented marker."""
    try:
        with open(abs_path, errors="replace") as fh:
            head = fh.read(4096)
    except OSError:
        return False
    return C3_EXEMPT_MARKER in head


def in_c3_scope(rel):
    """C3 scans the **test trees**, the crate sources, and `scripts/`.

    The test trees are where #5405 lived and where the C6 gate (home
    directory paths) already draws its line. Crate sources are included
    because a production path that hardcodes a binary name drifts exactly
    the same way.

    `scripts/` is now included as well (#5516). It used to be excluded with
    the reason "it has its own backlog, tracked separately — mixing them in
    here would make this gate's verdict depend on a backlog that is being
    closed elsewhere". That reason expired when the backlog was closed:
    #5516's own breakage (six scripts treating a **package name** as a
    **binary name**, one of which silently recorded a stub instead of a
    failure) is fixed, so the exclusion no longer protected anything — it
    only hid the next occurrence of the same mistake.

    `docs/` stays out of scope: it holds historical evidence records (they
    describe the past, they do not run).
    """
    rel = rel.replace(os.sep, "/")
    return (
        rel.startswith("tests/")
        or rel.startswith("crates/")
        or rel.startswith("scripts/")
    )


def const_map(text):
    """``{const name: string value}`` for ``&str`` constants."""
    return {m.group(1): m.group(2) for m in CONST_STR_RE.finditer(text)}


def resolve_const(item, consts, depth=0):
    """Resolve a candidate item: a string literal or a const identifier."""
    item = item.strip()
    if not item:
        return None
    if item.startswith('"') and item.endswith('"') and len(item) >= 2:
        return item[1:-1]
    if depth < 8 and item in consts:
        return consts[item]
    return None


def parse_external_tools(text):
    """The external-tool names declared in the shared resolver.

    Read from ``EXTERNAL_TOOL_BINS`` (a ``&[&str]`` constant), so that
    "what is not a workspace bin" has exactly one source: the resolver.
    """
    consts = const_map(text)
    for m in CONST_SLICE_RE.finditer(text):
        name = m.group(1)
        if name != "EXTERNAL_TOOL_BINS":
            continue
        names = []
        for raw in m.group(2).split(","):
            resolved = resolve_const(raw, consts)
            if resolved:
                names.append(resolved)
        return names
    return []


def parse_resolver_candidates(text):
    """Every candidate name passed to ``resolve_binary`` / ``resolve_external_tool``."""
    consts = const_map(text)
    out = []
    for m in RESOLVER_CALL_RE.finditer(text):
        fn, _env, raw = m.group(1), m.group(2), m.group(3)
        for part in raw.split(","):
            resolved = resolve_const(part, consts)
            if resolved is None:
                out.append(
                    {
                        "call": fn,
                        "name": part.strip(),
                        "resolved": False,
                        "kind": "unresolved",
                    }
                )
            else:
                out.append(
                    {
                        "call": fn,
                        "name": resolved,
                        "resolved": True,
                        "kind": "external" if fn == "resolve_external_tool" else "internal",
                    }
                )
    return out


def analyze(root):
    """Run every check against ``root`` and return a JSON-able report."""
    packages = collect_packages(root)
    owners = bin_owners(packages)
    pkg_by_dir = {info["dir"]: pkg for pkg, info in packages.items()}

    workspace_bins = set(owners.keys())

    resolver_path = os.path.join(root, "tests", "common", "bin_resolver.rs")
    resolver_text = ""
    external_tools = []
    resolver_candidates = []
    if os.path.isfile(resolver_path):
        with open(resolver_path, errors="replace") as fh:
            resolver_text = fh.read()
        external_tools = parse_external_tools(resolver_text)
        resolver_candidates = parse_resolver_candidates(resolver_text)
    external_set = set(external_tools)

    # --- C1: duplicate [[bin]] names ---------------------------------------
    duplicate_bins = [
        {"name": name, "packages": sorted(pkgs)}
        for name, pkgs in sorted(owners.items())
        if len(pkgs) > 1
    ]

    # --- C2 / C3: scan the source tree ------------------------------------
    cargo_bin_exe = []
    target_literals = []
    c3_exempt = []
    for rel, abs_path in iter_source_files(root):
        try:
            with open(abs_path, errors="replace") as fh:
                src = fh.read()
        except OSError:
            continue
        rel_norm = rel.replace(os.sep, "/")
        suffix = os.path.splitext(rel_norm)[1]

        # C2 — `env!("CARGO_BIN_EXE_<name>")`. Cargo only sets it for the
        # current package's own binaries, so a name borrowed from a sibling
        # package is a compile error waiting to happen.
        #
        # Scope: Rust sources under the test trees and the crates. The
        # macro is Rust-only, and the gate's own shell script carries
        # *synthetic* `CARGO_BIN_EXE_*` samples for its self-test.
        if suffix == ".rs" and in_c3_scope(rel_norm):
            for m in CARGO_BIN_EXE_RE.finditer(src):
                name = m.group(1)
                owner = owning_package(rel_norm, pkg_by_dir)
                declared = owners.get(name, [])
                cargo_bin_exe.append(
                    {
                        "file": rel_norm,
                        "name": name,
                        "owner": owner,
                        "declared_in": sorted(declared),
                        "ok": bool(owner) and owner in declared,
                    }
                )

        # C3 — `target/<profile>/<name>` literals.
        if not in_c3_scope(rel_norm):
            continue
        if has_c3_exemption(abs_path):
            c3_exempt.append(rel_norm)
            continue
        for line in code_lines(src, suffix):
            for m in TARGET_LITERAL_RE.finditer(line):
                name = m.group(1)
                if name in CARGO_SUBDIRS:
                    continue
                target_literals.append(
                    {
                        "file": rel_norm,
                        "name": name,
                        "known": name in workspace_bins or name in external_set,
                        "is_external": name in external_set,
                    }
                )

    # --- C4: resolver candidate lists -------------------------------------
    for cand in resolver_candidates:
        if not cand["resolved"]:
            cand["known"] = False
        elif cand["kind"] == "external":
            cand["known"] = cand["name"] in external_set
        else:
            cand["known"] = cand["name"] in workspace_bins or cand["name"] in external_set

    return {
        "package_count": len(packages),
        "workspace_bins": sorted(workspace_bins),
        "external_tools": sorted(external_set),
        "duplicate_bins": duplicate_bins,
        "cargo_bin_exe": cargo_bin_exe,
        "target_literals": target_literals,
        "c3_exempt": sorted(c3_exempt),
        "resolver_candidates": resolver_candidates,
    }


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--json", dest="json_out", default=None)
    args = parser.parse_args(argv)

    root = os.path.abspath(args.repo_root)
    if not os.path.isfile(os.path.join(root, "Cargo.toml")):
        sys.stderr.write("ERROR: no Cargo.toml under %s\n" % root)
        return 2

    report = analyze(root)

    failures = 0
    if report["duplicate_bins"]:
        failures += 1
    for entry in report["cargo_bin_exe"]:
        if not entry["ok"]:
            failures += 1
    for entry in report["target_literals"]:
        if not entry["known"]:
            failures += 1
    for cand in report["resolver_candidates"]:
        if not cand.get("known"):
            failures += 1

    report["failure_count"] = failures
    report["status"] = "PASS" if failures == 0 else "FAIL"

    if args.json_out:
        with open(args.json_out, "w") as fh:
            json.dump(report, fh, indent=2, ensure_ascii=False)
            fh.write("\n")
    else:
        json.dump(report, sys.stdout, indent=2, ensure_ascii=False)
        sys.stdout.write("\n")

    return 0 if failures == 0 else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
