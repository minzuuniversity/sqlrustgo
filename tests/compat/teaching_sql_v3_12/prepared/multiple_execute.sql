# name: prepared_stmt_multiple_execute
# expect: PASS
# description: Multiple execute of same prepared statement
PREPARE stmt FROM 'SELECT 1 AS num UNION SELECT 2 UNION SELECT 3';
EXECUTE stmt;
EXECUTE stmt;
