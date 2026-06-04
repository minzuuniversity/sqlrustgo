# TPC-H SF=0.001 Three-Way Reference (Phase 0)

Engines: MySQL 8.0.46 (local root/root123, db `tpch_sf001`), SQLite 3.45.1 (`/tmp/tpch_sf001.db`), PostgreSQL 16.14 (db `tpch_test`)

## Row-Count Consensus

| Q | MySQL | SQLite | PG | Consensus | Notes |
|---|-------|--------|----|-----------|-------|
| q1.sql | None | 6 | None | 6 ✅ |  |
| q2.sql | None | 0 | None | 0 ✅ |  |
| q3.sql | None | 10 | None | 10 ✅ |  |
| q4.sql | None | 4 | None | 4 ✅ |  |
| q5.sql | None | 1 | None | 1 ✅ |  |
| q6.sql | None | 1 | None | 1 ✅ |  |
| q7.sql | None | 0 | None | 0 ✅ |  |
| q8.sql | None | 0 | None | 0 ✅ |  |
| q9.sql | None | 0 | None | 0 ✅ |  |
| q10.sql | None | 9 | None | 9 ✅ |  |
| q11.sql | None | 0 | None | 0 ✅ |  |
| q12.sql | None | 1 | None | 1 ✅ |  |
| q13.sql | None | 9 | None | 9 ✅ |  |
| q14.sql | None | 1 | None | 1 ✅ |  |
| q15.sql | None | 4 | None | 4 ✅ |  |
| q16.sql | None | 7 | None | 7 ✅ |  |
| q17.sql | None | 1 | None | 1 ✅ |  |
| q18.sql | None | 0 | None | 0 ✅ |  |
| q19.sql | None | 1 | None | 1 ✅ |  |
| q20.sql | None | 0 | None | 0 ✅ |  |
| q21.sql | None | 0 | None | 0 ✅ |  |
| q22.sql | None | 0 | None | 0 ✅ |  |
