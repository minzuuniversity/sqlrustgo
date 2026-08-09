# name: alter_change_full_syntax_deferred
# expect: DEFERRED: ALTER TABLE CHANGE not fully implemented; see ISSUE #3908
ALTER TABLE t CHANGE COLUMN old_name new_name INT NOT NULL;
