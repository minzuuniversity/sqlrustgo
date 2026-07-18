# ALTER TABLE ADD COLUMN AFTER/FIRST

## Overview

支持 MySQL 兼容的列位置控制语法。

## Syntax

```sql
-- Add column after existing column
ALTER TABLE t ADD COLUMN c INT AFTER col_b;

-- Add column as first column
ALTER TABLE t ADD COLUMN c INT FIRST;
```

## Parser Changes

Modify `parse_alter_table_add()` in parser to handle `AFTER/FIRST` modifiers.

## Catalog Changes

Update column ordering in catalog when ADD COLUMN is executed.

## Testing

```sql
CREATE TABLE t (a INT, b INT);
ALTER TABLE t ADD COLUMN c INT AFTER a;
-- Expected: a, c, b
```
