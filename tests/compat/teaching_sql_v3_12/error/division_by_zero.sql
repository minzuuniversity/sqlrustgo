# name: error_division_by_zero
# expect: PASS (SQLite returns NULL for division by zero)
SELECT 1 / 0;
