# name: explain_aggregate
# expect: PASS
# description: EXPLAIN output for GROUP BY with aggregate
CREATE TABLE orders (id INT PRIMARY KEY, user_id INT, amount INT);
EXPLAIN SELECT user_id, COUNT(*), SUM(amount) FROM orders GROUP BY user_id;
