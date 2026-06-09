//! Cost Optimizer Harness (P3-3 #3182)
//!
//! Shared utilities for Cost-Based Optimizer (CBO) testing. Provides:
//! - PlanKind enum (SeqScan, IndexScan, HashJoin, NestedLoop, Sort)
//! - MockPlan (mirrors the real plan node structure used by
//!   crates/optimizer/src/{cost,unified_cost}.rs)
//! - estimate_cost — compute cost using the same formula family as
//!   SimpleCostModel
//! - choose_best_plan — pick the cheapest of N plans
//!
//! This file is **not** a test target itself (no `#[test]`); shared
//! by `cost_optimizer_test.rs`.

#![allow(dead_code)] // helpers consumed by test targets

/// Plan node kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanKind {
    SeqScan,
    IndexScan,
    HashJoin,
    NestedLoop,
    Sort,
}

/// Mock plan node. Only the fields relevant to cost estimation
/// are stored.
#[derive(Debug, Clone, PartialEq)]
pub struct MockPlan {
    pub kind: PlanKind,
    pub row_count: u64,
    pub page_count: u64,
    pub index_pages: u64,
    pub data_pages: u64,
    pub left_rows: u64,
    pub right_rows: u64,
    pub avg_row_size: u32,
    pub selectivity: f64,
}

impl MockPlan {
    pub fn seq_scan(row_count: u64, page_count: u64) -> Self {
        Self {
            kind: PlanKind::SeqScan,
            row_count,
            page_count,
            index_pages: 0,
            data_pages: 0,
            left_rows: 0,
            right_rows: 0,
            avg_row_size: 0,
            selectivity: 1.0,
        }
    }

    pub fn index_scan(row_count: u64, index_pages: u64, data_pages: u64) -> Self {
        Self {
            kind: PlanKind::IndexScan,
            row_count,
            page_count: 0,
            index_pages,
            data_pages,
            left_rows: 0,
            right_rows: 0,
            avg_row_size: 0,
            selectivity: 1.0,
        }
    }

    pub fn hash_join(left_rows: u64, right_rows: u64) -> Self {
        Self {
            kind: PlanKind::HashJoin,
            row_count: 0,
            page_count: 0,
            index_pages: 0,
            data_pages: 0,
            left_rows,
            right_rows,
            avg_row_size: 0,
            selectivity: 1.0,
        }
    }

    pub fn nested_loop(left_rows: u64, right_rows: u64) -> Self {
        Self {
            kind: PlanKind::NestedLoop,
            row_count: 0,
            page_count: 0,
            index_pages: 0,
            data_pages: 0,
            left_rows,
            right_rows,
            avg_row_size: 0,
            selectivity: 1.0,
        }
    }

    pub fn sort(row_count: u64, avg_row_size: u32) -> Self {
        Self {
            kind: PlanKind::Sort,
            row_count,
            page_count: 0,
            index_pages: 0,
            data_pages: 0,
            left_rows: 0,
            right_rows: 0,
            avg_row_size,
            selectivity: 1.0,
        }
    }

    pub fn with_selectivity(mut self, sel: f64) -> Self {
        self.selectivity = sel;
        self
    }
}

/// Cost estimate for a plan.
#[derive(Debug, Clone, PartialEq)]
pub struct CostEstimate {
    pub plan_kind: PlanKind,
    pub cost: f64,
    pub selectivity: f64,
}

impl CostEstimate {
    pub fn passed(&self) -> bool {
        self.cost >= 0.0
            && self.cost.is_finite()
            && self.selectivity > 0.0
            && self.selectivity <= 1.0
    }
}

// Cost model parameters (mirrors SimpleCostModel::default_model).
const DEFAULT_CPU_COST_PER_ROW: f64 = 0.01;
const DEFAULT_IO_COST_PER_PAGE: f64 = 1.0;
const DEFAULT_NETWORK_COST_PER_BYTE: f64 = 0.001;

/// Estimate cost using the same formula family as
/// `SimpleCostModel::seq_scan_cost` etc.
pub fn estimate_cost(plan: &MockPlan) -> CostEstimate {
    let cost = match plan.kind {
        PlanKind::SeqScan => {
            // pages * seq_io_cost
            plan.page_count as f64 * DEFAULT_IO_COST_PER_PAGE
                + plan.row_count as f64 * DEFAULT_CPU_COST_PER_ROW
        }
        PlanKind::IndexScan => {
            // log(N) * random_io_cost + data pages
            let n = (plan.row_count as f64).max(2.0);
            let log_n = n.log2();
            plan.index_pages as f64 * log_n * DEFAULT_IO_COST_PER_PAGE * 2.0
                + plan.data_pages as f64 * DEFAULT_IO_COST_PER_PAGE
        }
        PlanKind::HashJoin => {
            // (outer + inner) * hash_cost
            (plan.left_rows + plan.right_rows) as f64 * DEFAULT_CPU_COST_PER_ROW * 5.0
        }
        PlanKind::NestedLoop => {
            // outer * inner * index_cost
            (plan.left_rows as f64) * (plan.right_rows as f64) * DEFAULT_CPU_COST_PER_ROW
        }
        PlanKind::Sort => {
            // N * log(N) * sort_cost
            let n = (plan.row_count as f64).max(2.0);
            n * n.log2() * DEFAULT_CPU_COST_PER_ROW * 2.0
        }
    };

    // Apply selectivity
    let cost = cost * plan.selectivity.max(0.0001);

    CostEstimate {
        plan_kind: plan.kind,
        cost,
        selectivity: plan.selectivity,
    }
}

/// Choose the cheapest plan from a list.
pub fn choose_best_plan(plans: &[MockPlan]) -> Option<(MockPlan, CostEstimate)> {
    if plans.is_empty() {
        return None;
    }
    let mut best: Option<(MockPlan, CostEstimate)> = None;
    for p in plans {
        let est = estimate_cost(p);
        match &best {
            None => best = Some((p.clone(), est)),
            Some((_, b_est)) if est.cost < b_est.cost => best = Some((p.clone(), est)),
            _ => {}
        }
    }
    best
}

#[cfg(test)]
mod harness_tests {
    use super::*;

    #[test]
    fn seq_scan_cost_positive() {
        let p = MockPlan::seq_scan(1000, 10);
        let e = estimate_cost(&p);
        assert!(e.cost > 0.0);
        assert_eq!(e.plan_kind, PlanKind::SeqScan);
    }

    #[test]
    fn index_scan_cheaper_for_small_selectivity() {
        let seq = MockPlan::seq_scan(10_000, 100);
        let idx = MockPlan::index_scan(100, 5, 1).with_selectivity(0.01);
        let seq_e = estimate_cost(&seq);
        let idx_e = estimate_cost(&idx);
        assert!(idx_e.cost < seq_e.cost);
    }

    #[test]
    fn hash_join_cheaper_than_nested_loop_for_large() {
        let hash = MockPlan::hash_join(10_000, 10_000);
        let nested = MockPlan::nested_loop(10_000, 10_000);
        let hash_e = estimate_cost(&hash);
        let nested_e = estimate_cost(&nested);
        assert!(hash_e.cost < nested_e.cost);
    }

    #[test]
    fn sort_cost_grows_with_n() {
        let small = MockPlan::sort(100, 100);
        let large = MockPlan::sort(10_000, 100);
        assert!(estimate_cost(&large).cost > estimate_cost(&small).cost);
    }

    #[test]
    fn choose_best_plan_returns_lowest_cost() {
        let plans = vec![
            MockPlan::seq_scan(10_000, 100),
            MockPlan::index_scan(10, 2, 1).with_selectivity(0.001),
            MockPlan::hash_join(10_000, 10_000),
        ];
        let (best, est) = choose_best_plan(&plans).unwrap();
        assert_eq!(best.kind, PlanKind::IndexScan);
        assert!(est.passed());
    }
}
