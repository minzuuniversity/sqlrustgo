# G17 4-Way TPC-H Comparison Report (v3.9.0)

> **Date**: 2026-06-06  
> **Test**: tests/four_way_compare_test.rs  
> **Data**: SF=1 (simplified — 1500/15000/60000 + 5/25/100/2000/8000)  
> **Engines**: sqlrustgo, SQLite, MariaDB, PostgreSQL  

## Per-Query Results

| Q | sqlrustgo (rows,ms) | SQLite (rows,ms) | MariaDB (rows,ms) | PostgreSQL (rows,ms) | Row count match |
|---|---------------------|------------------|-------------------|----------------------|------------------|
| Q01 | 6 / 55 | 6 / 0 | 6 / 71 | 6 / 53 | ✓ all match |
| Q02 | 0 / 7993 | 0 / 0 | 0 / 11 | 0 / 11 | ✓ all match |
| Q03 | 10 / 4528 | 10 / 0 | 10 / 20 | 10 / 19 | ✓ all match |
| Q04 | 5 / 6 | 5 / 0 | 5 / 17 | 5 / 14 | ✓ all match |
| Q05 | 1 / 4605 | 1 / 0 | 1 / 18 | 1 / 12 | ✓ all match |
| Q06 | 1 / 31 | 1 / 0 | 1 / 14 | 0 / 12 | ✗ MISMATCH |
| Q07 | 3 / 615 | ERR: prepare: near "FROM": syntax error in SELECT n1.n_ | 3 / 17 | 3 / 12 | ✓ all match |
| Q08 | 1 / 6648 | ERR: prepare: near "FROM": syntax error in SELECT EXTRA | 1 / 19 | 1 / 12 | ✓ all match |
| Q09 | 0 / 5022 | ERR: prepare: near "FROM": syntax error in SELECT n_nam | 0 / 11 | 0 / 9 | ✓ all match |
| Q10 | 20 / 4733 | 20 / 0 | 20 / 19 | 20 / 14 | ✓ all match |
| Q11 | 0 / 12 | 0 / 0 | 0 / 9 | 0 / 8 | ✓ all match |
| Q12 | 2 / 5825 | 2 / 0 | 2 / 16 | 2 / 14 | ✓ all match |
| Q13 | 22 / 191 | 22 / 0 | 22 / 390 | 22 / 12 | ✓ all match |
| Q14 | 1 / 180 | 1 / 0 | 1 / 18 | 1 / 12 | ✓ all match |
| Q15 | 91 / 88 | 91 / 0 | 91 / 15 | 91 / 12 | ✓ all match |
| Q16 | 282 / 25 | 282 / 0 | 282 / 12 | 282 / 10 | ✓ all match |
| Q17 | 1 / 177 | 1 / 0 | 1 / 81 | 1 / 529 | ✓ all match |
| Q18 | 100 / 8132 | 100 / 0 | 100 / 51 | 100 / 35 | ✓ all match |
| Q19 | 1 / 224 | 1 / 0 | 1 / 25 | 0 / 20 | ✗ MISMATCH |
| Q20 | 6 / 0 | 0 / 0 | 0 / 49333 | 0 / 9 | ✗ MISMATCH |
| Q21 | 6 / 949 | 0 / 0 | 0 / 32729 | 0 / 15 | ✗ MISMATCH |
| Q22 | 0 / 1 | 0 / 0 | 0 / 22 | 0 / 9 | ✓ all match |

## Per-Engine Total Time

- **sqlrustgo**: 22/22 PASS, 66.02s total
- **sqlite**: 19/22 PASS, 41.47s total
- **mariadb**: 22/22 PASS, 82.93s total
- **postgresql**: 22/22 PASS, 0.87s total
