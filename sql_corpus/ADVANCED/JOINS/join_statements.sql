-- JOIN Statement Test Cases
-- Compatibility: MySQL 5.7+

-- === CASE: inner_join_section ===
-- === SETUP ===
CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT, owner_id INTEGER, city_id INTEGER);
CREATE TABLE orders (id INTEGER PRIMARY KEY, user_id INTEGER, total INTEGER);
CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT, category_id INTEGER, owner_id INTEGER, price INTEGER);
CREATE TABLE categories (id INTEGER PRIMARY KEY, name TEXT);
CREATE TABLE order_items (id INTEGER PRIMARY KEY, order_id INTEGER, product_id INTEGER, quantity INTEGER);
CREATE TABLE employees (id INTEGER PRIMARY KEY, name TEXT, manager_id INTEGER, department_id INTEGER);
CREATE TABLE table_a (id INTEGER PRIMARY KEY, name TEXT);
CREATE TABLE table_b (id INTEGER PRIMARY KEY, name TEXT);
CREATE TABLE cities (id INTEGER PRIMARY KEY, name TEXT);
INSERT INTO cities VALUES (1, 'NYC'), (2, 'LA'), (3, 'SF');
INSERT INTO users VALUES (1, 'Alice', 'alice@example.com', NULL, 1), (2, 'Bob', 'bob@test.org', NULL, 2), (3, 'Charlie', 'c@x.com', 1, 3);
INSERT INTO orders VALUES (1, 1, 500), (2, 1, 200), (3, 2, 150), (4, 3, 300);
INSERT INTO products VALUES (1, 'Apple', 1, 1, 100), (2, 'Banana', 1, 1, 50), (3, 'Cherry', 2, 2, 200);
INSERT INTO categories VALUES (1, 'Fruit'), (2, 'Veggie');
INSERT INTO order_items VALUES (1, 1, 1, 2), (2, 1, 2, 1), (3, 2, 3, 1);
INSERT INTO employees VALUES (1, 'CEO', NULL, 1), (2, 'Alice', 1, 1), (3, 'Bob', 1, 2), (4, 'Charlie', 2, 1);
INSERT INTO table_a VALUES (1, 'a1'), (2, 'a2');
INSERT INTO table_b VALUES (1, 'b1'), (2, 'b2');



-- === CASE: j_001 ===
SELECT u.id, u.name, o.id as order_id, o.total
FROM users u
INNER JOIN orders o ON u.id = o.user_id;

-- === CASE: j_002 ===
SELECT p.name, c.name as category
FROM products p
INNER JOIN categories c ON p.category_id = c.id;

-- === CASE: j_003 ===
SELECT o.id, o.total, u.name, u.email
FROM orders o
INNER JOIN users u ON o.user_id = u.id
WHERE o.total > 100;

-- === CASE: left_join_section ===

-- === CASE: j_004 ===
SELECT u.id, u.name, o.id as order_id, o.total
FROM users u
LEFT JOIN orders o ON u.id = o.user_id;

-- === CASE: j_005 ===
SELECT u.name, COALESCE(SUM(o.total), 0) as total_spent
FROM users u
LEFT JOIN orders o ON u.id = o.user_id
GROUP BY u.id, u.name;

-- === CASE: j_006 ===
SELECT c.name, p.name
FROM categories c
LEFT JOIN products p ON c.id = p.category_id
WHERE p.id IS NULL;

-- === CASE: right_join_section ===

-- === CASE: j_007 ===
SELECT u.id, u.name, o.id as order_id
FROM users u
RIGHT JOIN orders o ON u.id = o.user_id;

-- === CASE: j_008 ===
SELECT p.name, COALESCE(u.name, 'No Owner') as owner
FROM products p
RIGHT JOIN users u ON p.owner_id = u.id;

-- === CASE: j_009 ===
SELECT c.name, COUNT(p.id) as product_count
FROM categories c
RIGHT JOIN products p ON c.id = p.category_id
GROUP BY c.id, c.name;

-- === CASE: cross_join_section ===

-- === CASE: j_010 ===
SELECT u.name, p.name
FROM users u
CROSS JOIN products p;

-- === CASE: j_011 ===
SELECT a.name as table1, b.name as table2
FROM table_a a
CROSS JOIN table_b b;

-- === CASE: self_join_section ===

-- === CASE: j_012 ===
SELECT e.name as employee, m.name as manager
FROM employees e
LEFT JOIN employees m ON e.manager_id = m.id;

-- === CASE: j_013 ===
SELECT p1.name as product1, p2.name as product2, p1.price
FROM products p1
CROSS JOIN products p2
WHERE p1.id < p2.id AND p1.price = p2.price;

-- === CASE: j_014 ===
SELECT CONCAT(e1.name, ' and ', e2.name) as coworker_pair
FROM employees e1
INNER JOIN employees e2 ON e1.department_id = e2.department_id
WHERE e1.id < e2.id;

-- === CASE: multiple_joins_section ===

-- === CASE: j_015 ===
SELECT u.name, o.id as order_id, p.name as product, oi.quantity
FROM users u
INNER JOIN orders o ON u.id = o.user_id
INNER JOIN order_items oi ON o.id = oi.order_id
INNER JOIN products p ON oi.product_id = p.id;

-- === CASE: j_016 ===
SELECT u.name, o.id, o.total, oi.product_id, p.name
FROM users u
JOIN orders o ON u.id = o.user_id
JOIN order_items oi ON o.id = oi.order_id
JOIN products p ON oi.product_id = p.id
WHERE o.status = 'completed';

-- 7. JOIN with aggregate functions

-- === CASE: j_017 ===
SELECT u.name, COUNT(o.id) as order_count, SUM(o.total) as total_spent
FROM users u
LEFT JOIN orders o ON u.id = o.user_id
GROUP BY u.id, u.name;

-- === CASE: j_018 ===
SELECT c.name, COUNT(p.id) as products, AVG(p.price) as avg_price
FROM categories c
LEFT JOIN products p ON c.id = p.category_id
GROUP BY c.id, c.name;

-- === CASE: j_019 ===
SELECT o.status, COUNT(DISTINCT u.id) as unique_customers, SUM(o.total) as revenue
FROM orders o
INNER JOIN users u ON o.user_id = u.id
GROUP BY o.status;

-- 8. LEFT JOIN with IS NULL

-- === CASE: j_020 ===
SELECT u.name
FROM users u
LEFT JOIN orders o ON u.id = o.user_id
WHERE o.id IS NULL;

-- === CASE: j_021 ===
SELECT c.name
FROM categories c
LEFT JOIN products p ON c.id = p.category_id
WHERE p.id IS NULL;

-- === CASE: j_022 ===
SELECT p.name
FROM products p
LEFT JOIN order_items oi ON p.id = oi.product_id
WHERE oi.id IS NULL;

-- 9. JOIN with ORDER BY

-- === CASE: j_023 ===
SELECT u.name, o.total, o.created_at
FROM users u
INNER JOIN orders o ON u.id = o.user_id
ORDER BY o.total DESC;

-- === CASE: j_024 ===
SELECT p.name, c.name as category
FROM products p
LEFT JOIN categories c ON p.category_id = c.id
ORDER BY c.name, p.name;

-- === CASE: j_025 ===
SELECT o.id, u.name, SUM(oi.quantity * oi.price) as order_value
FROM orders o
INNER JOIN users u ON o.user_id = u.id
INNER JOIN order_items oi ON o.id = oi.order_id
GROUP BY o.id, u.name
ORDER BY order_value DESC
LIMIT 10;

-- 10. JOIN with WHERE conditions

-- === CASE: j_026 ===
SELECT u.name, o.total
FROM users u
INNER JOIN orders o ON u.id = o.user_id
WHERE u.age > 30 AND o.total > 50;

-- === CASE: j_027 ===
SELECT p.name, c.name
FROM products p
INNER JOIN categories c ON p.category_id = c.id
WHERE c.name IN ('Electronics', 'Clothing') AND p.price > 100;

-- === CASE: j_028 ===
SELECT u.name, o.status, o.total
FROM users u
INNER JOIN orders o ON u.id = o.user_id
WHERE (o.status = 'completed' AND o.total > 100) OR (o.status = 'pending' AND o.total > 200);

-- 11. Implicit JOIN (comma syntax)

-- === CASE: j_029 ===
SELECT u.name, o.total
FROM users u, orders o
WHERE u.id = o.user_id;

-- === CASE: j_030 ===
SELECT p.name, c.name
FROM products p, categories c
WHERE p.category_id = c.id AND p.price > 50;

-- 12. JOIN with subqueries

-- === CASE: j_031 ===
SELECT u.name, order_stats.order_count
FROM users u
INNER JOIN (
-- === CASE: j_032 ===
    SELECT user_id, COUNT(*) as order_count
    FROM orders
    GROUP BY user_id
) order_stats ON u.id = order_stats.user_id;

-- === CASE: j_033 ===
SELECT p.name, p.price, avg_stats.avg_price
FROM products p
INNER JOIN (
-- === CASE: j_034 ===
    SELECT category_id, AVG(price) as avg_price
    FROM products
    GROUP BY category_id
) avg_stats ON p.category_id = avg_stats.category_id;

-- 13. LEFT OUTER JOIN (explicit)

-- === CASE: j_035 ===
SELECT u.name, o.id
FROM users u
LEFT OUTER JOIN orders o ON u.id = o.user_id;

-- 14. RIGHT OUTER JOIN (explicit)

-- === CASE: j_036 ===
SELECT u.name, o.id
FROM users u
RIGHT OUTER JOIN orders o ON u.id = o.user_id;

-- 15. FULL OUTER JOIN (emulated with UNION)

-- === CASE: j_037 ===
SELECT u.name, o.id as order_id
FROM users u
LEFT JOIN orders o ON u.id = o.user_id
UNION
-- === CASE: j_038 ===
SELECT u.name, o.id as order_id
FROM users u
RIGHT JOIN orders o ON u.id = o.user_id
WHERE u.id IS NULL;

-- 16. JOIN with DISTINCT

-- === CASE: j_039 ===
SELECT DISTINCT u.name, c.name as city
FROM users u
INNER JOIN cities c ON u.city_id = c.id;

-- === CASE: j_040 ===
SELECT DISTINCT p.name
FROM products p
INNER JOIN order_items oi ON p.id = oi.product_id
INNER JOIN orders o ON oi.order_id = o.id
WHERE o.status = 'completed';

-- 17. JOIN with LIMIT

-- === CASE: j_041 ===
SELECT u.name, o.total
FROM users u
INNER JOIN orders o ON u.id = o.user_id
LIMIT 10;

-- === CASE: j_042 ===
SELECT p.name, c.name
FROM products p
LEFT JOIN categories c ON p.category_id = c.id
LIMIT 20;

-- 18. Natural JOIN

-- === CASE: j_043 ===
SELECT *
FROM users
NATURAL JOIN orders;

-- 19. STRAIGHT_JOIN (MySQL specific)

-- === CASE: j_044 ===
SELECT u.name, o.total
FROM users u
STRAIGHT_JOIN orders o ON u.id = o.user_id;

-- 20. JOIN with expressions

-- === CASE: j_045 ===
SELECT u.name, o.total, o.total * 1.1 as with_tax
FROM users u
INNER JOIN orders o ON u.id = o.user_id;

-- === CASE: j_046 ===
SELECT p.name, p.price, p.price * 0.9 as discounted
FROM products p
INNER JOIN categories c ON p.category_id = c.id
WHERE c.name = 'Sale';

-- 21. Multi-table JOIN (4+ tables)

-- === CASE: j_047 ===
SELECT u.name, o.id, p.name, c.name, oi.quantity
FROM users u
INNER JOIN orders o ON u.id = o.user_id
INNER JOIN order_items oi ON o.id = oi.order_id
INNER JOIN products p ON oi.product_id = p.id
INNER JOIN categories c ON p.category_id = c.id;

-- 22. JOIN with COALESCE

-- === CASE: j_048 ===
SELECT u.name, COALESCE(o.total, 0) as total_spent
FROM users u
LEFT JOIN orders o ON u.id = o.user_id;

-- === CASE: j_049 ===
SELECT p.name, COALESCE(c.name, 'Uncategorized') as category
FROM products p
LEFT JOIN categories c ON p.category_id = c.id;

-- 23. JOIN with CASE

-- === CASE: j_050 ===
SELECT u.name, o.total,
    CASE
        WHEN o.total > 1000 THEN 'High Value'
        WHEN o.total > 500 THEN 'Medium Value'
        WHEN o.total > 0 THEN 'Low Value'
        ELSE 'No Orders'
    END as value_category
FROM users u
LEFT JOIN orders o ON u.id = o.user_id;

-- 24. JOIN with date functions

-- === CASE: j_051 ===
SELECT u.name, o.created_at, DATE_FORMAT(o.created_at, '%Y-%m') as month
FROM users u
INNER JOIN orders o ON u.id = o.user_id
WHERE o.created_at >= DATE_SUB(CURDATE(), INTERVAL 1 YEAR);

-- === CASE: j_052 ===
SELECT p.name, oi.created_at
FROM products p
INNER JOIN order_items oi ON p.id = oi.product_id
WHERE YEAR(oi.created_at) = 2024;

-- 25. JOIN with JSON

-- === CASE: j_053 ===
SELECT u.name, o.id, o.metadata->>'$.shipping_address' as address
FROM users u
INNER JOIN orders o ON u.id = o.user_id;

-- === CASE: j_054 ===
SELECT p.name, p.metadata->>'$.color' as color
FROM products p
LEFT JOIN categories c ON p.category_id = c.id
WHERE p.metadata->>'$.featured' = 'true';

-- 26. Batched JOIN operations

-- === CASE: j_055 ===
SELECT u.id, u.name, COUNT(o.id) as orders
FROM users u
LEFT JOIN orders o ON u.id = o.user_id
WHERE u.id BETWEEN 1 AND 100
GROUP BY u.id, u.name;

-- === CASE: j_056 ===
SELECT p.id, p.name, SUM(oi.quantity) as total_sold
FROM products p
LEFT JOIN order_items oi ON p.id = oi.product_id
WHERE p.category_id IN (1, 2, 3)
GROUP BY p.id, p.name;
