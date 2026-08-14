# name: create_table_basic
# expect: PASS
CREATE TABLE users (id INT PRIMARY KEY, name TEXT, age INT);
INSERT INTO users VALUES (1, 'Alice', 30);
SELECT * FROM users;
