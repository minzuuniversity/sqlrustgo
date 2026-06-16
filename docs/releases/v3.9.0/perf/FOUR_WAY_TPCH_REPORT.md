# G17 4-Way TPC-H Comparison Report (v3.9.0)

> **Date**: 2026-06-06  
> **Test**: tests/four_way_compare_test.rs  
> **Data**: SF=1 (simplified — 1500/15000/60000 + 5/25/100/2000/8000)  
> **Engines**: sqlrustgo, SQLite, MariaDB, PostgreSQL  

## Per-Query Results

| Q | sqlrustgo (rows,ms) | SQLite (rows,ms) | MariaDB (rows,ms) | PostgreSQL (rows,ms) | Row count match |
|---|---------------------|------------------|-------------------|----------------------|------------------|
| Q01 | 0 / 1 | 0 / 0 | 0 / 8 | 6 / 29 | ✗ MISMATCH |
| Q02 | 0 / 1 | 0 / 0 | 0 / 7 | 5 / 16 | ✗ MISMATCH |
| Q03 | 0 / 0 | 0 / 0 | 0 / 6 | 10 / 20 | ✗ MISMATCH |
| Q04 | 0 / 0 | 0 / 0 | 0 / 7 | 5 / 13 | ✗ MISMATCH |
| Q05 | 0 / 1 | 0 / 0 | 0 / 6 | 2 / 10 | ✗ MISMATCH |
| Q06 | 1 / 0 | 1 / 0 | 1 / 6 | 1 / 10 | ✓ all match |
| Q07 | 0 / 2 | ERR: prepare: near "FROM": syntax error in SELECT n1.n_ | 0 / 6 | 2 / 10 | ✗ MISMATCH |
| Q08 | 0 / 3 | ERR: prepare: near "FROM": syntax error in SELECT EXTRA | 0 / 6 | 2 / 11 | ✗ MISMATCH |
| Q09 | 0 / 2 | ERR: prepare: near "FROM": syntax error in SELECT n_nam | 0 / 6 | 0 / 8 | ✓ all match |
| Q10 | 0 / 1 | 0 / 0 | 0 / 6 | 20 / 12 | ✗ MISMATCH |
| Q11 | 0 / 0 | 0 / 0 | 0 / 6 | 0 / 7 | ✓ all match |
| Q12 | 0 / 1 | 0 / 0 | 0 / 6 | 2 / 12 | ✗ MISMATCH |
| Q13 | 0 / 1 | 0 / 0 | 0 / 6 | 21 / 12 | ✗ MISMATCH |
| Q14 | 1 / 0 | 1 / 0 | 1 / 6 | 1 / 10 | ✓ all match |
| Q15 | 0 / 0 | 0 / 0 | 0 / 7 | 93 / 10 | ✗ MISMATCH |
| Q16 | 0 / 1 | 0 / 0 | 0 / 6 | 286 / 9 | ✗ MISMATCH |
| Q17 | 1 / 0 | 1 / 0 | 1 / 6 | 1 / 341 | ✓ all match |
| Q18 | 0 / 0 | 0 / 0 | 0 / 6 | 100 / 22 | ✗ MISMATCH |
| Q19 | 1 / 0 | 1 / 0 | 1 / 6 | 1 / 13 | ✓ all match |
| Q20 | 0 / 1 | 0 / 0 | 0 / 6 | 0 / 7 | ✓ all match |
| Q21 | 0 / 2 | 0 / 0 | 0 / 6 | 0 / 13 | ✓ all match |
| Q22 | 0 / 1 | 0 / 0 | 0 / 7 | 0 / 8 | ✓ all match |

## Per-Engine Total Time

- **sqlrustgo**: 22/22 PASS, 0.03s total
- **sqlite**: 19/22 PASS, 0.00s total
- **mariadb**: 22/22 PASS, 0.15s total
- **postgresql**: 22/22 PASS, 0.61s total
