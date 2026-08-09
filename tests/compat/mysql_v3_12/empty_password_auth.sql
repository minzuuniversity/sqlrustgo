# name: empty_password_auth
# expect: DEFERRED: empty password auth not implemented in compat runner; see v313-01
-- Test empty password authentication behavior
-- This surface is deferred: the compat runner does not support
-- testing auth via empty password connection in v3.12
SELECT 1;
