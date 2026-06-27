#!/bin/bash
# G14 Case 8: process_hang
# Process hang (kill -STOP/CONT)
# Refs: V390_TEST_PLAN_ROUND2_REVIEW §G14

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
bash "$SCRIPT_DIR/run_real_crash_test.sh" process_hang
