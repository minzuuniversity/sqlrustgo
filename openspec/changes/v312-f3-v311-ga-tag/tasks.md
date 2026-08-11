## 1. Verify tag sync

- [x] 1.1 `git ls-remote --tags origin` — confirm v3.11.0-ga present
- [x] 1.2 `git tag -l v3.11.0-ga` — confirm local tag present
- [x] 1.3 Document the 1/5 gap with full mirror inventory

## 2. Update gate output

- [ ] 2.1 Re-run `check_v312_01_blocker_closed.sh` with current state
- [ ] 2.2 Capture new evidence_hash

## 3. Close ISSUE #4026

- [ ] 3.1 Post comment with verification evidence
- [ ] 3.2 Close ISSUE with reference to #3887 condition #4