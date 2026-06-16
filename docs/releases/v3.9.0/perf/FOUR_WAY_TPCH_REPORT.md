# G17 4-Way TPC-H Comparison Report (v3.9.0)

> **Date**: 2026-06-06  
> **Test**: tests/four_way_compare_test.rs  
> **Data**: SF=1 (simplified — 1500/15000/60000 + 5/25/100/2000/8000)  
> **Engines**: sqlrustgo, SQLite, MariaDB, PostgreSQL  

## Per-Query Results

| Q | sqlrustgo (rows,ms) | SQLite (rows,ms) | MariaDB (rows,ms) | PostgreSQL (rows,ms) | Row count match |
|---|---------------------|------------------|-------------------|----------------------|------------------|
| Q01 | 6 / 179 | 6 / 0 | 6 / 70 | 6 / 54 | ✓ all match |
| Q02 | 0 / 53656 | 0 / 0 | 0 / 11 | 0 / 9 | ✓ all match |
| Q03 | 10 / 28662 | 10 / 0 | 10 / 20 | 10 / 18 | ✓ all match |
| Q04 | 5 / 17 | 5 / 0 | 5 / 18 | 5 / 15 | ✓ all match |
| Q05 | 1 / 29217 | 1 / 0 | 1 / 18 | 1 / 13 | ✓ all match |
| Q06 | 1 / 90 | 1 / 0 | 1 / 14 | 0 / 11 | ✗ MISMATCH |
| Q07 | 3 / 3337 | ERR: prepare: near "FROM": syntax error in SELECT n1.n_ | 3 / 17 | 3 / 12 | ✓ all match |
| Q08 | 1 / 35591 | ERR: prepare: near "FROM": syntax error in SELECT EXTRA | 1 / 19 | 1 / 13 | ✓ all match |
| Q09 | 0 / 30049 | ERR: prepare: near "FROM": syntax error in SELECT n_nam | 0 / 11 | 0 / 9 | ✓ all match |
| Q10 | 20 / 30450 | 20 / 0 | 20 / 18 | 20 / 14 | ✓ all match |
| Q11 | 0 / 48 | 0 / 0 | 0 / 8 | 0 / 8 | ✓ all match |
| Q12 | 2 / 26467 | 2 / 0 | 2 / 15 | 2 / 15 | ✓ all match |
| Q13 | 22 / 1695 | 22 / 0 | 22 / 398 | 22 / 12 | ✓ all match |
| Q14 | 1 / 1212 | 1 / 0 | 1 / 18 | 1 / 12 | ✓ all match |
| Q15 | 91 / 265 | 91 / 0 | 91 / 15 | 91 / 12 | ✓ all match |
| Q16 | 282 / 170 | 282 / 0 | 282 / 12 | 282 / 10 | ✓ all match |
| Q17 | 1 / 1140 | 1 / 0 | 1 / 76 | 1 / 505 | ✓ all match |
| Q18 | 100 / 30205 | 100 / 0 | 100 / 49 | 100 / 38 | ✓ all match |
| Q19 | 1 / 1167 | 1 / 0 | 1 / 17 | 0 / 19 | ✗ MISMATCH |
| Q20 | 6 / 2 | 0 / 0 | 0 / 50018 | 0 / 9 | ✗ MISMATCH |
| Q21 | 6 / 3058 | 0 / 0 | 0 / 33477 | 0 / 17 | ✗ MISMATCH |
| Q22 | 0 / 5 | 0 / 0 | 0 / 27 | 0 / 10 | ✓ all match |

## Per-Engine Total Time

- **sqlrustgo**: 22/22 PASS, 328.00s total
- **sqlite**: 19/22 PASS, 86.37s total
- **mariadb**: 22/22 PASS, 84.36s total
- **postgresql**: 22/22 PASS, 0.85s total
