# V311-11: F-03 GIS — POINT + ST_WITHIN

## Summary

Implement basic GIS spatial data types `POINT` and `ST_WITHIN` function for v3.11.0.

## Context

F-03 GIS is the largest outstanding `NOT_IMPLEMENTED` item deferred from v3.6.0.
This change targets only the minimum viable GIS feature set to close the gap:
- `POINT` data type
- `ST_WITHIN(a, b)` predicate

## Goals

1. Add `Value::Point(f64, f64)` variant to types crate
2. Parse `POINT(x, y)` text format in SQL
3. Implement `ST_WITHIN(p, polygon)` function
4. Add gis_basic_test.rs integration tests
5. Feature gate: `gis`

## Non-Goals

- Spatial indexes (R-Tree, Grid) — future work
- Other geometry types (LINESTRING, POLYGON, etc.) — future work
- Well-Known Text (WKT) parser beyond POINT
- Geography type (SRID support)

## Design

```
┌─────────────────────────────────────────────────────────┐
│                    GIS Architecture                      │
└─────────────────────────────────────────────────────────┘

  SQL Input                 Parser                 Executor
  ─────────                ───────                ─────────
  POINT(1, 2)    ───▶    Token::POINT   ───▶   Value::Point
                         Token::LPAR             │
                         Token::NUM              ▼
                         Token::COMMA      ST_WITHIN(p, poly)
                         Token::NUM              │
                         Token::RPAR             ▼
                                           Geometric predicate
                                           (bounding box first,
                                            then precise)
```

### Data Type: Value::Point

```rust
// crates/types/src/value.rs
pub enum Value {
    // ... existing variants ...
    /// Geographic point (x, y) = (longitude, latitude)
    Point(f64, f64),
}
```

### ST_WITHIN Algorithm

```
ST_WITHIN(point, polygon)
├── Phase 1: Bounding Box Filter (fast)
│   ├── Extract polygon AABB
│   └── Reject if point outside AABB → return FALSE
└── Phase 2: Point-in-Polygon (if in AABB)
    ├── Ray casting algorithm
    └── Return TRUE if odd intersections
```

## SQL Syntax

```sql
-- Create table with point column
CREATE TABLE locations (
    id INT,
    pos POINT
);

-- Insert point
INSERT INTO locations VALUES (1, POINT(40.7128, -74.0060));

-- Query points within polygon
SELECT * FROM locations
WHERE ST_WITHIN(pos, POLYGON((0,0, 10,0, 10,10, 0,10, 0,0)));
```

### MySQL Compatibility

```sql
-- MySQL also uses POINT(x, y) syntax
SELECT ST_WITHIN(pos, ST_GEOMFROMTEXT('POLYGON(...)'));
```

## Implementation Tasks

### 1. Types Crate
- [ ] Add `Value::Point(f64, f64)` to value.rs
- [ ] Add Hash/PartialEq/Ord for Point variant
- [ ] Add Display/Debug for Point
- [ ] Add value_to_bytes / value_from_bytes for Point

### 2. Parser Crate
- [ ] Add `POINT` keyword token
- [ ] Add `Point(f64, f64)` expression variant
- [ ] Parse `POINT(x, y)` syntax
- [ ] Add ST_WITHIN function expression

### 3. Executor Crate
- [ ] Implement `st_within_point_in_polygon()`
- [ ] Wire ST_WITHIN function call
- [ ] Add GIS function registry

### 4. Tests
- [ ] Add `tests/gis_basic_test.rs`
- [ ] Test POINT parsing
- [ ] Test ST_WITHIN with known results

## Dependencies

- None (self-contained)

## Risks

1. **Performance**: Ray casting is O(n) in polygon edges. Acceptable for MVP.
2. **Precision**: Float comparison. Use epsilon for equality.

## Estimated Hours

- 40h (revised from 80h, MVP scope)
