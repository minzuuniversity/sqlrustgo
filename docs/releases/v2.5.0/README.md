# v2.5.0 发布文档 — 内容缺失说明

> **状态**: [archived] 文档内容不在当前树中
> **最后持有内容的 commit**: `e374c86700` (2026-04-17, "docs: update v2.5.0 documentation and fix symlinks")
> **被移除的 commit**: `c79b39033f` (2026-05-02, hermes-z6g4, "Sync main with release/v2.8.0 content")
> **本文件作用**: 让引用 v2.5.0 的链接解析成功，并记录内容缺失的事实与恢复方式

## 1. 发生了什么

`c79b39033f` 在一次 main 同步中，把 `docs/releases/v2.5.0/` 下的 **21 个文档
（159 KB）全部删除**，同时在原地加入了一个指向 `../../releases/v2.5.0` 的符号链接。

该链接的目标是仓库根的 `releases/` 目录，而根 `.gitignore:122` 明确忽略它：

```
/releases/  # repo-root release artifacts; docs/releases/ intentionally tracked
```

`releases/` 从未入库，所以这个被跟踪的符号链接（mode `120000`）在**任何干净
clone 上都是断链**。这正是 `check_docs_links.sh` 报出的两条断链来源：

| 引用方 | 断链目标 |
|---|---|
| `docs/README.md:122` | `releases/v2.5.0/` |
| `docs/releases/v2.6.0/README.md:112` | `../v2.5.0/` |
| `docs/tutorials/LOGGING_CONFIG.md` | `../../releases/v2.5.0/DEPLOYMENT_GUIDE.md` |

## 2. 恢复方式

内容完整保存在 git 历史里，未丢失：

```bash
# 恢复 21 个文档（会覆盖本 README.md）
git checkout e374c86700 -- docs/releases/v2.5.0/
```

**恢复前请注意**：这批 2026-04 的文档内部有 12 条断链（`oo/README.md` 与
`oo/architecture/ARCHITECTURE_V2.5.md` 引用了已不存在的 `oo/modules/{mvcc,wal,
executor,graph,vector,openclaw}/` 目录，`README.md` 引用了已不存在的
`../../ROADMAP.md`），恢复后 `bash scripts/gate/check_docs_links.sh --all`
会重新报出这些断链，需要一并修。

恢复还与 `docs/governance/DIRECTORY_POLICY.md` §4.2「发布文档 → 永久保留 →
`docs/releases/`」相冲突：该策略要求发布文档永久保留，因此 v2.5.0 的内容移除
本身是一个待决策的治理问题，而不是单纯的链接问题。

## 3. 相关文档

- [v2.4.0 发布文档](../v2.4.0/)
- [v2.6.0 发布文档](../v2.6.0/)
- [发布文档索引](../../README.md)
