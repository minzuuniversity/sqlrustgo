# v3.12.0 SQL Corpus — All-Targets Report

- source_agent: `minimax`
- source_run: `minimax-v312-19-corpus-79e9c883f9`
- timestamp: `2026-08-10T02:41:03Z`
- branch: `feature/v312-24-impl`
- commit: `79e9c883f98a3a94d8d085d6adac05ce7cdf1843`

| target | cases | pass | fail | skipped | status | evidence_hash | timestamp | source_run |
|--------|-------|------|------|---------|--------|---------------|-----------|------------|
| parser_fixtures | 34 | 34 | 0 | 0 | pass | 541a85b5c3fdd06b34fc1a968efb5417379a3eb7fc70db78a8078568b2abd32f | 2026-08-10T02:41:03Z | minimax-v312-19-corpus-79e9c883f9 |
| sqllogictest_local | 16 | 6 | 10 | 0 | fail | d0cb55d093fc742d25eb1f58f1066d54bb8f8d42e5bd3f320e558e0011ea5b6c | 2026-08-10T02:41:03Z | minimax-v312-19-corpus-79e9c883f9 |
| tpch_sf1 | 22 | 22 | 0 | 0 | pass | 901505da3a061e82d80129bca93157fa5c27115496417fcf3105769a87c203b5 | 2026-08-10T02:41:05Z | minimax-v312-19-corpus-79e9c883f9 |
| tpch_sf10 | 0 | 0 | 0 | 0 | deferred | 667c7a9fe43ec928756040092b7394788c54927e4fd1451d5822ed9d1e69a5a6 | 2026-08-10T02:42:46Z | minimax-v312-19-corpus-79e9c883f9 |
| wire_corpus | 10 | 4 | 2 | 4 | fail | 58d782ba4acf464b25eec229d303d372bb4b065095ebcbb12f3bcf9aae86dab9 | 2026-08-10T02:42:46Z | minimax-v312-19-corpus-79e9c883f9 |
| mysql_compat | 14 | 11 | 1 | 2 | fail | c610c47f6f64d3cbd015ccccb26af3d7e69d1a2db26655796dbcb0a3120360e3 | 2026-08-10T02:43:06Z | minimax-v312-19-corpus-79e9c883f9 |
| v312_13_typed_wrappers | 22 | 21 | 1 | 0 | fail | 8f472b5f624b02aced08e646880a3d4b3b8432bcd78c7736ca6bd3564fb1b1bd | 2026-08-10T02:43:11Z | minimax-v312-19-corpus-79e9c883f9 |
| mysql_wire_protocol_regression | 28 | 28 | 0 | 0 | pass | 669b1a54ef40658886057a2f2d77ed9024b5515de4956fd7074476062a440199 | 2026-08-10T02:43:12Z | minimax-v312-19-corpus-79e9c883f9 |
| e2e_wire_protocol | 46 | 37 | 9 | 0 | fail | 4677005560e07f75a93ccbfac2c746d4c1446f6632b414831fc5faa18d169f9d | 2026-08-10T02:43:15Z | minimax-v312-19-corpus-79e9c883f9 |

---

<!-- report_sha256: 36af2bcfbb028b43ce35d2f6c5a117d84f00b47c68f50002c794386b121c7502 -->
