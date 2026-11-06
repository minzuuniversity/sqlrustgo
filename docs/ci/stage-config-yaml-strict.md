# stage config 门禁：缺 PyYAML / YAML 损坏时必须 fail loudly

> 背景 issue：#3838　依赖 PR：#3839（先装 PyYAML，再上本改动）

## 一句话

**缺 PyYAML 或 YAML 不可解析 = FATAL（exit 2），绝不再返回空规则集。**

## 为什么

`scripts/gate/stage_config_common.sh` 里的 YAML 读取函数原先长这样：

```bash
python3 - "$cfg" <<'PYEOF' 2>/dev/null || true
...
except Exception:
    raise SystemExit(0)
PYEOF
```

三个问题叠在一起：

1. `2>/dev/null` 吞掉报错，运维看不到任何线索；
2. `|| true` 吞掉退出码；
3. `except Exception: raise SystemExit(0)` —— **解析失败被当成"成功但没有内容"**。

于是 CI 容器 `sqlrustgo-runner:v1` 缺 PyYAML 时（`ModuleNotFoundError: No module named 'yaml'`），
每个读取函数都返回**空规则集**，stage config 门禁变成空转：

- 一致性门禁发现不到任何违规 → 静默 PASS；
- selftest 的 8 个样本 c1..c8 全部「检测不出违规」→ SELFTEST FAIL。

**一个门禁如果分不清"文档干净"和"环境坏了"，它就不是门禁。**

## 三层防线

| 层 | 位置 | 作用 |
|---|---|---|
| `stage_require_yaml [ctx]` | `stage_config_common.sh` | `python3 -c 'import yaml'` 失败 → stderr 打印 FATAL（含安装命令）→ `exit 2` |
| `stage_validate_yaml <file>` | `stage_config_common.sh` | **在调用者 shell 里**完整解析一次文档；失败 → FATAL `exit 2` |
| reader 内 FATAL | 6 个读取函数 + 一致性脚本自带的 3 个本地读取器 | 去掉 `2>/dev/null \|\| true` 与 `raise SystemExit(0)`；解析失败写 stderr 并 `exit 2` |

### 为什么"顶层 validate"这层不能省

读取函数几乎都在进程替换里被调用：

```bash
done < <(stage_gate_entries "$cfg" "$stage")
```

进程替换是**子 shell**——它的非零退出码对父 shell 不可见，父循环只会看到"没有行"，
门禁照样报 `RESULT: PASS`（实测：坏 YAML 时打了 7 条 FATAL，最后仍然 `rc=0 / RESULT: PASS`）。

所以坏配置必须在检查开始之前、于父 shell 里被拒绝。

## 本地怎么跑这几个门禁

```bash
python3 -m pip install pyyaml          # 或 apt-get install -y python3-yaml
python3 -c "import yaml; print(yaml.__version__)"

bash scripts/gate/check_stage_config_consistency.sh --selftest   # 期望 SELFTEST PASS
bash scripts/gate/check_stage_config_consistency.sh              # 期望 RESULT: PASS
bash scripts/gate/check_release_artifacts.sh --version v4.1.0    # 期望 RESULT: PASS
bash scripts/gate/check_stage.sh --version v4.1.0 --dry-run      # 期望 7 PASS, 0 FAIL
```

## 怎么证明门禁**真的在检测**（反向验证）

```bash
# 1) 坏 YAML —— 期望 rc=2 + FATAL（而不是 RESULT: PASS, rc=0）
printf 'framework: [unclosed\n  bad: : :\n' > /tmp/bad.yaml
bash scripts/gate/check_stage_config_consistency.sh --config /tmp/bad.yaml; echo "rc=$?"

# 2) 缺 PyYAML —— 期望 rc=2 + FATAL
mkdir -p /tmp/fake-no-yaml && printf 'raise ImportError("simulated")\n' > /tmp/fake-no-yaml/yaml.py
PYTHONPATH=/tmp/fake-no-yaml bash scripts/gate/check_stage_config_consistency.sh --selftest; echo "rc=$?"
```

## 没有逃生开关

刻意不提供 `STAGE_YAML_STRICT=0` 之类的开关：开关一旦存在，迟早会有人把它设成 0 让红变绿，
门禁就又能被静默关闭——那正是本改动要消灭的东西。

迁移期（开发机 / 镜像缺 PyYAML）的正解是 **先装依赖**：PR #3839 在 CI workflow 里安装 PyYAML，
本改动只负责"装不上就必须报错"。

## 合并顺序（铁律）

1. 先让 **#3839**（装 PyYAML）CI 全绿并合并；
2. 再合本改动。

顺序反了会怎样：CI 容器还没有 PyYAML → 所有依赖 `STAGE_CONFIG.yaml` 的门禁立刻 FATAL exit 2
→ CI 全红，真实问题被淹没。
