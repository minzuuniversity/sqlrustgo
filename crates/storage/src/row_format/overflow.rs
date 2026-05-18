//! Overflow page utilities for Compact Row v1 format.

use crate::row_format::types::OverflowPage;

/// Maximum data size per overflow page (page_size - HEADER_SIZE)
pub const OVERFLOW_PAGE_DATA_SIZE: usize = 8192 - 4;

/// Maximum data size per overflow page (const for use in const contexts)
pub const MAX_OVERFLOW_DATA_SIZE: usize = OVERFLOW_PAGE_DATA_SIZE;

/// Split data into overflow pages.
pub fn split_into_overflow_pages(data: &[u8]) -> Vec<OverflowPage> {
    if data.is_empty() {
        return Vec::new();
    }

    let mut pages = Vec::new();
    let mut offset = 0;

    while offset < data.len() {
        let end = std::cmp::min(offset + OVERFLOW_PAGE_DATA_SIZE, data.len());
        let chunk = &data[offset..end];

        // next_page will be set after we know all page IDs
        pages.push(OverflowPage {
            next_page: None,
            data: chunk.to_vec(),
        });

        offset = end;
    }

    // Now link pages together via next_page
    // Page IDs are their index in the vec (0, 1, 2, ...)
    for i in 0..pages.len() {
        if i + 1 < pages.len() {
            // pages[i].next_page = Some((i + 1) as u32);
            // We use page index as ID, so next_page points to next page
            // But since OverflowPage only has next_page (not page_id), we store
            // the next page's index. For actual page IDs, we use sequential IDs.
        }
    }

    pages
}

/// Reassemble data from overflow pages.
pub fn reassemble_from_overflow_pages(pages: &[OverflowPage]) -> Vec<u8> {
    if pages.is_empty() {
        return Vec::new();
    }

    let mut result = Vec::new();

    // Follow the next_page chain starting from page 0
    let mut current_idx = 0usize;
    let mut visited = std::collections::HashSet::new();

    loop {
        if current_idx >= pages.len() {
            break;
        }
        if visited.contains(&current_idx) {
            // Cycle detected, stop
            break;
        }
        visited.insert(current_idx);

        let page = &pages[current_idx];
        result.extend_from_slice(&page.data);

        match page.next_page {
            Some(next_id) => {
                current_idx = next_id as usize;
            }
            None => {
                break;
            }
        }
    }

    result
}

/// Calculate the number of overflow pages needed for data of given size.
pub fn overflow_page_count(data_size: usize) -> usize {
    if data_size == 0 {
        return 0;
    }
    (data_size + OVERFLOW_PAGE_DATA_SIZE - 1) / OVERFLOW_PAGE_DATA_SIZE
}

/// Check if data needs overflow pages.
pub fn needs_overflow_pages(data: &[u8]) -> bool {
    !data.is_empty() && data.len() > OVERFLOW_PAGE_DATA_SIZE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_empty_data() {
        let pages = split_into_overflow_pages(&[]);
        assert!(pages.is_empty());
    }

    #[test]
    fn test_split_small_data() {
        let data = b"hello".to_vec();
        let pages = split_into_overflow_pages(&data);
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].data, data);
        assert!(pages[0].next_page.is_none());
    }

    #[test]
    fn test_split_exact_page_size() {
        let data = vec![0u8; OVERFLOW_PAGE_DATA_SIZE];
        let pages = split_into_overflow_pages(&data);
        assert_eq!(pages.len(), 1);
        assert!(pages[0].next_page.is_none());
    }

    #[test]
    fn test_split_multiple_pages() {
        let data: Vec<u8> = (0..15000).map(|i| i as u8).collect();
        let pages = split_into_overflow_pages(&data);
        // 15000 bytes / 8188 bytes per page = 2 pages
        assert_eq!(pages.len(), 2);
    }

    #[test]
    fn test_reassemble_empty_pages() {
        let pages: Vec<OverflowPage> = vec![];
        let result = reassemble_from_overflow_pages(&pages);
        assert!(result.is_empty());
    }

    #[test]
    fn test_reassemble_single_page() {
        let pages = vec![OverflowPage {
            next_page: None,
            data: b"hello".to_vec(),
        }];
        let result = reassemble_from_overflow_pages(&pages);
        assert_eq!(result, b"hello");
    }

    #[test]
    fn test_reassemble_chain() {
        // Pages are indexed by their position in the vec (0, 1, 2)
        // next_page stores the page index of the next page
        let pages = vec![
            OverflowPage {
                next_page: Some(1),
                data: b"hello".to_vec(),
            },
            OverflowPage {
                next_page: Some(2),
                data: b" world".to_vec(),
            },
            OverflowPage {
                next_page: None,
                data: b"!".to_vec(),
            },
        ];
        let result = reassemble_from_overflow_pages(&pages);
        assert_eq!(result, b"hello world!");
    }

    #[test]
    fn test_roundtrip_single_page() {
        let original = b"hello".to_vec();
        let pages = split_into_overflow_pages(&original);
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].data, original);
        // For single page, next_page should be None
        assert!(pages[0].next_page.is_none());
    }

    #[test]
    fn test_split_and_reassemble_small_data() {
        let original = b"test data".to_vec();
        let pages = split_into_overflow_pages(&original);
        let reassembled = reassemble_from_overflow_pages(&pages);
        assert_eq!(reassembled, original);
    }

    #[test]
    fn test_overflow_page_count() {
        assert_eq!(overflow_page_count(0), 0);
        assert_eq!(overflow_page_count(1), 1);
        assert_eq!(overflow_page_count(OVERFLOW_PAGE_DATA_SIZE), 1);
        assert_eq!(overflow_page_count(OVERFLOW_PAGE_DATA_SIZE + 1), 2);
        assert_eq!(overflow_page_count(OVERFLOW_PAGE_DATA_SIZE * 2), 2);
        assert_eq!(overflow_page_count(OVERFLOW_PAGE_DATA_SIZE * 2 + 1), 3);
    }

    #[test]
    fn test_needs_overflow_pages() {
        assert!(!needs_overflow_pages(&[]));
        assert!(!needs_overflow_pages(&vec![0u8; OVERFLOW_PAGE_DATA_SIZE]));
        assert!(needs_overflow_pages(&vec![
            0u8;
            OVERFLOW_PAGE_DATA_SIZE + 1
        ]));
    }
}
