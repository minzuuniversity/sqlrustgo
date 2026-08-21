CREATE TABLE exec_filter (id INTEGER, val INTEGER);
INSERT INTO exec_filter VALUES (1, 10), (2, 20), (3, 30), (4, 40), (5, 50);
SELECT id FROM exec_filter WHERE val > 25 ORDER BY id;