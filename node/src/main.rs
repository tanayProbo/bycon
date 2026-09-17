mod chain;
mod sync;
mod network;
mod state;
mod mempool;
mod consensus;
mod execution;
mod evm;
mod miner;
mod rpc;

use std::sync::Arc;
use tokio::sync::Mutex;
use protocol::genesis;
use protocol::transaction::Address;
use chain::Blockchain;
use state::StateDb;
use mempool::Mempool;
use sync::SyncManager;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder().with_max_level(Level::INFO).finish();
    tracing::subscriber::set_global_default(subscriber).expect("logging fail");
    info!("BYCON Node Starting...");

    let db_path = "node/db";
    let _ = std::fs::create_dir_all(db_path);
    
    // We share the Blockchain and StateDb via Arc<Mutex>
    let state_db = Arc::new(Mutex::new(StateDb::new("node/db/state")?));
    let blockchain = Arc::new(Mutex::new(Blockchain::new("node/db/chain")?));

    {
        let chain = blockchain.lock().await;
        if chain.latest_height()? == 0 {
            chain.append_block(&genesis::get_genesis_block())?;
            info!("Genesis block inserted");
        }
    }

    let tip = blockchain.lock().await.latest_height()?;
    info!("Blockchain head height: {}", tip);

    let mempool = Arc::new(Mutex::new(Mempool::new(1024)));
    let mut sync_manager = SyncManager::new(tip);
    info!(local_tip = sync_manager.local_tip, "Sync manager initialised");

    let miner = miner::Miner::new(
        blockchain.clone(),
        mempool.clone(),
        state_db.clone(),
        Address::from_low_u64_be(1) // dummy coinbase
    );

    // Start RPC Server
    let _rpc_addr = rpc::start_server(
        blockchain.clone(),
        mempool.clone(),
        state_db.clone()
    ).await?;

    // Run miner in background
    tokio::spawn(async move {
        miner.start().await;
    });

    network::start_p2p().await?;
    Ok(())
}
