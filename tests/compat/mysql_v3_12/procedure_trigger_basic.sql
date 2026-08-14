# name: procedure_trigger_basic
# expect: PASS
# V312-55G / Issue #4244: Procedure + Trigger 基础功能 smoke
# 验证 CREATE/DROP PROCEDURE + CALL + IN 参数 + CREATE TRIGGER + DROP TRIGGER
# 通过 compat-runner 真实执行端到端（wire protocol, 不是 parser-only）
# Round-29 (V55G) — fixture 创建于 branch fix/v312-55gh-fixture-and-rollup

CREATE TABLE base_t (id INT PRIMARY KEY, val INT);
CREATE TABLE audit_t (op VARCHAR(8), sid INT, nval INT);

CREATE PROCEDURE inc(IN p INT)
BEGIN
  UPDATE base_t SET val = val + p WHERE id = 1;
END;

INSERT INTO base_t VALUES (1, 10);
CALL inc(5);
SELECT val FROM base_t WHERE id = 1;

CREATE TRIGGER base_ai AFTER INSERT ON base_t
FOR EACH ROW
BEGIN
  INSERT INTO audit_t VALUES ('I', NEW.id, NEW.val);
END;

INSERT INTO base_t VALUES (2, 20);
SELECT op, sid, nval FROM audit_t ORDER BY sid;

DROP TRIGGER base_ai;
DROP PROCEDURE inc;
DROP TABLE base_t;
DROP TABLE audit_t;
