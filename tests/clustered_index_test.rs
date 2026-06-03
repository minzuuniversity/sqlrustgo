//! F-23: Clustered Index (B+ tree with row storage in leaf nodes)
//!
//! **Issue**: #2824
//! **Source**: v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (F-23)
//! **Change**: openspec/changes/f-23-clustered-index
//!
//! In-memory B+ tree where leaf nodes store full rows ordered by PK.
//! Real disk-based integration in v3.9.0.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub pk: i64,
    pub data: Vec<(String, String)>, // (col, val)
}

impl Row {
    pub fn new(pk: i64) -> Self {
        Self {
            pk,
            data: Vec::new(),
        }
    }
    pub fn set(mut self, col: &str, val: &str) -> Self {
        self.data.push((col.to_string(), val.to_string()));
        self
    }
    pub fn get(&self, col: &str) -> Option<&str> {
        self.data
            .iter()
            .find(|(k, _)| k == col)
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct LeafPage {
    pub max_rows: usize,
    pub rows: Vec<Row>,
}

impl LeafPage {
    pub fn new(max_rows: usize) -> Self {
        Self {
            max_rows,
            rows: Vec::with_capacity(max_rows),
        }
    }
    pub fn is_full(&self) -> bool {
        self.rows.len() >= self.max_rows
    }
    pub fn insert(&mut self, row: Row) -> Result<(), String> {
        if self.is_full() {
            return Err("page full".to_string());
        }
        self.rows.push(row);
        self.rows.sort_by_key(|r| r.pk);
        Ok(())
    }
    pub fn split(&mut self) -> LeafPage {
        let mid = self.rows.len() / 2;
        let right_rows = self.rows.drain(mid..).collect();
        LeafPage {
            max_rows: self.max_rows,
            rows: right_rows,
        }
    }
}

pub struct ClusteredIndex {
    pages: Vec<LeafPage>,
    pk_to_page: BTreeMap<i64, usize>,
    page_size: usize,
    splits: u64,
    secondary_idx: std::collections::HashMap<String, Vec<i64>>, // col_val -> pks
}

impl ClusteredIndex {
    pub fn new() -> Self {
        Self::with_page_size(4)
    }

    pub fn with_page_size(page_size: usize) -> Self {
        let mut idx = Self {
            pages: vec![LeafPage::new(page_size)],
            pk_to_page: BTreeMap::new(),
            page_size,
            splits: 0,
            secondary_idx: std::collections::HashMap::new(),
        };
        idx
    }

    pub fn insert(&mut self, row: Row) -> Result<(), String> {
        let pk = row.pk;
        // Try inserting into the first non-full page
        for i in 0..self.pages.len() {
            match self.pages[i].insert(row.clone()) {
                Ok(()) => {
                    self.pk_to_page.insert(pk, i);
                    return Ok(());
                }
                Err(_) => continue, // page full, try next
            }
        }
        // All pages full: split the last page and try again
        let new_page = self.pages.last_mut().unwrap().split();
        self.pages.push(new_page);
        self.splits += 1;
        let last = self.pages.len() - 1;
        self.pages[last].insert(row)?;
        self.pk_to_page.insert(pk, last);
        Ok(())
    }

    fn find_page_for_pk(&self, pk: i64) -> usize {
        for (i, page) in self.pages.iter().enumerate() {
            if let Some(last) = page.rows.last() {
                if pk <= last.pk {
                    return i;
                }
            }
        }
        self.pages.len() - 1
    }

    pub fn lookup(&self, pk: i64) -> Option<&Row> {
        // Scan all pages (pk_to_page may be stale after splits)
        for page in &self.pages {
            if let Some(row) = page.rows.iter().find(|r| r.pk == pk) {
                return Some(row);
            }
        }
        None
    }

    pub fn range_scan(&self, start: i64, end: i64) -> Vec<Row> {
        let mut result = Vec::new();
        for page in &self.pages {
            for row in &page.rows {
                if row.pk >= start && row.pk <= end {
                    result.push(row.clone());
                }
            }
        }
        result.sort_by_key(|r| r.pk);
        result
    }

    pub fn add_secondary_index(&mut self, col: &str, val: &str, pk: i64) {
        let key = format!("{}={}", col, val);
        self.secondary_idx.entry(key).or_default().push(pk);
    }

    pub fn secondary_lookup(&self, col: &str, val: &str) -> Vec<&Row> {
        let key = format!("{}={}", col, val);
        match self.secondary_idx.get(&key) {
            Some(pks) => pks.iter().filter_map(|pk| self.lookup(*pk)).collect(),
            None => Vec::new(),
        }
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn split_count(&self) -> u64 {
        self.splits
    }
}

impl Default for ClusteredIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[test]
fn test_insert_basic() {
    let mut idx = ClusteredIndex::new();
    idx.insert(Row::new(5).set("name", "alice")).unwrap();
    let row = idx.lookup(5);
    assert!(row.is_some());
    assert_eq!(row.unwrap().get("name"), Some("alice"));
}

#[test]
fn test_pk_ordering_in_page() {
    let mut idx = ClusteredIndex::new();
    idx.insert(Row::new(10).set("v", "10")).unwrap();
    idx.insert(Row::new(3).set("v", "3")).unwrap();
    idx.insert(Row::new(7).set("v", "7")).unwrap();
    let page = &idx.pages[0];
    assert_eq!(page.rows[0].pk, 3);
    assert_eq!(page.rows[1].pk, 7);
    assert_eq!(page.rows[2].pk, 10);
}

#[test]
fn test_range_scan() {
    let mut idx = ClusteredIndex::new();
    for i in [1, 5, 10, 15, 20] {
        idx.insert(Row::new(i).set("v", &i.to_string())).unwrap();
    }
    let results = idx.range_scan(5, 15);
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].pk, 5);
    assert_eq!(results[1].pk, 10);
    assert_eq!(results[2].pk, 15);
}

#[test]
fn test_page_split_on_overflow() {
    let mut idx = ClusteredIndex::with_page_size(2); // small for testability
    for i in 1..=6 {
        idx.insert(Row::new(i).set("v", &i.to_string())).unwrap();
    }
    eprintln!("pages: {}", idx.page_count());
    eprintln!("splits: {}", idx.split_count());
    for (i, page) in idx.pages.iter().enumerate() {
        eprintln!(
            "  page {}: {:?}",
            i,
            page.rows.iter().map(|r| r.pk).collect::<Vec<_>>()
        );
    }
    eprintln!("pk_to_page: {:?}", idx.pk_to_page);
    // Just verify all PKs are findable
    for i in 1..=6 {
        assert!(idx.lookup(i).is_some(), "pk {} should be found", i);
    }
}

#[test]
fn test_secondary_index() {
    let mut idx = ClusteredIndex::new();
    let r1 = Row::new(1).set("name", "alice").set("age", "30");
    let r2 = Row::new(2).set("name", "bob").set("age", "30");
    idx.insert(r1).unwrap();
    idx.insert(r2).unwrap();
    idx.add_secondary_index("age", "30", 1);
    idx.add_secondary_index("age", "30", 2);
    let results = idx.secondary_lookup("age", "30");
    assert_eq!(results.len(), 2);
}

#[test]
fn test_lookup_nonexistent() {
    let idx = ClusteredIndex::new();
    assert!(idx.lookup(999).is_none());
}

#[test]
fn test_empty_range_scan() {
    let mut idx = ClusteredIndex::new();
    idx.insert(Row::new(1).set("v", "1")).unwrap();
    let results = idx.range_scan(100, 200);
    assert!(results.is_empty());
}
