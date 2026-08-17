# SPRINT-S0 TPC-H SF=1 Fixture Evidence

| 项 | 值 |
|---|---|
| Fixture 路径 | /tmp/tpch-sf1/ |
| 来源 | dbgen (electrum/tpch-dbgen) |
| Scale Factor | 1 |
| 生成时间 | 2026-08-17T18:40:00Z |
| 表数 | 8 (region/nation/supplier/customer/part/partsupp/orders/lineitem) |

## 行数验证

| 表 | 文件 | 期望行数 | 实际行数 | 状态 |
|---|---|---|---|---|
| region | region.tbl | 5 | 5 | ✅ |
| nation | nation.tbl | 25 | 25 | ✅ |
| supplier | supplier.tbl | 10000 | 10000 | ✅ |
| customer | customer.tbl | 150000 | 150000 | ✅ |
| part | part.tbl | 200000 | 200000 | ✅ |
| partsupp | partsupp.tbl | 800000 | 800000 | ✅ |
| orders | orders.tbl | 1500000 | 1500000 | ✅ |
| lineitem | lineitem.tbl | 6001215 | 6001215 | ✅ |

## SHA-256 清单

```
4483680548a965833877c911ed43e795f4d3543c7a3f7d1dba9ccb24ea5989d6  /tmp/tpch-sf1/customer.tbl
96d555e07a1ae8cf5196387d9edd9427f9af70c56fa5f4b18affee5555ddb184  /tmp/tpch-sf1/lineitem.tbl
66f96949939fa8fdf1c4ffed1e5f6c2842fe11a14b51fdc6ed1e17460031e8c5  /tmp/tpch-sf1/nation.tbl
8709061d7bbc81932356fdfc664f8d582252747c2d7e204ae6d3cde624586357  /tmp/tpch-sf1/orders.tbl
43c37f99918f06d4de6b99b05c0a28d5c46f71d66424cffcc595cb059a499254  /tmp/tpch-sf1/partsupp.tbl
f0e4ccdfb5f6d19428ce54f9c84b17037d20f00ac8d2b2272c8d43b18a0b4880  /tmp/tpch-sf1/part.tbl
6022658d673924389b54dcb70fa8c3d6da1b0d7afa3c1c017bab62a019df404f  /tmp/tpch-sf1/region.tbl
9b99cf155974e6db8773970b40746bfccfa64fa078169574165f3e19e2158391  /tmp/tpch-sf1/supplier.tbl
```
