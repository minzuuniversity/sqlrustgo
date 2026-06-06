# G17 4-Way TPC-H Comparison Report (v3.9.0)

> **Date**: 2026-06-06  
> **Test**: tests/four_way_compare_test.rs  
> **Data**: SF=1 (simplified — 1500/15000/60000 + 5/25/100/2000/8000)  
> **Engines**: sqlrustgo, SQLite, MariaDB, PostgreSQL  

## Per-Query Results

| Q | sqlrustgo (rows,ms) | SQLite (rows,ms) | MariaDB (rows,ms) | PostgreSQL (rows,ms) | Row count match |
|---|---------------------|------------------|-------------------|----------------------|------------------|
| Q01 | 6 / 55 | 6 / 0 | 6 / 80 | 6 / 61 | ✓ all match |
| Q02 | 0 / 6906 | 0 / 0 | 0 / 13 | 0 / 13 | ✓ all match |
| Q03 | 10 / 4426 | 10 / 0 | 10 / 22 | 10 / 25 | ✓ all match |
| Q04 | 0 / 6 | 5 / 0 | 5 / 19 | 5 / 20 | ✗ MISMATCH |
| Q05 | 1 / 4412 | 1 / 0 | 1 / 19 | 1 / 17 | ✓ all match |
| Q06 | 1 / 32 | 1 / 0 | 1 / 15 | 0 / 14 | ✗ MISMATCH |
| Q07 | 3 / 601 | ERR: prepare: near "FROM": syntax error in SELECT n1.n_ | 3 / 18 | 3 / 15 | ✓ all match |
| Q08 | 1 / 6409 | ERR: prepare: near "FROM": syntax error in SELECT EXTRA | 1 / 21 | 1 / 15 | ✓ all match |
| Q09 | 0 / 4687 | ERR: prepare: near "FROM": syntax error in SELECT n_nam | 0 / 12 | 0 / 11 | ✓ all match |
| Q10 | 20 / 4438 | 20 / 0 | 20 / 20 | 20 / 17 | ✓ all match |
| Q11 | 0 / 10 | 0 / 0 | 0 / 9 | 0 / 10 | ✓ all match |
| Q12 | 2 / 4524 | 2 / 0 | 2 / 16 | 2 / 16 | ✓ all match |
| Q13 | 22 / 177 | 22 / 0 | 22 / 379 | 22 / 13 | ✓ all match |
| Q14 | 1 / 170 | 1 / 0 | 1 / 23 | 1 / 13 | ✓ all match |
| Q15 | 0 / 0 | 91 / 0 | 91 / 19 | 91 / 13 | ✗ MISMATCH |
| Q16 | 282 / 50 | 282 / 0 | 282 / 14 | 282 / 11 | ✓ all match |
| Q17 | 1 / 161 | 1 / 0 | 1 / 77 | 1 / 496 | ✓ all match |
| Q18 | 100 / 4487 | 100 / 0 | 100 / 53 | 100 / 37 | ✓ all match |
| Q19 | 1 / 168 | 1 / 0 | 1 / 20 | 0 / 21 | ✗ MISMATCH |
| Q20 | 0 / 0 | 0 / 0 | 0 / 45736 | 0 / 12 | ✓ all match |
| Q21 | 0 / 516 | 0 / 0 | 0 / 30070 | 0 / 18 | ✓ all match |
| Q22 | 0 / 0 | 0 / 0 | 0 / 26 | 0 / 11 | ✓ all match |

## Per-Engine Total Time

- **sqlrustgo**: 22/22 PASS, 57.86s total
- **sqlite**: 19/22 PASS, 29.60s total
- **mariadb**: 22/22 PASS, 76.69s total
- **postgresql**: 22/22 PASS, 0.89s total
