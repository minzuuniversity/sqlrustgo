-- sysbench-style OLTP query workload for sqlrustgo-soak
-- One SQL statement per line. Lines starting with # are comments.
-- This file is used with: sqlrustgo-soak soak --query-file=...
-- Workload: 75% reads / 25% updates (no INSERTs to avoid duplicate-key issues on re-runs)

-- ═══ Reads (point lookups) ═══
SELECT c_id, c_name, c_balance FROM customers WHERE c_id = 1
SELECT c_id, c_name, c_balance FROM customers WHERE c_id = 42
SELECT c_id, c_name, c_balance FROM customers WHERE c_id = 100
SELECT c_id, c_name, c_balance FROM customers WHERE c_id = 500
SELECT c_id, c_name, c_balance FROM customers WHERE c_id = 1000
SELECT c_id, c_name, c_balance FROM customers WHERE c_id = 5000
SELECT c_id, c_name, c_balance FROM customers WHERE c_id = 9999

SELECT o_id, o_customer_id, o_status, o_total FROM orders WHERE o_id = 1
SELECT o_id, o_customer_id, o_status, o_total FROM orders WHERE o_id = 100
SELECT o_id, o_customer_id, o_status, o_total FROM orders WHERE o_id = 1000
SELECT o_id, o_customer_id, o_status, o_total FROM orders WHERE o_id = 10000
SELECT o_id, o_customer_id, o_status, o_total FROM orders WHERE o_id = 25000
SELECT o_id, o_customer_id, o_status, o_total FROM orders WHERE o_id = 49999

-- ═══ Reads (range queries) ═══
SELECT o_id, o_status, o_total FROM orders WHERE o_customer_id = 1 LIMIT 5
SELECT o_id, o_status, o_total FROM orders WHERE o_customer_id = 100 LIMIT 5
SELECT o_id, o_status, o_total FROM orders WHERE o_customer_id = 500 LIMIT 10
SELECT o_id, o_status, o_total FROM orders WHERE o_customer_id = 1000 LIMIT 5
SELECT o_id, o_status, o_total FROM orders WHERE o_customer_id = 5000 LIMIT 5

-- ═══ Reads (aggregates, no GROUP BY columns issue) ═══
SELECT COUNT(*) AS total_customers FROM customers
SELECT COUNT(*) AS total_orders FROM orders
SELECT COUNT(*) AS total_items FROM order_items
SELECT SUM(o_total) FROM orders
SELECT AVG(o_total) FROM orders
SELECT MAX(o_total) FROM orders
SELECT MIN(o_total) FROM orders

-- ═══ Updates (indexed columns, no PK collision) ═══
UPDATE customers SET c_balance = c_balance + 10.50 WHERE c_id = 1
UPDATE customers SET c_balance = c_balance - 5.25 WHERE c_id = 42
UPDATE customers SET c_balance = c_balance + 100.00 WHERE c_id = 100
UPDATE customers SET c_balance = c_balance - 50.00 WHERE c_id = 500
UPDATE customers SET c_balance = c_balance + 25.75 WHERE c_id = 1000
UPDATE customers SET c_balance = c_balance - 10.00 WHERE c_id = 5000
UPDATE customers SET c_balance = c_balance + 5.00 WHERE c_id = 9999

UPDATE orders SET o_status = 'shipped' WHERE o_id = 1
UPDATE orders SET o_status = 'delivered' WHERE o_id = 100
UPDATE orders SET o_status = 'processing' WHERE o_id = 1000
UPDATE orders SET o_status = 'cancelled' WHERE o_id = 10000
UPDATE orders SET o_status = 'pending' WHERE o_id = 25000
UPDATE orders SET o_status = 'shipped' WHERE o_id = 49999
