mod chain;
mod sync;
mod network;
mod state;
mod mempool;

use protocol::genesis;
use chain::Blockchain;
use state::StateDb;
use mempool::Mempool;
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
    // Ensure directory exists (sled will create if missing)
    let _ = std::fs::create_dir_all(db_path);
    let state_db = StateDb::new(db_path)?;
    let blockchain = Blockchain::new(db_path)?;

    // Insert genesis block if chain is empty
    if blockchain.latest_height()? == 0 {
        let genesis_block = genesis::get_genesis_block();
        blockchain.append_block(&genesis_block)?;
        info!("Genesis block inserted");
    }

    info!("Initialized Blockchain with head hash: {:?}", blockchain.head().hash());

    // Initialise mempool with a reasonable default size
    let mut mempool = Mempool::new(1024);
    // Placeholder: you could preload pending transactions here

    // Stub for starting the network
    network::start_p2p().await?;

    Ok(())
}
