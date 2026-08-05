mod chain;
mod sync;
mod network;

use protocol::genesis;
use chain::Blockchain;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    info!("BYCON Node Starting...");

    let genesis_block = genesis::get_genesis_block();
    let blockchain = Blockchain::new(genesis_block);

    info!("Initialized Blockchain with Genesis Block: {:?}", blockchain.head().hash());
    
    // Stub for starting the network
    network::start_p2p().await?;

    Ok(())
}
