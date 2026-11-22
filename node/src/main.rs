//! BYCONN Node — async entry point.
//!
//! Startup order:
//! 1. Open sled databases (chain + state) — single instance each, shared via Arc<Mutex<>>
//! 2. Insert genesis block if the chain is empty
//! 3. Start the JSON-RPC server (background task via jsonrpsee)
//! 4. Start the PoW miner (background tokio::spawn)
//! 5. Start the P2P swarm (background tokio::spawn — non-blocking)
//! 6. Park the main task until Ctrl-C

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
use tracing::{info, error, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── Logging ──────────────────────────────────────────────────────────────
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("logging setup failed");
    info!("BYCONN Node Starting...");

    // ── Database setup ───────────────────────────────────────────────────────
    let db_path = "node/db";
    std::fs::create_dir_all(db_path)?;

    // Both the chain and state databases are opened ONCE here and shared via
    // Arc<Mutex<>>. Previously, Blockchain::new() also opened its own internal
    // StateDb pointing to a different path, causing a coherency split — fixed.
    let state_db = Arc::new(Mutex::new(StateDb::new("node/db/state")?));
    let blockchain = Arc::new(Mutex::new(Blockchain::new("node/db/chain")?));

    // ── Genesis ──────────────────────────────────────────────────────────────
    {
        let chain = blockchain.lock().await;
        // Use is_empty() rather than latest_height() == 0, because height 0
        // is also returned when genesis exists — they are ambiguous otherwise.
        if chain.is_empty() {
            chain.append_block(&genesis::get_genesis_block())?;
            info!("Genesis block inserted");
        }
    }

    let tip = blockchain.lock().await.latest_height()?;
    info!(tip, "Blockchain head height");

    // ── Mempool & Sync ───────────────────────────────────────────────────────
    let mempool = Arc::new(Mutex::new(Mempool::new(1024)));
    let sync_manager = SyncManager::new(tip);
    info!(local_tip = sync_manager.local_tip, "Sync manager initialised");

    // ── Miner ────────────────────────────────────────────────────────────────
    let coinbase = std::env::var("BYCONN_COINBASE")
        .ok()
        .and_then(|s| hex::decode(s.trim_start_matches("0x")).ok())
        .filter(|b| b.len() == 20)
        .map(|b| Address::from_slice(&b))
        .unwrap_or_else(|| Address::from_low_u64_be(1));
    info!(coinbase = %format!("0x{:x}", coinbase), "Miner coinbase configured");

    let miner = miner::Miner::new(
        blockchain.clone(),
        mempool.clone(),
        state_db.clone(),
        coinbase,
    );

    // ── JSON-RPC Server ──────────────────────────────────────────────────────
    let _rpc_addr = rpc::start_server(
        blockchain.clone(),
        mempool.clone(),
        state_db.clone(),
    ).await?;

    // ── Background tasks ─────────────────────────────────────────────────────

    // Miner — runs the PoW loop in a dedicated async task
    tokio::spawn(async move {
        miner.start().await;
    });

    // P2P network — spawned as a non-blocking background task.
    // Previously this was `.await`-ed directly in main(), which caused main()
    // to block forever inside the infinite sleep loop in network::start_p2p().
    tokio::spawn(async {
        if let Err(e) = network::start_p2p().await {
            error!("P2P network error: {}", e);
        }
    });

    // ── Keep alive ───────────────────────────────────────────────────────────
    info!("BYCONN Node is running. Press Ctrl-C to stop.");
    // Park the main task indefinitely without consuming CPU.
    // `std::future::pending()` never resolves, keeping the Tokio runtime alive.
    std::future::pending::<()>().await;

    Ok(())
}
