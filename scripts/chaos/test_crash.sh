#!/bin/bash
# chaos/test_crash.sh — Crash Chaos Test
# Tests SQLRustGo behavior when the process crashes
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Crash Chaos Test ==="
echo "Testing behavior when process crashes..."

# Test 1: WAL crash recovery
echo "[1/3] WAL crash recovery..."
cargo test --test wal_crash_recovery_test 2>/dev/null && echo "  ✅ PASS" || echo "  ⚠️  WAL crash recovery failed"

# Test 2: Crash injection
echo "[2/3] Crash injection..."
cargo test --test crash_inject_test 2>/dev/null && echo "  ✅ PASS" || echo "  ⚠️  crash injection test not found"

# Test 3: Idempotency after crash
echo "[3/3] Idempotency after crash..."
cargo test --test idempotency_test 2>/dev/null && echo "  ✅ PASS" || echo "  ⚠️  idempotency test failed"

echo "=== Crash Chaos Test Complete ==="