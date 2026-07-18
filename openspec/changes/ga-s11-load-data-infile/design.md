## Overview

LOAD DATA LOCAL INFILE 批量导入。

## Syntax

```sql
LOAD DATA LOCAL INFILE '/path/to/file.csv'
INTO TABLE t
FIELDS TERMINATED BY ','
LINES TERMINATED BY '\n';
```

## Implementation

1. **Parser**: Add `LOAD DATA` statement parsing
2. **Executor**: File reading and batch INSERT
3. **Optimization**: Use Change Buffer for bulk inserts
