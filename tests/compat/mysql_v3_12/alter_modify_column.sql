# name: alter_modify_column
# expect: PASS
CREATE TABLE t (x INT);
INSERT INTO t VALUES (1);
ALTER TABLE t MODIFY COLUMN x BIGINT;
SELECT x FROM t;
