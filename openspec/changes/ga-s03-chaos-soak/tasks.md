## 1. Chaos Controller Implementation

- [x] 1.1 Create `scripts/soak/chaos_inject.py` with `ChaosController` class
- [x] 1.2 Implement `inject_io_latency()` using `tc qdisc` (Linux only)
- [x] 1.3 Implement `inject_memory_pressure()` using `stress-ng --vm`
- [x] 1.4 Implement `kill_server_process()` with PID tracking
- [x] 1.5 Implement `verify_recovery()` with timeout and health check
- [x] 1.6 Add platform detection (Linux vs macOS) with graceful degradation

## 2. Integration with SOAK Driver

- [x] 2.1 Modify `scripts/soak/tpch_mixed_soak_driver.py` to accept `--chaos` flag
- [x] 2.2 Add `--chaos-types` argument to specify which experiments to run
- [x] 2.3 Integrate chaos phase between normal phase and cleanup
- [x] 2.4 Add chaos results summary to SOAK report

## 3. Rust Chaos Test Suite

- [x] 3.1 Create `tests/soak/chaos_soak_test.rs` with test harness
- [x] 3.2 Implement `test_chaos_io_latency_recovery` — inject 100ms delay, verify 5s recovery
- [x] 3.3 Implement `test_chaos_memory_pressure_no_oom` — verify no panic under memory pressure
- [x] 3.4 Implement `test_chaos_kill9_data_integrity` — verify data checksum after kill -9

## 4. GA Gate Script

- [x] 4.1 Create `scripts/gate/check_chaos_soak.sh` gate script
- [x] 4.2 Verify chaos_inject.py exists and is valid Python
- [x] 4.3 Verify chaos_soak_test.rs compiles with `cargo check`
- [x] 4.4 Run chaos_soak_test with appropriate timeouts
- [x] 4.5 Add to RC gate check list

## 5. Documentation

- [x] 5.1 Update `docs/soak-test-plan.md` with chaos testing section
- [x] 5.2 Add chaos injection usage to `scripts/soak/README.md`
- [x] 5.3 Document platform requirements (Linux + sudo for tc/stress-ng)
