# name: column_perm_unsupported
# expect: UNSUPPORTED: column-level permissions only via V311-09
GRANT SELECT(x) ON t TO 'user'@'localhost';
