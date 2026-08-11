# v3.12.0 SQL Corpus — All-Targets Report

- source_agent: `minimax`
- source_run: `minimax-v312-19-corpus-5f82b1891a`
- timestamp: `2026-08-11T07:01:44Z`
- branch: `develop/v3.12.0`
- commit: `5f82b1891af360a85911f24045cdfcf01fe7f926`

| target | cases | pass | fail | skipped | status | evidence_hash | timestamp | source_run |
|--------|-------|------|------|---------|--------|---------------|-----------|------------|
| parser_fixtures | 34 | 34 | 0 | 0 | pass | 9df61c1c0cb565b49a45c3eb8183df0177e473c95131a036f4c23cf1a6a74fe2 | 2026-08-11T07:01:44Z | minimax-v312-19-corpus-5f82b1891a |
| sqllogictest_local | 8 | 14 | -6 | 0 | fail | 799fce4546d89f61d6f0f963a66e3fcfbdfdaf5b5b96077f340aa2e33c666168 | 2026-08-11T07:01:50Z | minimax-v312-19-corpus-5f82b1891a |
| tpch_sf1 | 22 | 22 | 0 | 0 | pass | a51ab60277b1929b53e43629c3bf399e058fb824ac24d39b9e59ae317b0e6ae4 | 2026-08-11T07:01:52Z | minimax-v312-19-corpus-5f82b1891a |
| tpch_sf10 | 0 | 0 | 0 | 0 | deferred | 667c7a9fe43ec928756040092b7394788c54927e4fd1451d5822ed9d1e69a5a6 | 2026-08-11T07:03:58Z | minimax-v312-19-corpus-5f82b1891a |
| wire_corpus | 10 | 6 | 0 | 4 | pass | e748a3c01c321dd0f49eb4dd9a642da2e97dabe6ed7fba867f5f052c7752b976 | 2026-08-11T07:03:58Z | minimax-v312-19-corpus-5f82b1891a |
| mysql_compat | 14 | 10 | 2 | 2 | fail | 2c2414ad596ec6761d327a0763a32ff417c89037c6d5a6233f78698fd97f0386 | 2026-08-11T07:04:15Z | minimax-v312-19-corpus-5f82b1891a |
| v312_13_typed_wrappers | 22 | 22 | 0 | 0 | pass | f91cc63de6bcccc19d52636bf75c86e3ed8da83aa4bc335cc64e290646dddcea | 2026-08-11T07:04:20Z | minimax-v312-19-corpus-5f82b1891a |
| mysql_wire_protocol_regression | 28 | 28 | 0 | 0 | pass | 639082020d373372ea80d3ff611ef7ddf4ae982fd3beb8f2679a7d1e1c19bdd2 | 2026-08-11T07:04:21Z | minimax-v312-19-corpus-5f82b1891a |
| e2e_wire_protocol | 37 | 37 | 0 | 9 | pass | a419ca5e5f1f2e0c97d1387991fded6c388fba06b6ef6d50aaf3df327715f833 | 2026-08-11T07:04:22Z | minimax-v312-19-corpus-5f82b1891a |

---

<!-- report_sha256: ff20021ab29f96c2a6d179fb6b4b4e72ca0203f540dd63bfe89b0a00d0ec3e34 -->
