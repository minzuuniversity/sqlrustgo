# name: replace_into_complex_unsupported
# expect: PASS
DROP TABLE IF EXISTS replace_into_dst;
DROP TABLE IF EXISTS replace_into_src;
CREATE TABLE replace_into_dst (id INT PRIMARY KEY);
CREATE TABLE replace_into_src (id INT PRIMARY KEY);
INSERT INTO replace_into_src VALUES (1);
REPLACE INTO replace_into_dst SELECT * FROM replace_into_src;