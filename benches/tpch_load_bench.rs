//! L4: TPC-H SF=1 load throughput benchmark for BINT v3 binary storage.
//!
//! Measures `BinaryTableStorageV2::insert_streaming + flush` wall-time for
//! 1M TPC-H lineitem rows. Extrapolates to 6M for SF=1.
//!
//! Run: cargo bench --bench tpch_load_bench
//!
//! Schema uses BIGINT for all integer/date columns and Float/Text for the
//! rest to match the actual `Value` enum variants. See task-6-3-brief.md
//! for the full mapping rationale.

use criterion::{criterion_group, criterion_main, Criterion};
use sqlrustgo_storage::binary_storage_v2::BinaryTableStorageV2;
use sqlrustgo_storage::engine::{ColumnDefinition, Record, Value};
use tempfile::TempDir;

fn lineitem_schema() -> Vec<ColumnDefinition> {
    vec![
        ColumnDefinition::new("l_orderkey", "BIGINT"),
        ColumnDefinition::new("l_partkey", "BIGINT"),
        ColumnDefinition::new("l_suppkey", "BIGINT"),
        ColumnDefinition::new("l_linenumber", "BIGINT"), // INT in TPC-H; BIGINT avoids width trap
        ColumnDefinition::new("l_quantity", "DOUBLE PRECISION"),
        ColumnDefinition::new("l_extendedprice", "DOUBLE PRECISION"),
        ColumnDefinition::new("l_discount", "DOUBLE PRECISION"),
        ColumnDefinition::new("l_tax", "DOUBLE PRECISION"),
        ColumnDefinition::new("l_returnflag", "TEXT"),
        ColumnDefinition::new("l_linestatus", "TEXT"),
        ColumnDefinition::new("l_shipdate", "BIGINT"), // encoded as days since epoch
        ColumnDefinition::new("l_commitdate", "BIGINT"),
        ColumnDefinition::new("l_receiptdate", "BIGINT"),
        ColumnDefinition::new("l_shipinstruct", "TEXT"),
        ColumnDefinition::new("l_shipmode", "TEXT"),
        ColumnDefinition::new("l_comment", "TEXT"),
    ]
}

fn bench_lineitem_1m(c: &mut Criterion) {
    c.bench_function("lineitem_load_1m", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().expect("tempdir");
            let mut storage =
                BinaryTableStorageV2::new(temp_dir.path().to_path_buf()).expect("V2 init");
            storage
                .create_table("lineitem", lineitem_schema())
                .expect("create_table");

            // Split into batches to fit within 64MB segment limit (~500K rows per segment)
            let batch_size = 100_000;
            for batch_start in (0..1_000_000).step_by(batch_size) {
                let batch_end = (batch_start + batch_size).min(1_000_000);
                let records: Vec<Record> = (batch_start..batch_end)
                    .map(|i| {
                        vec![
                            Value::Integer(i as i64),          // l_orderkey
                            Value::Integer(i as i64),          // l_partkey
                            Value::Integer(i as i64),          // l_suppkey
                            Value::Integer(1i64),              // l_linenumber
                            Value::Float(1.0),                 // l_quantity
                            Value::Float(1000.0),              // l_extendedprice
                            Value::Float(0.05),                // l_discount
                            Value::Float(0.01),                // l_tax
                            Value::Text("R".to_string()),      // l_returnflag
                            Value::Text("F".to_string()),      // l_linestatus
                            Value::Integer((9000 + (i % 1000)) as i64), // l_shipdate
                            Value::Integer((9100 + (i % 1000)) as i64), // l_commitdate
                            Value::Integer((9150 + (i % 1000)) as i64), // l_receiptdate
                            Value::Text("N".to_string()),     // l_shipinstruct - abbreviated
                            Value::Text("A".to_string()),      // l_shipmode - abbreviated
                            Value::Text("x".to_string()),      // l_comment - minimal
                        ]
                    })
                    .collect();

                storage.insert_streaming("lineitem", records).expect("insert");
                storage.flush().expect("flush");
            }
        })
    });
}

criterion_group!(benches, bench_lineitem_1m);
criterion_main!(benches);
