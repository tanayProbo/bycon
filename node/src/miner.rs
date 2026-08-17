use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::info;

use protocol::block::{Block, BlockHeader};
use protocol::hash::Hash;
use protocol::transaction::Address;

use crate::chain::Blockchain;
use crate::mempool::Mempool;
use crate::state::StateDb;
use crate::execution::ExecutionEngine;

pub struct Miner {
    chain: Arc<Mutex<Blockchain>>,
    mempool: Arc<Mutex<Mempool>>,
    state_db: Arc<Mutex<StateDb>>,
    coinbase: Address,
}

impl Miner {
    pub fn new(
        chain: Arc<Mutex<Blockchain>>,
        mempool: Arc<Mutex<Mempool>>,
        state_db: Arc<Mutex<StateDb>>,
        coinbase: Address,
    ) -> Self {
        Self { chain, mempool, state_db, coinbase }
    }

    pub async fn start(&self) {
        info!("Miner task started");
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            self.mine_block().await;
        }
    }

    async fn mine_block(&self) {
        let chain = self.chain.lock().await;
        let mut mempool = self.mempool.lock().await;
        let state_db = self.state_db.lock().await;
        
        let latest_height = chain.latest_height().unwrap_or(0);
        let parent_hash = chain.head().map(|b| b.hash()).unwrap_or(Hash::zero());

        let mut txs = Vec::new();
        let mut engine = ExecutionEngine::new(&state_db);

        // Very basic execution loop (in a real node, we'd limit by block gas)
        let mut cumulative_gas = 0;
        let ordered_txs = mempool.ordered_txs().into_iter().cloned().collect::<Vec<_>>();
        
        for tx in ordered_txs {
            if let Ok(receipt) = engine.execute_transaction(&tx, self.coinbase, cumulative_gas) {
                if receipt.success {
                    cumulative_gas += receipt.gas_used;
                    txs.push(tx.clone());
                    mempool.remove(&tx.recover_sender().unwrap_or_default(), tx.nonce);
                }
            }
        }

        let new_block = Block {
            header: BlockHeader {
                parent_hash,
                number: latest_height + 1,
                state_root: Hash::zero(), // Placeholder
                transactions_root: Hash::zero(), // Placeholder
                receipts_root: Hash::zero(), // Placeholder
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                proposer: self.coinbase,
            },
            transactions: txs,
        };

        if let Ok(()) = chain.append_block(&new_block) {
            info!("Mined block {} with {} txs", new_block.header.number, new_block.transactions.len());
        }
    }
}
