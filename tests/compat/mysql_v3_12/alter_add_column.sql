# name: alter_add_column
# expect: PASS
CREATE TABLE t (x INT);
ALTER TABLE t ADD COLUMN y INT DEFAULT 0;
SELECT y FROM t;
