CREATE TABLE t (id INTEGER, name TEXT);
INSERT INTO t VALUES (1, 'alice'), (2, 'bob');
.mode csv
SELECT id, name FROM t ORDER BY id;