use gmp_storage::{StorageConfig, StorageEngine};

fn main() {
    let config = StorageConfig {
        data_dir: "./data".to_string(),
        db_path: "data/gmp.db".to_string(),
        embedding_dim: 768,
        use_sqlrustgo_storage: true,
    };
    let storage = StorageEngine::new(config).expect("Failed to create storage");
    
    // Check chunk count
    let chunks = storage.get_chunk_ids(5);
    println!("Chunk IDs in cache: {:?}", chunks);
    println!("Chunk cache size from get_chunk_ids(5): {}", chunks.len());
    
    // Validate
    let v = storage.validate().expect("validate failed");
    println!("Validation: docs={}, chunks={}, fts_token_count={}", v.documents, v.chunks, v.fts_token_count);
    println!("is_valid={}", v.is_valid);
    if !v.errors.is_empty() {
        for e in &v.errors { println!("  ERROR: {}", e); }
    }
    
    // Try a simple search
    let results = storage.search_fts("内审", 5).expect("search failed");
    println!("\nsearch_fts('内审') returned {} results", results.len());
    for r in &results {
        println!("  chunk_id={}, doc_id={}, title={}", r.chunk_id, r.doc_id, r.title);
    }
    
    // Try LIKE search
    let results2 = storage.search_like("审计", 5).expect("search_like failed");
    println!("\nsearch_like('审计') returned {} results", results2.len());
    for r in &results2 {
        println!("  chunk_id={}, doc_id={}, score={}", r.chunk_id, r.doc_id, r.score);
    }
}
