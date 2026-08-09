# name: alter_drop_column
# expect: PASS
CREATE TABLE t (x INT, y INT);
INSERT INTO t VALUES (1, 2);
ALTER TABLE t DROP COLUMN y;
SELECT x FROM t;
