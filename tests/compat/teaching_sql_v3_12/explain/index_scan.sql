# name: explain_index_scan
# expect: PASS
# description: EXPLAIN output for index scan on primary key
CREATE TABLE users (id INT PRIMARY KEY, name TEXT, age INT);
CREATE INDEX idx_age ON users(age);
EXPLAIN SELECT * FROM users WHERE age > 25;
