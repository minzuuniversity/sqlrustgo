CREATE TABLE customers (id INTEGER PRIMARY KEY, name TEXT);
CREATE TABLE orders (id INTEGER PRIMARY KEY, cust_id INTEGER, total INTEGER);
INSERT INTO customers VALUES (1, 'alice'), (2, 'bob'), (3, 'carol');
INSERT INTO orders VALUES (10, 1, 100), (20, 2, 200), (30, 1, 150);
SELECT c.name, o.total FROM customers c INNER JOIN orders o ON c.id = o.cust_id ORDER BY o.total;