# name: inner_join
# expect: PASS
CREATE TABLE users (id INT PRIMARY KEY, name TEXT);
CREATE TABLE orders (id INT PRIMARY KEY, user_id INT, amount INT);
INSERT INTO users VALUES (1, 'Alice');
INSERT INTO users VALUES (2, 'Bob');
INSERT INTO orders VALUES (1, 1, 100);
INSERT INTO orders VALUES (2, 1, 200);
INSERT INTO orders VALUES (3, 2, 150);
SELECT u.name, o.amount FROM users u JOIN orders o ON u.id = o.user_id;
