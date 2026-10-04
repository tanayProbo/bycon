use crate::hash::{keccak256, Hash};
use std::collections::BTreeMap;

/// Merkle-Patricia Trie (Phase 2).
/// Currently a deterministic placeholder that computes a root hash
/// over sorted key-value pairs using a binary Merkle tree.
/// This fulfills the architectural requirement of a unique state root
/// until a full hex-prefix Ethereum MPT crate is integrated.
pub struct Mpt {
    items: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl Default for Mpt {
    fn default() -> Self {
        Self::new()
    }
}

impl Mpt {
    pub fn new() -> Self {
        Self { items: BTreeMap::new() }
    }

    pub fn insert(&mut self, key: &[u8], value: &[u8]) {
        self.items.insert(key.to_vec(), value.to_vec());
    }

    pub fn remove(&mut self, key: &[u8]) {
        self.items.remove(key);
    }

    pub fn root(&self) -> Hash {
        if self.items.is_empty() {
            // Standard empty trie root
            return crate::account::EMPTY_STORAGE_ROOT;
        }
        
        let mut encoded = Vec::new();
        for (k, v) in &self.items {
            let mut stream = rlp::RlpStream::new_list(2);
            stream.append(k);
            stream.append(v);
            encoded.push(stream.out().to_vec());
        }
        crate::trie::merkle_root(&encoded)
    }
}
