-- === SETUP ===
CREATE TABLE users (id INT PRIMARY KEY, name TEXT);
INSERT INTO users VALUES (1, 'Alice'), (2, 'Bob'), (3, 'Carol'), (4, 'Dave'), (5, 'Eve');
CREATE TABLE products (id INT PRIMARY KEY, name TEXT);
INSERT INTO products VALUES (10, 'Widget'), (20, 'Gadget'), (30, 'Gizmo'), (40, 'Thingamajig'), (50, 'Doohickey'),
                              (60, 'Whatsit'), (70, 'Whatchamacallit'), (80, 'Contraption'), (90, 'Gimmick'), (100, 'Apparatus');
CREATE TABLE orders (order_id INT PRIMARY KEY, user_id INT, total REAL);
INSERT INTO orders VALUES (101, 1, 50.0), (102, 2, 75.0), (103, 1, 30.0);
CREATE TABLE order_items (order_id INT, product_id INT, qty INT);
INSERT INTO order_items VALUES (101, 10, 2), (102, 20, 1), (103, 30, 4);
CREATE TABLE employees (id INT PRIMARY KEY, name TEXT, manager_id INT);
INSERT INTO employees VALUES (1, 'CEO', NULL), (2, 'CTO', 1), (3, 'CFO', 1), (4, 'Eng1', 2), (5, 'Eng2', 2);

-- === CASE: Cross Join ===
-- EXPECT: 50 rows (5 users x 10 products)
SELECT u.id as user_id, p.id as product_id, u.name, p.name as product_name
FROM users u CROSS JOIN products p;

-- === CASE: Natural Join ===
-- EXPECT: 5 rows
SELECT * FROM orders NATURAL JOIN order_items;

-- === CASE: Join with aggregate ===
-- EXPECT: 6 rows
SELECT u.name, COUNT(o.order_id) as order_count, SUM(o.total) as total_spent
FROM users u
LEFT JOIN orders o ON u.id = o.user_id
GROUP BY u.id, u.name;

-- === CASE: Join with multiple conditions ===
-- EXPECT: 3 rows
SELECT u.name, o.order_id, o.total
FROM users u
JOIN orders o ON u.id = o.user_id AND o.total > 150 AND o.order_date > '2024-01-01';

-- === CASE: Join with GROUP BY and HAVING ===
-- EXPECT: 3 rows
SELECT u.name, COUNT(*) as order_count
FROM users u
JOIN orders o ON u.id = o.user_id
GROUP BY u.id, u.name
HAVING COUNT(*) > 2;

-- === CASE: Join with ORDER BY ===
-- EXPECT: 15 rows
SELECT u.id, u.name, o.order_id, o.total
FROM users u
JOIN orders o ON u.id = o.user_id
ORDER BY u.id, o.total DESC;

-- === CASE: Join with LIMIT ===
-- EXPECT: 5 rows
SELECT u.id, u.name, o.order_id, o.total
FROM users u
JOIN orders o ON u.id = o.user_id
ORDER BY o.total DESC
LIMIT 5;

-- === CASE: Join with DISTINCT ===
-- EXPECT: 4 rows
SELECT DISTINCT u.id, u.name
FROM users u
JOIN orders o ON u.id = o.user_id
WHERE o.total > 100;

-- === CASE: Join with subquery in SELECT ===
-- EXPECT: 5 rows
SELECT u.id, u.name,
  (SELECT COUNT(*) FROM orders WHERE user_id = u.id) as order_count
FROM users u;

-- === CASE: Join with subquery in FROM ===
-- EXPECT: 5 rows
SELECT t.order_count, t.total_spent, u.name
FROM users u
JOIN (
  SELECT user_id, COUNT(*) as order_count, SUM(total) as total_spent
  FROM orders GROUP BY user_id
) t ON u.id = t.user_id;

-- === CASE: Join with UNION ===
-- EXPECT: 8 rows
SELECT u.id, u.name, o.order_id FROM users u
JOIN orders o ON u.id = o.user_id
UNION
SELECT u.id, u.name, NULL as order_id FROM users u WHERE id > 8;

-- === CASE: Join with COALESCE ===
-- EXPECT: 10 rows
SELECT u.id, u.name, COALESCE(o.order_id, 0) as order_id
FROM users u
LEFT JOIN orders o ON u.id = o.user_id;

-- === CASE: Join with CASE in SELECT ===
-- EXPECT: 6 rows
SELECT u.id, u.name,
  CASE WHEN o.order_id IS NULL THEN 'No Orders' ELSE 'Has Orders' END as status
FROM users u
LEFT JOIN orders o ON u.id = o.user_id;

-- === CASE: Join with IN clause ===
-- EXPECT: 4 rows
SELECT u.id, u.name, o.order_id
FROM users u
JOIN orders o ON u.id = o.user_id
WHERE o.total IN (100, 200, 300);

-- === CASE: Join with BETWEEN ===
-- EXPECT: 5 rows
SELECT u.id, u.name, o.order_id, o.total
FROM users u
JOIN orders o ON u.id = o.user_id
WHERE o.total BETWEEN 100 AND 300;

-- === CASE: Join with LIKE ===
-- EXPECT: 3 rows
SELECT u.id, u.name, o.order_id
FROM users u
JOIN orders o ON u.id = o.user_id
WHERE u.email LIKE '%@example.com';

-- === CASE: Join with NULL comparison ===
-- EXPECT: 2 rows
SELECT u.id, u.name, o.order_id
FROM users u
LEFT JOIN orders o ON u.id = o.user_id
WHERE o.order_id IS NULL;

-- === CASE: Three table join ===
-- EXPECT: 12 rows
SELECT u.name, o.order_id, p.name as product_name
FROM users u
JOIN orders o ON u.id = o.user_id
JOIN order_items oi ON o.order_id = oi.order_id
JOIN products p ON oi.product_id = p.id;

-- === CASE: Self join ===
-- EXPECT: 5 rows
SELECT e.name as employee, m.name as manager
FROM employees e
LEFT JOIN employees m ON e.manager_id = m.id;
