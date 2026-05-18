//! Evidence chain visualization

#[derive(Debug, Clone)]
pub struct EvidenceChainViz {
    pub chain_id: String,
    pub entries: Vec<ChainEntry>,
    pub is_verified: bool,
}

#[derive(Debug, Clone)]
pub struct ChainEntry {
    pub entry_type: String,
    pub description: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub signer: Option<String>,
    pub hash: String,
}

impl EvidenceChainViz {
    pub fn new(chain_id: &str) -> Self {
        Self {
            chain_id: chain_id.to_string(),
            entries: Vec::new(),
            is_verified: false,
        }
    }

    pub fn add_entry(&mut self, entry: ChainEntry) {
        self.entries.push(entry);
    }

    pub fn verify_chain(&mut self) -> bool {
        self.is_verified = self.entries.windows(2).all(|w| {
            w[0].timestamp <= w[1].timestamp
        });
        self.is_verified
    }
}
