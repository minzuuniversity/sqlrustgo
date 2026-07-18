# LOAD DATA INFILE

## Overview

MySQL-compatible LOAD DATA LOCAL INFILE bulk import.

## Syntax

```sql
LOAD DATA LOCAL INFILE '/path/to/file.csv'
INTO TABLE t
FIELDS TERMINATED BY ','
LINES TERMINATED BY '\n';
```

## Parser Changes

Add `LOAD DATA` statement to parser grammar.

## Executor Changes

1. Read file in chunks
2. Batch INSERT for performance
3. Use Change Buffer (V311-12) for optimization

## Performance

Compared to row-by-row INSERT:
- 10x faster for 100k rows
- 100x faster for 1M rows
