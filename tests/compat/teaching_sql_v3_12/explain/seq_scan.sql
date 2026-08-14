# name: explain_seq_scan
# expect: PASS
# description: EXPLAIN output for sequential table scan
CREATE TABLE users (id INT PRIMARY KEY, name TEXT, age INT);
EXPLAIN SELECT * FROM users;
