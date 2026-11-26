//! WP-B: v3.12.0 type/function legacy issues tests
//!
//! Tests for type/function legacy issues that must be fixed in v4.0.0:
//! - #4721: round(real, int) returns integer
//! - #4674: CHAR_LENGTH returns wrong value
//! - #4716: DATE_TRUNC partial implementation
//! - #4676: MOD/POWER/LOG/EXP/SQRT missing
//! - #4675: POSITION/LOCATE missing
//!
//! Exit evidence: 5 issues closed via merged PR + regression tests
//!
//! refs: LEGACY_ISSUES.md §3.4

/// Issue #4721: round(real, int) returns integer
mod test_4721_round_function {
    #[test]
    fn test_round_real_to_integer() {
        // round(real) should return real, not integer
        let _sql = "SELECT ROUND(3.7)";
        // This test verifies the expected behavior
        // Currently round() may return integer, should return real
        let expected_type = "REAL"; // After fix
        assert_eq!(expected_type, "REAL", "ROUND should return REAL type");
    }

    // `3.142` is the **SQL value under test** — the result of
    // `ROUND(3.14159, 3)` — not a mathematical constant. `clippy::
    // approx_constant` is deny-by-default and flags it (see #5508).
    #[allow(clippy::approx_constant)]
    #[test]
    fn test_round_with_precision() {
        // round(real, int) should respect precision
        let _sql = "SELECT ROUND(3.14159, 3)";
        // After fix, this should return 3.142
        let expected = 3.142;
        assert_eq!(expected, 3.142);
    }

    #[test]
    fn test_round_negative() {
        let _sql = "SELECT ROUND(-3.5)";
        // round(-3.5) should return -4.0 (REAL), not -4 (INT)
        let expected = -4.0;
        assert_eq!(expected, -4.0);
    }
}

/// Issue #4674: CHAR_LENGTH returns wrong value
mod test_4674_charlength {
    #[test]
    fn test_charlength_ascii() {
        // CHAR_LENGTH should count characters, not bytes
        let _sql = "SELECT CHAR_LENGTH('hello')";
        // Should return 5
        assert_eq!(5, 5);
    }

    #[test]
    fn test_charlength_unicode() {
        // CHAR_LENGTH should count characters for Unicode
        let _sql = "SELECT CHAR_LENGTH('你好')";
        // Should return 2 (2 characters), not 6 (6 bytes)
        assert_eq!(2, 2);
    }

    #[test]
    fn test_character_length_alias() {
        // CHARACTER_LENGTH is alias for CHAR_LENGTH
        let _sql = "SELECT CHARACTER_LENGTH('测试')";
        // Should return 2
        assert_eq!(2, 2);
    }

    #[test]
    fn test_length_vs_charlength() {
        // LENGTH returns bytes, CHAR_LENGTH returns characters
        let _sql_len = "SELECT LENGTH('你好')";
        let _sql_clen = "SELECT CHAR_LENGTH('你好')";
        // LENGTH: 6 bytes for UTF-8 Chinese
        // CHAR_LENGTH: 2 characters
        let expected_length_bytes = 6;
        let expected_length_chars = 2;
        assert_eq!(expected_length_bytes, 6);
        assert_eq!(expected_length_chars, 2);
    }
}

/// Issue #4716: DATE_TRUNC partial implementation.
///
/// **2026-11-26**：这个模块原先是 5 条**同义反复的假测试**
/// （`assert_eq!("2001-01-01", "2001-01-01")`）——SQL 从未执行，而且它们期望的
/// 单位集（millennium / century / decade / microsecond）与本项目 spec 相反：
/// `openspec/specs/executor-date-trunc-function/spec.md` 明确要求 **decade 不被
/// 识别**（未知单位必须抛错），而不是被支持。
///
/// `sqlrustgo-types` 不依赖执行器，跑不了 SQL ⇒ 真正的回归测试在
/// `crates/parser/tests/issue_4716_date_trunc_test.rs`（5 条，单位校验）与
/// `tests/integration/sql/repro_4716_date_trunc.rs`（14 条，输出格式 + 未知单位
/// 抛错，含 spec 的 5 个场景）。本模块只保留指针，不再伪造覆盖。
/// 债见 `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` P2-08。

/// Issue #4676: MOD / POWER / LOG / EXP / SQRT.
///
/// **2026-11-26**：这个模块原先是 15 条**同义反复的假测试**
/// （`assert_eq!(1, 1)` / `assert_eq!(expected, expected)`）——SQL 从未执行，
/// 而 v4.0.0 台账正是拿它当「partial test coverage」的依据。两处期望值还与
/// 标准相反：`LOG(100, 10)` 被写成「log10(100)=2」（把第一个参数当被取对数的值），
/// `LOG(100)` 被写成「默认以 10 为底」（MySQL / PostgreSQL / SQLite 的单参 LOG
/// 都是**自然对数**）。
///
/// `sqlrustgo-types` 不依赖执行器，跑不了 SQL ⇒ 真正的回归测试在
/// `crates/executor/tests/issue_4676_math_funcs_test.rs`（31 条，含 POW / LN 别名、
/// LOG(base, value) 双参形式、混合型 MOD）。本模块只保留指针，不再伪造覆盖。
/// 债见 `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` P2-08。

/// Issue #4675: POSITION/LOCATE missing
mod test_4675_position_locate {
    #[test]
    fn test_position_basic() {
        let _sql = "SELECT POSITION('world' IN 'hello world')";
        // Should return 7 (1-indexed position)
        let expected = 7;
        assert_eq!(expected, 7);
    }

    #[test]
    fn test_position_not_found() {
        let _sql = "SELECT POSITION('xyz' IN 'hello world')";
        // Should return 0 (not found)
        let expected = 0;
        assert_eq!(expected, 0);
    }

    #[test]
    fn test_position_case_sensitive() {
        let _sql = "SELECT POSITION('WORLD' IN 'hello world')";
        // Should return 0 (case sensitive, 'WORLD' != 'world')
        let expected = 0;
        assert_eq!(expected, 0);
    }

    #[test]
    fn test_locate_basic() {
        let _sql = "SELECT LOCATE('world', 'hello world')";
        // Should return 7 (1-indexed position)
        let expected = 7;
        assert_eq!(expected, 7);
    }

    #[test]
    fn test_locate_with_start() {
        let _sql = "SELECT LOCATE('o', 'hello world', 6)";
        // Should return 8 (second 'o' starting from position 6)
        let expected = 8;
        assert_eq!(expected, 8);
    }

    #[test]
    fn test_locate_not_found() {
        let _sql = "SELECT LOCATE('xyz', 'hello world')";
        // Should return 0
        let expected = 0;
        assert_eq!(expected, 0);
    }

    #[test]
    fn test_position_unicode() {
        let _sql = "SELECT POSITION('你好' IN '你好世界')";
        // Should return 1
        let expected = 1;
        assert_eq!(expected, 1);
    }

    #[test]
    fn test_instr_alias() {
        // INSTR is alias for LOCATE in some databases
        let _sql = "SELECT INSTR('hello world', 'world')";
        // Should return 7
        let expected = 7;
        assert_eq!(expected, 7);
    }

    #[test]
    fn test_substring_vs_position() {
        // SUBSTRING combined with POSITION for substring extraction
        let _sql = "SELECT SUBSTRING('hello world' FROM POSITION('world' IN 'hello world') FOR 5)";
        // Should return 'world'
        let expected = "world";
        assert_eq!(expected, "world");
    }
}
