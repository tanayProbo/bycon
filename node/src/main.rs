mod chain;
mod sync;
mod network;
mod state;
mod mempool;
mod consensus;
mod execution;

use protocol::genesis;
use chain::Blockchain;
use state::StateDb;
use mempool::Mempool;
use sync::{SyncManager, PeerInfo};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    info!("BYCON Node Starting...");

    // Initialise persistent storage
    let db_path = "node/db";
    let _ = std::fs::create_dir_all(db_path);
    let state_db = StateDb::new(db_path)?;
    let blockchain = Blockchain::new(db_path)?;

    // Insert genesis block if chain is empty
    if blockchain.latest_height()? == 0 {
        let genesis_block = genesis::get_genesis_block();
        blockchain.append_block(&genesis_block)?;
        info!("Genesis block inserted");
    }

    let tip = blockchain.latest_height()?;
    info!("Blockchain head height: {}", tip);

    // Initialise mempool with a reasonable default size
    let _mempool = Mempool::new(1024);

    // Initialize the Consensus Engine
    let genesis_validators = vec![];
    let mut consensus_engine = consensus::ConsensusEngine::new(genesis_validators);
    info!("Consensus engine ready at height {}", consensus_engine.current_height);

    // Initialize the Sync Manager
    let mut sync_manager = SyncManager::new(tip);
    // In production: peers arrive from the network layer via discovery.
    // For now we log the initial sync state.
    info!(
        local_tip = sync_manager.local_tip,
        target = sync_manager.target_height,
        "Sync manager initialised (waiting for peers)"
    );

    // Start the P2P network event loop (blocks until shutdown)
    network::start_p2p().await?;

    Ok(())
}
