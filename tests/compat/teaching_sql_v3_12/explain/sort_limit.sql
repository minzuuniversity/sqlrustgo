# name: explain_sort_limit
# expect: PASS
# description: EXPLAIN output for ORDER BY with LIMIT
CREATE TABLE users (id INT PRIMARY KEY, name TEXT, age INT);
EXPLAIN SELECT * FROM users ORDER BY age DESC LIMIT 10;
