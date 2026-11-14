# G1 TPC-H 基线 provenance 取证（2026-11-14）

> **性质**：执行者取证记录；结论是 **G1 = INCOMPLETE（不是 PASS，也不把 mismatch 说成代码回归）**。
> **专家咨询**（chatGPT-web `web-session-thinking`，2026-11-14；deepseek 本轮 HTTP 500）：
> 推荐顺序 (c) 保持 INCOMPLETE → (a) 做逐 query provenance → (b) 再考虑 per-query baseline；
> **禁止直接 bump aggregate hash**。
> **Owner 2026-11-14 指令**：需裁定的项咨询专家并据此推进；本文是执行依据，Owner 可复核/推翻。

---

## 1. 基线是什么

`tests/tpch_hashes_v380.json`：

| 字段 | 值 |
|---|---|
| `tpc_h_hash_sha256` | `b8854271b6636811c95e254791793973482036d09b1a96b4c5aefda6e8b715c1` |
| `scale_factor` | 0.001 |
| `captured_from` | `545814c3d1 (develop/v3.9.0 @ post Phase 0-2 merge)` |
| `captured_at` | 2026-06-28 |
| 引入提交 | `f3595e7361`（v4.0.0「fix 4 pre-existing DML/storage regressions」，**新加文件**） |

## 2. 2026-11-14 实测

| 夹具 | 结果 |
|---|---|
| 无 `TPCH_DATA_DIR` | `tpch_hash_test` 打印 `TPCH-NOT-RUN: reason=fixture_missing`，rc=0 ⇒ **INCOMPLETE** |
| 当前生成器 SF=0.01 | `profile_mismatch`（baseline 是 0.001）⇒ **INCOMPLETE** |
| 当前生成器 SF=0.001, seed=42 | `tpch_hash_test` **红**：actual `7439ce80589f262b8e3291735fdef9507d8e13d62029708b3682a3d02cf883a3` vs baseline `b8854271…` |
| 当前生成器 SF=0.001, 14 个 seed（0/1/2/3/7/11/42/100/123/777/999/2024/2025/2026） | **没有任何 seed 命中 `b8854271…`**（见 §4 原始读数） |

> 注意：`prepare_tpch_fixture.sh` 对 SF=0.001 会自报
> `NOT-RUN: reason=SCALE_TOO_SMALL — only usable for load-path / row-count checks`。
> 也就是说，用它跑 G1 哈希本身就不满足“标准查询适用”前提；G1 门禁当前只检查
> 8/8 `.tbl` 存在，**不检查 fixture 是否适合标准查询**，这是门禁口径缺口。

## 3. 历史夹具不可恢复（关键证据）

v3.9.0 的 harness 用常量目录：

```rust
// git show v3.9.0:tests/common/tpch_wire_harness.rs
pub const SF001_DIR: &str = "tests/data/tpch-sf001";
```

该目录在 v3.9.0 树里**是 Git LFS 指针**，不是数据：

```
$ git archive v3.9.0 tests/data/tpch-sf001 | tar -x -C /tmp/tpch-v390
$ cat /tmp/tpch-v390/tests/data/tpch-sf001/nation.tbl
version https://git-lfs.github.com/spec/v1
oid sha256:77ad48cafee2eec409d2b01e37d03c59ebf9e38f151e797cb3aa3bc346fbf2f4
size 1592
```

本机 LFS 对象缺失，远端 LFS 端点不可用：

```
$ git lfs ls-files --all | grep tests/data/tpch-sf001
d5f859b464 - tests/data/tpch-sf001/customer.tbl
...
$ git lfs fetch origin v3.9.0 --include="tests/data/tpch-sf001/*"
batch response: Repository or object not found:
  http://192.168.0.252:3000/openclaw/sqlrustgo.git/info/lfs/objects/batch
```

⇒ **无法在干净检出上复现 v3.9.0 的原始 fixture**。

同时，当前生成器在 v3.9.0 之后改过两次：

```
$ git log --oneline --follow -- scripts/gate/generate_tpch_sf.py
009bb985ee fix(test): 测试层判别力（#5449 / #5413 / #5482）
de50610a66 fix(tpch): TPC-H fixture 自包含化与 CI 执行完整性 (#5379)
a6f378d09e docs: add TPC-H multi-scale-factor benchmark system ...
```

（该生成器在 v3.9.0 tag 里甚至不存在 —— `git show v3.9.0:scripts/gate/generate_tpch_sf.py` 报 path not in tag。）

## 4. seed 扫描原始读数

| seed | actual hash |
|---|---|
| 0 | `c74d4a6aeaa20d8ad53725de4e776e8fcaa4c8b6e89fa334f589805e9a1984f0` |
| 1 | `d806b7f3c322800c8c64f77386066873428b699361bd55d41f5dd7e70adbfbcb` |
| 2 | `2f00969b9439d2974494c269e794e135809d5a308430eeae29cd094bf3177429` |
| 3 | `5376d7132e6e82f9e73c999c54fbb7666c80d8f67cf7077380621e17f259f45b` |
| 7 | `c7f116c4c8538753c45862a9793ef1419af35ac6fec05a3ce115712bd873fd5b` |
| 11 | `daa67d95b33675c37fbef7ca8f008fd00aa84cdb427324f7a64721fbdb5dc8b5` |
| 42 | `7439ce80589f262b8e3291735fdef9507d8e13d62029708b3682a3d02cf883a3` |
| 100 | `287a84e2c0af116be45538a28ea5d4d265404a9893011d4ab5b95cf57c823406` |
| 123 | `dcd04b7a596ac46c92e4d90f13d3c260606a707e77ac801ba24519c8d8c86018` |
| 777 | `8d53f2073f9fbc70fc0d5e44b87c49c1251558de5c46304689977269e09f56b7` |
| 999 | `05dfce3763587d33cc2bf334f3719236edb9022021e0a421a55ea1afa210b403` |
| 2024 | `4c5a05f13c7594d0c0015d600f1f8099fd454834e0a0f99c503b56ef39a89c85` |
| 2025 | `1a7fa7d763ce4c7ea8f4dfa326580322f60f9f81ed5cd085c0cffbdb7fec2619` |
| 2026 | `dbcc71fd98cb46009e5fbdf98bf73ebe8cb94b9e1601090b5d4feb478a13e236` |

基线 `b8854271…` 不在其中。

## 5. 结论与可执行选项

**结论**：当前无法区分「代码结果回归」与「fixture 本身换了」。因此：

- G1 不能判 PASS；
- 也不能把 `7439ce80…` 说成「哈希漂移 = 产品回归」（没有同 profile 的可复现夹具）；
- 正确状态是 **INCOMPLETE**，并把「aggregate hash 不一致」作为已知事实保留。

**选项（需 Owner/Reviewer 选一个）**：

| 选项 | 动作 | 代价 | 判定 |
|---|---|---|---|
| A | 恢复 v3.9.0 的 LFS 对象（或从其他镜像/备份取回 `tests/data/tpch-sf001/*.tbl`），用**原始夹具**重跑当前测试 | 需要 LFS 服务或离线备份 | 若命中 `b885…` ⇒ 基线有效，G1 可 PASS；若不命中 ⇒ 代码结果确实变了 |
| B | 用当前生成器建立**新 baseline**，并明确标注「new baseline, not continuity」；先做逐 query provenance（Q1..Q22 行数/结果哈希/与旧 expected 的差异） | 中；需独立审查 | 只能证明“当前实现与当前夹具一致”，不能证明历史连续性 |
| C | 保持 INCOMPLETE，直到 A/B 任一完成 | 零改动 | G1 继续阻断 RC/GA 的 TPC-H 项 |

**禁止**：直接改 `tests/tpch_hashes_v380.json` 让门禁变绿；用 SF=0.01 或换 seed 绕过 profile；把 NOT-RUN 当 PASS。

## 6. 建议的下一步（不越权）

1. 先向 Owner/基础设施确认 v3.9.0 的 LFS 对象是否还有备份（选项 A 最便宜且能判定回归）。
2. 若没有备份，按选项 B 做逐 query provenance：在测试里增加 `--capture-per-query`（输出 Q1..Q22 的 row_count + 结果哈希），把当前结果与 v3.9.0 的 `tests/data/tpch-sf001/expected/*.json` 对比；**expected 也可能来自同一旧夹具**，所以只能作为“差异清单”，不能直接当新基线。
3. 无论 A/B，`tests/tpch_hashes_v380.json` 的变更必须单独 PR，附逐 query 差异与修复 PR 关联，按文件头的 BUMP WORKFLOW 审查。
