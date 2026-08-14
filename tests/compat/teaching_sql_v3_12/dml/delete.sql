# name: delete_basic
# expect: PASS
CREATE TABLE t (a INT, b TEXT);
INSERT INTO t VALUES (1, 'keep'), (2, 'delete');
DELETE FROM t WHERE a = 2;
SELECT COUNT(*) FROM t;
