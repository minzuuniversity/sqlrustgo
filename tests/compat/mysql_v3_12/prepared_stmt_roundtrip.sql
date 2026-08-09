# name: prepared_stmt_roundtrip
# expect: PASS
PREPARE stmt FROM 'SELECT 1';
EXECUTE stmt;
