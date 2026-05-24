//! LRU Embedding Cache for Ollama API responses
//!
//! Avoids redundant Ollama API calls for repeated query texts.
//! Uses text as key with LRU eviction and configurable TTL.

use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Cached embedding with timestamp for TTL checking and last-access for LRU
#[derive(Clone)]
struct CachedEntry {
    embedding: Vec<f32>,
    created_at: Instant,
    last_accessed: Instant,
}

impl CachedEntry {
    fn new(embedding: Vec<f32>) -> Self {
        let now = Instant::now();
        Self {
            embedding,
            created_at: now,
            last_accessed: now,
        }
    }
}

/// Thread-safe embedding cache with TTL and capacity limits
#[derive(Clone)]
pub struct EmbeddingCache {
    inner: Arc<RwLock<HashMap<String, CachedEntry>>>,
    ttl_seconds: u64,
    max_capacity: usize,
}

impl EmbeddingCache {
    /// Create a new cache with the given TTL and maximum capacity
    pub fn new(ttl_seconds: u64, max_capacity: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::with_capacity(max_capacity))),
            ttl_seconds,
            max_capacity,
        }
    }

    /// Get a cached embedding if valid (not expired), refreshes LRU timestamp
    pub async fn get(&self, key: &str) -> Option<Vec<f32>> {
        let mut cache = self.inner.write().await;
        cache.get_mut(key).and_then(|entry| {
            if entry.created_at.elapsed() < Duration::from_secs(self.ttl_seconds) {
                entry.last_accessed = Instant::now();
                Some(entry.embedding.clone())
            } else {
                None
            }
        })
    }

    /// Insert an embedding into the cache
    pub async fn insert(&self, key: String, embedding: Vec<f32>) {
        // Check if we need to evict first (before acquiring write lock)
        let need_evict = {
            let cache = self.inner.read().await;
            cache.len() >= self.max_capacity
        };

        if need_evict {
            // Evict LRU entry (oldest last_accessed)
            let lru_key = {
                let cache = self.inner.read().await;
                cache
                    .iter()
                    .min_by_key(|(_, entry)| entry.last_accessed)
                    .map(|(k, _)| k.clone())
            };
            if let Some(key_to_remove) = lru_key {
                self.inner.write().await.remove(&key_to_remove);
            }
        }

        // Insert new entry
        self.inner.write().await.insert(key, CachedEntry::new(embedding));
    }

    /// Get current cache size
    #[allow(dead_code)]
    pub async fn len(&self) -> usize {
        self.inner.read().await.len()
    }

    /// Check if cache is empty
    #[allow(dead_code)]
    pub async fn is_empty(&self) -> bool {
        self.inner.read().await.is_empty()
    }

    /// Clear the cache
    #[allow(dead_code)]
    pub async fn clear(&self) {
        self.inner.write().await.clear();
    }
}

/// Embedding store that wraps Ollama API with LRU caching
#[derive(Clone)]
pub struct CachedEmbeddingStore {
    cache: EmbeddingCache,
    ollama_url: String,
    model: String,
}

impl CachedEmbeddingStore {
    /// Create a new cached embedding store (24h TTL, 10000 entries)
    pub fn new(ollama_url: String, model: String) -> Self {
        Self {
            cache: EmbeddingCache::new(86400, 10000),
            ollama_url,
            model,
        }
    }

    /// Create with custom cache config
    #[allow(dead_code)]
    pub fn with_cache(ollama_url: String, model: String, ttl_seconds: u64, max_capacity: usize) -> Self {
        Self {
            cache: EmbeddingCache::new(ttl_seconds, max_capacity),
            ollama_url,
            model,
        }
    }

    /// Get embedding (cache-first, Ollama fallback)
    pub async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        // Try cache first
        if let Some(embedding) = self.cache.get(text).await {
            return Ok(embedding);
        }

        // Cache miss: call Ollama
        let embedding = self.call_ollama(text).await?;

        // Store in cache
        self.cache.insert(text.to_string(), embedding.clone()).await;

        Ok(embedding)
    }

    /// Direct Ollama API call
    async fn call_ollama(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let client = Client::new();
        let response = client
            .post(format!("{}/api/embeddings", self.ollama_url))
            .json(&serde_json::json!({
                "model": self.model,
                "prompt": text
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("Ollama API error: {}", response.status());
        }

        #[derive(Deserialize)]
        struct EmbedResponse {
            embedding: Vec<f32>,
        }

        let result: EmbedResponse = response.json().await?;
        Ok(result.embedding)
    }

    /// Get cache statistics
    #[allow(dead_code)]
    pub async fn cache_stats(&self) -> CacheStats {
        CacheStats {
            size: self.cache.len().await,
            max_capacity: self.cache.max_capacity,
            ttl_seconds: self.cache.ttl_seconds,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size: usize,
    pub max_capacity: usize,
    pub ttl_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_basic() {
        let cache = EmbeddingCache::new(3600, 100);

        // Initially empty
        assert!(cache.get("test text").await.is_none());

        // Insert
        cache.insert("test text".to_string(), vec![0.1, 0.2, 0.3]).await;

        // Should be found
        let result = cache.get("test text").await;
        assert!(result.is_some());
        assert_eq!(result.unwrap(), vec![0.1, 0.2, 0.3]);
    }

    #[tokio::test]
    async fn test_cache_ttl() {
        let cache = EmbeddingCache::new(1, 100); // 1 second TTL

        cache.insert("ttl test".to_string(), vec![1.0]).await;
        assert!(cache.get("ttl test").await.is_some());

        tokio::time::sleep(Duration::from_secs(2)).await;
        assert!(cache.get("ttl test").await.is_none());
    }

    #[tokio::test]
    async fn test_cache_eviction() {
        let cache = EmbeddingCache::new(3600, 2);

        cache.insert("key1".to_string(), vec![1.0]).await;
        cache.insert("key2".to_string(), vec![2.0]).await;
        assert_eq!(cache.len().await, 2);

        // Adding third key should evict LRU entry (key1 was inserted first)
        cache.insert("key3".to_string(), vec![3.0]).await;
        assert_eq!(cache.len().await, 2);

        // key1 should be evicted (it was the LRU entry when key3 was inserted)
        assert!(cache.get("key1").await.is_none());
        assert!(cache.get("key2").await.is_some());
        assert!(cache.get("key3").await.is_some());

        // Verify LRU is based on insertion order when no get() refreshes
        let cache2 = EmbeddingCache::new(3600, 2);
        cache2.insert("a".to_string(), vec![1.0]).await;
        cache2.insert("b".to_string(), vec![2.0]).await;
        // Access a to make it MRU
        let _ = cache2.get("a").await;
        // Now b is LRU — adding c should evict b
        cache2.insert("c".to_string(), vec![3.0]).await;
        assert_eq!(cache2.len().await, 2);
        assert!(cache2.get("b").await.is_none(), "b should be evicted (LRU after a was accessed)");
        assert!(cache2.get("a").await.is_some());
        assert!(cache2.get("c").await.is_some());
    }
}
