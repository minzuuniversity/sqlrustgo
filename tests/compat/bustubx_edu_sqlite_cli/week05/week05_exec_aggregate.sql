CREATE TABLE exec_agg (group_id INTEGER, amount INTEGER);
INSERT INTO exec_agg VALUES (1, 10), (1, 20), (2, 30), (2, 40), (3, 50);
SELECT group_id, COUNT(*), SUM(amount) FROM exec_agg GROUP BY group_id ORDER BY group_id;