# G17 4-Way TPC-H Comparison Report (v3.9.0)

> **Date**: 2026-06-06  
> **Test**: tests/four_way_compare_test.rs  
> **Data**: SF=1 (simplified — 1500/15000/60000 + 5/25/100/2000/8000)  
> **Engines**: sqlrustgo, SQLite, MariaDB, PostgreSQL  

## Per-Query Results

| Q | sqlrustgo (rows,ms) | SQLite (rows,ms) | MariaDB (rows,ms) | PostgreSQL (rows,ms) | Row count match |
|---|---------------------|------------------|-------------------|----------------------|------------------|
| Q01 | 0 / 0 | 0 / 0 | 0 / 8 | 6 / 23 | ✗ MISMATCH |
| Q02 | 0 / 0 | 0 / 0 | 0 / 7 | 5 / 9 | ✗ MISMATCH |
| Q03 | 0 / 0 | 0 / 0 | 0 / 7 | 10 / 17 | ✗ MISMATCH |
| Q04 | 0 / 0 | 0 / 0 | 0 / 7 | 5 / 14 | ✗ MISMATCH |
| Q05 | 0 / 0 | 0 / 0 | 0 / 7 | 2 / 12 | ✗ MISMATCH |
| Q06 | 1 / 0 | 1 / 0 | 1 / 7 | 1 / 11 | ✓ all match |
| Q07 | 0 / 0 | ERR: prepare: near "FROM": syntax error in SELECT n1.n_ | 0 / 7 | 2 / 11 | ✗ MISMATCH |
| Q08 | 0 / 0 | ERR: prepare: near "FROM": syntax error in SELECT EXTRA | 0 / 8 | 2 / 12 | ✗ MISMATCH |
| Q09 | 0 / 0 | ERR: prepare: near "FROM": syntax error in SELECT n_nam | 0 / 8 | 0 / 9 | ✓ all match |
| Q10 | 0 / 0 | 0 / 0 | 0 / 8 | 20 / 14 | ✗ MISMATCH |
| Q11 | 0 / 0 | 0 / 0 | 0 / 7 | 0 / 8 | ✓ all match |
| Q12 | 0 / 0 | 0 / 0 | 0 / 6 | 2 / 13 | ✗ MISMATCH |
| Q13 | 0 / 0 | 0 / 0 | 0 / 7 | 21 / 11 | ✗ MISMATCH |
| Q14 | 1 / 0 | 1 / 0 | 1 / 7 | 1 / 11 | ✓ all match |
| Q15 | 0 / 0 | 0 / 0 | 0 / 6 | 93 / 12 | ✗ MISMATCH |
| Q16 | 0 / 0 | 0 / 0 | 0 / 6 | 286 / 10 | ✗ MISMATCH |
| Q17 | 1 / 0 | 1 / 0 | 1 / 7 | 1 / 349 | ✓ all match |
| Q18 | 0 / 0 | 0 / 0 | 0 / 6 | 100 / 23 | ✗ MISMATCH |
| Q19 | 1 / 0 | 1 / 0 | 1 / 7 | 1 / 14 | ✓ all match |
| Q20 | 0 / 0 | 0 / 0 | 0 / 8 | 0 / 8 | ✓ all match |
| Q21 | 0 / 0 | 0 / 0 | 0 / 7 | 0 / 14 | ✓ all match |
| Q22 | 0 / 0 | 0 / 0 | 0 / 7 | 0 / 9 | ✓ all match |

## Per-Engine Total Time

- **sqlrustgo**: 22/22 PASS, 0.00s total
- **sqlite**: 19/22 PASS, 0.00s total
- **mariadb**: 22/22 PASS, 0.17s total
- **postgresql**: 22/22 PASS, 0.62s total
