## Why

V312-16 delivers controlled SQL features for Window Functions, GIS, and JSON.

## What Changes

### Window Functions Assessment
- Parser: IMPLEMENTED (ROW_NUMBER, RANK, DENSE_RANK, PARTITION BY, ORDER BY)
- Planner: IMPLEMENTED (WindowFunction enum, WindowFrame)
- Executor: IMPLEMENTED (WindowVolcanoExecutor with 10 passing tests)
- **Status**: OPERATIONAL ✅

### GIS Assessment
- ST_Distance, ST_Intersects, GeoJSON: NOT FOUND ❌
- **Status**: NOT IMPLEMENTED

### JSON Assessment
- JSON type support: NOT FOUND in SQL layer ❌
- serde_json used for config/backup manifests (not SQL)
- **Status**: NOT IMPLEMENTED for SQL queries

## Capabilities

### New Capabilities
- `window-functions`: ROW_NUMBER, RANK, DENSE_RANK with PARTITION BY/ORDER BY

## Impact

### Affected Modules
- Parser, Planner, Executor for Window Functions
- GIS: new module needed
- JSON: new type/expressions needed
