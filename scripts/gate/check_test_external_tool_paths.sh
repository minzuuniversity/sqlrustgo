#!/usr/bin/env bash
#
# check_test_external_tool_paths.sh — gate the **machine-specific** location
# of external tools in the test tree (criterion **C7**, issue #5500).
#
# # Why this is a separate criterion and not an extension of C6
# # ---------------------------------------------------------------
# C6 (`scripts/gate/check_test_manifest_honesty.sh`) states one invariant:
#
#     "the test tree contains no absolute path rooted at a user's home
#      directory (`/home/<user>/…` or `/Users/<user>/…`)"
#
# That invariant is clean, and it is **not** the same claim as the one this
# gate makes. `/opt/homebrew/bin/mysql` is not home-rooted — C6 cannot see
# it — yet it is just as machine-specific: it exists only on the machine
# whose homebrew prefix it is. Mixing the two shapes into one criterion
# would make the invariant unstateable ("no machine-specific paths" is a
# goal, not an invariant; "no home-rooted absolute paths" **is** one).
#
# So C7 states its own two invariants, each separately:
#
#   R1  the test tree contains no absolute path under a machine-specific
#       **install root** — `/opt/`, `/usr/local/`, `/Applications/`,
#       `/homebrew/`, `/Library/`, `/usr/lib/postgresql/`
#
#   R2  a file that talks to an **external** database client contains no
#       **literal** credential argument — `"-U"`, `"-u"`, `"--user"`,
#       `"--password"` immediately followed by a **string literal**, or the
#       inline password form `"-psecret"`. A *variable* is fine
#       (`"-u", user`): that value is resolved at run time.
#
#       `-p` is deliberately **not** in the separated form: in `psql` it is
#       the **port**, and in `cargo test -p` it is the **package selector**.
#       Only the inline form (`-p` glued to its value, which is how the
#       `mysql` client spells a password) is treated as a credential.
#
# R2's exemption is **explicit**, not inferred: a file that starts the
# repository's own server (`sqlrustgo-mysql-server`, `start_ephemeral`, a
# docker container) and therefore creates its own credentials on purpose
# opts out with the marker `c7-allow-local-credentials` in its first 20
# lines. Every exemption is visible in the source and reviewable — a
# heuristic exemption would hide the very thing a reviewer needs to see.
#
# R1's exemption marker is `c7-allow-external-tool-paths`: it exists for
# the file that **defines** the install-root list (the detector's own data),
# which cannot avoid spelling the roots out.
#
# Why it matters
# --------------
# `tests/integration/sql/four_way_harness.rs` hardcoded one machine's
# homebrew prefix *and* one machine's user name. On any other machine the
# client could not be found — and because the failure was recorded as an
# **empty result set**, the four-way comparison reported "the engine
# returned no rows" instead of "the client was never there".
#
# Exit codes
#   0  [PASS] — no machine-specific external-tool location in the test tree
#   1  [FAIL] — at least one violation
#   2  usage error
#
# Usage:
#   bash scripts/gate/check_test_external_tool_paths.sh
#   bash scripts/gate/check_test_external_tool_paths.sh --selftest
#
# `--selftest` proves the gate has **discriminating power**: it builds a
# synthetic tree containing known violations and known-clean samples, and
# fails unless the scanner reports exactly the violations. A gate that
# cannot tell a violation from a clean sample is not a gate.
#
# Refs: issue #5500

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

SELFTEST=0
for arg in "$@"; do
    case "$arg" in
        --selftest) SELFTEST=1 ;;
        -h|--help) sed -n '2,70p' "$0"; exit 0 ;;
        *) echo "usage: $0 [--selftest]" >&2; exit 2 ;;
    esac
done

if ! command -v python3 >/dev/null 2>&1; then
    echo "  [FAIL] python3 is not on PATH (needed to scan the test tree)"
    exit 1
fi

echo "=== External-Tool Path Gate (C7) ==="

# ---------------------------------------------------------------------------
# The scanner.
#
# R1 and R2 live in one Python program so that the gate and its self-test
# exercise **the same code** — a self-test that used a second implementation
# would not prove anything about the one that runs in CI.
#
# The Rust-side equivalents live in `tests/common/bin_resolver.rs`
# (`machine_specific_install_paths` / `hardcoded_client_credentials`) and
# are unit-tested against the same sample corpus.
# ---------------------------------------------------------------------------
SCANNER_FILE="$(mktemp -t c7_scanner.XXXXXX.py)"
trap 'rm -f "$SCANNER_FILE"' EXIT

cat > "$SCANNER_FILE" <<'PYEOF'
import os, sys, re

INSTALL_ROOTS = [
    "/opt/",
    "/usr/local/",
    "/Applications/",
    "/homebrew/",
    "/Library/",
    "/usr/lib/postgresql/",
]

CREDENTIAL_FLAGS = ["-U", "-u", "--user", "--password"]

# The credential flag list itself ("-U", "-u", …) must not be read as a
# credential *value*: `&["-U", "-u", "--user", "--password"]` is a list of
# flag names, not a credential.
CREDENTIAL_FLAG_SET = set(CREDENTIAL_FLAGS)

EXTERNAL_CLIENT = re.compile(r"\b(psql|mysql|mariadb|pg_ctl|mysqld)\b", re.IGNORECASE)

# The exemption markers. They must appear near the top of the file, so that
# a stray marker deep inside a real test cannot silently exempt it.
R1_EXEMPT_MARKER = "c7-allow-external-tool-paths"
R2_EXEMPT_MARKER = "c7-allow-local-credentials"


def install_root_hits(text):
    """R1 — absolute paths under a machine-specific install root."""
    hits = []
    pos = 0
    while pos < len(text):
        best = None
        for root in INSTALL_ROOTS:
            idx = text.find(root, pos)
            if idx >= 0 and (best is None or idx < best[0]):
                best = (idx, root)
        if best is None:
            break
        idx, root = best
        rest = text[idx:]
        end = len(rest)
        for i, ch in enumerate(rest):
            if ch.isspace() or ch in "\"'()[];,":
                end = i
                break
        hit = rest[:end].rstrip(".:")
        if hit and hit not in hits:
            hits.append(hit)
        pos = idx + len(root)
    return hits


def credential_hits(text):
    """R2 — literal credential arguments for an external DB client."""
    hits = []
    for flag in CREDENTIAL_FLAGS:
        needle = '"%s"' % flag
        start = 0
        while True:
            idx = text.find(needle, start)
            if idx < 0:
                break
            after = text[idx + len(needle):].lstrip()
            if after.startswith(","):
                after = after[1:].lstrip()
                if after.startswith('"'):
                    value = after[1:].split('"')[0]
                    if value and value not in CREDENTIAL_FLAG_SET:
                        hit = '"%s", "%s"' % (flag, value)
                        if hit not in hits:
                            hits.append(hit)
            start = idx + len(needle)
    # The inline password form: "-psecret" (the value glued to the flag,
    # which is how the `mysql` client spells a password). A `psql` port is
    # always written separately (`-p 5432`), and a format placeholder
    # ("-p{}") is not a secret.
    for m in re.finditer(r'"-p([^"]+)"', text):
        value = m.group(1)
        if len(value) < 2 or any(c in value for c in "{}%"):
            continue
        if value in CREDENTIAL_FLAG_SET:
            continue
        hit = '"-p%s"' % value
        if hit not in hits:
            hits.append(hit)
    return hits


def scan_file(path):
    """Return the list of (rule, detail) violations in one file."""
    try:
        with open(path, "r", errors="replace") as fh:
            text = fh.read()
    except OSError:
        return []

    violations = []
    head = "\n".join(text.splitlines()[:20])
    if R1_EXEMPT_MARKER not in head:
        for hit in install_root_hits(text):
            violations.append(("R1", hit))

    # R2 only applies to files that talk to an **external** client and that
    # have not explicitly opted out.
    if EXTERNAL_CLIENT.search(text) and R2_EXEMPT_MARKER not in head:
        for hit in credential_hits(text):
            violations.append(("R2", hit))
    return violations


def scan_tree(root):
    """Scan every .rs/.sh file under `root`; return [(relpath, rule, detail)]."""
    out = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames.sort()
        for name in sorted(filenames):
            if not (name.endswith(".rs") or name.endswith(".sh")):
                continue
            full = os.path.join(dirpath, name)
            rel = os.path.relpath(full, root)
            for rule, detail in scan_file(full):
                out.append((rel, rule, detail))
    return out


def scan_exempt(root):
    """Return [(relpath, rule)] for every file that claims an exemption."""
    out = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames.sort()
        for name in sorted(filenames):
            if not (name.endswith(".rs") or name.endswith(".sh")):
                continue
            full = os.path.join(dirpath, name)
            rel = os.path.relpath(full, root)
            try:
                with open(full, "r", errors="replace") as fh:
                    head = "\n".join(fh.read().splitlines()[:20])
            except OSError:
                continue
            if R1_EXEMPT_MARKER in head:
                out.append((rel, "R1"))
            if R2_EXEMPT_MARKER in head:
                out.append((rel, "R2"))
    return out


def selftest(tmp):
    # The sample corpus is assembled at run time so that this script's own
    # source does not contain the literal shapes it is looking for.
    bad_homebrew = "/" + "opt" + "/homebrew/bin/mysql"
    bad_usrlocal = "/" + "usr" + "/local/bin/psql"
    bad_user = '"' + "-U" + '", "' + "someuser" + '"'
    bad_mysql_user = '"' + "-u" + '", "' + "someuser" + '"'
    bad_password = '"' + "-p" + '", "' + "hunter2" + '"'
    bad_inline = '"' + "-p" + "hunter2" + '"'
    bad_long_flag = '"' + "--password" + '", "' + "hunter2" + '"'
    clean_relative = "tests/data/tpch-fixture/lineitem.tbl"
    clean_variable = '"' + "-u" + '", user'
    clean_flag_list = '&["' + "-U" + '", "' + "-u" + '"]'
    clean_port = '"' + "-p" + '", "' + "5432" + '"'
    client = "psql"

    os.makedirs(os.path.join(tmp, "sub"), exist_ok=True)

    def write(name, body):
        with open(os.path.join(tmp, name), "w") as fh:
            fh.write(body)

    write("bad_install_root.rs", 'let p = "%s";\n' % bad_homebrew)
    write("bad_install_root2.rs", 'let p = "%s";\n' % bad_usrlocal)
    write("bad_credential.rs", '// talks to %s\ncmd.args([%s]);\n' % (client, bad_user))
    write("bad_mysql_user.rs", '// talks to %s\ncmd.args([%s]);\n' % (client, bad_mysql_user))
    write("bad_inline_password.rs", '// talks to %s\ncmd.args([%s]);\n' % (client, bad_inline))
    write("bad_long_flag.rs", '// talks to %s\ncmd.args([%s]);\n' % (client, bad_long_flag))
    write("clean_relative.rs", 'let p = "%s";\n' % clean_relative)
    write("clean_variable.rs", '// talks to %s\ncmd.args([%s]);\n' % (client, clean_variable))
    write("clean_flag_list.rs", '// talks to %s\nlet flags = %s;\n' % (client, clean_flag_list))
    write("clean_port.rs", '// talks to %s\ncmd.args([%s]);\n' % (client, clean_port))
    write("clean_separated_password.rs", '// talks to %s\ncmd.args([%s]);\n' % (client, bad_password))
    write("clean_exempt.rs",
          '// %s: this test starts the repository\'s own server\n'
          '// talks to %s\ncmd.args([%s]);\n' % (R2_EXEMPT_MARKER, client, bad_user))
    write("clean_r1_exempt.rs",
          '// %s: this file defines the install-root list\n'
          'let p = "%s";\n' % (R1_EXEMPT_MARKER, bad_homebrew))
    write("sub/clean_local.rs", '// talks to %s\ncmd.args([%s]);\n' % (client, clean_variable))

    findings = scan_tree(tmp)
    by_file = {}
    for rel, rule, detail in findings:
        by_file.setdefault(rel, []).append((rule, detail))

    failures = []

    def expect_hit(filename, label):
        if filename not in by_file:
            failures.append("%s was NOT detected (%s)" % (filename, label))

    def expect_clean(filename):
        if filename in by_file:
            failures.append("%s was flagged but must be clean: %r"
                            % (filename, by_file[filename]))

    expect_hit("bad_install_root.rs", "R1 homebrew install root")
    expect_hit("bad_install_root2.rs", "R1 /usr/local install root")
    expect_hit("bad_credential.rs", "R2 literal -U")
    expect_hit("bad_mysql_user.rs", "R2 literal -u")
    expect_hit("bad_inline_password.rs", "R2 inline -psecret")
    expect_hit("bad_long_flag.rs", "R2 literal --password")
    expect_clean("clean_relative.rs")
    expect_clean("clean_variable.rs")
    expect_clean("clean_flag_list.rs")
    expect_clean("clean_port.rs")
    expect_clean("clean_separated_password.rs")
    expect_clean("clean_exempt.rs")
    expect_clean("clean_r1_exempt.rs")
    expect_clean("sub/clean_local.rs")

    if failures:
        for f in failures:
            print("  [FAIL] selftest: %s" % f)
        return 1
    print("  [OK] selftest: %d violation(s) detected, %d clean sample(s) accepted"
          % (len(by_file), 8))
    return 0


if __name__ == "__main__":
    mode = sys.argv[1]
    if mode == "scan":
        root = sys.argv[2]
        for rel, rule, detail in scan_tree(root):
            print("%s\t%s\t%s" % (rel, rule, detail))
    elif mode == "exempt":
        root = sys.argv[2]
        for rel, rule in scan_exempt(root):
            print("%s\t%s" % (rel, rule))
    elif mode == "selftest":
        sys.exit(selftest(sys.argv[2]))
    else:
        print("unknown mode", file=sys.stderr)
        sys.exit(2)
PYEOF

if [ "$SELFTEST" -eq 1 ]; then
    SELFTEST_DIR="$(mktemp -d -t c7_selftest.XXXXXX)"
    trap 'rm -f "$SCANNER_FILE"; rm -rf "$SELFTEST_DIR"' EXIT
    python3 "$SCANNER_FILE" selftest "$SELFTEST_DIR"
    STATUS=$?
    if [ "$STATUS" -eq 0 ]; then
        echo "  [PASS] External-Tool Path Gate (selftest)"
    else
        echo "  [FAIL] the gate has no discriminating power — see above"
    fi
    exit "$STATUS"
fi

HITS="$(python3 "$SCANNER_FILE" scan "$PROJECT_ROOT/tests")"
EXEMPTS="$(python3 "$SCANNER_FILE" exempt "$PROJECT_ROOT/tests")"

if [ -n "$EXEMPTS" ]; then
    echo "  exemptions (files that opted out, each one is reviewable in the source):"
    printf '%s\n' "$EXEMPTS" | while IFS=$'\t' read -r rel rule; do
        echo "         ${rel}  [${rule}]"
    done
fi

if [ -n "$HITS" ]; then
    echo "  [FAIL] the test tree pins the location of an external tool:"
    printf '%s\n' "$HITS" | while IFS=$'\t' read -r rel rule detail; do
        echo "         ${rel}  [${rule}]  ${detail}"
    done
    echo ""
    echo "         R1 — resolve the tool at run time instead of hardcoding an"
    echo "              install root (tests/common/bin_resolver.rs)."
    echo "         R2 — take the credential from the environment; never name a"
    echo "              user or a password in the source."
    exit 1
fi

echo "  [OK] no machine-specific external-tool location in the test tree"
echo "  [PASS] External-Tool Path Gate (C7)"
exit 0
