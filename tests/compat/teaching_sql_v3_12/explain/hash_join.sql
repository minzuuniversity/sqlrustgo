# name: explain_hash_join
# expect: PASS
# description: EXPLAIN output for hash join plan
CREATE TABLE users (id INT PRIMARY KEY, name TEXT);
CREATE TABLE orders (id INT PRIMARY KEY, user_id INT, amount INT);
EXPLAIN SELECT u.name, o.amount FROM users u JOIN orders o ON u.id = o.user_id;
