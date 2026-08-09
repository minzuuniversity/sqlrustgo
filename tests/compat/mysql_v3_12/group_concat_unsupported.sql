# name: group_concat_unsupported
# expect: UNSUPPORTED: GROUP_CONCAT not implemented
SELECT GROUP_CONCAT(x) FROM t;
