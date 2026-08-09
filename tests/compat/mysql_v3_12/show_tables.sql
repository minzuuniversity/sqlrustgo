# name: show_tables
# expect: PASS
CREATE TABLE a (x INT);
CREATE TABLE b (y INT);
CREATE TABLE c (z INT);
DROP TABLE b;
SHOW TABLES;
