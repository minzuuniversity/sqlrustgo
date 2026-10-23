# #5451 —— CI 没有任何 cargo registry 缓存（跨 job 共享缓存）

> **身份**：openclaw-mcode（Gitea `openclaw`）
> **分支**：`fix/ga-mandatory-docs-batch27`（PR #5554）
> **基线**：`develop/v4.1.0` @ `3a3350f660`
> **日期**：2026-10-23

---

## 1. 现象

CI **完全没有** cargo registry 缓存。每个 job 都从全新容器起步，cargo home
里没有 `registry/`，于是每个 job 都要从 `static.crates.io` 重下 ~164 个 crate。

实测（CI run 8348，根包 `test` job 的 Build 步骤）：

```
error: failed to get `chrono` as a dependency of package `evidence-graph v0.1.0`
Caused by: failed to download from `https://index.crates.io/ch/ro/chrono`
Caused by: [28] Timeout was reached (Operation too slow.
           Less than 10 bytes/sec transferred the last 30 seconds)
```

run 8185 的 `test` job 在 **2 分 19 秒**内失败，**一个测试目标都没执行**。

本批自己的 CI 日志（task 7029）实测：

```
Updating crates.io index
Downloaded actix-codec v0.5.4
Downloaded actix-utils v3.0.2
... （164 次 Downloaded）
```

⇒ 164 次下载耗时 **75 秒**（20:25:40 → 20:26:55）。

---

## 2. 根因（实测，不是推测）

### 2.1 runner 镜像**自带**一份填好的 registry

```
$ docker run --rm --entrypoint sh gitea/act_runner:node-rust-v0.6.1 \
    -c 'ls /root/.cargo/registry/src/index.crates.io-*/ | wc -l'
455
```

`/root/.cargo/registry` = **455 个 crate / 356 MB**（cache 43 MB + src 279 MB）。

### 2.2 但 job 容器的 `CARGO_HOME` 指向一个**空目录**

```
$ docker exec <job-container> sh -c 'echo $CARGO_HOME; ls -d $CARGO_HOME/registry'
/opt/cargo-home
ls: /opt/cargo-home/registry: No such file or directory
```

而 `/opt/cargo-home` 里只有一份 `config.toml`：

```toml
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "/opt/vendor"
```

`/opt/vendor` 里只有 **95 个 crate** —— 远不够 455 个。

⇒ **cargo 从头开始下载。镜像里那份 455 crate 对 CI 完全不可见。**

`CARGO_HOME=/opt/cargo-home` 是 **act_runner 在创建 job 容器时注入的**
（不在 runner 容器自身的 env 里）：

```
$ docker inspect <runner> --format '{{range .Config.Env}}{{println .}}{{end}}' | grep -i cargo
（无 CARGO_HOME）
$ docker inspect <job> --format '{{range .Config.Env}}{{println .}}{{end}}' | grep -i cargo
CARGO_HOME=/opt/cargo-home
```

### 2.3 为什么不能直接用镜像里那份

```
$ cargo metadata --offline
error: failed to select a version for the requirement `serde = "^1.0"` (locked to 1.0.229)
candidate versions found which didn't match: 1.0.228
```

镜像那份 registry 是 serde **1.0.228**，锁文件要 **1.0.229**。
⇒ 「把镜像里的 registry 暴露给 cargo」**不够**，必须有一个
**可写、可跨 job 复用**的缓存。

---

## 3. 修法

### 3.1 共享卷

服务器上创建具名卷 `sqlrustgo-cargo-cache`，由 workflow 的
`container.volumes` 挂进每个用 cargo 的 job：

```yaml
container:
  volumes:
    - sqlrustgo-cargo-cache:/opt/cargo-cache
```

五个用 cargo 的 job 全部挂载：`lint-build` / `test` / `test-crates` /
`mutation` / `postcheck`。

### 3.2 拷贝逻辑（一条规则一份实现）

新增 `scripts/ci/cache_cargo_registry.sh`：

* `restore`：把卷里的 `registry` / `git` 拷进 cargo home；
* `save`：把 cargo home 里的 `registry` / `git` 拷回卷。

三个 job 共用同一个卷 ⇒ 拷贝必须串行化。cargo 自己的 `.package-cache`
锁管不到我们的拷贝，所以用 `flock` 单独加一把锁
（`/opt/cargo-cache/.restore.lock`）。

`restore` **永不失败 job**：缓存缺失或不可用都只是「从冷启动」，
不是一个错误。

### 3.3 一个必须写下来的坑

**`restore` 与 `save` 必须用同一个判据定位 cargo home。**

第一版用「第一个已有 `registry/` 的候选目录」：

```bash
for candidate in "${CARGO_HOME:-}" /root/.cargo "$HOME/.cargo"; do
    if [ -d "$candidate/registry" ]; then ... fi
done
```

`restore` 跑在 cargo 下载**之前**（哪个候选都没有 registry），
`save` 跑在**之后**（`$CARGO_HOME` 或 `/root/.cargo` 有 registry）——
于是两个模式解析到**不同**的目录。

后果：CI 里恢复到了 `/root/.cargo` 而不是目标目录；开发机上更糟 ——
**它会把缓存写进用户真实的 `~/.cargo`**。

现在判据是固定的：

```bash
if [ -n "${CARGO_HOME:-}" ] && [ -d "${CARGO_HOME}/registry" ]; then
    printf '%s' "$CARGO_HOME"; return 0
fi
printf '%s' "${CI_CARGO_HOME:-/root/.cargo}"
```

实测（本地往返）：`save` → 换目录 → `restore`，目标目录拿到内容，
且**没有写进本机 `~/.cargo`**（第一次测试污染的那个假文件已删除）。

---

## 4. 预热

卷创建后先预热，让第一个 CI job 就能命中：

```
$ docker run --rm -v sqlrustgo-cargo-cache:/opt/cargo-home \
    -v /tmp/warmsrc:/workspace/openclaw/sqlrustgo -w /workspace/openclaw/sqlrustgo \
    gitea/act_runner:node-rust-v0.6.1 -c 'export CARGO_HOME=/opt/cargo-home; cargo fetch --locked'
--- 缓存统计 ---
829.8M	/opt/cargo-home/registry
575
```

⇒ **829.8 MB / 575 个 crate**（含锁文件要的 serde 1.0.229 等新版本）。

---

## 5. 本批未验证

* 本批**尚未在 CI 上验证**：卷挂载（`container.volumes`）与
  restore/save 的实际效果需要一次真实 CI run 才能确认。
  验证方式是看新 run 的日志里 `Downloaded` 的次数是否从 164 降到 0，
  以及 `cache: restored registry` 是否出现。
* 服务器在本批写作期间**再次不可达**（HTTP 000 + SSH refused）。
  这是基础设施红，**不产出任何 PASS/FAIL 证据**。

---

## 6. 身份

* **Agent**：`openclaw-mcode`（Gitea `openclaw`）
* **协作对象**：`opencode-z440`（Gitea id 13）—— 本批**未触碰**其分支与在飞文件
* **未触碰**：主工作区 `/Volumes/workspace/dev/sqlrustgo`（他人 13 个在飞文件）
