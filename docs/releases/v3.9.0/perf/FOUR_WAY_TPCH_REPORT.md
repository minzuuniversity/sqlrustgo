# G17 4-Way TPC-H Comparison Report (v3.9.0)

> **Date**: 2026-06-06  
> **Test**: tests/four_way_compare_test.rs  
> **Data**: SF=1 (simplified — 1500/15000/60000 + 5/25/100/2000/8000)  
> **Engines**: sqlrustgo, SQLite, MariaDB, PostgreSQL  

## Per-Query Results

| Q | sqlrustgo (rows,ms) | SQLite (rows,ms) | MariaDB (rows,ms) | PostgreSQL (rows,ms) | Row count match |
|---|---------------------|------------------|-------------------|----------------------|------------------|
| Q01 | 6 / 51 | 6 / 0 | 6 / 82 | 6 / 66 | ✓ all match |
| Q02 | 0 / 6332 | 0 / 0 | 0 / 13 | 0 / 15 | ✓ all match |
| Q03 | 10 / 3806 | 10 / 0 | 10 / 25 | 10 / 26 | ✓ all match |
| Q04 | 0 / 5 | 5 / 0 | 5 / 21 | 5 / 22 | ✗ MISMATCH |
| Q05 | 1 / 3731 | 1 / 0 | 1 / 21 | 1 / 18 | ✓ all match |
| Q06 | 1 / 30 | 1 / 0 | 1 / 17 | 0 / 17 | ✗ MISMATCH |
| Q07 | 3 / 530 | ERR: prepare: near "FROM": syntax error in SELECT n1.n_ | 3 / 19 | 3 / 16 | ✓ all match |
| Q08 | 1 / 5744 | ERR: prepare: near "FROM": syntax error in SELECT EXTRA | 1 / 20 | 1 / 17 | ✓ all match |
| Q09 | 0 / 3967 | ERR: prepare: near "FROM": syntax error in SELECT n_nam | 0 / 11 | 0 / 11 | ✓ all match |
| Q10 | 20 / 3861 | 20 / 0 | 20 / 20 | 20 / 19 | ✓ all match |
| Q11 | 0 / 10 | 0 / 0 | 0 / 9 | 0 / 10 | ✓ all match |
| Q12 | 2 / 3599 | 2 / 0 | 2 / 17 | 2 / 19 | ✓ all match |
| Q13 | 22 / 171 | 22 / 0 | 22 / 376 | 22 / 14 | ✓ all match |
| Q14 | 1 / 163 | 1 / 0 | 1 / 23 | 1 / 14 | ✓ all match |
| Q15 | 91 / 79 | 91 / 0 | 91 / 21 | 91 / 14 | ✓ all match |
| Q16 | 282 / 22 | 282 / 0 | 282 / 16 | 282 / 12 | ✓ all match |
| Q17 | 1 / 158 | 1 / 0 | 1 / 78 | 1 / 480 | ✓ all match |
| Q18 | 100 / 3846 | 100 / 0 | 100 / 55 | 100 / 34 | ✓ all match |
| Q19 | 1 / 164 | 1 / 0 | 1 / 22 | 0 / 21 | ✗ MISMATCH |
| Q20 | 0 / 0 | 0 / 0 | 0 / 44965 | 0 / 11 | ✓ all match |
| Q21 | 0 / 443 | 0 / 0 | 0 / 29725 | 0 / 17 | ✓ all match |
| Q22 | 0 / 0 | 0 / 0 | 0 / 31 | 0 / 10 | ✓ all match |

## Per-Engine Total Time

- **sqlrustgo**: 22/22 PASS, 51.89s total
- **sqlite**: 19/22 PASS, 29.68s total
- **mariadb**: 22/22 PASS, 75.60s total
- **postgresql**: 22/22 PASS, 0.89s total
