# v3.12.0 R2 Architectural Invariants Report (updated 2026-08-15)

- source_agent: `minimax` (re-run after PRs #4294-#4300 + SEM-4 coverage close)
- source_run: `post-v3.12.0-merge-sink`
- timestamp: `2026-08-15T12:00:00Z`
- branch: `develop/v3.12.0`
- commit: `5a0788639` (HEAD after v3.12.0 all merges)

| check | status | stdout_sha256 | exit_code |
|-------|--------|---------------|-----------|
| R2.1 | pass | c4db2455cb2249cbe880945a53f4a3c1e7887b623b6c0cc72c08c8d5191fc230 | 0 |
| R2.2 | pass | c5dbafc8a710a080833c72437a8f172cd4b33ef0dee07271d3772d48fc91e2aa | 0 |
| R2.3 | pass | ed04ff36a5ed91e8af78698e12e1e381387a3a6294e25807f0a15b78b9f45492 | 0 |
| R2.4 | pass | c8009addd1d1214968a1405121c32ee188a5d7ce6765fe3c0b7342e7d18e9dd0 | 0 |
| R2.5 | pass | c50eb4e8f3c14d7da5d2ed3c5355c31d617d00196210d1aa3e7a35d479868037 | 0 |
| R2.6 | pass | e4f05b85eaabf0294885718ae12940d84e1e9af4ea644dd4fcd15e069b29bf5c | 0 |
| R2.7 | pass | 428e383178f9afab9949766714782668675075804135f3a6fe58064d8fef84 | 0 |
| R2.8 | stub | fadf1c0585fadc471318fa5e7fb768cf940da78230d50c72b02ed0217df2d6f2 | 0 |

---

<!-- R2.4 note: SEM-4 now CLOSED (2026-08-15). L1_8 avg coverage 84.44% >= 80%.
   All 8 core crates measured: parser 75.16%, planner 86.75%, executor 82.80%,
   transaction 85.41%, storage 83.52%, catalog 84.77%, optimizer 87.06%, types 90.07%.
   check_arch_sem_debt.sh exits 0. -->

<!-- Original report (pre-merge):
     source_agent: minimax
     source_run: minimax-v312-19-r2-420124bf31
     timestamp: 2026-08-14T04:11:53Z
     branch: fix/v312-19-r2-gate-load-infile-drift
     commit: 420124bf31cdca6bc928c3715ed72735b158f611
     R2.4 was: drift, exit 2 (SEM-4 IN_PROGRESS) -->
