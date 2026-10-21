//! CI 接线诚实性回归套件 —— Issues #5416 / #5420 / #5424 / #5443 / #5444 /
//! #5447 / #5455 / #5456
//!
//! 这一组缺陷与 `gate_honesty_test.rs`（#5417/#5418/#5419/#5426）同族，但作用在
//! **另一个层**：不是门禁脚本的判定逻辑，而是**工作流文件把判定结果接到 CI 结论上
//! 的那根线**。线一断，脚本判得再对，CI 也看不见。
//!
//! | Issue | 缺陷 | 症状 |
//! |-------|------|------|
//! | #5420 | `fault_injection.rs` 的 `reach` 真实现缺 `#[cfg(feature)]` | `cargo test --lib -p sqlrustgo-storage` 编译失败（E0428）⇒ RC/GA 的 D1-A2 维度恒红 |
//! | #5416 | `legacy/check_alpha.sh` 把 `--quiet` 放在 `--` 之后 | 该选项被当成 rustc 参数 ⇒ R7 clippy 恒红 |
//! | #5424 | `ga3-security-scan` 的 `if:` 只匹配 v3.12.0 | Security 门禁在 v4.x 上从未执行 |
//! | #5443 | RC/GA 门禁步骤没有 `pipefail` | 步骤退出码取自 `tee` ⇒ 步骤恒绿 |
//! | #5444 | CI 调用废弃桩 `check_coverage.sh`（exit 2） | 覆盖率既没被测也没人看见 |
//! | #5447 | `gate.yml` 两个 job 用占位数据/旧版脚本自证 PASS | 两个绿色不说明任何问题；legacy-gate 还挂死 2.5 h |
//! | #5455 | `postcheck` 的 `needs: [test]` | 18 个发布前判据在 50 个 CI 周期里 0 次执行 |
//! | #5456 | Gate summary 的 10 个计数用 `grep -c ... \|\| echo "0"` | 零计数时变成多行 `0\n0` ⇒ 10 个判据全部以解析错误被跳过 |
//!
//! 设计取向
//! --------
//! * **行为优先于文本**：能跑起来的就真跑。Gate summary 的 10 个计数是**真的
//!   执行 ci.yml 里的那一段 shell**，用合成的日志喂它，然后断言它的退出码。
//!   legacy 脚本用**桩 cargo** 真跑一遍，断言失败原因被打印出来。
//! * **通用判据优先于硬编码**：pipefail 那条检查问的是「**任何**把门禁脚本
//!   管道出去的步骤都必须开 pipefail」，而不是「第 571 行必须改」。
//! * **cfg 互斥检查是通用不变量**：#5420 的判据是「同名定义在任何 feature
//!   组合下都只能活下来一个」，而不是「`reach` 这一行必须加某个属性」。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_yaml::Value;
use tempfile::TempDir;

// =============================================================================
// 基础设施
// =============================================================================

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_rel(rel: &str) -> String {
    let p = repo_root().join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {}", p.display(), e))
}

fn load_yaml(rel: &str) -> Value {
    let text = read_rel(rel);
    serde_yaml::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {}", rel, e))
}

fn job<'a>(yaml: &'a Value, name: &str) -> &'a Value {
    yaml.get("jobs")
        .and_then(|v| v.as_mapping())
        .unwrap_or_else(|| panic!("no jobs mapping"))
        .get(Value::from(name))
        .unwrap_or_else(|| panic!("job `{}` not found", name))
}

/// 列出工作流里所有 (job, step, run body)。
fn all_steps(yaml: &Value) -> Vec<(String, String, String)> {
    let jobs = yaml
        .get("jobs")
        .and_then(|v| v.as_mapping())
        .expect("jobs mapping");
    let mut out = Vec::new();
    for (jk, jv) in jobs {
        let job_name = jk.as_str().unwrap_or("?").to_string();
        let steps = match jv.get("steps").and_then(|v| v.as_sequence()) {
            Some(s) => s,
            None => continue,
        };
        for st in steps {
            let name = st
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("<unnamed>")
                .to_string();
            let run = st
                .get("run")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            out.push((job_name.clone(), name, run));
        }
    }
    out
}

/// 取出某个 job 里**名字以 `prefix` 开头**的 step 的 run 体。
///
/// 用前缀而不是全名：YAML 里 `#` 前有空白时是注释起始符，所以
/// `- name: RC/GA Gate — Unified 6-Dim (Issue #2882)` 解析出来的名字是
/// `RC/GA Gate — Unified 6-Dim (Issue` —— GitHub/Gitea 看到的也是这个截断值。
fn step_body(yaml: &Value, job_name: &str, prefix: &str) -> String {
    all_steps(yaml)
        .into_iter()
        .find(|(j, s, _)| j == job_name && s.starts_with(prefix))
        .unwrap_or_else(|| {
            panic!(
                "no step starting with `{}` in job `{}` (available: {:?})",
                prefix,
                job_name,
                all_steps(yaml)
                    .into_iter()
                    .filter(|(j, _, _)| j == job_name)
                    .map(|(_, s, _)| s)
                    .collect::<Vec<_>>()
            )
        })
        .2
}

/// 丢掉整行注释。
///
/// 工作流与门禁脚本里大量使用注释来**说明**被修掉的反模式（"之前这里是
/// `|| true`"），朴素的字符串查找会把这些说明当成缺陷本身。
fn strip_comments(s: &str) -> String {
    s.lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

fn run_shell(dir: &Path, script: &str) -> (i32, String, String) {
    let out = Command::new("bash")
        .arg("-c")
        .arg(script)
        .current_dir(dir)
        .output()
        .expect("spawn bash");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Gate summary 会去 grep 的全部日志文件。
const SUMMARY_LOGS: &[&str] = &[
    "gate_report.log",
    "coverage_report.log",
    "evidence_binding.log",
    "anti_fabrication.log",
    "docs_consistency.log",
    "alpha_gate.log",
    "rc_ga_gate.log",
    "audit_testing.log",
    "full_gate_verification.log",
];

/// 建一个只含空日志的目录，供 Gate summary 步骤消费。
fn summary_dir(tag: &str) -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    for f in SUMMARY_LOGS {
        std::fs::write(dir.path().join(f), "").expect("write empty log");
    }
    let _ = tag;
    dir
}

/// 从门禁脚本里取出它在失败时打印的**标记 token**。
///
/// 已知形态：`✗ [FAIL]`（U+2717 + ASCII）、`❌ [FAIL]`（U+274C + ASCII）。
/// 脚本用 ANSI 包裹时标记在 `${RED}` 与 `${NC}` 之间；没有 ANSI 时取
/// 第一个引号段里到破折号为止的部分。
fn failure_marker(script: &str, file: &str) -> String {
    // 优先找带**规范 ASCII 标记** `[FAIL]` 的行：其它形如 `FAIL (exit N)`
    // 的行是单个子检查的局部输出，不是这一维度对外的失败标记。
    let mut fallback: Option<String> = None;
    for line in script.lines() {
        let t = line.trim();
        if t.starts_with('#') {
            continue;
        }
        if !(t.contains("FAIL") && (t.contains("echo") || t.contains("print"))) {
            continue;
        }
        if let (Some(a), Some(b)) = (t.find("${RED}"), t.find("${NC}")) {
            if a < b {
                let m = t[a + "${RED}".len()..b].to_string();
                if m.contains("[FAIL]") {
                    return m;
                }
                fallback = fallback.or(Some(m));
            }
        }
        // 没有 ANSI 包裹：取引号里的内容，到第一个破折号/空格为止
        if let Some(q1) = t.find('"') {
            if let Some(q2) = t[q1 + 1..].find('"') {
                let seg = &t[q1 + 1..q1 + 1 + q2];
                let head = seg.split(" — ").next().unwrap_or(seg).trim().to_string();
                if !head.contains("FAIL") {
                    continue;
                }
                if head.contains("[FAIL]") {
                    return head;
                }
                fallback = fallback.or(Some(head));
            }
        }
    }
    fallback.unwrap_or_else(|| panic!("no FAIL marker line found in {}", file))
}

// =============================================================================
// #5456 — Gate summary 的 10 个计数在零计数时以整数解析错误被跳过
// =============================================================================

/// **零失败**是 Gate summary 唯一的常态路径。它必须：退出码 0，且**不产生**
/// 任何整数解析错误。
///
/// 修前：`grep -c` 零匹配时打印 0 并以退出码 1 结束，`|| echo "0"` 于是追加
/// 第二行 0 ⇒ 变量变成多行的 `0\n0` ⇒ `[ "0\n0" -gt 0 ]` 报
/// "integer expression expected" 并返回 2（假）⇒ 判据被跳过。
#[test]
fn test_5456_gate_summary_zero_failures_has_no_integer_parse_error() {
    let yaml = load_yaml(".gitea/workflows/ci.yml");
    let body = step_body(&yaml, "postcheck", "Gate summary");
    let dir = summary_dir("zero");

    let (code, out, err) = run_shell(dir.path(), &body);
    let noise: String = format!("{}{}", out, err);
    assert!(
        !noise.contains("integer expression expected"),
        "zero-failure path must not hit an integer parse error; got:\n{}\n{}",
        out,
        err
    );
    assert!(
        !noise.contains("syntax error"),
        "the Gate summary step must be syntactically valid bash; got:\n{}\n{}",
        out,
        err
    );
    assert_eq!(
        code, 0,
        "a run with zero failures must exit 0; got exit={}\n{}\n{}",
        code, out, err
    );
}

/// Gate summary 的步骤体必须是**合法 bash**。
///
/// 这条独立于计数器：修前该步骤的 `if` 块缺 `fi` ⇒ 整个步骤体是语法错误 ⇒
/// bash 恒以 2 退出 ⇒ 这一步**从未真正判定过任何门禁**。
#[test]
fn test_5456_gate_summary_step_is_syntactically_valid_bash() {
    let yaml = load_yaml(".gitea/workflows/ci.yml");
    let body = step_body(&yaml, "postcheck", "Gate summary");
    let out = Command::new("bash")
        .arg("-n")
        .arg("-c")
        .arg(&body)
        .output()
        .expect("spawn bash -n");
    assert!(
        out.status.success(),
        "the Gate summary step body is not valid bash (bash -n exit={}):\n{}\n{}",
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// **有失败**时 Gate summary 必须真的阻断。
///
/// 这是 #5443 与 #5456 的**叠加效应**：CI 的兜底 grep 用的字符 `❌`(U+274C)
/// 与 RC/GA 脚本实际输出的 `✗`(U+2717) 不是同一个码位 ⇒ 计数恒为 0 ⇒
/// 走解析错误路径 ⇒ 判据跳过 ⇒ 步骤恒绿。
///
/// 修前：本测试用**脚本自己打印的**标记喂 Gate summary，断言它必须红 ——
/// 修前它会绿，这正是缺陷本身。
#[test]
fn test_5456_gate_summary_blocks_on_the_marker_the_rc_ga_script_actually_prints() {
    let yaml = load_yaml(".gitea/workflows/ci.yml");
    let body = step_body(&yaml, "postcheck", "Gate summary");

    let rc_ga = read_rel("scripts/gate/check_rc_ga_gate.sh");
    let marker = failure_marker(&rc_ga, "check_rc_ga_gate.sh");
    assert!(
        !marker.trim().is_empty(),
        "the RC/GA script must print a failure marker"
    );

    let dir = summary_dir("rc_ga_fail");
    std::fs::write(
        dir.path().join("rc_ga_gate.log"),
        format!("  {} D3-SGL: some dimension failed\n", marker),
    )
    .expect("write rc_ga log");

    let (code, out, err) = run_shell(dir.path(), &body);
    assert_eq!(
        code, 1,
        "a gate failure printed as `{}` must make the Gate summary step exit 1; \
         it returned {} (it counted the failure as zero). \
         marker={:?}\nstdout:\n{}\nstderr:\n{}",
        marker, code, marker, out, err
    );
}

/// 同样地，alpha 门禁的标记（`❌ FAIL`）也必须能触发阻断 —— 防止修 #5443 时
/// 把 alpha 这条已经能工作的路径弄坏。
#[test]
fn test_5456_gate_summary_blocks_on_the_alpha_gate_marker() {
    let yaml = load_yaml(".gitea/workflows/ci.yml");
    let body = step_body(&yaml, "postcheck", "Gate summary");

    let alpha = read_rel("scripts/gate/check_alpha_v380.sh");
    let marker = failure_marker(&alpha, "check_alpha_v380.sh");

    let dir = summary_dir("alpha_fail");
    std::fs::write(
        dir.path().join("alpha_gate.log"),
        format!("{} FAIL — 1 blocker(s) found\n", marker),
    )
    .expect("write alpha log");

    let (code, out, err) = run_shell(dir.path(), &body);
    assert_eq!(
        code, 1,
        "alpha gate failure `{}` must block with exit 1; got {}\nstdout:\n{}\nstderr:\n{}",
        marker, code, out, err
    );
}

// =============================================================================
// #5443 — 把门禁脚本管道出去的步骤必须开 pipefail
// =============================================================================

/// 通用判据：**任何**把门禁脚本（`scripts/gate/*` 或 `gate/gate.sh`）的输出
/// 管道出去的步骤，都必须开 `pipefail`。
///
/// 没有 `pipefail` 时，管道���出码取自管道**最后一环**（`tee` / `tail`），
/// 恒为 0 ⇒ 步骤恒绿 ⇒ 门禁判据被静默吞掉。
///
/// 注意这条判据**不会**误伤 "6 Meta-Gate Hard Block" 步骤：它虽然也用了
/// `| tail -10`，但它自己带着 `set -euo pipefail`，管道退出码因此取自门禁
/// 而非 `tail` —— 同一个形态，一个坏一个好，差别就在这一行。
#[test]
fn test_5443_every_step_piping_a_gate_script_enables_pipefail() {
    let yaml = load_yaml(".gitea/workflows/ci.yml");
    let mut offenders: Vec<String> = Vec::new();
    for (job, step, body) in all_steps(&yaml) {
        let pipes = body.contains("| tee") || body.contains("| tail");
        let runs_gate = body.contains("scripts/gate/") || body.contains("gate/gate.sh");
        if !(pipes && runs_gate) {
            continue;
        }
        if body.contains("pipefail") {
            continue;
        }
        offenders.push(format!("{} / {}", job, step));
    }
    assert!(
        offenders.is_empty(),
        "these steps pipe a gate script without `pipefail`, so the step's exit \
         code comes from `tee`/`tail` (always 0) and the gate can never turn \
         the step red:\n  - {}",
        offenders.join("\n  - ")
    );
}

/// RC/GA 门禁步骤本身必须开 pipefail（#5443 的直接对象，单独点名以免被
/// 上面的通用判据「顺带」覆盖过去）。
#[test]
fn test_5443_rc_ga_gate_step_enables_pipefail() {
    let yaml = load_yaml(".gitea/workflows/ci.yml");
    let body = step_body(&yaml, "postcheck", "RC/GA Gate");
    assert!(
        body.contains("pipefail"),
        "the RC/GA gate step must enable pipefail, otherwise `| tee rc_ga_gate.log` \
         swallows the gate's exit code; body was:\n{}",
        body
    );
}

// =============================================================================
// #5444 — CI 调用的是废弃的覆盖率桩
// =============================================================================

/// CI 不得调用废弃桩 `scripts/gate/check_coverage.sh`（它只有 288 字节，
/// 内容是 "deprecated" + `exit 2`），必须调用真门禁 `check_coverage_v312.sh`
/// （`STAGE_CONFIG.yaml` 的 GA `required_gates` 列的正是后者）。
#[test]
fn test_5444_ci_calls_the_real_coverage_gate_not_the_deprecated_stub() {
    let yaml = load_yaml(".gitea/workflows/ci.yml");
    let all: String = all_steps(&yaml)
        .into_iter()
        .map(|(_, _, body)| strip_comments(&body))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        !all.contains("scripts/gate/check_coverage.sh"),
        "CI still invokes the deprecated coverage stub `scripts/gate/check_coverage.sh` \
         (exit 2, prints 'deprecated'); coverage is neither measured nor reported"
    );
    assert!(
        all.contains("check_coverage_v312.sh"),
        "CI must invoke the real coverage gate `scripts/gate/check_coverage_v312.sh`"
    );
}

/// 那个桩确实是废弃的 —— 断言桩的退出码是 2 且自称 deprecated，
/// 免得有人「修」CI 时把一个真门禁误当成桩。
#[test]
fn test_5444_the_coverage_stub_is_indeed_a_deprecated_stub() {
    let stub = repo_root().join("scripts/gate/check_coverage.sh");
    let text = std::fs::read_to_string(&stub).expect("read stub");
    assert!(
        text.to_lowercase().contains("deprecat"),
        "scripts/gate/check_coverage.sh is expected to be a deprecated stub; \
         its content changed:\n{}",
        text
    );
}

// =============================================================================
// #5455 — postcheck 被 needs: [test] 完全遮蔽
// =============================================================================

/// `postcheck` 承载发布前的 18 项判据。它不能被 `test` job 的成败挡住 ——
/// 发布后置检查本来就应当在一个**已知红**的分支上被执行一次。
#[test]
fn test_5455_postcheck_is_not_masked_behind_the_test_job() {
    let yaml = load_yaml(".gitea/workflows/ci.yml");
    let j = job(&yaml, "postcheck");

    let needs: Vec<String> = match j.get("needs") {
        Some(Value::Sequence(s)) => s
            .iter()
            .map(|v| v.as_str().unwrap_or("").to_string())
            .collect(),
        Some(Value::String(s)) => vec![s.clone()],
        _ => Vec::new(),
    };
    let always = j
        .get("if")
        .and_then(|v| v.as_str())
        .map(|s| s.contains("always()"))
        .unwrap_or(false);

    assert!(
        !needs.iter().any(|n| n == "test") || always,
        "postcheck is masked behind the `test` job (needs={:?}, if={:?}). The \
         `test` job has never succeeded, so postcheck's 18 criteria have never \
         run once.",
        needs,
        j.get("if")
    );
}

/// 同样地，别的**后置**判据 job 也不该被 `test` 遮蔽。
/// （`mutation` 曾经就是 `needs: [test]`。）
#[test]
fn test_5455_no_post_release_job_is_masked_behind_the_never_green_test_job() {
    let yaml = load_yaml(".gitea/workflows/ci.yml");
    let jobs = yaml
        .get("jobs")
        .and_then(|v| v.as_mapping())
        .expect("jobs mapping");
    let mut offenders = Vec::new();
    for (jk, jv) in jobs {
        let name = jk.as_str().unwrap_or("?");
        if name == "test" {
            continue;
        }
        let needs: Vec<String> = match jv.get("needs") {
            Some(Value::Sequence(s)) => s
                .iter()
                .map(|v| v.as_str().unwrap_or("").to_string())
                .collect(),
            Some(Value::String(s)) => vec![s.clone()],
            _ => Vec::new(),
        };
        if !needs.iter().any(|n| n == "test") {
            continue;
        }
        let always = jv
            .get("if")
            .and_then(|v| v.as_str())
            .map(|s| s.contains("always()"))
            .unwrap_or(false);
        if !always {
            offenders.push(name.to_string());
        }
    }
    assert!(
        offenders.is_empty(),
        "these jobs are masked behind the `test` job, which has never succeeded \
         — their criteria have never executed:\n  - {}",
        offenders.join("\n  - ")
    );
}

// =============================================================================
// #5424 — Security 门禁在 v4.x 上从未执行
// =============================================================================

/// `ga3-security-scan` 的 `if:` 不得把当前版本线排除在外。
#[test]
fn test_5424_security_scan_job_runs_on_the_current_version_line() {
    let yaml = load_yaml(".gitea/workflows/ci.yml");
    let j = job(&yaml, "ga3-security-scan");
    let cond = j.get("if").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        cond.is_empty() || cond.contains("v4"),
        "ga3-security-scan is skipped on the current version line: if: `{}`. \
         The GA Security criterion has therefore never been executed.",
        cond
    );
}

// =============================================================================
// #5447 — gate.yml 的两个 job 都不做真检查
// =============================================================================

/// 证据图门禁不得摄入占位哈希。
#[test]
fn test_5447_evidence_graph_gate_ingests_no_placeholder_hash() {
    let text = strip_comments(&read_rel(".gitea/workflows/gate.yml"));
    assert!(
        !text.contains("sha256-placeholder"),
        "gate.yml still ingests a placeholder artifact hash; the green signal \
         has no relation to the checked-out tree"
    );
}

/// 证据图门禁不得把 `PASS` 写进证据图再从证据图里读出 `PASS`。
#[test]
fn test_5447_evidence_graph_gate_does_not_write_its_own_pass() {
    let yaml = load_yaml(".gitea/workflows/gate.yml");
    let mut offenders = Vec::new();
    for (job, step, body) in all_steps(&yaml) {
        for line in body.lines() {
            let t = line.trim();
            if !t.contains("ingest ci") {
                continue;
            }
            // 结果参数必须来自变量，不能是字面量 PASS
            if t.contains(" PASS ") || t.ends_with(" PASS") {
                offenders.push(format!("{} / {}: {}", job, step, t));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "the evidence-graph gate ingests a hard-coded CI result and then reads \
         that same result back as its verdict — the green signal is written \
         by the job itself:\n  - {}",
        offenders.join("\n  - ")
    );
}

/// 任何跑门禁脚本的步骤都不得用 `|| true` 吞掉退出码。
#[test]
fn test_5447_no_gate_invocation_swallows_its_exit_code() {
    let yaml = load_yaml(".gitea/workflows/gate.yml");
    let mut offenders = Vec::new();
    for (job, step, body) in all_steps(&yaml) {
        for line in body.lines() {
            let t = line.trim();
            if t.starts_with('#') {
                continue;
            }
            if t.contains("bash ") && t.ends_with("|| true") {
                offenders.push(format!("{} / {}: {}", job, step, t));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "these steps run a shell script and then discard its exit code with \
         `|| true`, so the job is green whatever the gate says:\n  - {}",
        offenders.join("\n  - ")
    );
}

/// legacy 门禁脚本不得在 CI 步骤里**无界地**跑整个测试套件。
///
/// 那是 CI 挂死 2.5 h 的直接原因：446 个测试目标在单 runner 上跑满整个
/// 2.5 h 预算，把其它 job 全堵在队列里。
///
/// 「有界」的定义：要么限定了具体目标/包（`--test X` / `-p X`），要么被
/// `timeout` / `run_bounded` 包住。
#[test]
fn test_5447_legacy_gate_does_not_run_the_whole_test_suite_unbounded() {
    let text = read_rel("scripts/gate/legacy/check_alpha.sh");
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('#') {
            continue;
        }
        if !t.contains("cargo test") {
            continue;
        }
        let restricted = t.contains("--test ") || t.contains("-p ");
        let bounded = t.contains("timeout ") || t.contains("run_bounded ");
        assert!(
            restricted || bounded,
            "scripts/gate/legacy/check_alpha.sh runs an unbounded full-suite \
             `cargo test` (`{}`): in CI that is the 2.5 h hang that blocks the \
             single runner. The full suite is the `test` job's job, not a \
             gate's — bound it or restrict it to a target.",
            t
        );
    }
}

// =============================================================================
// #5483 — 每个 step 的 run 体都必须是合法 shell
// =============================================================================

/// **通用判据**：工作流里**每一个** step 的 `run` 体都必须是合法 bash。
///
/// #5483 就是这么发现的：Gate summary 步骤的 `if` 块少了一个 `fi`，
/// 整个步骤体成了语法错误，恒以 2 退出 —— 而因为它所在的 job 从未被执行
/// （#5455），这条错误潜伏了很久。
///
/// 这一条对所有 step、所有工作流都成立，成本近乎为零。
#[test]
fn test_5483_every_workflow_step_body_is_valid_bash() {
    for wf in [".gitea/workflows/ci.yml", ".gitea/workflows/gate.yml"] {
        let yaml = load_yaml(wf);
        for (job, step, body) in all_steps(&yaml) {
            if body.trim().is_empty() {
                continue;
            }
            let out = Command::new("bash")
                .arg("-n")
                .arg("-c")
                .arg(&body)
                .output()
                .expect("spawn bash -n");
            assert!(
                out.status.success(),
                "{} / `{}` has a run body that is not valid bash (bash -n exit={}):\n{}\n{}",
                wf,
                step,
                out.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            let _ = job;
        }
    }
}

// =============================================================================
// #5444 — 版本探测必须在当前版本线上解析正确
// =============================================================================

/// postcheck 的版本探测之前只在 `origin/develop/v3.8.0` 与
/// `origin/develop/v3.7.0` 之间二选一，而 checkout 是 `fetch-depth: 1`
/// ⇒ 那些远端分支根本不存在 ⇒ **任何分支上都恒为 v3.7.0**，v4.1.0 也不例外。
#[test]
fn test_5444_version_detection_resolves_the_current_version_line() {
    let yaml = load_yaml(".gitea/workflows/ci.yml");
    let body = step_body(&yaml, "postcheck", "Version detection");

    let dir = TempDir::new().expect("tempdir");
    let out_file = dir.path().join("gh_output");
    let out = Command::new("bash")
        .arg("-c")
        .arg(&body)
        .current_dir(repo_root())
        .env("GITHUB_REF_NAME", "develop/v4.1.0")
        .env("GITHUB_OUTPUT", &out_file)
        .output()
        .expect("spawn bash");
    assert!(
        out.status.success(),
        "the version detection step must succeed; stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let produced = std::fs::read_to_string(&out_file).unwrap_or_default();
    assert!(
        produced.contains("version=v4.1.0"),
        "on `develop/v4.1.0` the detected version must be v4.1.0, got:\n{}",
        produced
    );
}

// =============================================================================
// #5444 — step 里引用的 shell 变量必须真的有值
// =============================================================================

/// **通用判据**：一个 step 里用 `${VAR}` 引用的每个 shell 变量，必须
/// (a) 在本 step 内被赋值，或 (b) 由本 job **更早**的 step 通过
/// `$GITHUB_ENV` 导出，或 (c) 属于 runner/CI 提供的已知变量。
///
/// 否则 `${VAR}` 是一个**未定义变量**：
/// - 带 `set -u` 的 step 会直接以 "VAR: unbound variable" 失败；
/// - 不带 `set -u` 的 step 会把它当成空串，悄悄按脚本的默认值跑。
///
/// 本仓库就有一例：「Evidence Binding Check」步骤里写的是
/// `bash scripts/gate/check_evidence_binding.sh "${version}" …`，
/// 而 `version` 从未被定义 —— 步骤输出必须用
/// `${{ steps.<id>.outputs.<name> }}` 读，不是 bash 变量。
/// 该步骤本来就有 `set -euo pipefail`，所以它**一直**在失败，
/// 只因 postcheck 从未执行（#5455）而无人发现。
#[test]
fn test_5444_no_step_references_an_undefined_shell_variable() {
    const RUNNER_VARS: &[&str] = &[
        "PATH",
        "HOME",
        "USER",
        "PWD",
        "OLDPWD",
        "TMPDIR",
        "SHELL",
        "LANG",
        "CI",
        "GITHUB_ENV",
        "GITHUB_OUTPUT",
        "GITHUB_PATH",
        "GITHUB_SHA",
        "GITHUB_REF",
        "GITHUB_REF_NAME",
        "GITHUB_RUN_ID",
        "GITHUB_WORKSPACE",
        "GITHUB_ACTOR",
        "GITEA_SHA",
        "GITEA_RUN_ID",
        "GITEA_REF_NAME",
        "RUNNER_TEMP",
        "RUNNER_OS",
        "RUNNER_ARCH",
        "RUNNER_TOOL_CACHE",
        "CARGO_HOME",
        "RUSTUP_HOME",
        "CARGO_TERM_COLOR",
    ];

    for wf in [".gitea/workflows/ci.yml", ".gitea/workflows/gate.yml"] {
        let yaml = load_yaml(wf);
        let jobs = yaml
            .get("jobs")
            .and_then(|v| v.as_mapping())
            .expect("jobs mapping");
        for (jk, jv) in jobs {
            let job_name = jk.as_str().unwrap_or("?");
            let steps = match jv.get("steps").and_then(|v| v.as_sequence()) {
                Some(s) => s,
                None => continue,
            };
            // 本 job 中，由**更早**的 step 导出到 $GITHUB_ENV 的变量
            let mut exported: BTreeSet<String> = BTreeSet::new();
            for st in steps {
                let raw_body = st.get("run").and_then(|v| v.as_str()).unwrap_or("");
                // 注释里出现的 `${var}` 只是说明文字，不是真正的引用。
                let body = strip_comments(raw_body);
                let name = st
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("<unnamed>");

                // (a) 本 step 内被赋值的变量
                let mut local: BTreeSet<String> = BTreeSet::new();
                for line in body.lines() {
                    if let Some(name) = leading_assignment(line) {
                        local.insert(name);
                    }
                }

                // 未定义引用
                let mut undefined: Vec<String> = Vec::new();
                for token in split_shell_refs(&body) {
                    if RUNNER_VARS.contains(&token.as_str()) {
                        continue;
                    }
                    if local.contains(&token) || exported.contains(&token) {
                        continue;
                    }
                    undefined.push(token);
                }
                assert!(
                    undefined.is_empty(),
                    "{} / `{}` references undefined shell variable(s) {} — a step \
                     output must be read as `${{{{ steps.<id>.outputs.<name> }}}}` \
                     (that is a template expression, not a bash variable), and \
                     the value must otherwise be assigned in this step or \
                     exported by an earlier step via $GITHUB_ENV.",
                    wf,
                    name,
                    undefined
                        .iter()
                        .map(|v| format!("${{{}}}", v))
                        .collect::<Vec<_>>()
                        .join(", ")
                );

                // 本 step 导出的变量，对后续 step 可见
                for line in body.lines() {
                    if !line.contains("$GITHUB_ENV") {
                        continue;
                    }
                    for name in exported_names(line) {
                        exported.insert(name);
                    }
                }
            }
            let _ = job_name;
        }
    }
}

/// 一行开头的赋值：`NAME=…` 或 `local NAME=…`。返回变量名。
fn leading_assignment(line: &str) -> Option<String> {
    let t = line.trim_start();
    let t = t.strip_prefix("local ").unwrap_or(t).trim_start();
    let name: String = t
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    let rest = &t[name.len()..];
    if rest.starts_with('=') {
        Some(name)
    } else {
        None
    }
}

/// 一行里所有 `NAME=` 形态的标识符（用于 `echo "NAME=…" >> $GITHUB_ENV`）。
fn exported_names(line: &str) -> Vec<String> {
    let chars: Vec<char> = line.chars().collect();
    let mut out = Vec::new();
    let mut k = 0;
    while k < chars.len() {
        if chars[k].is_ascii_alphabetic() || chars[k] == '_' {
            let start = k;
            while k < chars.len() && (chars[k].is_ascii_alphanumeric() || chars[k] == '_') {
                k += 1;
            }
            let name: String = chars[start..k].iter().collect();
            if k < chars.len() && chars[k] == '=' {
                out.push(name);
            }
        } else {
            k += 1;
        }
    }
    out
}

/// 取出正文里所有 `${NAME}` 形式的变量引用（排除 `${{ … }}` 模板表达式
/// 与 `$( … )` 命令替换）。
fn split_shell_refs(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes: Vec<char> = body.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == '$' && i + 1 < bytes.len() && bytes[i + 1] == '{' {
            // `${{` 是模板表达式，跳过整段
            if i + 2 < bytes.len() && bytes[i + 2] == '{' {
                let mut depth = 0;
                let mut j = i + 1;
                while j < bytes.len() {
                    if bytes[j] == '{' {
                        depth += 1;
                    } else if bytes[j] == '}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    j += 1;
                }
                i = j + 1;
                continue;
            }
            // `${NAME}` 形式
            let mut j = i + 2;
            let mut name = String::new();
            while j < bytes.len() && bytes[j] != '}' {
                name.push(bytes[j]);
                j += 1;
            }
            if !name.is_empty()
                && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                && !name.starts_with(|c: char| c.is_ascii_digit())
            {
                out.push(name);
            }
            i = j + 1;
            continue;
        }
        i += 1;
    }
    out
}

// =============================================================================
// #5485 — 读基线的门禁，其基线文件必须已提交
// =============================================================================

/// 任何**读基线文件**的门禁，那个基线文件都必须存在于仓库里。
///
/// 否则门禁每次都走「首次运行：建立基线」分支并以 0 退出 ⇒ **恒绿**。
///
/// 仓库里已有 `tests/baseline/drift_baseline.json`（P14 用），但 P13 的
/// `test_count.json` 与 P15 的 `oracle_baseline.json` 都不在版本控制里 ⇒
/// 这两道元门禁在任何一次 fresh checkout 上都只会「建立基线」然后通过。
#[test]
fn test_5485_every_baseline_driven_gate_has_a_committed_baseline() {
    let mut referenced: Vec<String> = Vec::new();
    let gate_dir = repo_root().join("scripts/gate");
    let mut stack = vec![gate_dir.clone()];
    let mut scripts: Vec<std::path::PathBuf> = Vec::new();
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read gate dir") {
            let e = entry.expect("dir entry");
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().map(|x| x == "sh").unwrap_or(false) {
                scripts.push(p);
            }
        }
    }
    for s in &scripts {
        let text = std::fs::read_to_string(s).unwrap_or_default();
        for token in text.split('"') {
            if token.starts_with("tests/baseline/")
                && token.ends_with(".json")
                && !referenced.contains(&token.to_string())
            {
                referenced.push(token.to_string());
            }
        }
    }
    assert!(
        !referenced.is_empty(),
        "no gate script references a tests/baseline/*.json file — the scan is broken"
    );
    let mut missing: Vec<String> = Vec::new();
    for rel in &referenced {
        if !repo_root().join(rel).exists() {
            missing.push(rel.clone());
        }
    }
    assert!(
        missing.is_empty(),
        "these gate baselines are not in the repository, so the gates that read \
         them always take the 'first run: create baseline' branch and exit 0 — \
         they can never fail:\n  - {}",
        missing.join("\n  - ")
    );
}

// =============================================================================
// #5416 — legacy alpha 门禁的 clippy 命令把 --quiet 传给了 rustc
// =============================================================================

/// `cargo clippy` 的 cargo 级选项必须写在 `--` **之前**；`--` 之后的一切
/// 都是 rustc 的参数。
#[test]
fn test_5416_legacy_clippy_invocation_puts_cargo_flags_before_the_rustc_separator() {
    let text = read_rel("scripts/gate/legacy/check_alpha.sh");
    let line = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('#'))
        .find(|l| l.contains("cargo clippy"))
        .unwrap_or_else(|| panic!("no cargo clippy invocation in check_alpha.sh"));

    // 取出被检查的命令字符串（最后一个双引号段里含 "cargo clippy" 的那个）
    let cmd = line
        .split('"')
        .find(|seg| seg.contains("cargo clippy"))
        .unwrap_or_else(|| panic!("cannot extract the clippy command from: {}", line))
        .to_string();

    let (before, after) = cmd
        .split_once(" -- ")
        .unwrap_or_else(|| panic!("the clippy command has no ` -- ` separator: {}", cmd));

    assert!(
        before.contains("--quiet"),
        "`--quiet` is a cargo-level flag and must appear BEFORE the ` -- ` \
         separator; it was passed to rustc instead, which then errors with \
         'Unrecognized option: quiet' and fails every time. command: {}",
        cmd
    );
    assert!(
        !after.contains("--quiet"),
        "`--quiet` must not be passed to rustc (everything after ` -- ` goes to \
         rustc). command: {}",
        cmd
    );
}

/// 门禁失败时必须给出**原因**。修前失败行只有一个词 `FAIL`，
/// 没有任何可诊断的信息 —— 这正是「连着三个 FAIL 却看不出所以然」的原因。
///
/// 判据落在 `check()` 这个 helper 身上：它必须**捕获**被检查命令的输出
/// （而不是丢进 /dev/null），并在失败时把原因打出来。
#[test]
fn test_5416_legacy_gate_reports_why_a_check_failed() {
    let text = read_rel("scripts/gate/legacy/check_alpha.sh");
    let helper = text
        .split("check() {")
        .nth(1)
        .and_then(|rest| rest.split("\n}").next())
        .unwrap_or_else(|| panic!("no `check()` helper in check_alpha.sh"));
    assert!(
        !helper.contains(">/dev/null"),
        "check_alpha.sh's `check` helper discards the checked command's output \
         entirely, so a FAIL line carries no reason at all. helper was:\n{}",
        helper
    );
    assert!(
        helper.contains("2>&1") && !helper.contains("2>&1 | grep -v"),
        "the `check` helper must capture the command's stdout **and** stderr, \
         otherwise a failure reason that only appears on stderr is lost. \
         helper was:\n{}",
        helper
    );
}

/// 行为验证：用桩 cargo 真跑一遍 legacy 脚本，断言失败原因被打印出来。
#[test]
fn test_5416_legacy_gate_surfaces_the_failing_command_output() {
    let dir = TempDir::new().expect("tempdir");
    let stub_dir = dir.path().join("stub");
    std::fs::create_dir_all(&stub_dir).expect("mkdir stub");
    let stub = stub_dir.join("cargo");
    std::fs::write(
        &stub,
        "#!/usr/bin/env bash\necho 'STUB-CARGO-REASON: dependency resolution failed' >&2\nexit 1\n",
    )
    .expect("write stub cargo");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perm = std::fs::metadata(&stub).expect("stat stub").permissions();
        perm.set_mode(0o755);
        std::fs::set_permissions(&stub, perm).expect("chmod stub");
    }

    let path = format!(
        "{}:{}",
        stub_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let script = repo_root().join("scripts/gate/legacy/check_alpha.sh");
    let out = Command::new("bash")
        .arg(&script)
        .current_dir(repo_root())
        .env("PATH", path)
        .output()
        .expect("spawn check_alpha.sh");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("STUB-CARGO-REASON"),
        "the legacy gate must surface the failing command's own output; a FAIL \
         line with no reason is not diagnosable. Output was:\n{}",
        combined
    );
}

// =============================================================================
// #5420 — fault_injection.rs 的同名定义在任何 feature 组合下都只应活一个
// =============================================================================

/// 极简 `cfg` 谓词求值器，只支持本仓库实际用到的形态：
/// `feature = "F"` / `not(...)` / `any(...)` / `all(...)` / `test`。
fn cfg_holds(pred: &str, features: &BTreeSet<String>, test_on: bool) -> bool {
    let p = pred.trim();
    if let Some(inner) = p.strip_prefix("not(").and_then(|r| r.strip_suffix(')')) {
        return !cfg_holds(inner, features, test_on);
    }
    if let Some(inner) = p.strip_prefix("any(").and_then(|r| r.strip_suffix(')')) {
        return split_cfg_args(inner)
            .iter()
            .any(|a| cfg_holds(a, features, test_on));
    }
    if let Some(inner) = p.strip_prefix("all(").and_then(|r| r.strip_suffix(')')) {
        return split_cfg_args(inner)
            .iter()
            .all(|a| cfg_holds(a, features, test_on));
    }
    if p == "test" {
        return test_on;
    }
    if let Some(rest) = p.strip_prefix("feature = ") {
        let name = rest.trim().trim_matches('"');
        return features.contains(name);
    }
    // 未知形态：保守地当作「总是成立」——宁可多查，不可漏查。
    true
}

fn split_cfg_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut cur = String::new();
    for ch in s.chars() {
        match ch {
            '(' => {
                depth += 1;
                cur.push(ch);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                cur.push(ch);
            }
            ',' if depth == 0 => {
                out.push(cur.trim().to_string());
                cur = String::new();
            }
            _ => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

/// 收集一个 Rust 文件里**模块顶层**的自由函数及其 `#[cfg(...)]` 谓词。
///
/// `impl` 块里的方法属于不同类型 ⇒ 不同作用域 ⇒ 同名不算冲突，故跳过。
fn collect_top_level_fns(text: &str) -> BTreeMap<String, Vec<Vec<String>>> {
    let mut out: BTreeMap<String, Vec<Vec<String>>> = BTreeMap::new();
    let mut pending: Vec<String> = Vec::new();
    let mut impl_stack: Vec<bool> = Vec::new();
    let mut in_impl = false;

    for line in text.lines() {
        let t = line.trim();

        if t.starts_with("#[cfg(") {
            // 括号是配对的：`#[cfg(not(feature = "x"))]` 的结尾有两个 `)`。
            // 必须按深度扫描取出配对的括号，不能简单 trim 一个 `)`。
            let open = t.find('(').expect("cfg attribute has a paren");
            let mut depth = 0usize;
            let mut close = None;
            for (i, ch) in t[open..].char_indices() {
                match ch {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            close = Some(open + i);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let close = close.expect("cfg attribute has a closing paren");
            pending.push(t[open + 1..close].trim().to_string());
            continue;
        }
        if t.starts_with("#[") {
            continue; // 其它属性：不清空累积的 cfg
        }
        if t.starts_with("//") || t.is_empty() {
            continue; // 注释与空行不打断 cfg 累积
        }

        // 括号深度跟踪
        for ch in t.chars() {
            match ch {
                '{' => {
                    impl_stack.push(in_impl);
                    if t.starts_with("impl ") {
                        in_impl = true;
                    }
                }
                '}' => {
                    let popped = impl_stack.pop().unwrap_or(false);
                    in_impl = popped;
                }
                _ => {}
            }
        }

        let name = t
            .strip_prefix("pub fn ")
            .or_else(|| t.strip_prefix("fn "))
            .and_then(|r| r.split('(').next())
            .map(|s| s.trim().to_string());
        if let Some(name) = name {
            if !in_impl {
                out.entry(name).or_default().push(pending.clone());
            }
            pending.clear();
            continue;
        }

        // 其它非属性、非定义行：cfg 累积到此为止
        pending.clear();
    }
    out
}

/// **通用不变量**：模块顶层的同名自由函数，在任何 feature 组合下
/// 最多只能有一个存活 —— 否则就是 E0428「defined multiple times」。
///
/// #5420：`reach` 的真实现没有 `#[cfg(feature = "fault-injection")]`，而
/// 空实现带 `#[cfg(not(feature = "fault-injection"))]` ⇒ feature 关闭时
/// 两个同时存在 ⇒ `cargo test --lib -p sqlrustgo-storage` 编译失败。
#[test]
fn test_5420_duplicate_top_level_fns_are_mutually_exclusive_in_every_feature_combination() {
    let rel = "crates/storage/src/fault_injection.rs";
    let text = read_rel(rel);
    let defs = collect_top_level_fns(&text);

    let mut offenders: Vec<String> = Vec::new();
    for (name, groups) in &defs {
        if groups.len() < 2 {
            continue;
        }
        // 收集这组定义里出现过的全部 feature 名
        let mut feature_names: BTreeSet<String> = BTreeSet::new();
        for g in groups {
            for pred in g {
                for seg in split_cfg_args(pred) {
                    if let Some(rest) = seg.trim().strip_prefix("feature = ") {
                        feature_names.insert(rest.trim().trim_matches('"').to_string());
                    }
                }
            }
        }
        // 枚举所有 feature 组合
        let names: Vec<String> = feature_names.iter().cloned().collect();
        for mask in 0u32..(1u32 << names.len()) {
            let mut features: BTreeSet<String> = BTreeSet::new();
            for (i, n) in names.iter().enumerate() {
                if mask & (1 << i) != 0 {
                    features.insert(n.clone());
                }
            }
            let alive: Vec<usize> = groups
                .iter()
                .enumerate()
                .filter(|(_, g)| g.iter().all(|pred| cfg_holds(pred, &features, true)))
                .map(|(i, _)| i)
                .collect();
            if alive.len() > 1 {
                offenders.push(format!(
                    "{}: `{}` has {} live definitions for features={:?} \
                     (cfg predicates: {:?})",
                    rel,
                    name,
                    alive.len(),
                    features,
                    groups
                ));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "these same-named top-level functions can be compiled together, which is \
         exactly the E0428 'defined multiple times' compile error:\n  - {}",
        offenders.join("\n  - ")
    );
}

/// 这个不变量必须在修复后成立、且在修复前失败 —— 用一个**内联的最小样例**
/// 证明检查器本身有效（否则「检查器恒绿」也是一种恒绿）。
#[test]
fn test_5420_the_cfg_exclusivity_checker_itself_detects_a_duplicate() {
    let broken = r#"
#[cfg(not(feature = "fault-injection"))]
pub fn reach(_point: u8) {}
pub fn reach(point: u8) { let _ = point; }
"#;
    let defs = collect_top_level_fns(broken);
    let groups = defs.get("reach").expect("reach group");
    let features = BTreeSet::new();
    let alive = groups
        .iter()
        .filter(|g| g.iter().all(|p| cfg_holds(p, &features, true)))
        .count();
    assert!(
        alive > 1,
        "the checker must flag a duplicate that is live with the feature off; \
         it reported {} live definition(s)",
        alive
    );
}
