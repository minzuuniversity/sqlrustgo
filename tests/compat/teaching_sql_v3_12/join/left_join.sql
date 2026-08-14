# name: left_join
# expect: PASS
CREATE TABLE users (id INT PRIMARY KEY, name TEXT);
CREATE TABLE orders (id INT PRIMARY KEY, user_id INT, amount INT);
INSERT INTO users VALUES (1, 'Alice');
INSERT INTO users VALUES (2, 'Bob');
INSERT INTO orders VALUES (1, 1, 100);
SELECT u.name, o.amount FROM users u LEFT JOIN orders o ON u.id = o.user_id;
