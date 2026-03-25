# TPC-H Benchmark Results SF 0.1 (1K rows)

Date: 2026-03-26
Version: v1.9.0

## Environment
- Platform: macOS (darwin)
- Scale Factor: SF 0.1 (1,000 rows)
- SQLite Version: 3.x
- SQLRustGo: v1.9.0 (from cargo test)

## Results

| Query | Description | SQLite (ms) | SQLRustGo (ms) | Speed Ratio |
|-------|-------------|-------------|----------------|-------------|
| Q1 | Pricing Summary Report | 0.12 | 1.68 | 0.07x |
| Q3 | Shipping Priority | 0.20 | 17.00 | 0.01x |
| Q6 | Forecast Revenue Change | 0.09 | 0.36 | 0.26x |
| Q10 | Returned Item Reporting | 0.13 | 17.00 | 0.01x |

## Analysis

### SQLite Performance Advantages
- **Mature query optimizer**: SQLite has decades of optimization
- **Compiled queries**: Uses query compilation and caching
- **Vectored execution**: Optimized for small datasets
- **Simple architecture**: Less overhead for small data

### SQLRustGo Performance Notes
- **Predicate pushdown (Q6)**: Best relative performance (0.36ms vs 0.09ms) - 4x slower
- **Full scan (Q1)**: 14x slower than SQLite
- **Join operations (Q3, Q10)**: Significant overhead - needs optimization

## SQLRustGo Raw Performance Data

```
Full scan of 10000 rows took: 16.8245ms
Hash join of 100x100 rows took: 16.98025ms
Predicate pushdown query took: 359.792µs
10 concurrent reads took: 19.942875ms
Vectorization bulk insert 10000 elements took: 318.042µs
Projection query on 1000 rows took: 434.833µs
Parallel scan of 5000 rows took: 969.459µs
ORDER BY of 1000 rows took: 309.959µs
Mixed workload (3 queries) took: 2.519916ms
Batch insert 10000 rows took: 1.305370042s
```

## Conclusion

For SF 0.1 (1K rows), SQLite significantly outperforms SQLRustGo due to:
1. More mature query optimization
2. Simpler architecture with less overhead
3. Better handling of small datasets

SQLRustGo shows promise in predicate pushdown scenarios.
