# name: alter_table_add_column
# expect: PASS
CREATE TABLE t (x INT);
ALTER TABLE t ADD COLUMN y INT DEFAULT 0;
INSERT INTO t VALUES (1, 10);
SELECT y FROM t;
