# name: order_by_asc_desc
# expect: PASS
CREATE TABLE t (a INT, b TEXT);
INSERT INTO t VALUES (3, 'c'), (1, 'a'), (2, 'b');
SELECT * FROM t ORDER BY a ASC;
