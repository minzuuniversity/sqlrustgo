#!/usr/bin/env python3
"""
Layer 3 Semantic Gate — Semantic Governance Layer (SGL)
=========================================================
Purpose: Validate CI behavior matches documented contracts.

Layer 1 (Syntactic):  command → exit code (pass/fail)
Layer 2 (Behavioral):  execution + assertions (tests/lint/format)
Layer 3 (Semantic):    spec vs implementation drift detection ← THIS FILE

Scope: v3.8.0 Beta Gate
Exit codes: 0=ALL_PASS, 1=FAIL, 2=DRIFT_DETECTED
"""

import subprocess
import sys
import os
import re
import hashlib

REPO = os.environ.get("GIT_REPO", os.path.expanduser("~/sqlrustgo"))
os.chdir(REPO)

RED = "\033[0;31m"
GREEN = "\033[0;32m"
YELLOW = "\033[1;33m"
NC = "\033[0m"

overall_result = 0  # 0=PASS, 1=FAIL, 2=DRIFT
drift_count = 0
fail_count = 0


def run(cmd, capture=True):
    r = subprocess.run(cmd, shell=True, capture_output=capture, text=True)
    return r


def log_pass(msg):
    print(f"{GREEN}[PASS]{NC} {msg}")


def log_fail(msg):
    global fail_count, overall_result
    fail_count += 1
    overall_result = max(overall_result, 1)
    print(f"{RED}[FAIL]{NC} {msg}")


def log_drift(msg):
    global drift_count, overall_result
    drift_count += 1
    overall_result = max(overall_result, 2)
    print(f"{YELLOW}[DRIFT]{NC} {msg}")


# ============================================================================
# SGL-001: Tool Semantics Registry — B4 Format Check
# Contract: "B4 Format must pass 'cargo fmt --all -- --check' without auto-fix"
# Tool rule: cargo fmt --check must NOT mutate files
# ============================================================================
print("\n=== SGL-001: B4 Format — Tool Semantics Audit ===")

before_hash = hashlib.md5()
r = run("git status --short")
before_hash.update(r.stdout.encode())

r = run("cargo fmt --all -- --check")
exit_code = r.returncode

after_hash = hashlib.md5()
r2 = run("git status --short")
after_hash.update(r2.stdout.encode())

if before_hash.hexdigest() != after_hash.hexdigest():
    log_drift("SGL-001: cargo fmt --check mutated working tree (silent auto-fix detected)")
elif exit_code == 0:
    log_pass("SGL-001: fmt check is truly read-only (no mutation, exit 0)")
else:
    log_fail(f"SGL-001: fmt check exits {exit_code} — format violations exist on HEAD")
    print(r.stdout[:500])

# ============================================================================
# SGL-002: WAL-002 — advance_checkpoint in commit path
# Contract (PR-830F): "commit_transaction must advance checkpoint"
# Check in WalStorage since WAL lifecycle is implemented there
# ============================================================================
print("\n=== SGL-002: WAL-002 — advance_checkpoint in commit path ===")

wal_storage_path = "crates/storage/src/wal_storage.rs"
with open(wal_storage_path) as f:
    wal_content = f.read()

# Extract commit_transaction from WalStorage
match = re.search(
    r"pub fn commit_transaction\s*\([^)]*\)\s*(?:->[^=]+)?\s*\{",
    wal_content,
)
if not match:
    log_fail("SGL-002: commit_transaction not found in wal_storage.rs")
else:
    start = match.end()
    depth = 1
    pos = start
    while depth > 0 and pos < len(wal_content):
        if wal_content[pos] == "{":
            depth += 1
        elif wal_content[pos] == "}":
            depth -= 1
        pos += 1
    commit_fn_body = wal_content[start : pos - 1]

    # WAL-002: checkpoint advance via record_checkpoint (the actual checkpoint mechanism)
    if "record_checkpoint" in commit_fn_body or "advance_checkpoint" in commit_fn_body:
        log_pass("SGL-002: checkpoint advance triggered in WalStorage.commit_transaction")
    else:
        log_fail(
            "SGL-002: checkpoint NOT advanced in WalStorage.commit_transaction (WAL truncation never triggers)"
        )
        print("  WAL-002 invariant violated: checkpoint_manager never receives record_checkpoint")

# ============================================================================
# SGL-003: WAL-003 — WAL truncation after commit
# Invariant WAL-003: "truncation only after durable commit"
# ============================================================================
print("\n=== SGL-003: WAL-003 — WAL truncation in commit path ===")

wal_commit_fn_body = ""
match3 = re.search(
    r"pub fn commit_transaction\s*\([^)]*\)\s*(?:->[^=]+)?\s*\{",
    wal_content,
)
if match3:
    start = match3.end()
    depth = 1
    pos = start
    while depth > 0 and pos < len(wal_content):
        if wal_content[pos] == "{":
            depth += 1
        elif wal_content[pos] == "}":
            depth -= 1
        pos += 1
    wal_commit_fn_body = wal_content[start : pos - 1]

if wal_commit_fn_body and "truncate_before" in wal_commit_fn_body:
    log_pass("SGL-003: truncate_before called in WalStorage.commit_transaction")
else:
    log_fail(
        "SGL-003: WAL truncation NOT triggered in commit_transaction (WAL grows unbounded)"
    )
    print("  WAL-003 invariant violated: WAL never truncates after commit")

# ============================================================================
# SGL-004: WAL-004 — DELETE replay idempotency
# Check if DELETE implementation does delete+insert (causes phantom row on crash)
# ============================================================================
print("\n=== SGL-004: WAL-004 — DELETE replay idempotency ===")

# Find WalStorage source
wal_paths = [
    "crates/storage/src/wal_storage.rs",
    "crates/storage/src/wal/mod.rs",
    "crates/storage/src/wal.rs",
]
wal_content = ""
for p in wal_paths:
    if os.path.exists(p):
        with open(p) as f:
            wal_content = f.read()
        break

if not wal_content:
    log_drift("SGL-004: WalStorage source not found — cannot audit replay semantics")
else:
    # Check for delete+insert pattern in delete handling
    delete_patterns = [
        r"apply_delete.*insert",
        r"DeleteEntry.*insert",
        r"delete.*insert.*row",
    ]
    found_violation = False
    for pat in delete_patterns:
        if re.search(pat, wal_content, re.IGNORECASE):
            found_violation = True
            break

    # Also check via grep for direct patterns
    r = run('grep -n "delete.*insert" crates/storage/src/wal*.rs 2>/dev/null | head -5')
    if r.stdout.strip() or found_violation:
        log_fail(
            "SGL-004: DELETE replays as delete+insert (non-idempotent — crash leaves phantom row)"
        )
        print("  WAL-004 violated: DELETE entry type causes phantom row on crash recovery")
        print(f"  Evidence: {r.stdout[:200]}")
    else:
        log_pass("SGL-004: WAL delete replay appears idempotent")

# ============================================================================
# SGL-005: TX-002 — Storage direct bypass detection (AV-001~AV-007 legacy)
# Invariant TX-002: "all mutations must go through TransactionManager"
#
# Classification rules:
#   SKIP: harness.rs (test fixtures)
#   SKIP: #[test] functions
#   SKIP: wal_transactional_facade.rs (correct via log_mutation)
#   SKIP: parallel_executor.rs memory_storage (batch loading, non-OLTP)
#   SKIP: vector_executor.rs (benchmark fixtures)
#   REAL: trigger.rs (trigger body bypass)
#   REAL: local_executor.rs non-facade storage ops
# ============================================================================
print("\n=== SGL-005: TX-002 — Storage direct bypass detection ===")

violations = []
skip_reasons = {}

def should_skip(filepath, linenum, content):
    """Returns (skip: bool, reason: str or None)"""
    # Skip test files
    if '[test]' in content or 'fn test_' in content:
        return (True, "test-function")
    # Skip harness
    if 'harness' in filepath:
        return (True, "test-harness")
    # Skip wal facade (correct implementation)
    if 'wal_transactional_facade' in filepath:
        return (True, "wal-facade")
    # Skip parallel_executor memory_storage batch loading
    if 'parallel_executor' in filepath and 'memory_storage' in content:
        return (True, "batch-loading")
    # Skip vector_executor fixtures
    if 'vector_executor' in filepath:
        return (True, "vector-fixture")
    # Check if inside execute_dml closure (WAL-aware path) — look back 3 lines
    if 'storage.delete' in content or 'storage.insert' in content or 'storage.update' in content:
        r_ctx = run(f'sed -n "{linenum-3},{linenum}p" "{filepath}"')
        if 'execute_dml' in r_ctx.stdout and 'facade' in r_ctx.stdout:
            return (True, "facade-closure")
    return (False, None)

for crate in ["executor", "server"]:
    crate_path = f"crates/{crate}/src"
    if not os.path.exists(crate_path):
        continue
    r = run(
        f'grep -rn "storage.insert\\|storage.update\\|storage.delete\\|memory_storage.insert\\|memory_storage.update\\|memory_storage.delete" "{crate_path}/" --include="*.rs" 2>/dev/null'
    )
    if r.stdout.strip():
        for line in r.stdout.strip().split("\n"):
            if not line:
                continue
            parts = line.split(":")
            if len(parts) < 3:
                continue
            filepath = parts[0]
            linenum = int(parts[1])
            content = ":".join(parts[2:])

            skip, reason = should_skip(filepath, linenum, content)
            if skip:
                skip_reasons[line] = reason
            else:
                violations.append(line)

# Separate real violations from skipped
skipped = skip_reasons
print(f"  Skipped: {len(skipped)} (test/batch/facade/harness)")
print(f"  Real violations: {len(violations)}")

if violations:
    log_drift(f"SGL-005: TX-002 — {len(violations)} production path storage bypasses (AV-001~AV-007)")
    for v in violations[:10]:
        print(f"    {v}")
else:
    log_pass("SGL-005: No production path storage bypasses in executor/server crates")
# ============================================================================
# Summary
# ============================================================================
print("\n" + "=" * 50)
print("SGL Beta Gate Summary")
print("=" * 50)
total = 5
passed = total - fail_count - drift_count
print(f"  PASS : {passed}/{total}")
print(f"  FAIL : {fail_count}")
print(f"  DRIFT: {drift_count}")
print()

if overall_result == 0:
    print(f"{GREEN}SGL-ALL-PASS{NC} — Semantic invariants satisfied")
elif overall_result == 2:
    print(f"{YELLOW}SGL-DRIFT-DETECTED{NC} — Contract/implementation drift found")
else:
    print(f"{RED}SGL-FAIL{NC} — Hard invariant violations")

print(f"\nExit code: {overall_result}")
sys.exit(overall_result)
