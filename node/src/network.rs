//! P2P network service scaffolding (libp2p placeholder).
use std::collections::HashMap;
use std::error::Error;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub enum NetworkEvent {
    PeerConnected(String),
    PeerDisconnected(String),
    BlockReceived { from: String, data: Vec<u8> },
    TransactionReceived { from: String, data: Vec<u8> },
}

#[derive(Debug, Clone)]
pub struct PeerEntry {
    pub peer_id: String,
    pub tip_height: u64,
    pub protocol_version: String,
}

#[derive(Default)]
pub struct PeerRegistry {
    pub peers: HashMap<String, PeerEntry>,
}

impl PeerRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn upsert(&mut self, entry: PeerEntry) {
        info!(peer_id = %entry.peer_id, tip = entry.tip_height, "PeerRegistry: upsert peer");
        self.peers.insert(entry.peer_id.clone(), entry);
    }

    pub fn remove(&mut self, peer_id: &str) -> Option<PeerEntry> {
        let r = self.peers.remove(peer_id);
        if r.is_some() { warn!(peer_id, "PeerRegistry: peer removed"); }
        r
    }

    pub fn count(&self) -> usize { self.peers.len() }

    pub fn best_peer(&self) -> Option<&PeerEntry> {
        self.peers.values().max_by_key(|p| p.tip_height)
    }
}

pub struct NetworkService {
    pub local_peer_id: String,
    pub peers: PeerRegistry,
}

impl NetworkService {
    pub fn new() -> Self {
        Self { local_peer_id: "bycon-local-peer".to_string(), peers: PeerRegistry::new() }
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn Error>> {
        info!("NetworkService: local peer id = {}", self.local_peer_id);
        info!("NetworkService: initialising Kademlia DHT...");
        info!("NetworkService: initialising Gossipsub (blocks, txs)...");
        self.event_loop().await;
        Ok(())
    }

    async fn event_loop(&mut self) {
        info!("NetworkService: event loop started");
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            info!(peers = self.peers.count(), "NetworkService: heartbeat");
        }
    }

    pub fn broadcast_block(&self, block_data: &[u8]) {
        info!(bytes = block_data.len(), peers = self.peers.count(), "NetworkService: broadcasting block");
    }

    pub fn broadcast_transaction(&self, tx_data: &[u8]) {
        info!(bytes = tx_data.len(), peers = self.peers.count(), "NetworkService: broadcasting transaction");
    }

    pub fn send_to_peer(&self, peer_id: &str, msg: &[u8]) {
        if let Some(peer) = self.peers.peers.get(peer_id) {
            info!(peer_id = %peer.peer_id, bytes = msg.len(), "NetworkService: sending direct message");
        } else {
            warn!(peer_id, "NetworkService: send_to_peer - peer not found");
        }
    }
}

pub async fn start_p2p() -> Result<(), Box<dyn Error>> {
    let mut service = NetworkService::new();
    service.start().await
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entry(id: &str, tip: u64) -> PeerEntry {
        PeerEntry { peer_id: id.to_string(), tip_height: tip, protocol_version: "bycon/1".to_string() }
    }

    #[test]
    fn upsert_and_count() {
        let mut r = PeerRegistry::new();
        r.upsert(entry("a", 10));
        r.upsert(entry("b", 20));
        assert_eq!(r.count(), 2);
    }

    #[test]
    fn upsert_updates_existing() {
        let mut r = PeerRegistry::new();
        r.upsert(entry("a", 10));
        r.upsert(entry("a", 99));
        assert_eq!(r.count(), 1);
        assert_eq!(r.best_peer().unwrap().tip_height, 99);
    }

    #[test]
    fn remove_peer() {
        let mut r = PeerRegistry::new();
        r.upsert(entry("a", 10));
        assert!(r.remove("a").is_some());
        assert_eq!(r.count(), 0);
    }

    #[test]
    fn best_peer_highest_tip() {
        let mut r = PeerRegistry::new();
        r.upsert(entry("low", 5));
        r.upsert(entry("high", 1000));
        assert_eq!(r.best_peer().unwrap().peer_id, "high");
    }
}
