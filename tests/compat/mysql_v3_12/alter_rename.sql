# name: alter_rename
# expect: PASS
CREATE TABLE t (x INT);
ALTER TABLE t RENAME TO t2;
SHOW TABLES;
