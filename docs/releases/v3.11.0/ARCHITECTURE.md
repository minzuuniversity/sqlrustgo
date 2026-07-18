# SQLRustGo v3.11.0 Architecture

## Overview

SQLRustGo is a relational database system implementing a SQL-92 subset with a Volcano-style execution model.

## System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Clients                                 │
│   (MySQL CLI, JDBC, mysql-client, sqlrustgo-cli)                 │
└────────────────────────────┬────────────────────────────────────┘
                             │ MySQL Wire Protocol
┌────────────────────────────▼────────────────────────────────────┐
│                    sqlrustgo-mysql-server                        │
│   ┌──────────────────────────────────────────────────────────┐  │
│   │              Connection Manager (port 3307)              │  │
│   └──────────────────────────────────────────────────────────┘  │
│   ┌──────────────────────────────────────────────────────────┐  │
│   │                   Parser (lalrpop)                       │  │
│   │         SQL → AST → Relational Algebra                   │  │
│   └──────────────────────────────────────────────────────────┘  │
└────────────────────────────┬────────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────────┐
│                    sqlrustgo-planner                            │
│   ┌──────────────────────────────────────────────────────────┐  │
│   │              Logical Optimizer                           │  │
│   │   - Predicate pushdown                                 │  │
│   │   - Join reordering                                    │  │
│   │   - Projection elimination                             │  │
│   └──────────────────────────────────────────────────────────┘  │
└────────────────────────────┬────────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────────┐
│                   sqlrustgo-optimizer                          │
│   ┌──────────────────────────────────────────────────────────┐  │
│   │              CBO Cost Estimator                          │  │
│   │   - Hash Semi Join                                     │  │
│   │   - Hash Anti Join                                    │  │
│   │   - Decorrelation                                     │  │
│   └──────────────────────────────────────────────────────────┘  │
└────────────────────────────┬────────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────────┐
│                   sqlrustgo-executor                           │
│   ┌──────────────────────────────────────────────────────────┐  │
│   │               VolcanoExecutor                            │  │
│   │   - Hash Join / Hash Semi Join / Hash Anti Join        │  │
│   │   - Sort + Merge Join                                  │  │
│   │   - Aggregation (Hash + Sort)                         │  │
│   │   - Table Scan / Index Scan                           │  │
│   └──────────────────────────────────────────────────────────┘  │
│   ┌──────────────────────────────────────────────────────────┐  │
│   │              Storage Engine                             │  │
│   │   - Clustered Index (V311-01)                         │  │
│   │   - Adaptive Hash Index (V311-02)                     │  │
│   │   - Buffer Pool (LRU)                                 │  │
│   │   - WAL + Double-Write Buffer                         │  │
│   └──────────────────────────────────────────────────────────┘  │
└────────────────────────────┬────────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────────┐
│                   sqlrustgo-storage                            │
│   ┌──────────────────────────────────────────────────────────┐  │
│   │               Page Manager                              │  │
│   │   - FileStorage / MemoryStorage                         │  │
│   │   - B+ Tree Index                                     │  │
│   │   - Compression (LZ4/zstd)                            │  │
│   └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## New Components in v3.11.0

### Clustered Index (V311-01)

Primary key data stored with the index structure for fast point lookups.

### Adaptive Hash Index (V311-02)

Automatically built for frequently accessed pages. Reduces B+ tree traversal for hot data.

### Hash Semi Join (V311-15)

Optimized for correlated EXISTS/IN subqueries:
```sql
SELECT * FROM orders o WHERE EXISTS (SELECT 1 FROM customers c WHERE c.id = o.customer_id)
```

### Hash Anti Join (V311-17)

Optimized for NOT EXISTS/NOT IN:
```sql
SELECT * FROM orders o WHERE NOT EXISTS (SELECT 1 FROM invalid_customers ic WHERE ic.id = o.customer_id)
```

## Execution Flow

1. Client sends SQL via MySQL protocol
2. `mysql-server` parses SQL to AST
3. `planner` converts AST to logical plan
4. `optimizer` applies CBO transformations
5. `executor` runs VolcanoIterator model
6. `storage` reads/writes pages via Buffer Pool
7. WAL ensures durability
