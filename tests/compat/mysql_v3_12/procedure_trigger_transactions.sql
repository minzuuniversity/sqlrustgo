# name: procedure_trigger_transactions
# expect: PASS
# V312-55G / Issue #4244: 事务一致性 — trigger 在 BEGIN/ROLLBACK 内不留下部分提交
# 验证 V55D WAL recovery 的事务边界 + V55C AFTER trigger 的回滚一致
# Round-29 (V55G) — fixture 创建于 branch fix/v312-55gh-fixture-and-rollup

CREATE TABLE base_t (id INT PRIMARY KEY, val INT);
CREATE TABLE audit_t (op VARCHAR(8), sid INT, nval INT);

CREATE TRIGGER base_ai AFTER INSERT ON base_t
FOR EACH ROW
BEGIN
  INSERT INTO audit_t VALUES ('I', NEW.id, NEW.val);
END;

CREATE PROCEDURE load_pair(IN a INT, IN b INT)
BEGIN
  INSERT INTO base_t VALUES (a, b);
  INSERT INTO base_t VALUES (a + 100, b + 100);
END;

BEGIN;
CALL load_pair(1, 10);
SELECT COUNT(*) FROM base_t;
SELECT COUNT(*) FROM audit_t;
ROLLBACK;

SELECT COUNT(*) FROM base_t;
SELECT COUNT(*) FROM audit_t;

BEGIN;
CALL load_pair(2, 20);
COMMIT;

SELECT COUNT(*) FROM base_t;
SELECT COUNT(*) FROM audit_t;

DROP PROCEDURE load_pair;
DROP TRIGGER base_ai;
DROP TABLE base_t;
DROP TABLE audit_t;
