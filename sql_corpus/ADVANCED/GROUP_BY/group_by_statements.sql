-- GROUP BY and HAVING Test Cases
-- Compatibility: MySQL 5.7+

-- === SETUP ===
CREATE TABLE products (id INTEGER PRIMARY KEY, category_id INTEGER, name TEXT, price INTEGER, stock INTEGER, cost INTEGER, created_at TEXT);
CREATE TABLE orders (id INTEGER PRIMARY KEY, user_id INTEGER, status TEXT, total INTEGER, product_id INTEGER, quantity INTEGER, price INTEGER, created_at TEXT, updated_at TEXT, customer_id INTEGER, amount INTEGER, city TEXT, country TEXT, region TEXT);
CREATE TABLE order_items (id INTEGER PRIMARY KEY, order_id INTEGER, product_id INTEGER, category_id INTEGER, quantity INTEGER, price INTEGER);
CREATE TABLE employees (id INTEGER PRIMARY KEY, department_id INTEGER, title TEXT, salary INTEGER, name TEXT, age INTEGER);
CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT, status TEXT, phone TEXT, country TEXT, city TEXT, created_at TEXT);
CREATE TABLE locations (id INTEGER PRIMARY KEY, country TEXT, city TEXT);
CREATE TABLE sales_data (id INTEGER PRIMARY KEY, YEAR INTEGER, MONTH INTEGER, DAY INTEGER, sales INTEGER);
INSERT INTO products VALUES (1, 1, 'Apple', 100, 50, 60, '2024-01-01'), (2, 1, 'Apricot', 80, 30, 50, '2024-01-02'), (3, 2, 'Banana', 30, 100, 20, '2024-01-03'), (4, 2, 'Blueberry', 50, 80, 30, '2024-01-04'), (5, 3, 'Cherry', 200, 20, 150, '2024-01-05'), (6, 1, 'Avocado', 150, 10, 100, '2024-01-06'), (7, 3, 'Coconut', 300, 5, 200, '2024-01-07'), (8, 2, 'Blackberry', 60, 70, 40, '2024-01-08');
INSERT INTO orders VALUES (1, 1, 'completed', 500, 1, 2, 100, '2024-01-01', '2024-01-05', 1, 500, 'NYC', 'US', 'east'), (2, 2, 'pending', 300, 3, 1, 50, '2024-02-01', '2024-02-05', 2, 300, 'LA', 'US', 'west'), (3, 1, 'completed', 700, 5, 1, 200, '2024-03-01', '2024-03-10', 3, 700, 'Chicago', 'US', 'central'), (4, 3, 'shipped', 200, 2, 3, 30, '2024-04-01', '2024-04-02', 4, 200, 'Boston', 'US', 'east'), (5, 2, 'completed', 1000, 7, 1, 300, '2024-05-01', '2024-05-15', 5, 1000, 'Seattle', 'US', 'west'), (6, 1, 'refunded', 150, 4, 1, 50, '2024-06-01', '2024-06-05', 6, 150, 'Miami', 'US', 'south'), (7, 3, 'completed', 800, 6, 1, 150, '2024-07-01', '2024-07-10', 7, 800, 'Denver', 'US', 'central'), (8, 1, 'cancelled', 100, 1, 1, 100, '2024-08-01', '2024-08-02', 8, 100, 'NYC', 'US', 'east');
INSERT INTO order_items VALUES (1, 1, 1, 1, 2, 100), (2, 1, 3, 2, 1, 50), (3, 2, 5, 3, 1, 200), (4, 3, 2, 2, 3, 30), (5, 4, 7, 3, 1, 300), (6, 5, 1, 1, 1, 100);
INSERT INTO employees VALUES (1, 1, 'Manager', 80000, 'Alice', 35), (2, 1, 'Engineer', 60000, 'Bob', 28), (3, 2, 'Manager', 90000, 'Charlie', 40), (4, 2, 'Engineer', 55000, 'Dave', 30), (5, 3, 'Manager', 70000, 'Eve', 33), (6, 3, 'Sales', 45000, 'Frank', 29);
INSERT INTO users VALUES (1, 'Alice', 'alice@example.com', 'active', '555-1234', 'US', 'NYC', '2024-01-01'), (2, 'Bob', 'bob@test.org', 'inactive', '555-5678', 'US', 'LA', '2024-02-01'), (3, 'Charlie', 'c@x.com', 'active', '555-9012', 'UK', 'London', '2024-03-01'), (4, 'Dave', 'dave@hello.io', 'active', '555-3456', 'US', 'Chicago', '2024-04-01'), (5, 'Eve', 'eve@world.net', 'inactive', '555-7890', 'FR', 'Paris', '2024-05-01');
INSERT INTO locations VALUES (1, 'US', 'NYC'), (2, 'US', 'LA'), (3, 'US', 'Chicago'), (4, 'UK', 'London'), (5, 'FR', 'Paris'), (6, 'US', 'Boston'), (7, 'US', 'Miami'), (8, 'US', 'Seattle'), (9, 'US', 'Denver');
INSERT INTO sales_data VALUES (1, 2024, 1, 1, 100), (2, 2024, 1, 2, 200), (3, 2024, 2, 1, 150), (4, 2024, 2, 15, 300), (5, 2024, 3, 1, 250);

-- === CASE: basic_category_count ===
-- === CASE: 001_SELECT_category_id_COUNTstar_FROM_produc ===
SELECT category_id, COUNT(*) FROM products GROUP BY category_id;

-- === CASE: 002_SELECT_status_SUMtotal_AVGtotal_FROM_ord ===
SELECT status, SUM(total), AVG(total) FROM orders GROUP BY status;

-- === CASE: 003_SELECT_YEARcreated_at_AS_year_MONTHcreat ===
SELECT YEAR(created_at) AS year, MONTH(created_at) AS month, SUM(total) FROM orders GROUP BY YEAR(created_at), MONTH(created_at);

-- === CASE: 004_SELECT_category_id_COUNTstar_AS_cnt_AVGp ===
SELECT category_id, COUNT(*) AS cnt, AVG(price) AS avg_price FROM products GROUP BY category_id HAVING cnt > 5;

-- === CASE: 005_SELECT_department_id_COUNTstar_FROM_empl ===
SELECT department_id, COUNT(*) FROM employees GROUP BY department_id HAVING COUNT(*) > 3;

-- === CASE: 006_SELECT_user_id_SUMtotal_AS_total_spent_F ===
SELECT user_id, SUM(total) AS total_spent FROM orders GROUP BY user_id HAVING SUM(total) > 1000 ORDER BY total_spent DESC;

-- === CASE: 007_SELECT_category_id_MAXprice_MINprice_AVG ===
SELECT category_id, MAX(price), MIN(price), AVG(price) FROM products GROUP BY category_id;

-- === CASE: 008_SELECT_LEFTname_1_AS_initial_COUNTstar_F ===
SELECT LEFT(name, 1) AS initial, COUNT(*) FROM users GROUP BY LEFT(name, 1);

-- === CASE: 009_SELECT_DATEcreated_at_AS_date_COUNTstar_ ===
SELECT DATE(created_at) AS date, COUNT(*) AS orders FROM orders GROUP BY DATE(created_at) ORDER BY date;

-- === CASE: 010_SELECT_category_id_SUMquantity_AS_total_ ===
SELECT category_id, SUM(quantity) AS total_sold FROM order_items GROUP BY category_id;

-- === CASE: 011_SELECT_YEARcreated_at_COUNTstar_FROM_use ===
SELECT YEAR(created_at), COUNT(*) FROM users GROUP BY YEAR(created_at);

-- === CASE: 012_SELECT_status_AVGTIMESTAMPDIFFDAY_create ===
SELECT status, AVG(TIMESTAMPDIFF(DAY, created_at, updated_at)) AS avg_days FROM orders GROUP BY status;

-- === CASE: 013_SELECT_category_id_COUNTDISTINCT_user_id ===
SELECT category_id, COUNT(DISTINCT user_id) AS unique_customers FROM orders GROUP BY category_id;

-- === CASE: 014_SELECT_DATE_FORMATcreated_at_'%Y-%m'_AS_ ===
SELECT DATE_FORMAT(created_at, '%Y-%m') AS month, SUM(total) FROM orders GROUP BY DATE_FORMAT(created_at, '%Y-%m');

-- === CASE: 015_SELECT_user_id_COUNTstar_AS_orders_SUMto ===
SELECT user_id, COUNT(*) AS orders, SUM(total) AS total FROM orders GROUP BY user_id HAVING COUNT(*) > 5;

-- === CASE: 016_SELECT_category_SUMrevenue_FROM_SELECT_c ===
SELECT category, SUM(revenue) FROM (SELECT category_id, SUM(price * quantity) AS revenue FROM order_items GROUP BY category_id) t GROUP BY category;

-- === CASE: 017_SELECT_department_id_AVGsalary_FROM_empl ===
SELECT department_id, AVG(salary) FROM employees GROUP BY department_id HAVING AVG(salary) > 50000;

-- === CASE: 018_SELECT_name_COUNTstar_FROM_products_GROU ===
SELECT name, COUNT(*) FROM products GROUP BY name HAVING COUNT(*) > 1;

-- === CASE: 019_SELECT_YEAR_MONTH_DAY_SUMsales_FROM_sale ===
SELECT YEAR, MONTH, DAY, SUM(sales) FROM sales_data GROUP BY YEAR, MONTH, DAY WITH ROLLUP;

-- === CASE: 020_SELECT_region_COUNTDISTINCT_customer_id_ ===
SELECT region, COUNT(DISTINCT customer_id), SUM(amount) FROM orders GROUP BY region;

-- === CASE: 021_SELECT_MONTHNAMEcreated_at_AS_month_COUN ===
SELECT MONTHNAME(created_at) AS month, COUNT(*) FROM orders WHERE YEAR(created_at) = 2024 GROUP BY MONTHNAME(created_at);

-- === CASE: 022_SELECT_user_id_GROUP_CONCATDISTINCT_stat ===
SELECT user_id, GROUP_CONCAT(DISTINCT status ORDER BY created_at SEPARATOR ',') FROM orders GROUP BY user_id;

-- === CASE: 023_SELECT_category_id_SUMprice_star_stock_A ===
SELECT category_id, SUM(price * stock) AS inventory_value FROM products GROUP BY category_id;

-- === CASE: 024_SELECT_WEEKcreated_at_AS_week_num_AVGtot ===
SELECT WEEK(created_at) AS week_num, AVG(total) FROM orders GROUP BY WEEK(created_at);

-- === CASE: 025_SELECT_LEFTemail_POSITION'@'_IN_email_-_ ===
SELECT LEFT(email, POSITION('@' IN email) - 1) AS domain, COUNT(*) FROM users GROUP BY LEFT(email, POSITION('@' IN email) - 1);

-- === CASE: 026_SELECT_price_range_COUNTstar_FROM_SELECT ===
SELECT price_range, COUNT(*) FROM (SELECT CASE WHEN price < 50 THEN 'cheap' WHEN price < 100 THEN 'medium' ELSE 'expensive' END AS price_range FROM products) t GROUP BY price_range;

-- === CASE: 027_SELECT_department_id_COUNTstar_AS_headco ===
SELECT department_id, COUNT(*) AS headcount FROM employees WHERE title LIKE '%Manager%' GROUP BY department_id;

-- === CASE: 028_SELECT_DATEcreated_at_AS_date_COUNTDISTI ===
SELECT DATE(created_at) AS date, COUNT(DISTINCT user_id) FROM orders GROUP BY DATE(created_at) HAVING COUNT(DISTINCT user_id) > 10;

-- === CASE: 029_SELECT_category_id_AVGprice_-_cost_AS_av ===
SELECT category_id, AVG(price - cost) AS avg_profit FROM products GROUP BY category_id;

-- === CASE: 030_SELECT_YEARcreated_at_AS_year_QUARTERcre ===
SELECT YEAR(created_at) AS year, QUARTER(created_at) AS quarter, SUM(total) FROM orders GROUP BY YEAR(created_at), QUARTER(created_at);

-- === CASE: 031_SELECT_country_COUNTDISTINCT_city_FROM_l ===
SELECT country, COUNT(DISTINCT city) FROM locations GROUP BY country HAVING COUNT(DISTINCT city) > 5;

-- === CASE: 032_SELECT_LEFTname_3_COUNTstar_FROM_product ===
SELECT LEFT(name, 3), COUNT(*) FROM products GROUP BY LEFT(name, 3) ORDER BY COUNT(*) DESC LIMIT 5;

-- === CASE: 033_SELECT_user_id_COUNTstar_AS_orders_SUMto ===
SELECT user_id, COUNT(*) AS orders, SUM(total) AS total, CASE WHEN SUM(total) > 5000 THEN 'VIP' WHEN SUM(total) > 1000 THEN 'Regular' ELSE 'New' END AS tier FROM orders GROUP BY user_id;

-- === CASE: 034_SELECT_category_id_SUMquantity_FROM_orde ===
SELECT category_id, SUM(quantity) FROM order_items GROUP BY category_id HAVING SUM(quantity) > 100;

-- === CASE: 035_SELECT_HOURcreated_at_AS_hour_COUNTstar_ ===
SELECT HOUR(created_at) AS hour, COUNT(*) FROM orders GROUP BY HOUR(created_at);

-- === CASE: 036_SELECT_department_id_SUMsalary_FROM_empl ===
SELECT department_id, SUM(salary) FROM employees GROUP BY department_id WITH ROLLUP;

-- === CASE: 037_SELECT_status_COUNTstar_FROM_orders_WHER ===
SELECT status, COUNT(*) FROM orders WHERE created_at > DATE_SUB(NOW(), INTERVAL 30 DAY) GROUP BY status;

-- === CASE: 038_SELECT_DATE_SUBDATEcreated_at_INTERVAL_W ===
SELECT DATE_SUB(DATE(created_at), INTERVAL WEEKDAY(created_at) DAY) AS week_start, SUM(total) FROM orders GROUP BY week_start;

-- === CASE: 039_SELECT_category_id_COUNTstar_FROM_produc ===
SELECT category_id, COUNT(*) FROM products GROUP BY category_id ORDER BY COUNT(*) DESC LIMIT 3;

-- === CASE: 040_SELECT_user_id_COUNTstar_AS_orders_FROM_ ===
SELECT user_id, COUNT(*) AS orders FROM orders WHERE YEAR(created_at) = 2024 GROUP BY user_id HAVING COUNT(*) > 12;

-- === CASE: 041_SELECT_country_AVGage_FROM_users_GROUP_B ===
SELECT country, AVG(age) FROM users GROUP BY country HAVING AVG(age) > 30;

-- === CASE: 042_SELECT_MONTHcreated_at_AS_m_COUNTDISTINC ===
SELECT MONTH(created_at) AS m, COUNT(DISTINCT user_id), SUM(total) FROM orders WHERE YEAR(created_at) = 2024 GROUP BY MONTH(created_at);

-- === CASE: 043_SELECT_LEFTphone_3_AS_area_code_COUNTsta ===
SELECT LEFT(phone, 3) AS area_code, COUNT(*) FROM users GROUP BY LEFT(phone, 3);

-- === CASE: 044_SELECT_product_id_SUMquantity_star_price ===
SELECT product_id, SUM(quantity * price) AS revenue FROM order_items GROUP BY product_id ORDER BY revenue DESC LIMIT 5;

-- === CASE: 045_SELECT_category_id_COUNTstar_AS_products ===
SELECT category_id, COUNT(*) AS products, AVG(price) AS avg_price FROM products WHERE price > 0 GROUP BY category_id;

-- === CASE: 046_SELECT_YEARWEEKcreated_at_AS_yearweek_SU ===
SELECT YEARWEEK(created_at) AS yearweek, SUM(total) FROM orders GROUP BY YEARWEEK(created_at);

-- === CASE: 047_SELECT_user_id_MAXcreated_at_AS_last_ord ===
SELECT user_id, MAX(created_at) AS last_order FROM orders GROUP BY user_id;

-- === CASE: 048_SELECT_status_MINtotal_MAXtotal_AVGtotal ===
SELECT status, MIN(total), MAX(total), AVG(total) FROM orders GROUP BY status;

-- === CASE: 049_SELECT_DATEcreated_at_AS_date_COUNTstar_ ===
SELECT DATE(created_at) AS date, COUNT(*) FROM orders GROUP BY date HAVING COUNT(*) > 5;

-- === CASE: 050_SELECT_LEFTname_1_AS_letter_COUNTDISTINC ===
SELECT LEFT(name, 1) AS letter, COUNT(DISTINCT status) FROM users GROUP BY LEFT(name, 1);

-- === CASE: 051_SELECT_country_COUNTstar_FROM_users_GROU ===
SELECT country, COUNT(*) FROM users GROUP BY country HAVING COUNT(*) BETWEEN 10 AND 100;

-- === CASE: 052_SELECT_department_id_GROUP_CONCATDISTINC ===
SELECT department_id, GROUP_CONCAT(DISTINCT title SEPARATOR '; ') FROM employees GROUP BY department_id;

-- === CASE: 053_SELECT_category_id_SUMstock_AS_total_sto ===
SELECT category_id, SUM(stock) AS total_stock FROM products GROUP BY category_id HAVING SUM(stock) > 1000;

-- === CASE: 054_SELECT_HOURcreated_at_AS_hour_bucket_COU ===
SELECT HOUR(created_at) AS hour_bucket, COUNT(*) FROM orders GROUP BY FLOOR(HOUR(created_at) / 4);

-- === CASE: 055_SELECT_user_id_COUNTstar_FROM_orders_WHE ===
SELECT user_id, COUNT(*) FROM orders WHERE status IN ('completed', 'shipped') GROUP BY user_id HAVING COUNT(*) >= 3;

-- === CASE: 056_SELECT_category_SUMrevenue_FROM_SELECT_c ===
SELECT category, SUM(revenue) FROM (SELECT category_id, SUM(price * quantity) AS revenue FROM order_items GROUP BY category_id) t GROUP BY category;

-- === CASE: 057_SELECT_status_YEARcreated_at_AS_year_COU ===
SELECT status, YEAR(created_at) AS year, COUNT(*) FROM orders GROUP BY status, YEAR(created_at);

-- === CASE: 058_SELECT_user_id_SUMtotal_FROM_orders_WHER ===
SELECT user_id, SUM(total) FROM orders WHERE created_at > '2024-01-01' GROUP BY user_id HAVING SUM(total) > 500;

-- === CASE: 059_SELECT_department_id_COUNTstar_AS_employ ===
SELECT department_id, COUNT(*) AS employees, MAX(salary) AS max_sal FROM employees GROUP BY department_id;

-- === CASE: 060_SELECT_LEFTemail_POSITION'.'_IN_email_-_ ===
SELECT LEFT(email, POSITION('.' IN email) - 1) AS name_part, COUNT(*) FROM users GROUP BY name_part;

-- === CASE: 061_SELECT_DAYOFWEEKcreated_at_AS_dow_AVGtot ===
SELECT DAYOFWEEK(created_at) AS dow, AVG(total) FROM orders GROUP BY dow;

-- === CASE: 062_SELECT_category_id_SUMABSprice_-_AVGpric ===
SELECT category_id, SUM(ABS(price - AVG(price))) FROM products GROUP BY category_id;

-- === CASE: 063_SELECT_YEARcreated_at_MONTHcreated_at_CO ===
SELECT YEAR(created_at), MONTH(created_at), COUNT(*) FROM users GROUP BY YEAR(created_at), MONTH(created_at);

-- === CASE: 064_SELECT_region_SUMtotal_AS_total_FROM_ord ===
SELECT region, SUM(total) AS total FROM orders GROUP BY region ORDER BY total DESC LIMIT 5;

-- === CASE: 065_SELECT_MONTHcreated_at_AS_m_COUNTDISTINC ===
SELECT MONTH(created_at) AS m, COUNT(DISTINCT user_id) FROM orders WHERE YEAR(created_at) = 2024 GROUP BY m;

-- === CASE: 066_SELECT_name_COUNTstar_FROM_products_GROU ===
SELECT name, COUNT(*) FROM products GROUP BY name HAVING COUNT(*) = (SELECT COUNT(*) FROM products GROUP BY name ORDER BY COUNT(*) DESC LIMIT 1);

-- === CASE: 067_SELECT_category_id_COUNTDISTINCT_user_id ===
SELECT category_id, COUNT(DISTINCT user_id) FROM orders GROUP BY category_id;

-- === CASE: 068_SELECT_DATEcreated_at_AS_date_SUMtotal_F ===
SELECT DATE(created_at) AS date, SUM(total) FROM orders WHERE status = 'completed' GROUP BY date;

-- === CASE: 069_SELECT_LEFTphone_3_AS_area_COUNTDISTINCT ===
SELECT LEFT(phone, 3) AS area, COUNT(DISTINCT user_id) FROM users GROUP BY area;

-- === CASE: 070_SELECT_department_id_COUNTstar_FROM_empl ===
SELECT department_id, COUNT(*) FROM employees GROUP BY department_id HAVING COUNT(*) > AVG(COUNT(*)) OVER ();

-- === CASE: 071_SELECT_category_id_SUMquantity_FROM_orde ===
SELECT category_id, SUM(quantity) FROM order_items GROUP BY category_id WITH ROLLUP;

-- === CASE: 072_SELECT_user_id_COUNTstar_AS_cnt_FROM_ord ===
SELECT user_id, COUNT(*) AS cnt FROM orders GROUP BY user_id ORDER BY cnt DESC LIMIT 10;

-- === CASE: 073_SELECT_status_SUMtotal_FROM_orders_WHERE ===
SELECT status, SUM(total) FROM orders WHERE created_at >= DATE_SUB(NOW(), INTERVAL 7 DAY) GROUP BY status;

-- === CASE: 074_SELECT_YEARcreated_at_COUNTstar_FROM_use ===
SELECT YEAR(created_at), COUNT(*) FROM users WHERE YEAR(created_at) >= 2020 GROUP BY YEAR(created_at);

-- === CASE: 075_SELECT_country_AVGprice_FROM_products_GR ===
SELECT country, AVG(price) FROM products GROUP BY country HAVING AVG(price) > 50;

-- === CASE: 076_SELECT_MONTHNAMEcreated_at_AS_month_SUMt ===
SELECT MONTHNAME(created_at) AS month, SUM(total) FROM orders WHERE YEAR(created_at) = 2024 GROUP BY MONTHNAME(created_at);

-- === CASE: 077_SELECT_department_id_COUNTstar_FROM_empl ===
SELECT department_id, COUNT(*) FROM employees GROUP BY department_id HAVING COUNT(*) >= ALL (SELECT COUNT(*) FROM employees GROUP BY department_id);

-- === CASE: 078_SELECT_LEFTname_10_COUNTstar_FROM_produc ===
SELECT LEFT(name, 10), COUNT(*) FROM products GROUP BY LEFT(name, 10);

-- === CASE: 079_SELECT_user_id_SUMtotal_COUNTstar_FROM_o ===
SELECT user_id, SUM(total), COUNT(*) FROM orders GROUP BY user_id HAVING SUM(total) > 0 AND COUNT(*) > 0;

-- === CASE: 080_SELECT_category_id_AVGprice_FROM_product ===
SELECT category_id, AVG(price) FROM products GROUP BY category_id HAVING AVG(price) > (SELECT AVG(price) FROM products);

-- === CASE: 081_SELECT_status_YEARcreated_at_SUMtotal_FR ===
SELECT status, YEAR(created_at), SUM(total) FROM orders GROUP BY status, YEAR(created_at) WITH ROLLUP;

-- === CASE: 082_SELECT_country_COUNTDISTINCT_city_FROM_u ===
SELECT country, COUNT(DISTINCT city) FROM users GROUP BY country ORDER BY COUNT(DISTINCT city) DESC;

-- === CASE: 083_SELECT_DATEcreated_at_AS_date_COUNTDISTI ===
SELECT DATE(created_at) AS date, COUNT(DISTINCT user_id), SUM(total) FROM orders GROUP BY date ORDER BY date DESC LIMIT 30;

-- === CASE: 084_SELECT_category_id_COUNTstar_FROM_produc ===
SELECT category_id, COUNT(*) FROM products GROUP BY category_id HAVING COUNT(*) >= 1;

-- === CASE: 085_SELECT_user_id_COUNTDISTINCT_status_FROM ===
SELECT user_id, COUNT(DISTINCT status) FROM orders GROUP BY user_id HAVING COUNT(DISTINCT status) > 1;

-- === CASE: 086_SELECT_LEFTname_1_SUMstock_FROM_products ===
SELECT LEFT(name, 1), SUM(stock) FROM products GROUP BY LEFT(name, 1);

-- === CASE: 087_SELECT_department_id_AVGsalary_FROM_empl ===
SELECT department_id, AVG(salary) FROM employees GROUP BY department_id HAVING AVG(salary) > 30000 ORDER BY AVG(salary);

-- === CASE: 088_SELECT_YEARWEEKcreated_at_AS_yw_COUNTsta ===
SELECT YEARWEEK(created_at) AS yw, COUNT(*) FROM orders GROUP BY yw ORDER BY yw DESC LIMIT 10;

-- === CASE: 089_SELECT_status_AVGTIMESTAMPDIFFHOUR_creat ===
SELECT status, AVG(TIMESTAMPDIFF(HOUR, created_at, updated_at)) FROM orders GROUP BY status;

-- === CASE: 090_SELECT_user_id_GROUP_CONCATDISTINCT_prod ===
SELECT user_id, GROUP_CONCAT(DISTINCT product_id ORDER BY quantity DESC SEPARATOR ',') FROM order_items GROUP BY user_id;

-- === CASE: 091_SELECT_category_id_SUMprice_star_quantit ===
SELECT category_id, SUM(price * quantity) AS revenue FROM order_items GROUP BY category_id ORDER BY revenue DESC LIMIT 5;

-- === CASE: 092_SELECT_DATE_FORMATcreated_at_'%Y-%m-%d_% ===
SELECT DATE_FORMAT(created_at, '%Y-%m-%d %H:00:00') AS hour, COUNT(*) FROM orders GROUP BY hour;

-- === CASE: 093_SELECT_status_COUNTDISTINCT_user_id_FROM ===
SELECT status, COUNT(DISTINCT user_id) FROM orders GROUP BY status;

-- === CASE: 094_SELECT_LEFTemail_POSITION'@'_IN_email_AS ===
SELECT LEFT(email, POSITION('@' IN email)) AS domain, COUNT(*) FROM users GROUP BY domain;

-- === CASE: 095_SELECT_user_id_SUMtotal_FROM_orders_WHER ===
SELECT user_id, SUM(total) FROM orders WHERE status = 'completed' GROUP BY user_id HAVING SUM(total) > 1000;

-- === CASE: 096_SELECT_department_id_COUNTstar_FROM_empl ===
SELECT department_id, COUNT(*) FROM employees GROUP BY department_id HAVING COUNT(*) < 10;

-- === CASE: 097_SELECT_category_id_AVGprice_AS_avg_MINpr ===
SELECT category_id, AVG(price) AS avg, MIN(price) AS min, MAX(price) AS max FROM products GROUP BY category_id;

-- === CASE: 098_SELECT_YEARcreated_at_COUNTstar_FROM_ord ===
SELECT YEAR(created_at), COUNT(*) FROM orders GROUP BY YEAR(created_at) ORDER BY YEAR(created_at);

-- === CASE: 099_SELECT_status_SUMtotal_FROM_orders_GROUP ===
SELECT status, SUM(total) FROM orders GROUP BY status HAVING SUM(total) > 10000;

-- === CASE: 100_SELECT_user_id_COUNTstar_FROM_orders_WHE ===
SELECT user_id, COUNT(*) FROM orders WHERE created_at > DATE_SUB(NOW(), INTERVAL 30 DAY) GROUP BY user_id;

-- === CASE: 101_SELECT_country_COUNTDISTINCT_email_FROM_ ===
SELECT country, COUNT(DISTINCT email) FROM users GROUP BY country;

-- === CASE: 102_SELECT_category_id_SUMstock_FROM_product ===
SELECT category_id, SUM(stock) FROM products WHERE stock > 0 GROUP BY category_id;

-- === CASE: 103_SELECT_DATEcreated_at_COUNTstar_FROM_ord ===
SELECT DATE(created_at), COUNT(*) FROM orders WHERE status = 'pending' GROUP BY DATE(created_at);

-- === CASE: 104_SELECT_LEFTphone_2_AS_country_code_COUNT ===
SELECT LEFT(phone, 2) AS country_code, COUNT(*) FROM users GROUP BY LEFT(phone, 2);

-- === CASE: 105_SELECT_category_id_COUNTstar_AS_products ===
SELECT category_id, COUNT(*) AS products FROM products GROUP BY category_id ORDER BY products DESC LIMIT 5;

-- === CASE: 106_SELECT_user_id_SUMtotal_FROM_orders_GROU ===
SELECT user_id, SUM(total) FROM orders GROUP BY user_id HAVING SUM(total) BETWEEN 100 AND 1000;

-- === CASE: 107_SELECT_HOURcreated_at_AS_hour_COUNTstar_ ===
SELECT HOUR(created_at) AS hour, COUNT(*) FROM orders GROUP BY hour HAVING COUNT(*) > 10;

-- === CASE: 108_SELECT_department_id_SUMsalary_FROM_empl ===
SELECT department_id, SUM(salary) FROM employees GROUP BY department_id WITH ROLLUP HAVING SUM(salary) IS NOT NULL;

-- === CASE: 109_SELECT_status_COUNTstar_FROM_orders_WHER ===
SELECT status, COUNT(*) FROM orders WHERE YEAR(created_at) = 2024 GROUP BY status;

-- === CASE: 110_SELECT_category_id_COUNTDISTINCT_name_FR ===
SELECT category_id, COUNT(DISTINCT name) FROM products GROUP BY category_id;

-- === CASE: 111_SELECT_LEFTname_1_AS_initial_SUMquantity ===
SELECT LEFT(name, 1) AS initial, SUM(quantity) FROM order_items GROUP BY initial ORDER BY SUM(quantity) DESC;

-- === CASE: 112_SELECT_user_id_MAXtotal_FROM_orders_GROU ===
SELECT user_id, MAX(total) FROM orders GROUP BY user_id HAVING MAX(total) > 500;

-- === CASE: 113_SELECT_region_AVGage_FROM_users_GROUP_BY ===
SELECT region, AVG(age) FROM users GROUP BY region HAVING AVG(age) BETWEEN 25 AND 40;

-- === CASE: 114_SELECT_DATEcreated_at_AS_date_COUNTstar_ ===
SELECT DATE(created_at) AS date, COUNT(*) FROM orders GROUP BY date ORDER BY date DESC LIMIT 7;

-- === CASE: 115_SELECT_category_id_SUMprice_FROM_product ===
SELECT category_id, SUM(price) FROM products GROUP BY category_id HAVING SUM(price) > 1000;

-- === CASE: 116_SELECT_user_id_COUNTstar_FROM_orders_WHE ===
SELECT user_id, COUNT(*) FROM orders WHERE status = 'cancelled' GROUP BY user_id HAVING COUNT(*) > 1;

-- === CASE: 117_SELECT_YEARcreated_at_MONTHcreated_at_CO ===
SELECT YEAR(created_at), MONTH(created_at), COUNT(DISTINCT user_id) FROM orders GROUP BY YEAR(created_at), MONTH(created_at);

-- === CASE: 118_SELECT_department_id_COUNTDISTINCT_title ===
SELECT department_id, COUNT(DISTINCT title) FROM employees GROUP BY department_id;

-- === CASE: 119_SELECT_LEFTemail_POSITION'@'_IN_email_-_ ===
SELECT LEFT(email, POSITION('@' IN email) - 1) AS username, COUNT(*) FROM users GROUP BY username HAVING COUNT(*) > 1;

-- === CASE: 120_SELECT_status_AVGprice_FROM_orders_GROUP ===
SELECT status, AVG(price) FROM orders GROUP BY status;

-- === CASE: 121_SELECT_category_id_COUNTstar_FROM_produc ===
SELECT category_id, COUNT(*) FROM products GROUP BY category_id HAVING COUNT(*) != (SELECT COUNT(*) FROM products GROUP BY category_id ORDER BY COUNT(*) LIMIT 1);

-- === CASE: 122_SELECT_user_id_SUMtotal_FROM_orders_WHER ===
SELECT user_id, SUM(total) FROM orders WHERE created_at >= '2024-01-01' GROUP BY user_id;

-- === CASE: 123_SELECT_YEARcreated_at_MONTHNAMEcreated_a ===
SELECT YEAR(created_at), MONTHNAME(created_at), SUM(total) FROM orders GROUP BY YEAR(created_at), MONTH(created_at);

-- === CASE: 124_SELECT_LEFTname_2_AS_prefix_COUNTstar_FR ===
SELECT LEFT(name, 2) AS prefix, COUNT(*) FROM products GROUP BY LEFT(name, 2);

-- === CASE: 125_SELECT_status_SUMtotal_COUNTstar_FROM_or ===
SELECT status, SUM(total), COUNT(*) FROM orders GROUP BY status HAVING SUM(total) > 5000;

-- === CASE: 126_SELECT_user_id_COUNTDISTINCT_DATEcreated ===
SELECT user_id, COUNT(DISTINCT DATE(created_at)) FROM orders GROUP BY user_id;

-- === CASE: 127_SELECT_department_id_MAXsalary_-_MINsala ===
SELECT department_id, MAX(salary) - MIN(salary) AS salary_range FROM employees GROUP BY department_id;

-- === CASE: 128_SELECT_category_id_GROUP_CONCATDISTINCT_ ===
SELECT category_id, GROUP_CONCAT(DISTINCT name ORDER BY name SEPARATOR ', ') FROM products GROUP BY category_id;

-- === CASE: 129_SELECT_LEFTphone_4_AS_prefix_COUNTstar_F ===
SELECT LEFT(phone, 4) AS prefix, COUNT(*) FROM users GROUP BY prefix;

-- === CASE: 130_SELECT_status_YEARcreated_at_COUNTstar_F ===
SELECT status, YEAR(created_at), COUNT(*) FROM orders GROUP BY status, YEAR(created_at);

-- === CASE: 131_SELECT_user_id_AVGtotal_FROM_orders_WHER ===
SELECT user_id, AVG(total) FROM orders WHERE total > 0 GROUP BY user_id HAVING AVG(total) > 100;

-- === CASE: 132_SELECT_category_id_SUMrevenue_FROM_SELEC ===
SELECT category_id, SUM(revenue) FROM (SELECT category_id, SUM(price * quantity) AS revenue FROM order_items GROUP BY category_id) t GROUP BY category_id;

-- === CASE: 133_SELECT_YEARcreated_at_AS_year_COUNTstar_ ===
SELECT YEAR(created_at) AS year, COUNT(*) FROM users GROUP BY year;

-- === CASE: 134_SELECT_department_id_COUNTstar_FROM_empl ===
SELECT department_id, COUNT(*) FROM employees GROUP BY department_id HAVING COUNT(*) IN (3, 5, 7);

-- === CASE: 135_SELECT_LEFTname_1_COUNTDISTINCT_LEFTname ===
SELECT LEFT(name, 1), COUNT(DISTINCT LEFT(name, 1)) FROM users GROUP BY LEFT(name, 1);

-- === CASE: 136_SELECT_category_id_SUMstock_star_price_A ===
SELECT category_id, SUM(stock * price) AS inventory_value FROM products GROUP BY category_id ORDER BY inventory_value DESC LIMIT 5;

-- === CASE: 137_SELECT_user_id_SUMtotal_FROM_orders_WHER ===
SELECT user_id, SUM(total) FROM orders WHERE status = 'shipped' GROUP BY user_id HAVING SUM(total) > 2000;

-- === CASE: 138_SELECT_status_COUNTDISTINCT_user_id_FROM ===
SELECT status, COUNT(DISTINCT user_id) FROM orders GROUP BY status HAVING COUNT(DISTINCT user_id) > 10;

-- === CASE: 139_SELECT_DATEcreated_at_AS_date_COUNTstar_ ===
SELECT DATE(created_at) AS date, COUNT(*) FROM orders WHERE status = 'completed' GROUP BY date ORDER BY date DESC LIMIT 30;

-- === CASE: 140_SELECT_department_id_AVGsalary_FROM_empl ===
SELECT department_id, AVG(salary) FROM employees GROUP BY department_id HAVING AVG(salary) > (SELECT AVG(salary) FROM employees);

-- === CASE: 141_SELECT_LEFTemail_POSITION'.'_IN_email_AS ===
SELECT LEFT(email, POSITION('.' IN email)) AS domain, COUNT(*) FROM users GROUP BY domain;

-- === CASE: 142_SELECT_category_id_COUNTstar_AS_cnt_SUMs ===
SELECT category_id, COUNT(*) AS cnt, SUM(stock) AS total_stock FROM products GROUP BY category_id HAVING cnt > 0 AND total_stock > 100;

-- === CASE: 143_SELECT_user_id_MAXcreated_at_MINcreated_ ===
SELECT user_id, MAX(created_at), MIN(created_at) FROM orders GROUP BY user_id;

-- === CASE: 144_SELECT_YEARcreated_at_COUNTstar_FROM_ord ===
SELECT YEAR(created_at), COUNT(*) FROM orders WHERE status = 'refunded' GROUP BY YEAR(created_at);

-- === CASE: 145_SELECT_LEFTname_3_AS_key_prefix_COUNTsta ===
SELECT LEFT(name, 3) AS key_prefix, COUNT(*) FROM products GROUP BY key_prefix HAVING COUNT(*) > 1;

-- === CASE: 146_SELECT_region_COUNTDISTINCT_user_id_SUMt ===
SELECT region, COUNT(DISTINCT user_id), SUM(total) FROM orders GROUP BY region ORDER BY SUM(total) DESC;

-- === CASE: 147_SELECT_category_id_AVGprice_FROM_product ===
SELECT category_id, AVG(price) FROM products WHERE price > 0 GROUP BY category_id HAVING AVG(price) BETWEEN 10 AND 100;

-- === CASE: 148_SELECT_status_YEARcreated_at_SUMtotal_FR ===
SELECT status, YEAR(created_at), SUM(total) FROM orders GROUP BY status, YEAR(created_at) ORDER BY YEAR(created_at), status;

-- === CASE: 149_SELECT_user_id_COUNTstar_FROM_orders_WHE ===
SELECT user_id, COUNT(*) FROM orders WHERE YEAR(created_at) = 2024 GROUP BY user_id HAVING COUNT(*) > 3;

-- === CASE: 150_SELECT_department_id_SUMsalary_FROM_empl ===
SELECT department_id, SUM(salary) FROM employees GROUP BY department_id HAVING SUM(salary) > 500000;

-- === CASE: 151_SELECT_LEFTphone_3_AS_area_code_COUNTDIS ===
SELECT LEFT(phone, 3) AS area_code, COUNT(DISTINCT user_id) FROM users GROUP BY LEFT(phone, 3);

-- === CASE: 152_SELECT_DATEcreated_at_AS_date_SUMtotal_F ===
SELECT DATE(created_at) AS date, SUM(total) FROM orders WHERE status = 'pending' GROUP BY date HAVING SUM(total) > 1000;

-- === CASE: 153_SELECT_category_id_COUNTDISTINCT_user_id ===
SELECT category_id, COUNT(DISTINCT user_id) FROM orders GROUP BY category_id ORDER BY COUNT(DISTINCT user_id) DESC LIMIT 5;

-- === CASE: 154_SELECT_user_id_SUMtotal_FROM_orders_GROU ===
SELECT user_id, SUM(total) FROM orders GROUP BY user_id HAVING SUM(total) > 0 ORDER BY SUM(total) DESC LIMIT 10;

-- === CASE: 155_SELECT_YEARcreated_at_MONTHcreated_at_CO ===
SELECT YEAR(created_at), MONTH(created_at), COUNT(*) FROM users GROUP BY YEAR(created_at), MONTH(created_at) ORDER BY YEAR(created_at), MONTH(created_at);

-- === CASE: 156_SELECT_status_COUNTstar_FROM_orders_GROU ===
SELECT status, COUNT(*) FROM orders GROUP BY status HAVING COUNT(*) BETWEEN 10 AND 100;

-- === CASE: 157_SELECT_department_id_MAXsalary_FROM_empl ===
SELECT department_id, MAX(salary) FROM employees GROUP BY department_id HAVING MAX(salary) > 100000;

-- === CASE: 158_SELECT_LEFTname_2_AS_prefix_COUNTDISTINC ===
SELECT LEFT(name, 2) AS prefix, COUNT(DISTINCT category_id) FROM products GROUP BY LEFT(name, 2);

-- === CASE: 159_SELECT_user_id_GROUP_CONCATDISTINCT_stat ===
SELECT user_id, GROUP_CONCAT(DISTINCT status ORDER BY created_at) FROM orders GROUP BY user_id;

-- === CASE: 160_SELECT_DATEcreated_at_AS_date_COUNTDISTI ===
SELECT DATE(created_at) AS date, COUNT(DISTINCT user_id) FROM orders GROUP BY date HAVING COUNT(DISTINCT user_id) > 5;

-- === CASE: 161_SELECT_category_id_SUMprice_star_quantit ===
SELECT category_id, SUM(price * quantity) AS revenue FROM order_items WHERE quantity > 0 GROUP BY category_id;

-- === CASE: 162_SELECT_status_AVGprice_FROM_orders_GROUP ===
SELECT status, AVG(price) FROM orders GROUP BY status HAVING AVG(price) > 50;

-- === CASE: 163_SELECT_YEARcreated_at_COUNTstar_FROM_use ===
SELECT YEAR(created_at), COUNT(*) FROM users GROUP BY YEAR(created_at) HAVING COUNT(*) > 100;

-- === CASE: 164_SELECT_LEFTemail_POSITION'@'_IN_email_-_ ===
SELECT LEFT(email, POSITION('@' IN email) - 1) AS username, COUNT(DISTINCT status) FROM orders GROUP BY username;

-- === CASE: 165_SELECT_department_id_COUNTstar_FROM_empl ===
SELECT department_id, COUNT(*) FROM employees GROUP BY department_id ORDER BY COUNT(*) DESC LIMIT 3;

-- === CASE: 166_SELECT_category_id_SUMstock_FROM_product ===
SELECT category_id, SUM(stock) FROM products GROUP BY category_id HAVING SUM(stock) > AVG(stock) * COUNT(*);

-- === CASE: 167_SELECT_user_id_COUNTstar_FROM_orders_WHE ===
SELECT user_id, COUNT(*) FROM orders WHERE status IN ('pending', 'processing') GROUP BY user_id;

-- === CASE: 168_SELECT_region_COUNTDISTINCT_product_id_F ===
SELECT region, COUNT(DISTINCT product_id) FROM orders GROUP BY region;

-- === CASE: 169_SELECT_status_SUMtotal_FROM_orders_GROUP ===
SELECT status, SUM(total) FROM orders GROUP BY status HAVING SUM(total) > (SELECT AVG(SUM(total)) FROM orders GROUP BY status);

-- === CASE: 170_SELECT_DATEcreated_at_AS_date_COUNTstar_ ===
SELECT DATE(created_at) AS date, COUNT(*) FROM orders GROUP BY date ORDER BY date DESC LIMIT 7;

-- === CASE: 171_SELECT_category_id_COUNTstar_FROM_produc ===
SELECT category_id, COUNT(*) FROM products GROUP BY category_id HAVING COUNT(*) = (SELECT MAX(cnt) FROM (SELECT COUNT(*) AS cnt FROM products GROUP BY category_id) t);

-- === CASE: 172_SELECT_user_id_SUMtotal_FROM_orders_WHER ===
SELECT user_id, SUM(total) FROM orders WHERE created_at >= DATE_SUB(NOW(), INTERVAL 90 DAY) GROUP BY user_id HAVING SUM(total) > 500;

-- === CASE: 173_SELECT_LEFTname_1_COUNTstar_SUMstock_FRO ===
SELECT LEFT(name, 1), COUNT(*), SUM(stock) FROM products GROUP BY LEFT(name, 1);

-- === CASE: 174_SELECT_status_YEARcreated_at_COUNTstar_F ===
SELECT status, YEAR(created_at), COUNT(*) FROM orders GROUP BY status, YEAR(created_at) WITH ROLLUP;

-- === CASE: 175_SELECT_department_id_AVGsalary_FROM_empl ===
SELECT department_id, AVG(salary) FROM employees GROUP BY department_id HAVING AVG(salary) > 40000 ORDER BY AVG(salary) DESC;

-- === CASE: 176_SELECT_category_id_COUNTDISTINCT_user_id ===
SELECT category_id, COUNT(DISTINCT user_id) FROM orders WHERE total > 100 GROUP BY category_id;

-- === CASE: 177_SELECT_user_id_COUNTstar_FROM_orders_WHE ===
SELECT user_id, COUNT(*) FROM orders WHERE DATE(created_at) >= '2024-01-01' GROUP BY user_id HAVING COUNT(*) >= 5;

-- === CASE: 178_SELECT_DATEcreated_at_SUMtotal_FROM_orde ===
SELECT DATE(created_at), SUM(total) FROM orders WHERE status = 'completed' GROUP BY DATE(created_at) ORDER BY DATE(created_at) DESC LIMIT 30;

-- === CASE: 179_SELECT_LEFTphone_2_AS_country_COUNTDISTI ===
SELECT LEFT(phone, 2) AS country, COUNT(DISTINCT user_id) FROM users GROUP BY country;

-- === CASE: 180_SELECT_category_id_SUMquantity_FROM_orde ===
SELECT category_id, SUM(quantity) FROM order_items GROUP BY category_id HAVING SUM(quantity) > 50;

-- === CASE: 181_SELECT_status_COUNTDISTINCT_user_id_FROM ===
SELECT status, COUNT(DISTINCT user_id) FROM orders GROUP BY status HAVING COUNT(DISTINCT user_id) > 10;

-- === CASE: 182_SELECT_user_id_SUMtotal_FROM_orders_GROU ===
SELECT user_id, SUM(total) FROM orders GROUP BY user_id HAVING SUM(total) > (SELECT SUM(total) / COUNT(DISTINCT user_id) FROM orders);

-- === CASE: 183_SELECT_YEARcreated_at_MONTHcreated_at_CO ===
SELECT YEAR(created_at), MONTH(created_at), COUNT(*) FROM orders GROUP BY YEAR(created_at), MONTH(created_at) ORDER BY YEAR(created_at), MONTH(created_at);
