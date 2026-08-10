## 1. Add Value::Json arm to execution_engine.rs

- [x] 1.1 Identify the exact match block (line ~731)
- [x] 1.2 Add `&Value::Json(_) => "JSON".to_string()` arm
- [x] 1.3 Verify cargo build -p sqlrustgo-mysql-server passes

## 2. Verify V312-13 close gates

- [x] 2.1 cargo build -p sqlrustgo-mysql-server: OK
- [x] 2.2 check_anti_fabrication.sh: ERRORS=0 (or only pre-existing)
- [x] 2.3 check_load_data_infile.sh: 4/4 PASS
- [x] 2.4 wire_smoke_mysql_cli: 11/11 PASS
- [x] 2.5 check_arch_invariants.sh: 5/5 PASS

## 3. PR + merge

- [x] 3.1 Create PR with the fix
- [x] 3.2 hermes-z6g4 review + merge
- [x] 3.3 252↔250 sync

## 4. Re-apply closure of #3900

- [x] 4.1 Post final closure evidence comment to #3900
- [x] 4.2 Close #3900
- [x] 4.3 Update #3887 master checklist
