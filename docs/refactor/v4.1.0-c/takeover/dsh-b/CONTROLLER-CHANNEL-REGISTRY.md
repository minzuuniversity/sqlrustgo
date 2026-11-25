# CONTROLLER-CHANNEL-REGISTRY — 控制器模型通道登记（LOCAL_ONLY 诚实边界）

- source_agent: DSH-B
- source_run: autonomy-r10（模型通道多次变更后登记）
- timestamp: 2026-11-21T03:10+08:00
- 依据：DSH-B 指令 §6「若规划会话使用云端模型，不得读取受保护源码/完整 Diff 后声称 LOCAL_ONLY」

## 一、本会话控制器通道变更史（运行时事实）

| 时段 | 控制器路由 | 性质 |
|---|---|---|
| R1–R29 | opencode-go/glm-5.3-flash | 云端（OpenCode Go 云模型——协议明令禁用于 SQLRustGo 自动执行） |
| R29–R30 | strata-252/qwen3.8-flash-next-iq3_s | **本地 252:8001** |
| R30+ | minimax-cn/MiniMax-M3.1-Flash-Preview | 云端（MiniMax 云模型） |

## 二、诚实结论（不得含糊）

1. **控制器通道不构成 LOCAL_ONLY**：任一时段控制器为云端时，本会话读取源码、diff、测试日志的行为即处于云端处理范围——因此**本会话任何产出不得被标注为“端到端 LOCAL_ONLY”**。
2. **LOCAL_ONLY 声明的准确边界**：仅指 **SQLRustGo 工作包执行域的模型调用**——即由 DSH-A 调度器经 252:8001 / 250:8021 完成的 worker/reviewer 调用，全部记入 `call_ledger.jsonl`（domain=SQLRUSTGO_LOCAL_ONLY，classification=local）。该声明不覆盖控制器通道。
3. **C01-05/06/07/09 契约测试的产出路径**：测试代码由 DSH-B 控制器起草（控制器通道见上表，**非本地**），经本地编译与真实执行验证；执行证据（cargo 输出、退出码）由本地 cargo 产生，原始日志留档可复跑。**测试代码起草通道不是 LOCAL_ONLY，此点如实标注**，不用于宣称执行域合规。
4. **后续约束（立即生效）**：若属主要求“控制器通道也必须本地化”，则本会话需切回 252:8001 路由（该端点已验证可用，含 PRE/EXEC/POST 身份守卫），否则受保护源码的后续读取须经 DSH-A 的本地执行通道进行，而非在控制器会话中直接读取。

## 三、不受影响的部分

- 执行域合规性：工作包 worker 调用全部本地、零云端、零 fallback（账本可查）。
- 已交付证据的可复现性：全部为真实本地 cargo 执行产物（退出码 + 原始日志 + base SHA），不受控制器通道影响。

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)
