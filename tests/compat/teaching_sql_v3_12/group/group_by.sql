# name: group_by_basic
# expect: PASS
CREATE TABLE orders (id INT PRIMARY KEY, user_id INT, amount INT);
INSERT INTO orders VALUES (1, 1, 100), (2, 1, 200), (3, 2, 150);
SELECT user_id, COUNT(*), SUM(amount) FROM orders GROUP BY user_id;
