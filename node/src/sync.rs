//! Chain synchronisation manager.
use tracing::{info, warn};

const BATCH_SIZE: u64 = 64;

#[derive(Debug)]
pub enum SyncError {
    NoPeers,
    BadResponse(String),
    StorageError(String),
}
impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncError::NoPeers => write!(f, "no peers available for sync"),
            SyncError::BadResponse(m) => write!(f, "bad peer response: {}", m),
            SyncError::StorageError(m) => write!(f, "storage error: {}", m),
        }
    }
}
impl std::error::Error for SyncError {}

#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub peer_id: String,
    pub tip_height: u64,
}

pub struct SyncManager {
    pub local_tip: u64,
    pub target_height: u64,
    peers: Vec<PeerInfo>,
}

impl SyncManager {
    pub fn new(local_tip: u64) -> Self {
        Self { local_tip, target_height: local_tip, peers: Vec::new() }
    }

    pub fn add_peer(&mut self, peer: PeerInfo) {
        info!(peer_id = %peer.peer_id, tip = peer.tip_height, "SyncManager: registered peer");
        if peer.tip_height > self.target_height {
            self.target_height = peer.tip_height;
            info!(target = self.target_height, "SyncManager: updated sync target");
        }
        self.peers.push(peer);
    }

    pub fn remove_peer(&mut self, peer_id: &str) {
        self.peers.retain(|p| p.peer_id != peer_id);
        self.target_height = self.peers.iter().map(|p| p.tip_height).max().unwrap_or(self.local_tip);
        warn!(peer_id, "SyncManager: peer removed");
    }

    pub fn is_synced(&self) -> bool { self.local_tip >= self.target_height }

    fn best_peer(&self) -> Option<&PeerInfo> {
        self.peers.iter().filter(|p| p.tip_height > self.local_tip).max_by_key(|p| p.tip_height)
    }

    fn next_range(&self) -> (u64, u64) {
        let start = self.local_tip + 1;
        let end = (start + BATCH_SIZE - 1).min(self.target_height);
        (start, end)
    }

    pub async fn tick(&mut self) -> Result<u64, SyncError> {
        if self.is_synced() { return Ok(0); }
        let peer = self.best_peer().ok_or(SyncError::NoPeers)?;
        let (start, end) = self.next_range();
        info!(peer_id = %peer.peer_id, range_start = start, range_end = end, "SyncManager: requesting block range");
        let applied = end - start + 1;
        self.local_tip = end;
        info!(new_tip = self.local_tip, "SyncManager: advanced local tip (stub)");
        Ok(applied)
    }

    pub async fn run_until_synced(&mut self) -> Result<(), SyncError> {
        if self.peers.is_empty() { return Err(SyncError::NoPeers); }
        info!(from = self.local_tip, to = self.target_height, "SyncManager: starting sync");
        while !self.is_synced() { self.tick().await?; }
        info!("SyncManager: chain fully synced to height {}", self.local_tip);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn peer(id: &str, tip: u64) -> PeerInfo { PeerInfo { peer_id: id.to_string(), tip_height: tip } }

    #[test]
    fn synced_when_no_peers() { assert!(SyncManager::new(10).is_synced()); }

    #[test]
    fn target_updates_on_add_peer() {
        let mut m = SyncManager::new(5);
        m.add_peer(peer("a", 100));
        assert_eq!(m.target_height, 100);
        m.add_peer(peer("b", 50));
        assert_eq!(m.target_height, 100);
    }

    #[test]
    fn remove_peer_recalculates_target() {
        let mut m = SyncManager::new(0);
        m.add_peer(peer("a", 200));
        m.add_peer(peer("b", 100));
        m.remove_peer("a");
        assert_eq!(m.target_height, 100);
    }

    #[test]
    fn next_range_respects_batch_size() {
        let mut m = SyncManager::new(0);
        m.add_peer(peer("a", 1000));
        let (s, e) = m.next_range();
        assert_eq!(s, 1);
        assert_eq!(e, BATCH_SIZE);
    }

    #[tokio::test]
    async fn tick_advances_tip() {
        let mut m = SyncManager::new(0);
        m.add_peer(peer("x", 10));
        let n = m.tick().await.unwrap();
        assert!(n > 0);
        assert!(m.local_tip > 0);
    }

    #[tokio::test]
    async fn run_until_synced_works() {
        let mut m = SyncManager::new(0);
        m.add_peer(peer("x", 200));
        m.run_until_synced().await.unwrap();
        assert!(m.is_synced());
    }
}
