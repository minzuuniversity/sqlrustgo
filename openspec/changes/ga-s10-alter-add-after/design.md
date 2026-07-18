## Overview

ALTER TABLE ADD COLUMN 支持 AFTER/FIRST 位置控制。

## Syntax

```sql
ALTER TABLE t ADD COLUMN c INT AFTER col_b;
ALTER TABLE t ADD COLUMN c INT FIRST;
```

## Implementation

### Parser Changes

1. Modify `parse_alter_table_add()` to accept optional `AFTER/FIRST` modifier
2. Store column position in AST

### Catalog Changes

1. Modify column ordering logic in catalog module
2. Update table schema with new column positions
