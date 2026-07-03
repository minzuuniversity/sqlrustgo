-- OLTP SOAK Schema — Production-like OLTP tables
-- Inspired by sbtest but designed for realistic production scenarios

DROP TABLE IF EXISTS order_items;
DROP TABLE IF EXISTS orders;
DROP TABLE IF EXISTS customers;

-- Customers: reference table, read-heavy, occasionally updated (address changes)
CREATE TABLE customers (
    c_id INTEGER PRIMARY KEY,
    c_name TEXT NOT NULL,
    c_email TEXT NOT NULL,
    c_phone TEXT,
    c_address TEXT,
    c_city TEXT,
    c_region TEXT,
    c_balance REAL DEFAULT 0.0,
    c_created_at TEXT NOT NULL,
    c_updated_at TEXT NOT NULL,
    UNIQUE KEY idx_email (c_email)
);

-- Orders: primary transaction table, write-heavy, with index lookups
CREATE TABLE orders (
    o_id INTEGER PRIMARY KEY,
    o_customer_id INTEGER NOT NULL,
    o_status TEXT NOT NULL,
    o_total REAL NOT NULL,
    o_discount REAL DEFAULT 0.0,
    o_tax REAL DEFAULT 0.0,
    o_payment_method TEXT,
    o_created_at TEXT NOT NULL,
    o_updated_at TEXT NOT NULL,
    KEY idx_customer_id (o_customer_id),
    KEY idx_status (o_status),
    KEY idx_created_at (o_created_at),
    FOREIGN KEY (o_customer_id) REFERENCES customers(c_id)
);

-- Order items: detail table, read-heavy with occasional updates
CREATE TABLE order_items (
    oi_id INTEGER PRIMARY KEY,
    oi_order_id INTEGER NOT NULL,
    oi_product_id INTEGER NOT NULL,
    oi_quantity INTEGER NOT NULL,
    oi_unit_price REAL NOT NULL,
    oi_discount REAL DEFAULT 0.0,
    oi_subtotal REAL NOT NULL,
    KEY idx_order_id (oi_order_id),
    KEY idx_product_id (oi_product_id),
    FOREIGN KEY (oi_order_id) REFERENCES orders(o_id)
);
