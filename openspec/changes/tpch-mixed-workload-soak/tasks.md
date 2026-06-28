## 1. CRUD Templates Module

- [x] 1.1 Create `scripts/soak/crud_templates.py` with parameterized SQL templates for orders, lineitem, customer, partsupp, part (INSERT/UPDATE/DELETE for each)
- [x] 1.2 Add `IdPool` class with pre-allocated ID ranges per worker
- [x] 1.3 Add `validate_fk()` helper that checks foreign key targets exist before INSERT
- [x] 1.4 Verify `python3 -m py_compile scripts/soak/crud_templates.py` passes

## 2. Mixed SOAK Driver

- [x] 2.1 Create `scripts/soak/tpch_mixed_soak_driver.py` with argparse + worker pool skeleton
- [x] 2.2 Implement `MixedSoakStats` dataclass tracking CRUD counters and lock wait times
- [x] 2.3 Implement worker loop: each iteration picks query (80%) or CRUD (20%)
- [x] 2.4 Implement dynamic concurrency controller (4-32 random walk, 30-120s adjustment interval)
- [x] 2.5 Implement replenishment thread (60s check, bulk-INSERT when below MIN_ROWS)
- [x] 2.6 Implement metrics sampler (RSS, FD, per-CRD QPS, read-write conflicts)
- [x] 2.7 Implement SoakReport.json writer with extended fields
- [x] 2.8 Verify `python3 -m py_compile scripts/soak/tpch_mixed_soak_driver.py` passes

## 3. Gate Check Extension

- [x] 3.1 Extend `scripts/gate/check_p13_soak_test.sh` with check 10 (mixed driver file exists)
- [x] 3.2 Add check 11 (CRUD templates module compiles)
- [x] 3.3 Run `bash scripts/gate/check_p13_soak_test.sh` and verify all 11 checks PASS

## 4. Smoke Test

- [x] 4.1 Start a local sqlrustgo server on port 3397 with SF=0.1 TPC-H data
- [x] 4.2 Run `python3 scripts/soak/tpch_mixed_soak_driver.py --duration=30 --port=3397`
- [x] 4.3 Verify SoakReport.json contains all required fields and `errors == 0`
- [x] 4.4 Verify `crud_ratio_actual` is within [18%, 22%]

## 5. Commit & Push

- [x] 5.1 `git add` the 2 new files + gate check diff
- [x] 5.2 `git commit -m "feat(soak): mixed workload TPC-H driver (queries + CRUD)"`
- [x] 5.3 Push to feature branch (NOT develop/v3.9.0 directly — branch is protected)
- [x] 5.4 Create Gitea PR via API linking Issue #3175