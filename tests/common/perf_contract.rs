//! 性能契约的登记与判定 —— 区分「关于实现的判据」与「关于机器的判据」。
//!
//! #5481：`tpch_json_vs_bint_compare` 断言「BINT v3 至少比 JSON 快 2 倍」，
//! 在真实硬件上**不可满足**（实测 0.92x–1.08x）⇒ 该目标结构性恒红。
//!
//! ## 根因：那条判据把「特定硬件上的观测值」当成了「实现的不变量」
//!
//! 阈值 2.0 的来源是 `docs/releases/v3.12.0/perf/BIN_LOAD_PERF.md` 记录的
//! 实测 **~3.4x**，而那份实测的硬件是：
//!
//! ```text
//! 2x Intel Xeon Gold 6138 @ 2.00GHz (40 cores / 80 threads),
//! 404GB RAM, 1.9TB NVMe SSD
//! ```
//!
//! 把同一份代码放到另一台机器上，两侧速率的关系会**反转**：
//!
//! | | 基准服务器（文档实测） | 本机实测 |
//! |---|---|---|
//! | JSON 写入 | 16 857 rows/s | 87 221 rows/s（**快 5.2 倍**） |
//! | BINT v3 写入 | 57 373 rows/s | 80 256 rows/s（慢 1.4 倍） |
//! | **比值** | **3.40x** | **0.92x** |
//!
//! ⇒ **比值本身也是关于机器的陈述**：JSON 路径是「一行一文件 + fsync」的
//! 写放大路径，在慢盘上被拖垮；BINT 是顺序二进制写，在快盘上优势收窄。
//! 两个速率都随硬件变化，**比值的符号都可能翻转**。
//!
//! ## 两条判据，两种性质
//!
//! 本模块把一个性能判据拆成两层，各自保留其成立的边界：
//!
//! | 层 | 判据 | 成立的边界 | 恒等式 |
//! |---|---|---|---|
//! | **普适不变量** | `ratio >= 0.5` | 任何硬件 | 是（关于实现的陈述） |
//! | **性能契约** | `ratio >= 2.0` | 仅限基准硬件 | 否（关于机器的陈述） |
//!
//! * 普适不变量**始终执行**：BINT 是为写入吞吐设计的存储格式，它的写入路径
//!   不应该在任何机器上比 JSON 路径慢一倍以上。0.5 是**退化保护线**，不是
//!   性能承诺 —— 它远低于「BINT 应该更快」的任何合理预期，因此不会因机器
//!   差异误报；但它足以抓住「BINT 写入路径相对 JSON 崩溃了」这类真实缺陷。
//! * 性能契约**只在基准硬件上执行**：当前机器是否就是基准机器，由
//!   `TPCH_PERF_BASELINE_HW` 环境变量**显式声明**（值必须与契约登记的
//!   `baseline_hw` 逐字相同）。不匹配 ⇒ 契约**未被执行**。
//!
//! ## 未被执行 ≠ 通过
//!
//! 契约未被执行时，判定结果是 [`ContractOutcome::NotEvaluated`]，
//! **不是** [`ContractOutcome::Evaluated`]。调用方**必须**把它披露出来：
//!
//! ```text
//! PERF-CONTRACT NOT EVALUATED: id=tpch_json_vs_bint_ratio reason=hardware_not_declared ...
//! ```
//!
//! 「未执行过的判据等于不存在的判据」（`AGENTS.md` §8）—— 所以
//! **未执行必须可见**，不能沉默地算作通过。
//!
//! # 纯函数
//!
//! 本模块不碰文件系统、不读环境变量：判定逻辑是纯函数，
//! [`evaluate_ratio_contract`] 的输入全部显式给出。这样「判据有没有判别力」
//! 本身可以被单元测试证明，而不必真的跑一次重负载（见本文件末尾的测试）。

#![allow(dead_code)]

/// 环境变量名：声明「当前机器就是某条性能契约的基准机器」。
///
/// 值为契约登记的 `baseline_hw` 逐字相同才视为匹配。
/// 未设置 ⇒ **所有**性能契约都处于「未执行」状态。
pub const BASELINE_HW_ENV: &str = "TPCH_PERF_BASELINE_HW";

/// 性能契约未执行时的披露前缀。
pub const NOT_EVALUATED_MARKER: &str = "PERF-CONTRACT NOT EVALUATED";

/// 一条被登记的性能契约。
#[derive(Debug, Clone, PartialEq)]
pub struct PerfContract {
    /// 契约 id（用于披露与基线文件）。
    pub id: &'static str,
    /// 契约要求的比值下界（观测值 / 基准值 ≥ 该值才算满足）。
    pub min_ratio: f64,
    /// 该契约成立所依赖的**基准硬件标识**。
    ///
    /// 只有当前机器的 [`BASELINE_HW_ENV`] 与它逐字相同，契约才被执行。
    pub baseline_hw: &'static str,
    /// 基准硬件上的实测比值（仅供披露与文档引用）。
    pub measured_on_baseline: f64,
}

/// 普适退化保护线：观测比值不得低于该值。
///
/// 依据：实测在非基准硬件上比值为 0.92x–1.08x，在基准硬件上为 3.40x。
/// 取 0.5 意味着「BINT 写入路径不得比 JSON 路径慢一倍以上」——
/// 这条陈述在任何机器上都成立（否则说明 BINT 路径本身坏了），
/// 且远低于任何合理的性能预期，因此不会因机器差异误报。
pub const UNIVERSAL_FLOOR_RATIO: f64 = 0.5;

/// 一条性能判据的判定结果。
#[derive(Debug, Clone, PartialEq)]
pub enum ContractOutcome {
    /// 契约**被执行**了，且观测值满足阈值。
    Evaluated {
        /// 契约 id。
        id: &'static str,
        /// 实测比值。
        observed: f64,
        /// 契约阈值。
        threshold: f64,
    },
    /// 契约**未被执行**（当前机器不是基准机器，或未声明基准机器）。
    ///
    /// 这**不是**通过。调用方必须披露。
    NotEvaluated {
        /// 契约 id。
        id: &'static str,
        /// 未执行的原因码（机器可读）。
        reason: &'static str,
        /// 人类可读的说明。
        detail: String,
    },
    /// 判据被执行了，且**不满足**。
    Failed {
        /// 契约 id。
        id: &'static str,
        /// 实测比值。
        observed: f64,
        /// 不满足的阈值。
        threshold: f64,
        /// 失败原因的机器可读说明。
        detail: String,
    },
}

impl ContractOutcome {
    /// 本次判定是否**执行**了契约本体（而非只执行了普适不变量）。
    pub fn contract_was_evaluated(&self) -> bool {
        !matches!(self, ContractOutcome::NotEvaluated { .. })
    }

    /// 本次判定是否通过（`NotEvaluated` **不算**通过）。
    pub fn is_pass(&self) -> bool {
        matches!(self, ContractOutcome::Evaluated { .. })
    }

    /// 构造披露行（`NotEvaluated` 时使用）。
    pub fn disclosure_line(&self) -> Option<String> {
        match self {
            ContractOutcome::NotEvaluated { id, reason, detail } => Some(format!(
                "{NOT_EVALUATED_MARKER}: id={id} reason={reason} detail={detail}"
            )),
            _ => None,
        }
    }
}

/// 判定一条性能契约。
///
/// # 参数
///
/// * `contract` —— 被登记的契约
/// * `observed` —— 本次实测的比值
/// * `declared_hw` —— 当前机器声明的基准硬件标识（来自
///   [`BASELINE_HW_ENV`]）。`None` 表示未声明。
///
/// # 判定顺序
///
/// 1. **普适不变量**（始终执行）：`observed < UNIVERSAL_FLOOR_RATIO` ⇒
///    [`ContractOutcome::Failed`]。这条不受硬件声明影响。
/// 2. **性能契约**（仅在 `declared_hw == contract.baseline_hw` 时执行）：
///    * 不匹配 ⇒ [`ContractOutcome::NotEvaluated`]（**不是通过**）
///    * 匹配 ⇒ `observed < contract.min_ratio` ⇒ [`ContractOutcome::Failed`]，
///      否则 [`ContractOutcome::Evaluated`]
pub fn evaluate_ratio_contract(
    contract: &PerfContract,
    observed: f64,
    declared_hw: Option<&str>,
) -> ContractOutcome {
    // 1. 普适不变量：任何硬件都成立，先于一切。
    if observed < UNIVERSAL_FLOOR_RATIO {
        return ContractOutcome::Failed {
            id: contract.id,
            observed,
            threshold: UNIVERSAL_FLOOR_RATIO,
            detail: format!(
                "universal floor violated: the BINT write path must not be more than \
                 {:.1}x slower than the JSON write path, observed {:.3}x \
                 (this bound is about the implementation, not about the machine)",
                1.0 / UNIVERSAL_FLOOR_RATIO,
                observed
            ),
        };
    }

    // 2. 性能契约：只在基准硬件上执行。
    match declared_hw {
        Some(hw) if hw == contract.baseline_hw => {
            if observed < contract.min_ratio {
                ContractOutcome::Failed {
                    id: contract.id,
                    observed,
                    threshold: contract.min_ratio,
                    detail: format!(
                        "performance contract violated on the declared baseline hardware \
                         ({}): required >= {}x, observed {:.3}x (baseline measurement \
                         was {:.2}x)",
                        contract.baseline_hw,
                        contract.min_ratio,
                        observed,
                        contract.measured_on_baseline
                    ),
                }
            } else {
                ContractOutcome::Evaluated {
                    id: contract.id,
                    observed,
                    threshold: contract.min_ratio,
                }
            }
        }
        Some(hw) => ContractOutcome::NotEvaluated {
            id: contract.id,
            reason: "hardware_not_baseline",
            detail: format!(
                "declared baseline hardware `{hw}` is not the hardware this contract was \
                 calibrated on (`{}`); the contract was NOT evaluated",
                contract.baseline_hw
            ),
        },
        None => ContractOutcome::NotEvaluated {
            id: contract.id,
            reason: "hardware_not_declared",
            detail: format!(
                "{} is unset: this machine is not declared to be the baseline hardware \
                 (`{}`), so the `>= {}x` contract was NOT evaluated",
                BASELINE_HW_ENV, contract.baseline_hw, contract.min_ratio
            ),
        },
    }
}

/// 从环境读取当前机器声明的基准硬件标识。
pub fn declared_baseline_hw() -> Option<String> {
    std::env::var(BASELINE_HW_ENV)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

// =========================================================================
// 判别力自证
// =========================================================================
//
// 下面这组测试**证明**判据有判别力。它们本身是绿的 —— 它们断言的是
// 「判据对什么输入会红、对什么输入会绿」，而不是「当前实现是好的」。
//
// 没有这组测试，一条恒绿的判据和一个有判别力的判据在 CI 上长得一模一样。

#[cfg(test)]
mod tests {
    use super::*;

    fn contract() -> PerfContract {
        PerfContract {
            id: "tpch_json_vs_bint_ratio",
            min_ratio: 2.0,
            baseline_hw: "2x Intel Xeon Gold 6138 @ 2.00GHz, 404GB RAM, 1.9TB NVMe SSD",
            measured_on_baseline: 3.40,
        }
    }

    const BASELINE: &str = "2x Intel Xeon Gold 6138 @ 2.00GHz, 404GB RAM, 1.9TB NVMe SSD";

    /// 旧判据（`elapsed >= 40ms` 这类极松下界 / 硬编码倍数）放过的输入，
    /// 新判据必须判红。⇒ 判别力自证。
    #[test]
    fn a_severely_slow_bint_path_is_rejected_on_any_machine() {
        // 0.3x：BINT 写入比 JSON 慢三倍多。
        let out = evaluate_ratio_contract(&contract(), 0.30, None);
        assert!(
            matches!(out, ContractOutcome::Failed { .. }),
            "a 0.30x ratio must be rejected even with no baseline hardware declared, got {out:?}"
        );
    }

    /// 未声明基准硬件 ⇒ 契约**未被执行**，且**不算通过**。
    #[test]
    fn contract_is_not_evaluated_without_a_declared_baseline() {
        let out = evaluate_ratio_contract(&contract(), 0.92, None);
        assert!(
            matches!(out, ContractOutcome::NotEvaluated { .. }),
            "without a declared baseline the contract must not be evaluated, got {out:?}"
        );
        assert!(
            !out.is_pass(),
            "an unevaluated contract must never count as a pass"
        );
        assert!(
            !out.contract_was_evaluated(),
            "an unevaluated contract must not be reported as evaluated"
        );
    }

    /// 未执行 ⇒ 一定有披露行，且披露行含契约 id 与原因码。
    #[test]
    fn an_unevaluated_contract_always_produces_a_disclosure_line() {
        let out = evaluate_ratio_contract(&contract(), 0.92, None);
        let line = out
            .disclosure_line()
            .expect("an unevaluated contract must produce a disclosure line");
        assert!(line.starts_with(NOT_EVALUATED_MARKER), "got {line}");
        assert!(line.contains("id=tpch_json_vs_bint_ratio"), "got {line}");
        assert!(line.contains("reason=hardware_not_declared"), "got {line}");
    }

    /// 声明了**错误的**基准硬件 ⇒ 仍未执行（且原因码不同）。
    #[test]
    fn contract_is_not_evaluated_on_a_different_machine() {
        let out = evaluate_ratio_contract(&contract(), 3.90, Some("some other laptop"));
        assert!(
            matches!(out, ContractOutcome::NotEvaluated { .. }),
            "a different machine must not evaluate the contract, got {out:?}"
        );
        match out {
            ContractOutcome::NotEvaluated { reason, .. } => {
                assert_eq!(reason, "hardware_not_baseline");
            }
            other => panic!("expected NotEvaluated, got {other:?}"),
        }
    }

    /// 在基准硬件上，未达阈值 ⇒ 判红（这才是「2x 契约真的会红」的自证）。
    #[test]
    fn on_the_baseline_hardware_the_two_x_contract_actually_fails() {
        let out = evaluate_ratio_contract(&contract(), 1.08, Some(BASELINE));
        assert!(
            matches!(out, ContractOutcome::Failed { .. }),
            "on the baseline hardware a 1.08x ratio must fail the 2x contract, got {out:?}"
        );
        assert!(!out.is_pass());
    }

    /// 在基准硬件上，达到阈值 ⇒ 通过。
    #[test]
    fn on_the_baseline_hardware_the_two_x_contract_passes() {
        let out = evaluate_ratio_contract(&contract(), 3.40, Some(BASELINE));
        assert!(out.is_pass(), "expected pass, got {out:?}");
        assert!(out.contract_was_evaluated());
    }

    /// 非基准机器上，**即使比值高达 10x**，契约仍未执行 ——
    /// 防止「用一台快机器的漂亮数字冒充契约已验证」。
    #[test]
    fn a_high_ratio_on_a_non_baseline_machine_is_still_not_evaluated() {
        let out = evaluate_ratio_contract(&contract(), 10.0, None);
        assert!(
            matches!(out, ContractOutcome::NotEvaluated { .. }),
            "a high ratio on an undeclared machine must not be reported as an \
             evaluated contract, got {out:?}"
        );
    }

    /// 普适不变量先于契约判定：0.3x 在基准硬件上也判红。
    #[test]
    fn the_universal_floor_applies_even_on_the_baseline_hardware() {
        let out = evaluate_ratio_contract(&contract(), 0.3, Some(BASELINE));
        match out {
            ContractOutcome::Failed { threshold, .. } => {
                assert_eq!(threshold, UNIVERSAL_FLOOR_RATIO);
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }
}
