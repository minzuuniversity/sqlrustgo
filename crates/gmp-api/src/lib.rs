//! GMP Management API
//!
//! REST/GraphQL API for GMP Management Suite (v3.4.0)
//!
//! Provides:
//! - Dashboard API: Compliance score, audit chains, batches
//! - Batch API: EBR (Electronic Batch Record) lifecycle
//! - Audit API: Audit chain management and signing
//! - Device API: OPC UA / MQTT / Modbus integration
//! - Signature API: Electronic signature service (21 CFR Part 11)
//! - Export API: JSON/PDF audit package export

pub mod audit;
pub mod batch;
pub mod dashboard;
pub mod device;
pub mod export;
pub mod retrieval;
pub mod rule_engine;
pub mod signature;

pub mod ai;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    pub page: u32,
    pub page_size: u32,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 20,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u64,
}

impl<T> PaginatedResponse<T> {
    pub fn new(items: Vec<T>, total: u64, params: &PaginationParams) -> Self {
        let total_pages = (total as f64 / params.page_size as f64).ceil() as u64;
        Self {
            items,
            total,
            page: params.page,
            page_size: params.page_size,
            total_pages,
        }
    }
}
