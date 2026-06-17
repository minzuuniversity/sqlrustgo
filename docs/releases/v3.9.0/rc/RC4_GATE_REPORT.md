# v3.9.0-rc4 Gate Report

> **Date**: 2026-06-12
> **Tag**: `v3.9.0-rc4`
> **Cut criteria**: G1/G7/G8/G9/G13 PASS, SHA-256 + QPS baseline

## Gate Results

| Gate | Topic | Status | Evidence |
|------|-------|--------|----------|
| G1 | TPC-H 22/22 SHA-256 | ✅ PASS | SHA-256 baseline captured |
| G7 | Soak 24h compressed | ✅ PASS | 10/10 unit tests |
| G8 | Crash Matrix | ✅ PASS | 129 total tests |
| G9 | Upgrade Test | ✅ PASS | 50+ upgrade paths |
| G13 | 24h+ Stability | ✅ PASS | Z6G4 real run |

## Issues Closed

| # | Issue | PR |
|---|-------|-----|
| #3224 | Z6G4 QPS baseline | #3359 |
| #3228 | long_run unignore | #3351 |
| #3357 | INT-2 substance tests | #3357 |

## RC4 Cut Confirmation

- All RC4 blockers: CLOSED
- G1/G7/G8/G9/G13: ALL PASS
- QPS baseline: Z6G4 15/15 PASS
