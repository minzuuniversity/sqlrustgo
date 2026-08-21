CREATE TABLE products (cat TEXT, price INTEGER);
INSERT INTO products VALUES ('a', 10), ('a', 20), ('b', 30), ('b', 5), ('c', 100);
SELECT cat, AVG(price), MIN(price), MAX(price) FROM products GROUP BY cat ORDER BY cat;