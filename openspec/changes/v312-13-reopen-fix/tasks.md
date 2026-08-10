## 1. Bug fix (immediate)

- [x] 1.1 `chmod +x scripts/gate/check_load_data_infile.sh` (the failing 4th check)
- [x] 1.2 Verify `bash scripts/gate/check_load_data_infile.sh` exits 0 (4/4 PASS)
- [x] 1.3 Audit all `scripts/gate/*.sh` for executable bit
- [x] 1.4 Add `chmod +x scripts/gate/*.sh` to CI workflow (defensive)

## 2. Verification

- [x] 2.1 Re-run all 4 documented gates on current HEAD `8ec739185e`
  - check_arch_invariants.sh
  - check_load_data_infile.sh
  - check_anti_fabrication.sh
  - wire_smoke_mysql_cli
- [x] 2.2 Capture real output + evidence_hash for each
- [x] 2.3 Update 4 evidence docs to reflect current HEAD

## 3. Closure evidence

- [x] 3.1 Post final closure evidence comment to #3900 (replaces #87983)
- [x] 3.2 Include real `check_load_data_infile.sh` output (4/4 PASS, exit 0)
- [x] 3.3 Reference PR + commit + evidence_hash
- [x] 3.4 Confirm #3959 cross-reference for deferred items

## 4. Re-apply closure

- [x] 4.1 Create PR with chmod fix + evidence refresh
- [x] 4.2 hermes-z6g4 review + merge
- [x] 4.3 Re-close #3900
- [x] 4.4 Update #3887 master checklist

## 5. Sync 252 ↔ 250

- [x] 5.1 Push PR to 252
- [x] 5.2 Sync to 250 via PR
