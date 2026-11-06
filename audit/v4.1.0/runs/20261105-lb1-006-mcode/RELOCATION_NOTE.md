# run id 撞车说明（合并 20261106）

`20261105-lb1-006` 这个 id 被两条线各自占用：

- `omp-macmini` 线：frozen `07ac01f973…`，保留在 `20261105-lb1-006/`（原 id 未动）
- `mcode-macmini` 线：frozen `c1a5b0ba9d…`，本目录（`20261105-lb1-006-mcode/`）

本目录内容是从 `gitea252/mcode-macmini/governance-kanban-expert-ladder` 逐文件搬来的原文，只改了 manifest 的 `run_id` 一行以与目录名一致；命令、stdout/stderr、evidence_hash 未改动。

那条线的提交信息里写的 `20261106…lb1-006` 现指向本目录。
这是 IL-3（同一议题两条线重复劳动）的现场证据，已记入 `audit/v4.1.0/adjudications/merge-20261106-two-lines.md`。
