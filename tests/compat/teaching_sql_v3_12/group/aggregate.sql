# name: aggregate_functions
# expect: PASS
CREATE TABLE sales (id INT PRIMARY KEY, product TEXT, amount INT);
INSERT INTO sales VALUES (1, 'apple', 100), (2, 'banana', 200), (3, 'apple', 150);
SELECT COUNT(*), SUM(amount), AVG(amount), MIN(amount), MAX(amount) FROM sales;
