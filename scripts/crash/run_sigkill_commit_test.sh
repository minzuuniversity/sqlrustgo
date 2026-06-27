#!/bin/bash
# G14 Case 2: sigkill_commit
# SIGKILL during COMMIT flush
# Refs: V390_TEST_PLAN_ROUND2_REVIEW §G14

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
bash "$SCRIPT_DIR/run_real_crash_test.sh" sigkill_commit
