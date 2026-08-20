CREATE TABLE t (id INTEGER, name TEXT);
INSERT INTO t VALUES (1, 'alice'), (2, 'bob');
.mode list
SELECT id, name FROM t ORDER BY id;