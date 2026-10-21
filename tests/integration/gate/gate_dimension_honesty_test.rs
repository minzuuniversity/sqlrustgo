//! 门禁维度诚实性回归套件 —— Issues #5421 / #5422 / #5425
//!
//! 这一组与 `ci_wiring_honesty_test.rs`（工作流接线层）、`gate_honesty_test.rs`
//! （批次 2 的判定逻辑层）同族，但作用在**门禁脚本自己的内部**：
//!
//! | Issue | 缺陷 | 症状 |
//! |-------|------|------|
//! | #5421 | RC/GA 的 D2-B6 要求 `target/sqlancer-report.json` 等**门禁自己不产出**的生成物 | 任何 fresh checkout 上必红 ⇒ 只能靠「预先有人手工生成过」才能变绿 |
//! | #5422 | `check_cross_version_debt.sh` 的计数器只增不初始化 | `[ "" -gt 0 ]` 抛 "integer expression expected"，DEFERRED 计数打印为空 |
//! | #5425 | `check_anti_ignore_gate.sh` 的阈值在**三处**互不相同 | 同一个仓库按不同判据得到 PASS/FAIL 两种结论 |
//!
//! 设计取向
//! --------
//! * **判据写成不变量**：#5422 的判据是「**任何** `X=$((X + 1))` 的变量都必须
//!   在同文件内被初始化」，不是「第 415 行要补一行」；
//!   #5425 的判据是「脚本**实际生效**的阈值必须与治理文档写的阈值相等」，
//!   不是「把 47 改成 80」。
//! * **能跑的就真跑**：D2-B6 的检查块用**桩 cargo** 真跑两遍
//!   （桩能产出合法产物 / 桩什么都不产出），断言前者判 PASS、后者判 FAIL。

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_rel(rel: &str) -> String {
    let p = repo_root().join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {}", p.display(), e))
}

fn run_shell(dir: &Path, script: &str, path_prefix: Option<&Path>) -> (i32, String, String) {
    let mut cmd = Command::new("bash");
    cmd.arg("-c").arg(script).current_dir(dir);
    if let Some(pre) = path_prefix {
        let path = format!(
            "{}:{}",
            pre.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        cmd.env("PATH", path);
    }
    let out = cmd.output().expect("spawn bash");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// 丢掉整行注释。
fn strip_comments(s: &str) -> String {
    s.lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

// =============================================================================
// #5422 — 门禁脚本里只增不初始化的计数器
// =============================================================================

/// **通用不变量**：任何形如 `X=$((X + 1))` 的自增计数器，`X` 都必须在同文件内
/// 被初始化过。
///
/// bash 的算术展开会把未定义的变量当作 0，所以自增**表面上**能工作；
/// 但只要该计数器的当前值为 0（从未自增过），`[ "$X" -gt 0 ]` 就会变成
/// `[ "" -gt 0 ]` ⇒ 报 "integer expression expected" 并返回 2
/// ⇒ 在 `if` 里被当作「假」。
///
/// ⇒ 后果分两种：
/// - 该判定在计数为 0 时**打印一行 shell 报错**（噪声，且每次运行都出现）；
/// - 该判定在计数非 0 时**恰好仍然正确**（因为自增把未定义变成了 1、2、3…）。
///
/// 所以本 issue 的真实缺陷是「噪声 + 不可诊断」，
/// 而**不是** issue 原文所说的「判定永久不可达」—— 见下面的
/// `test_5422_the_increment_counts_correctly_from_an_unset_variable` 用例，
/// 它把这个区别固化成可执行的证据。
#[test]
fn test_5422_every_incremented_counter_is_initialized_in_the_same_script() {
    let mut offenders: Vec<String> = Vec::new();
    for rel in [
        "scripts/gate/check_cross_version_debt.sh",
        "scripts/gate/check_rc_ga_gate.sh",
        "scripts/gate/check_full_gate_verification.sh",
    ] {
        let text = read_rel(rel);
        let code = strip_comments(&text);
        let mut incremented: Vec<String> = Vec::new();
        for line in code.lines() {
            let t = line.trim();
            // 形如 `NAME=$((NAME + 1))` —— 只认**加一**的计数器。
            // `X=$((X + something))` 是累加器，语义不同，不在本判据内。
            if let Some(open) = t.find("$((") {
                let expr = &t[open + 3..];
                if let Some(close) = expr.find("))") {
                    let inner = expr[..close].trim();
                    let mut parts = inner.split('+');
                    let name = parts.next().unwrap_or("").trim();
                    let addend = parts.next().unwrap_or("").trim();
                    let only_addend = parts.next().is_none();
                    if only_addend
                        && (addend == "1")
                        && !name.is_empty()
                        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                        && !incremented.contains(&name.to_string())
                    {
                        incremented.push(name.to_string());
                    }
                }
            }
        }
        for name in incremented {
            let initialized = code.lines().any(|l| {
                // 一行里可能有多个赋值（`A=0; B=0; C=0`），也可能带 `local`。
                for part in l.split(';') {
                    let t = part.trim();
                    let t = t.strip_prefix("local ").unwrap_or(t).trim_start();
                    let mut it = t.splitn(2, '=');
                    let key = it.next().unwrap_or("").trim();
                    let val = it.next().unwrap_or("").trim();
                    if key == name && !val.is_empty() && val.chars().all(|c| c.is_ascii_digit()) {
                        return true;
                    }
                }
                false
            });
            if !initialized {
                offenders.push(format!("{}: `{}`", rel, name));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "these counters are incremented but never initialized, so while their \
         value is 0 every `[ \"$X\" -gt 0 ]` on them prints \
         'integer expression expected' and returns 2 (false):\n  - {}",
        offenders.join("\n  - ")
    );
}

/// 把 #5422 的归因差异固化成可执行证据：bash 的算术展开让未定义变量按 0 处理，
/// 所以自增本身是**正确**的 —— 真实缺陷是「零计数时的 shell 报错」，
/// 而不是 issue 原文所说的「判定永久不可达」。
#[test]
fn test_5422_the_increment_counts_correctly_from_an_unset_variable() {
    let (code, out, err) = run_shell(
        repo_root().as_path(),
        "unset X; X=$((X + 1)); X=$((X + 1)); echo \"X=$X\"",
        None,
    );
    assert_eq!(code, 0, "{}", err);
    assert_eq!(
        out.trim(),
        "X=2",
        "an unset counter counts from 0 correctly"
    );
}

/// 零计数时那行 shell 报错必须消失 —— 真跑一次门禁看它的输出。
#[test]
fn test_5422_a_zero_counter_produces_no_shell_error() {
    let out = Command::new("bash")
        .arg(repo_root().join("scripts/gate/check_cross_version_debt.sh"))
        .current_dir(repo_root())
        .output()
        .expect("run check_cross_version_debt.sh");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !text.contains("integer expression expected"),
        "the debt gate must not emit 'integer expression expected' — that is \
         what an uninitialized counter looks like at runtime. Output was:\n{}",
        text
    );
    // 计数必须真的打出来（修前 DEFERRED 那一行后面是空的）
    for label in ["CLOSED", "PARTIAL", "OPEN", "DEFERRED", "ACTIVE", "UNKNOWN"] {
        let needle = format!("{}:", label);
        let line = text
            .lines()
            .find(|l| l.contains(&needle))
            .unwrap_or_else(|| panic!("the debt summary has no `{}` row:\n{}", label, text));
        let value = line.split(':').nth(1).unwrap_or("").trim().to_string();
        assert!(
            !value.is_empty() && value.chars().all(|c| c.is_ascii_digit()),
            "the `{}` count must be printed as a number, got {:?} in {:?}",
            label,
            value,
            line
        );
    }
}

// =============================================================================
// #5421 — D2-B6 要求门禁自己不产出的生成物
// =============================================================================

/// 取出 RC/GA 门禁里的 D2-B6 检查块（从 `# B6:` 注释到 D2 汇总行）。
fn d2_b6_block() -> String {
    let text = read_rel("scripts/gate/check_rc_ga_gate.sh");
    let start = text
        .find("# B6:")
        .unwrap_or_else(|| panic!("no `# B6:` marker in check_rc_ga_gate.sh"));
    let end = text[start..]
        .find("D2 Result:")
        .map(|i| start + i)
        .unwrap_or_else(|| panic!("no `D2 Result:` after `# B6:`"));
    // 必须切在**行边界**上：`D2 Result:` 出现在一行 `echo -e "\n  D2 Result: …"`
    // 的中间，直接切会留下一个未闭合的引号。
    let end = text[..end]
        .rfind('\n')
        .expect("the D2 Result line must start on its own line");
    text[start..end].to_string()
}

/// 构造一个只含桩 `cargo` 的目录。
///
/// `mode`:
/// * `"produce"` —— 桩 cargo 会按 `--out` 参数写出**合法**的产物；
/// * `"silent"` —— 桩 cargo 什么都不做（模拟生成失败）。
fn stub_cargo_dir(tag: &str, mode: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sqlrustgo_b6_stub_{}", tag));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create stub dir");
    let cargo = dir.join("cargo");
    let body = if mode == "produce" {
        r#"#!/usr/bin/env bash
# stub cargo: writes the artifact the real generator would write
out=""
prev=""
for a in "$@"; do
  if [ "$prev" = "--out" ]; then out="$a"; fi
  prev="$a"
done
[ -n "$out" ] || exit 0
mkdir -p "$(dirname "$out")"
case "$out" in
  *sqlancer-report.json)
    printf '%s' '{"successful_queries":1000,"failed_queries":0,"iterations_requested":1000,"timeout":false,"errors":[],"duration_secs":1}' > "$out"
    ;;
  *test-runner-report.json)
    printf '%s' '{"started_at":"2026-01-01T00:00:00Z","finished_at":"2026-01-01T00:00:01Z","config":{},"mode":"probe","summary":{},"results":[]}' > "$out"
    ;;
esac
exit 0
"#
    } else {
        "#!/usr/bin/env bash\nexit 0\n"
    };
    std::fs::write(&cargo, body).expect("write stub cargo");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perm = std::fs::metadata(&cargo).expect("stat stub").permissions();
        perm.set_mode(0o755);
        std::fs::set_permissions(&cargo, perm).expect("chmod stub");
    }
    dir
}

/// 把 D2-B6 的检查块包成一个可独立执行的自包含脚本。
///
/// 注意：不能用 `format!` —— 被嵌入的块里有 python 片段带来的 `{` / `}`，
/// 会被当成占位符。这里用字符串拼接。
fn b6_snippet(repo_root: &Path) -> String {
    let block = d2_b6_block();
    let mut s = String::new();
    s.push_str("set -uo pipefail\n");
    s.push_str("REPO_ROOT=\"");
    s.push_str(&repo_root.display().to_string());
    s.push_str("\"\n");
    s.push_str("D2_PASS=0\nD2_TOTAL=0\nD2_BLOCKERS=0\n");
    s.push_str("log_pass() { echo \"  ✓ PASS: $1\"; }\n");
    s.push_str("log_fail() { echo \"  ✗ FAIL $1\"; }\n");
    s.push_str(&block);
    s.push_str("\necho \"COUNTS PASS=$D2_PASS TOTAL=$D2_TOTAL BLOCKERS=$D2_BLOCKERS\"\n");
    s
}

/// **核心行为**：D2-B6 必须**自己产出**它要检查的产物。
///
/// 桩 cargo 能产出合法产物时，检查必须判 PASS；
/// 桩 cargo 什么都不产出时，检查必须判 FAIL。
///
/// 修前：块里没有任何生成动作 ⇒ 第一种情况下**也判 FAIL**
/// （因为 fresh checkout 里本来就没有产物）⇒ 这就是「只能靠预先有人手工生成过
/// 才能变绿」的机制。
#[test]
fn test_5421_d2_b6_produces_the_artifacts_it_checks() {
    let stub = stub_cargo_dir("produce", "produce");
    let work = std::env::temp_dir().join("sqlrustgo_b6_work_produce");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).expect("create work dir");
    let snippet = b6_snippet(&work);
    let (code, out, err) = run_shell(&work, &snippet, Some(&stub));
    assert!(
        out.contains("PASS"),
        "B6 must PASS once the gate can produce its own artifacts.\n\
         stdout:\n{}\nstderr:\n{}",
        out,
        err
    );
    assert!(
        out.contains("PASS=1") && out.contains("BLOCKERS=0"),
        "B6 must count one pass and no blocker; stdout:\n{}",
        out
    );
    let _ = code;
}

/// 反向：产物真的产不出来时，检查必须判 FAIL（不能因为「判不了」就放过）。
#[test]
fn test_5421_d2_b6_fails_when_the_artifacts_cannot_be_produced() {
    let stub = stub_cargo_dir("silent", "silent");
    let work = std::env::temp_dir().join("sqlrustgo_b6_work_silent");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).expect("create work dir");
    let snippet = b6_snippet(&work);
    let (code, out, err) = run_shell(&work, &snippet, Some(&stub));
    assert!(
        out.contains("FAIL"),
        "B6 must FAIL when the artifacts cannot be produced — a check that \
         cannot decide must not report a pass.\nstdout:\n{}\nstderr:\n{}",
        out,
        err
    );
    assert!(
        out.contains("BLOCKERS=1"),
        "B6 must count one blocker; stdout:\n{}",
        out
    );
    let _ = code;
}

/// B6 检查的产物路径，必须与生成命令写出的路径一致。
#[test]
fn test_5421_the_generated_artifact_paths_match_the_checked_ones() {
    let block = d2_b6_block();
    for name in ["sqlancer-report.json", "test-runner-report.json"] {
        assert!(block.contains(name), "the B6 block must mention {}", name);
        // 生成命令必须把产物写到「被检查的那个路径」上
        let checked = format!("target/{}", name);
        assert!(
            block.contains(&checked),
            "the B6 block must produce the artifact at the path it checks \
             ({}); it only checks it",
            checked
        );
    }
}

// =============================================================================
// #5425 — anti-ignore 门禁的阈值三处不一致
// =============================================================================

/// 跑一次 anti-ignore 门禁，取出它**实际生效**的阈值。
fn effective_thresholds() -> (u64, u64) {
    let out = Command::new("bash")
        .arg(repo_root().join("scripts/gate/check_anti_ignore_gate.sh"))
        .current_dir(repo_root())
        .output()
        .expect("run check_anti_ignore_gate.sh");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    // 输出形如：
    //   ignore_registry.json: total_allowed=116 (max=150), active=69 (max=80)
    // ⇒ 要取的是**上限**（`max=`），不是当前计数。
    let total = max_after(&text, "total_allowed=")
        .unwrap_or_else(|| panic!("cannot read the total_allowed ceiling from:\n{}", text));
    let active = max_after(&text, "active=")
        .unwrap_or_else(|| panic!("cannot read the active ceiling from:\n{}", text));
    (active, total)
}

/// 取 `key` 之后紧跟的 `(max=N)` 里的 N。
fn max_after(text: &str, key: &str) -> Option<u64> {
    let pos = text.find(key)?;
    let rest = &text[pos + key.len()..];
    let m = rest.find("(max=")?;
    let num: String = rest[m + 5..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    num.parse().ok()
}

/// 从文本里取 `key=<数字>` 里的数字。
fn extract_u64_after(text: &str, key: &str) -> Option<u64> {
    for line in text.lines() {
        if let Some(pos) = line.find(key) {
            let rest = &line[pos + key.len()..];
            let rest = rest.trim_start();
            let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !num.is_empty() {
                return num.parse().ok();
            }
        }
    }
    None
}

/// **跨产物一致性**：治理文档 `GATE_CONDITIONS.md` 的 G19 表格里写的阈值，
/// 必须与门禁脚本**实际生效**的阈值一致。
///
/// 修前：文档写 47 / 96，脚本用 80 / 150，脚本自己的文件头注释又写 55 / 100
/// ⇒ 同一个仓库、同一个时刻按三套阈值得到三个不同的结论。
#[test]
fn test_5425_the_governance_doc_states_the_thresholds_the_gate_actually_enforces() {
    let (active, total) = effective_thresholds();
    let doc = read_rel("docs/governance/GATE_CONDITIONS.md");
    let g19 = doc
        .split("### G19")
        .nth(1)
        .unwrap_or_else(|| panic!("no G19 section in GATE_CONDITIONS.md"));
    let g19 = &g19[..g19.len().min(1200)];

    let doc_active = extract_u64_after(g19, "active <=")
        .unwrap_or_else(|| panic!("G19 table has no `active <=` threshold:\n{}", g19));
    let doc_total = extract_u64_after(g19, "total_allowed <=")
        .unwrap_or_else(|| panic!("G19 table has no `total_allowed <=` threshold:\n{}", g19));

    assert_eq!(
        doc_active, active,
        "GATE_CONDITIONS.md G19 says `active <= {}` but the gate enforces \
         `active <= {}` — the same repository gets two different verdicts \
         depending on which document you read",
        doc_active, active
    );
    assert_eq!(
        doc_total, total,
        "GATE_CONDITIONS.md G19 says `total_allowed <= {}` but the gate \
         enforces `total_allowed <= {}`",
        doc_total, total
    );
}

/// 脚本的**文件头注释**也不得再与代码里的阈值分叉。
#[test]
fn test_5425_the_script_header_does_not_contradict_its_own_thresholds() {
    let text = read_rel("scripts/gate/check_anti_ignore_gate.sh");
    let header: String = text
        .lines()
        .take_while(|l| !l.trim_start().starts_with("set -"))
        .collect::<Vec<_>>()
        .join("\n");
    let code = strip_comments(&text);

    let mut claimed: Vec<(String, u64)> = Vec::new();
    for cap in [
        "active entries <=",
        "active <=",
        "total_allowed <=",
        "total allowed <=",
    ] {
        if let Some(v) = extract_u64_after(&header, cap) {
            claimed.push((cap.to_string(), v));
        }
    }
    for (cap, v) in claimed {
        assert!(
            code.contains(&format!("={}", v)) || code.contains(&format!("= {}", v)),
            "the script header claims `{} {}` but no threshold variable is set \
             to {} in the code — the header must not contradict the code",
            cap,
            v,
            v
        );
    }
}
