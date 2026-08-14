# name: prepared_stmt_param
# expect: PASS
# description: Prepared statement with parameter binding
PREPARE stmt FROM 'SELECT ? AS param';
EXECUTE stmt USING 42;
