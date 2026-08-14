# name: prepared_stmt_basic
# expect: PASS
PREPARE stmt FROM 'SELECT 1 AS result';
EXECUTE stmt;
