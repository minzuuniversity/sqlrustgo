CREATE TABLE exec_demo (id INTEGER PRIMARY KEY, val INTEGER, label TEXT);
INSERT INTO exec_demo VALUES (1, 100, 'alpha'), (2, 200, 'beta'), (3, 300, 'gamma');
SELECT id, val, label FROM exec_demo ORDER BY id;