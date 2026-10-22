#!/usr/bin/env bash
# check_release_binary.sh — GA-P0 Release Binary Gate
#
# Verifies release binary infrastructure for Issue #3606:
# 1. Build script exists and is valid bash
# 2. Verification script exists
# 3. sqlrustgo-cli has --version flag
# 4. Cargo.lock exists
#
# Exit code: 0 = PASS, 1 = FAIL

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

echo "=== GA-P0 Release Binary Gate (Issue #3606) ==="
echo ""

PASS=0
FAIL=0

check() {
    local name="$1"
    local cmd="$2"
    if eval "$cmd" >/dev/null 2>&1; then
        echo "  [PASS] $name"
        PASS=$((PASS+1))
    else
        echo "  [FAIL] $name"
        FAIL=$((FAIL+1))
    fi
}

# 1. Build script exists
echo "--- Build Script ---"
if [ -f "scripts/build/release_binary.sh" ]; then
    echo "  [PASS] scripts/build/release_binary.sh exists"
    PASS=$((PASS+1))
else
    echo "  [FAIL] scripts/build/release_binary.sh missing"
    FAIL=$((FAIL+1))
fi

check "build script syntax" "bash -n scripts/build/release_binary.sh"
check "release_binary.sh function" "grep -q 'generate_checksum()' scripts/build/release_binary.sh"
check "generate_version_file function" "grep -q 'generate_version_file()' scripts/build/release_binary.sh"

# 2. Verification script exists
echo ""
echo "--- Verification Script ---"
if [ -f "scripts/verify_binary_checksum.sh" ]; then
    echo "  [PASS] scripts/verify_binary_checksum.sh exists"
    PASS=$((PASS+1))
else
    echo "  [FAIL] scripts/verify_binary_checksum.sh missing"
    FAIL=$((FAIL+1))
fi

check "verification script syntax" "bash -n scripts/verify_binary_checksum.sh"

# 3. --version flag
echo ""
echo "--- Version Flag ---"
# #5442 —— 版本判据必须是**相等**，不是「含有 sqlrustgo」。
#
# 旧判据 `grep -q "sqlrustgo"` 只要输出里出现这个名字就算过：
# `3.12.0-fix-zombie`、`3.9.0`、`4.1.0` 一视同仁。于是发布出去的二进制
# 自报 3.9.0、发布线却是 v4.1.0 时，**没有任何门禁会发现**。
#
# 期望版本取自 `Cargo.toml` 的 `[workspace.package] version` ——
# 版本号的**唯一**来源（策略见 VERSION_PROMOTION_SOP.md §0.1），
# 不在门禁里再写一份字面量。
EXPECTED_VERSION="$(awk '
  /^\[workspace\.package\]/{f=1;next}
  /^\[/{f=0}
  f && /^version[[:space:]]*=[[:space:]]*"/{
    gsub(/.*=[[:space:]]*"/,""); gsub(/".*/,""); print; exit
  }' Cargo.toml)"
ACTUAL_VERSION="$(cargo run --package sqlrustgo-cli -- --version 2>/dev/null | head -1 | awk '{print $NF}')"
if [ -z "$EXPECTED_VERSION" ]; then
    echo "  [FAIL] cannot read [workspace.package] version from Cargo.toml"
    FAIL=$((FAIL+1))
elif [ "$ACTUAL_VERSION" = "$EXPECTED_VERSION" ]; then
    echo "  [PASS] sqlrustgo-cli --version reports $ACTUAL_VERSION (== Cargo.toml [workspace.package] version)"
    PASS=$((PASS+1))
else
    echo "  [FAIL] sqlrustgo-cli --version reports '$ACTUAL_VERSION' but the release line is '$EXPECTED_VERSION'"
    echo "         (the release binary must self-report the release line version)"
    FAIL=$((FAIL+1))
fi

# 4. Cargo.lock exists
echo ""
echo "--- Build Environment ---"
if [ -f "Cargo.lock" ]; then
    echo "  [PASS] Cargo.lock exists"
    PASS=$((PASS+1))
else
    echo "  [FAIL] Cargo.lock missing"
    FAIL=$((FAIL+1))
fi

# Gate script executable
check "gate script executable" "[ -x '$0' ]"

echo ""
echo "=== Release Binary Gate Summary ==="
echo "PASS: $PASS"
echo "FAIL: $FAIL"
echo ""

if [ "$FAIL" -eq 0 ]; then
    echo "✅ Release Binary gate PASSED"
    exit 0
else
    echo "❌ Release Binary gate FAILED ($FAIL blocker(s))"
    exit 1
fi
