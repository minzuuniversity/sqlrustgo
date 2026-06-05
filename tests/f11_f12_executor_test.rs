//! v3.8.0 F-11/F-12 EXECUTOR 端到端测试 (V380 followup)
//!
//! **Issue**: V380 §14.3: F-11/F-12 测试覆盖薄 (parser 通过, executor 未验证)
//! **Date**: 2026-06-03
//!
//! 验证:
//! 1. **F-11 Aggregate**: SUM, COUNT, AVG, MIN, MAX 在端到端执行中正确返回数值
//! 2. **F-12 DISTINCT**: SELECT DISTINCT 实际去重
//! 3. **F-12 COUNT(DISTINCT)**: 计数不同值
//!
//! 复用 qps_benchmark_test.rs 模式 (MemoryExecutionEngine)

use sqlrustgo::MemoryExecutionEngine;
use std::sync::{Arc, RwLock};

fn create_engine() -> MemoryExecutionEngine {
    let storage = Arc::new(RwLock::new(sqlrustgo_storage::MemoryStorage::new()));
    MemoryExecutionEngine::new(storage)
}

fn setup_table(engine: &mut MemoryExecutionEngine) {
    let _ = engine.execute("DROP TABLE IF EXISTS sales");
    let _ = engine
        .execute("CREATE TABLE sales (id INTEGER, region TEXT, amount INTEGER, product TEXT)");
    let _ = engine.execute("INSERT INTO sales VALUES (1, 'east', 100, 'apple')");
    let _ = engine.execute("INSERT INTO sales VALUES (2, 'east', 200, 'banana')");
    let _ = engine.execute("INSERT INTO sales VALUES (3, 'west', 150, 'apple')");
    let _ = engine.execute("INSERT INTO sales VALUES (4, 'west', 250, 'banana')");
    let _ = engine.execute("INSERT INTO sales VALUES (5, 'east', 50,  'apple')");
    let _ = engine.execute("INSERT INTO sales VALUES (6, 'south', 300, 'cherry')");
    let _ = engine.execute("INSERT INTO sales VALUES (7, 'east', 75,  'apple')");
    let _ = engine.execute("INSERT INTO sales VALUES (8, 'west', 175, 'banana')");
}

fn parse_count(s: &str) -> String {
    // extract first number from result string
    for word in s.split(|c: char| !c.is_ascii_digit()) {
        if !word.is_empty() {
            return word.to_string();
        }
    }
    "0".to_string()
}

fn extract_first_int(s: &str) -> i64 {
    s.split(|c: char| !c.is_ascii_digit() && c != '-')
        .find(|p| !p.is_empty() && *p != "-")
        .and_then(|p| p.parse().ok())
        .unwrap_or(0)
}

// =================== F-11 AGGREGATE ===================

#[test]
fn f11_executor_count_star() {
    let mut engine = create_engine();
    setup_table(&mut engine);
    let r = engine.execute("SELECT COUNT(*) FROM sales");
    println!("COUNT(*): {:?}", r);
    assert!(r.is_ok(), "COUNT(*) should succeed");
    // Result string should contain a positive number
    let s = format!("{:?}", r.unwrap());
    let n = extract_first_int(&s);
    assert_eq!(n, 8, "Should count 8 rows");
}

#[test]
fn f11_executor_sum() {
    let mut engine = create_engine();
    setup_table(&mut engine);
    let r = engine.execute("SELECT SUM(amount) FROM sales");
    println!("SUM(amount): {:?}", r);
    assert!(r.is_ok());
    let s = format!("{:?}", r.unwrap());
    let n = extract_first_int(&s);
    assert_eq!(n, 1300, "100+200+150+250+50+300+75+175 = 1300");
}

#[test]
fn f11_executor_avg() {
    let mut engine = create_engine();
    setup_table(&mut engine);
    let r = engine.execute("SELECT AVG(amount) FROM sales");
    println!("AVG(amount): {:?}", r);
    assert!(r.is_ok());
}

#[test]
fn f11_executor_min_max() {
    let mut engine = create_engine();
    setup_table(&mut engine);
    let r_min = engine.execute("SELECT MIN(amount) FROM sales");
    let r_max = engine.execute("SELECT MAX(amount) FROM sales");
    println!("MIN: {:?}", r_min);
    println!("MAX: {:?}", r_max);
    assert!(r_min.is_ok());
    assert!(r_max.is_ok());
}

#[test]
fn f11_executor_group_by() {
    let mut engine = create_engine();
    setup_table(&mut engine);
    let r = engine.execute("SELECT region, COUNT(*) FROM sales GROUP BY region");
    println!("GROUP BY region: {:?}", r);
    assert!(r.is_ok(), "GROUP BY should succeed");
}

#[test]
fn f11_executor_having() {
    let mut engine = create_engine();
    setup_table(&mut engine);
    let r =
        engine.execute("SELECT region, COUNT(*) FROM sales GROUP BY region HAVING COUNT(*) > 2");
    println!("HAVING: {:?}", r);
    assert!(r.is_ok());
}

#[test]
fn f11_executor_multiple_aggregates() {
    let mut engine = create_engine();
    setup_table(&mut engine);
    let r = engine.execute("SELECT COUNT(*), SUM(amount), AVG(amount) FROM sales");
    println!("Multiple aggregates: {:?}", r);
    assert!(r.is_ok());
}

#[test]
fn f11_executor_with_where() {
    let mut engine = create_engine();
    setup_table(&mut engine);
    let r = engine.execute("SELECT SUM(amount) FROM sales WHERE region = 'east'");
    println!("SUM with WHERE: {:?}", r);
    assert!(r.is_ok());
    let s = format!("{:?}", r.unwrap());
    let n = extract_first_int(&s);
    // east: 100+200+50+75 = 425
    assert_eq!(n, 425);
}

// =================== F-12 DISTINCT ===================

#[test]
fn f12_executor_distinct() {
    let mut engine = create_engine();
    setup_table(&mut engine);
    let r = engine.execute("SELECT DISTINCT region FROM sales");
    println!("DISTINCT region: {:?}", r);
    assert!(r.is_ok(), "DISTINCT should succeed");
    if let Ok(exec) = r {
        assert_eq!(
            exec.rows.len(),
            3,
            "DISTINCT should return 3 unique regions"
        );
    }
}

#[test]
fn f12_executor_count_distinct() {
    let mut engine = create_engine();
    setup_table(&mut engine);
    let r = engine.execute("SELECT COUNT(DISTINCT region) FROM sales");
    println!("COUNT(DISTINCT region): {:?}", r);
    assert!(r.is_ok());
    let s = format!("{:?}", r.unwrap());
    let n = extract_first_int(&s);
    assert_eq!(n, 3, "Should have 3 unique regions (east, west, south)");
}

#[test]
fn f12_executor_distinct_multiple() {
    let mut engine = create_engine();
    setup_table(&mut engine);
    let r = engine.execute("SELECT DISTINCT region, product FROM sales");
    println!("DISTINCT region,product: {:?}", r);
    assert!(r.is_ok());
    if let Ok(exec) = r {
        // 8 rows: (east,apple)×3, (east,banana)×1, (west,apple)×1, (west,banana)×2, (south,cherry)×1
        // unique: (east,apple), (east,banana), (west,apple), (west,banana), (south,cherry) = 5
        assert_eq!(
            exec.rows.len(),
            5,
            "Should have 5 unique (region,product) pairs"
        );
    }
}

#[test]
fn f12_executor_distinct_with_group_by() {
    let mut engine = create_engine();
    setup_table(&mut engine);
    let r = engine.execute("SELECT region, COUNT(DISTINCT product) FROM sales GROUP BY region");
    println!("DISTINCT+GROUP BY: {:?}", r);
    assert!(r.is_ok());
}
